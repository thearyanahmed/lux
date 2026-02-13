use crate::executor::context::{Context, ExecutionMode};
use crate::executor::error::ExecutionError;
use crate::executor::expect::{evaluate_expectations, evaluate_input, process_captures};
use crate::executor::probes::execute_probe;
use crate::transpiler::ir::*;
use crate::transpiler::validate::topological_sort;
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
                    status: Status::Skipped,
                    steps: Vec::new(),
                    duration_ms: 0,
                });
                continue;
            }

            let phase_result = self.execute_phase(phase).await?;
            if phase_result.status != Status::Passed {
                failed_phases.insert(phase_name.clone());
                if overall_status == Status::Passed {
                    overall_status = phase_result.status.clone();
                }
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

        for step in &phase.steps {
            if let Some(ref slug) = self.task_slug {
                if let Some(ref step_slug) = step.meta.slug {
                    if step_slug != slug {
                        continue;
                    }
                }
            }

            let has_input = !step.inputs.is_empty();
            match self.ctx.mode {
                ExecutionMode::Validate if has_input => {
                    step_results.push(skipped_step(&step.name));
                    continue;
                }
                ExecutionMode::Result if !has_input => {
                    step_results.push(skipped_step(&step.name));
                    continue;
                }
                _ => {}
            }

            if step.requires.iter().any(|var| !self.ctx.has_variable(var)) {
                step_results.push(skipped_step(&step.name));
                continue;
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
        }))
    }

    async fn execute_step_once(
        &mut self,
        step: &Step,
        attempt: u32,
    ) -> Result<StepResult, ExecutionError> {
        let start = Instant::now();
        let timeout = step.timeout.or(Some(self.ctx.config.timeout));

        let probe_result = if let Some(t) = timeout {
            match tokio::time::timeout(t, execute_probe(&step.probe, &self.ctx)).await {
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
                    });
                }
            }
        } else {
            execute_probe(&step.probe, &self.ctx).await?
        };

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

        Ok(StepResult {
            name: step.name.clone(),
            status: if all_passed && input_ok {
                Status::Passed
            } else {
                Status::Failed
            },
            expectations: expect_results,
            captures: captured,
            input_matched,
            duration_ms: start.elapsed().as_millis() as u64,
            retry_count: attempt,
        })
    }
}

fn skipped_step(name: &str) -> StepResult {
    StepResult {
        name: name.to_string(),
        status: Status::Skipped,
        expectations: Vec::new(),
        captures: Vec::new(),
        input_matched: None,
        duration_ms: 0,
        retry_count: 0,
    }
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
