use crate::executor::context::{Context, ExecutionMode, HostFacts};
use crate::executor::error::ExecutionError;
use crate::executor::expect::{evaluate_expectations, evaluate_input, process_captures};
use crate::executor::probes::execute_probe;
use crate::transpiler::ir::*;
use crate::transpiler::validate::topological_sort;
use log::debug;
use std::collections::{HashMap, HashSet};
use std::time::Instant;

pub struct Engine {
    pub ctx: Context,
    task_slug: Option<String>,
}

impl Engine {
    pub fn new(ctx: Context) -> Self {
        Self {
            ctx,
            task_slug: None,
        }
    }

    pub fn with_task(mut self, slug: &str) -> Self {
        self.task_slug = Some(slug.to_string());
        self
    }

    pub async fn execute(&mut self, bp: &Blueprint) -> Result<BlueprintResult, ExecutionError> {
        let start = Instant::now();

        let phase_order =
            topological_sort(&bp.phases).map_err(|e| ExecutionError::new(e.to_string()))?;

        let phase_map: HashMap<&str, &Phase> =
            bp.phases.iter().map(|p| (p.name.as_str(), p)).collect();

        let mut phase_results = Vec::new();
        let mut failed_phases: HashSet<String> = HashSet::new();
        let mut overall_status = Status::Passed;

        for phase_name in &phase_order {
            let phase = match phase_map.get(phase_name.as_str()) {
                Some(p) => p,
                None => continue,
            };

            if phase.depends_on.iter().any(|d| failed_phases.contains(d)) {
                failed_phases.insert(phase_name.clone());
                phase_results.push(PhaseResult {
                    name: phase_name.clone(),
                    slug: phase.meta.slug.clone(),
                    status: Status::Skipped,
                    steps: Vec::new(),
                    duration_ms: 0,
                });
                continue;
            }

            let phase_result = self.execute_phase(phase).await?;
            if phase_result.status == Status::Failed {
                failed_phases.insert(phase_name.clone());
                overall_status = Status::Failed;
            }
            phase_results.push(phase_result);
        }

        Ok(BlueprintResult {
            name: bp.name.clone(),
            status: overall_status,
            phases: phase_results,
            duration_ms: start.elapsed().as_millis() as u64,
            captured: self.ctx.variables.clone(),
            input_provided: self.ctx.user_inputs.clone(),
        })
    }

    async fn execute_phase(&mut self, phase: &Phase) -> Result<PhaseResult, ExecutionError> {
        let start = Instant::now();
        let mut step_results = Vec::new();
        let mut phase_status = Status::Passed;

        debug!("phase: \"{}\" (slug: {:?})", phase.name, phase.meta.slug);

        // phase-level slug filtering: skip entire phase if slug doesn't match
        if let Some(ref slug) = self.task_slug {
            if let Some(ref phase_slug) = phase.meta.slug {
                if phase_slug != slug {
                    debug!(
                        "  skipping phase (slug mismatch: want={}, got={})",
                        slug, phase_slug
                    );
                    return Ok(PhaseResult {
                        name: phase.name.clone(),
                        slug: phase.meta.slug.clone(),
                        status: Status::Skipped,
                        steps: Vec::new(),
                        duration_ms: 0,
                    });
                }
            }
        }

        for step in &phase.steps {
            // step-level slug filtering: only when phase has no slug (backward compat)
            if let Some(ref slug) = self.task_slug {
                if phase.meta.slug.is_none() {
                    if let Some(ref step_slug) = step.meta.slug {
                        if step_slug != slug {
                            continue;
                        }
                    }
                }
            }

            let has_input = !step.inputs.is_empty();
            match self.ctx.mode {
                ExecutionMode::Validate if has_input => {
                    step_results.push(skipped_step(&step.name, None));
                    continue;
                }
                ExecutionMode::Result if !has_input => {
                    step_results.push(skipped_step(&step.name, None));
                    continue;
                }
                _ => {}
            }

            if step.requires.iter().any(|var| !self.ctx.has_variable(var)) {
                step_results.push(skipped_step(&step.name, None));
                continue;
            }

            // host-conditional steps skip with an explanation rather than
            // failing — the parity claim only stays credible if we say why
            if let Some(reqs) = &step.requires_env {
                if let Some(reason) = unmet_requirement(reqs, self.ctx.facts.as_ref()) {
                    step_results.push(skipped_step(&step.name, Some(reason)));
                    continue;
                }
            }

            if has_input && self.ctx.mode == ExecutionMode::Result {
                for input_decl in &step.inputs {
                    if self.ctx.get_user_input(&input_decl.name).is_none() {
                        let slug_display = step.meta.slug.as_deref().unwrap_or(&step.name);
                        return Err(ExecutionError::new(format!(
                            "missing required flag: --{}\n  Run: luxctl result --task {} --{} <your value>",
                            input_decl.name, slug_display, input_decl.name
                        )));
                    }
                }
            }

            let step_result = self.execute_step_with_retry(step).await?;
            if step_result.status != Status::Passed && step_result.status != Status::Skipped {
                phase_status = Status::Failed;
            }
            step_results.push(step_result);
        }

        Ok(PhaseResult {
            name: phase.name.clone(),
            slug: phase.meta.slug.clone(),
            status: phase_status,
            steps: step_results,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn execute_step_with_retry(&mut self, step: &Step) -> Result<StepResult, ExecutionError> {
        let max_attempts = step.retry.as_ref().map_or(1, |r| r.max_attempts);
        let delay = step.retry.as_ref().map(|r| r.delay);
        let mut last_result = None;

        for attempt in 0..max_attempts {
            let result = self.execute_step_once(step, attempt).await?;
            if result.status == Status::Passed {
                return Ok(result);
            }
            last_result = Some(result);
            if attempt + 1 < max_attempts {
                if let Some(d) = delay {
                    tokio::time::sleep(d).await;
                }
            }
        }

        Ok(last_result.unwrap_or_else(|| StepResult {
            name: step.name.clone(),
            status: Status::Failed,
            expectations: Vec::new(),
            captures: Vec::new(),
            input_matched: None,
            duration_ms: 0,
            retry_count: max_attempts,
            skip_reason: None,
            output: None,
        }))
    }

    async fn execute_step_once(
        &mut self,
        step: &Step,
        attempt: u32,
    ) -> Result<StepResult, ExecutionError> {
        let start = Instant::now();
        let timeout = step.timeout.or(Some(self.ctx.config.timeout));

        debug!(
            "  step: \"{}\" (timeout: {:?}, attempt: {})",
            step.name, timeout, attempt
        );
        debug!("    probe: {:?}", step.probe);

        // timeout is passed into exec probes so the child process is killed on expiry.
        // for non-exec probes, the outer tokio::time::timeout still applies.
        let probe_result = if let Some(t) = timeout {
            match tokio::time::timeout(t, execute_probe(&step.probe, &self.ctx, timeout)).await {
                Ok(result) => result?,
                Err(_) => {
                    return Ok(StepResult {
                        name: step.name.clone(),
                        status: Status::Error("probe timed out".to_string()),
                        expectations: Vec::new(),
                        captures: Vec::new(),
                        input_matched: None,
                        duration_ms: start.elapsed().as_millis() as u64,
                        retry_count: attempt,
                        skip_reason: None,
                        output: None,
                    });
                }
            }
        } else {
            execute_probe(&step.probe, &self.ctx, timeout).await?
        };

        debug!(
            "    probe result: exit={:?}, duration={}ms, stdout_len={}",
            probe_result.fields.get("exit"),
            probe_result.duration_ms,
            probe_result.raw_stdout.as_ref().map_or(0, |s| s.len()),
        );

        let has_input = !step.inputs.is_empty();
        let (expect_results, input_matched, captured) = if has_input
            && self.ctx.mode == ExecutionMode::Result
        {
            let input = &step.inputs[0];
            let user_value = self
                .ctx
                .get_user_input(&input.name)
                .unwrap_or("")
                .to_string();
            let (matched, exp_results) = evaluate_input(
                &input.name,
                &user_value,
                &step.expectations,
                &probe_result,
                &self.ctx,
            );
            let captured = process_captures(&step.captures, &probe_result, &mut self.ctx);
            (exp_results, Some(matched), captured)
        } else {
            let exp_results = evaluate_expectations(&step.expectations, &probe_result, &self.ctx);
            let captured = process_captures(&step.captures, &probe_result, &mut self.ctx);
            (exp_results, None, captured)
        };

        let all_passed = expect_results.iter().all(|r| r.status == Status::Passed);
        let input_ok = input_matched.unwrap_or(true);
        let failed = !(all_passed && input_ok);

        // only carried on failure — on success it is noise, and probe output can
        // be large
        let output = if failed {
            probe_output(&probe_result)
        } else {
            None
        };

        Ok(StepResult {
            name: step.name.clone(),
            status: if failed {
                Status::Failed
            } else {
                Status::Passed
            },
            expectations: expect_results,
            captures: captured,
            input_matched,
            duration_ms: start.elapsed().as_millis() as u64,
            retry_count: attempt,
            skip_reason: None,
            output,
        })
    }
}

fn skipped_step(name: &str, reason: Option<String>) -> StepResult {
    StepResult {
        name: name.to_string(),
        status: Status::Skipped,
        expectations: Vec::new(),
        captures: Vec::new(),
        input_matched: None,
        duration_ms: 0,
        retry_count: 0,
        skip_reason: reason,
        output: None,
    }
}

/// stdout from a failed probe, falling back to stderr, trimmed and capped.
fn probe_output(result: &ProbeResult) -> Option<String> {
    const MAX_LEN: usize = 800;

    let pick = |key: &str| match result.fields.get(key) {
        Some(Value::String(s)) if !s.trim().is_empty() => Some(s.trim().to_string()),
        _ => None,
    };

    let text = pick("stdout").or_else(|| pick("stderr"))?;

    Some(if text.len() > MAX_LEN {
        format!("{}\n… output truncated", &text[..MAX_LEN])
    } else {
        text
    })
}

/// check declared requirements against what the backend actually provides.
///
/// returns the reason the step cannot run, or `None` when it can. an absent
/// `HostFacts` means nobody ran preflight — local runs — so host-conditional
/// requirements are the only ones we can honestly judge.
fn unmet_requirement(reqs: &Requirements, facts: Option<&HostFacts>) -> Option<String> {
    let facts = match facts {
        Some(f) => f,
        None => {
            let host = reqs.host.as_deref()?;
            return (host != std::env::consts::OS)
                .then(|| format!("requires a {host} host, this is {}", std::env::consts::OS));
        }
    };

    if let Some(host) = &reqs.host {
        if host != &facts.host_os {
            return Some(format!("requires a {host} host, this is {}", facts.host_os));
        }
    }
    if let Some((maj, min, patch)) = reqs.kernel {
        match facts.kernel {
            Some(actual) if actual >= (maj, min, patch) => {}
            Some((a, b, c)) => {
                return Some(format!(
                    "needs kernel >={maj}.{min}.{patch}, {} has {a}.{b}.{c}",
                    facts.backend
                ))
            }
            None => return Some(format!("kernel version unknown on {}", facts.backend)),
        }
    }
    if reqs.cgroup_v2 && !facts.cgroup_v2 {
        return Some(format!(
            "needs a cgroup v2 unified hierarchy, {} does not provide one",
            facts.backend
        ));
    }
    if reqs.userns && !facts.userns {
        return Some(format!(
            "needs user namespaces, {} lacks them",
            facts.backend
        ));
    }
    if reqs.btf && !facts.btf {
        return Some(format!(
            "needs BTF at /sys/kernel/btf/vmlinux, {} does not expose it",
            facts.backend
        ));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::grammar::parse;
    use crate::transpiler::resolve::transpile;

    async fn run_bp(input: &str, mode: ExecutionMode) -> BlueprintResult {
        let ast = parse(input).unwrap_or_else(|e| panic!("parse: {e}"));
        let bp = transpile(&ast).unwrap_or_else(|e| panic!("transpile: {e}"));
        let ctx = Context::new(bp.config.clone(), mode);
        let mut engine = Engine::new(ctx);
        engine
            .execute(&bp)
            .await
            .unwrap_or_else(|e| panic!("execute: {e}"))
    }

    #[tokio::test]
    async fn test_failed_step_keeps_probe_output() {
        // a diff probe writes the mismatch to stdout; without it the learner only
        // ever sees the exit code
        let r = run_bp(
            r#"
blueprint "T" {
    phase "t" {
        step "mismatch" {
            probe exec sh -c "echo 'line differs'; exit 1"
            expect { exit: 0 }
        }
    }
}
"#,
            ExecutionMode::Validate,
        )
        .await;

        let step = &r.phases[0].steps[0];
        assert_eq!(step.status, Status::Failed);
        assert_eq!(step.output.as_deref(), Some("line differs"));
    }

    #[tokio::test]
    async fn test_passing_step_carries_no_output() {
        let r = run_bp(
            r#"
blueprint "T" {
    phase "t" {
        step "fine" {
            probe exec echo noise
            expect { exit: 0 }
        }
    }
}
"#,
            ExecutionMode::Validate,
        )
        .await;

        let step = &r.phases[0].steps[0];
        assert_eq!(step.status, Status::Passed);
        assert!(step.output.is_none());
    }

    #[tokio::test]
    async fn test_failed_step_falls_back_to_stderr() {
        let r = run_bp(
            r#"
blueprint "T" {
    phase "t" {
        step "stderr only" {
            probe exec sh -c "echo 'boom' >&2; exit 1"
            expect { exit: 0 }
        }
    }
}
"#,
            ExecutionMode::Validate,
        )
        .await;

        let step = &r.phases[0].steps[0];
        assert_eq!(step.status, Status::Failed);
        assert_eq!(step.output.as_deref(), Some("boom"));
    }

    #[tokio::test]
    async fn test_exec_probe_echo() {
        let r = run_bp(
            r#"
blueprint "T" {
    phase "t" {
        step "echo" {
            probe exec echo hello
            expect { stdout: "hello" exit: 0 }
        }
    }
}
"#,
            ExecutionMode::Validate,
        )
        .await;
        assert_eq!(r.status, Status::Passed);
    }

    #[tokio::test]
    async fn test_exec_exit_code() {
        let r = run_bp(
            r#"
blueprint "T" {
    phase "t" {
        step "exit" {
            probe exec sh -c "exit 42"
            expect { exit: 42 }
        }
    }
}
"#,
            ExecutionMode::Validate,
        )
        .await;
        assert_eq!(r.status, Status::Passed);
    }

    #[tokio::test]
    async fn test_phase_dependency_skipping() {
        let r = run_bp(
            r#"
blueprint "T" {
    phase "first" {
        step "fail" {
            probe exec sh -c "exit 1"
            expect { exit: 0 }
        }
    }
    phase "second" {
        depends-on: "first"
        step "skip" {
            probe exec echo "nope"
            expect { exit: 0 }
        }
    }
}
"#,
            ExecutionMode::Validate,
        )
        .await;
        assert_eq!(r.status, Status::Failed);
        assert_eq!(r.phases[1].status, Status::Skipped);
    }

    #[tokio::test]
    async fn test_validate_skips_input_steps() {
        let r = run_bp(
            r#"
blueprint "T" {
    phase "t" {
        step "probe only" {
            probe exec echo "hello"
            expect { exit: 0 }
        }
        step "with input" {
            input { answer: string }
            probe exec echo "world"
            expect { stdout: "world" }
        }
    }
}
"#,
            ExecutionMode::Validate,
        )
        .await;
        assert_eq!(r.status, Status::Passed);
        assert_eq!(r.phases[0].steps[1].status, Status::Skipped);
    }

    #[tokio::test]
    async fn test_capture_flows() {
        let r = run_bp(
            r#"
blueprint "T" {
    phase "t" {
        step "capture" {
            probe exec echo "abc123"
            expect {
                exit: 0
                capture stdout as $my_var
            }
        }
        step "use" {
            requires: $my_var
            probe exec echo $my_var
            expect { stdout: "abc123" exit: 0 }
        }
    }
}
"#,
            ExecutionMode::Validate,
        )
        .await;
        assert_eq!(r.status, Status::Passed);
        assert!(r.captured.contains_key("$my_var"));
    }

    #[tokio::test]
    async fn test_requires_skips_when_missing() {
        let r = run_bp(
            r#"
blueprint "T" {
    phase "t" {
        step "needs var" {
            requires: $nonexistent
            probe exec echo "hello"
            expect { exit: 0 }
        }
    }
}
"#,
            ExecutionMode::Validate,
        )
        .await;
        assert_eq!(r.phases[0].steps[0].status, Status::Skipped);
        assert!(
            r.phases[0].steps[0].skip_reason.is_none(),
            "a missing variable is not a host-capability skip"
        );
    }

    fn facts(backend: &str) -> HostFacts {
        HostFacts {
            host_os: "linux".to_string(),
            backend: backend.to_string(),
            kernel: Some((5, 15, 0)),
            cgroup_v2: true,
            userns: true,
            btf: false,
        }
    }

    async fn run_with_facts(input: &str, facts: Option<HostFacts>) -> BlueprintResult {
        let ast = parse(input).unwrap_or_else(|e| panic!("parse: {e}"));
        let bp = transpile(&ast).unwrap_or_else(|e| panic!("transpile: {e}"));
        let mut ctx = Context::new(bp.config.clone(), ExecutionMode::Validate);
        if let Some(f) = facts {
            ctx = ctx.with_facts(f);
        }
        Engine::new(ctx)
            .execute(&bp)
            .await
            .unwrap_or_else(|e| panic!("execute: {e}"))
    }

    const NEEDS_NEW_KERNEL: &str = r#"
blueprint "T" {
    phase "t" {
        step "ebpf" {
            requires { kernel: ">=6.1"  btf: true }
            probe exec echo "hello"
            expect { exit: 0 }
        }
        step "ordinary" {
            probe exec echo "hello"
            expect { exit: 0 }
        }
    }
}
"#;

    /// an unmet host requirement skips *with an explanation*, and does not drag
    /// the rest of the run down with it
    #[tokio::test]
    async fn test_requires_env_skips_with_reason() {
        let r = run_with_facts(NEEDS_NEW_KERNEL, Some(facts("colima"))).await;

        let skipped = &r.phases[0].steps[0];
        assert_eq!(skipped.status, Status::Skipped);
        let reason = skipped
            .skip_reason
            .as_deref()
            .unwrap_or_else(|| panic!("skip carried no reason"));
        assert!(
            reason.contains("6.1"),
            "reason should name the requirement: {reason}"
        );
        assert!(
            reason.contains("colima"),
            "reason should name the backend: {reason}"
        );

        assert_eq!(r.phases[0].steps[1].status, Status::Passed);
        assert_eq!(r.status, Status::Passed, "an honest skip is not a failure");
    }

    #[tokio::test]
    async fn test_requires_env_runs_when_satisfied() {
        let mut f = facts("lima");
        f.kernel = Some((6, 6, 0));
        f.btf = true;

        let r = run_with_facts(NEEDS_NEW_KERNEL, Some(f)).await;
        assert_eq!(r.phases[0].steps[0].status, Status::Passed);
        assert_eq!(r.status, Status::Passed);
    }

    /// with no preflight (a plain `local|` run) the only requirement we can
    /// honestly judge is the host one
    #[tokio::test]
    async fn test_host_conditional_skip_without_facts() {
        let bp = format!(
            r#"
blueprint "T" {{
    phase "t" {{
        step "elsewhere" {{
            requires {{ host: "definitely-not-{}" }}
            probe exec echo "hello"
            expect {{ exit: 0 }}
        }}
        step "here" {{
            requires {{ host: "{}" }}
            probe exec echo "hello"
            expect {{ exit: 0 }}
        }}
    }}
}}
"#,
            std::env::consts::OS,
            std::env::consts::OS
        );

        let r = run_with_facts(&bp, None).await;
        assert_eq!(r.phases[0].steps[0].status, Status::Skipped);
        assert!(r.phases[0].steps[0].skip_reason.is_some());
        assert_eq!(r.phases[0].steps[1].status, Status::Passed);
    }

    #[tokio::test]
    async fn test_contains_operator() {
        let r = run_bp(
            r#"
blueprint "T" {
    phase "t" {
        step "check" {
            probe exec echo "hello world foo"
            expect {
                stdout contains: "world"
                exit: 0
            }
        }
    }
}
"#,
            ExecutionMode::Validate,
        )
        .await;
        assert_eq!(r.status, Status::Passed);
    }

    #[tokio::test]
    async fn test_regex_match() {
        let r = run_bp(
            r#"
blueprint "T" {
    phase "t" {
        step "regex" {
            probe exec echo "abc123"
            expect {
                stdout matches: /^[a-z]+\d+$/
                exit: 0
            }
        }
    }
}
"#,
            ExecutionMode::Validate,
        )
        .await;
        assert_eq!(r.status, Status::Passed);
    }

    #[tokio::test]
    async fn test_multiple_phases() {
        let r = run_bp(
            r#"
blueprint "T" {
    phase "a" {
        step "s1" { probe exec echo "1" expect { exit: 0 } }
    }
    phase "b" {
        depends-on: "a"
        step "s2" { probe exec echo "2" expect { exit: 0 } }
    }
    phase "c" {
        depends-on: "b"
        step "s3" { probe exec echo "3" expect { exit: 0 } }
    }
}
"#,
            ExecutionMode::Validate,
        )
        .await;
        assert_eq!(r.status, Status::Passed);
        assert_eq!(r.phases.len(), 3);
    }

    #[tokio::test]
    async fn test_file_probe() {
        let tmp = tempfile::NamedTempFile::new().unwrap_or_else(|e| panic!("{e}"));
        let path = tmp.path().to_string_lossy().to_string();
        let bp_str = format!(
            r#"
blueprint "T" {{
    phase "t" {{
        step "file" {{
            probe file {path}
            expect {{ exists: true }}
        }}
    }}
}}
"#
        );
        let r = run_bp(&bp_str, ExecutionMode::Validate).await;
        assert_eq!(r.status, Status::Passed);
    }
}
