use crate::transpiler::ir::*;
use colored::Colorize;

pub struct CliReporter;

const DETAIL_LINE_WIDTH: usize = 60;

impl CliReporter {
    pub fn print_result(result: &BlueprintResult, detailed: bool) {
        println!();

        for phase in &result.phases {
            if phase.status == Status::Skipped {
                println!("  {} {}", "⊘".dimmed(), phase.name.dimmed());
                continue;
            }

            for step in &phase.steps {
                Self::render_step(step, detailed);
            }

            if detailed {
                let label = format!("phase \"{}\"", phase.name);
                let suffix = format!("{}ms", phase.duration_ms);
                let gap = right_align_gap(2 + label.len(), suffix.len());
                println!();
                println!("  {}{}{}", label.dimmed(), gap, suffix.dimmed());
            }
        }

        println!();

        match &result.status {
            Status::Passed => {
                println!("  {}", "All checks passed.".green().bold());
            }
            Status::Failed => {
                let total = count_steps(result);
                let passed = count_passed_steps(result);
                println!(
                    "  {} ({}/{} steps passed)",
                    "Some checks failed.".red().bold(),
                    passed,
                    total
                );
            }
            Status::Skipped => {
                println!("  {}", "All steps skipped.".dimmed());
            }
            Status::Error(msg) => {
                println!("  {} {}", "Error:".red().bold(), msg);
            }
        }

        println!("  Duration: {}ms", result.duration_ms);
        println!();
    }

    /// print a single step result (for --task targeting)
    pub fn print_step_result(step: &StepResult, detailed: bool) {
        Self::render_step(step, detailed);
    }

    fn render_step(step: &StepResult, detailed: bool) {
        match &step.status {
            Status::Passed => {
                if detailed {
                    let suffix = step_suffix(step);
                    let gap = right_align_gap(4 + step.name.len(), suffix.len());
                    println!("  {} {}{}{}", "✓".green(), step.name, gap, suffix.dimmed());
                    print_expectations(&step.expectations);
                } else {
                    println!("  {} {}", "✓".green(), step.name);
                }
            }
            Status::Failed => {
                if detailed {
                    let suffix = step_suffix(step);
                    let gap = right_align_gap(4 + step.name.len(), suffix.len());
                    println!(
                        "  {} {}{}{}",
                        "✗".red(),
                        step.name.red(),
                        gap,
                        suffix.dimmed()
                    );
                    print_expectations(&step.expectations);
                } else {
                    println!("  {} {}", "✗".red(), step.name.red());
                    for exp in &step.expectations {
                        if exp.status == Status::Failed {
                            if let Some(msg) = &exp.message {
                                println!("    {}", msg.dimmed());
                            }
                        }
                    }
                }
            }
            Status::Skipped => {
                println!("  {} {}", "⊘".dimmed(), step.name.dimmed());
            }
            Status::Error(msg) => {
                println!("  {} {} — {}", "!".yellow(), step.name.yellow(), msg);
            }
        }

        for (var, val) in &step.captures {
            println!("    {} = {}", var.cyan(), val);
        }
    }

    /// format a missing input error message
    pub fn print_missing_input(input_name: &str, task_slug: &str) {
        println!();
        println!("  {} missing required flag: --{}", "✗".red(), input_name);
        println!(
            "    Run: luxctl result --task {} --{} <your value>",
            task_slug, input_name
        );
        println!();
    }

    /// format a "correct" message for result mode
    pub fn print_correct() {
        println!();
        println!("  {}", "Correct.".green().bold());
        println!();
    }

    /// format an "incorrect" message for result mode
    pub fn print_incorrect() {
        println!();
        println!("  {}", "Incorrect.".red().bold());
        println!();
    }
}

fn right_align_gap(left_len: usize, right_len: usize) -> String {
    let total = left_len + right_len;
    if total >= DETAIL_LINE_WIDTH {
        "  ".to_string()
    } else {
        " ".repeat(DETAIL_LINE_WIDTH - total)
    }
}

fn step_suffix(step: &StepResult) -> String {
    if step.retry_count > 0 {
        format!("{}ms  retry {}", step.duration_ms, step.retry_count)
    } else {
        format!("{}ms", step.duration_ms)
    }
}

fn format_op_symbol(op: &Op) -> &str {
    match op {
        Op::Eq => "=",
        Op::Contains => "contains",
        Op::StartsWith => "starts_with",
        Op::Matches => "~",
        Op::MatchesFile => "matches_file",
        Op::Present => "present",
        Op::Absent => "absent",
        Op::Gt => ">",
        Op::Lt => "<",
        Op::Gte => ">=",
        Op::Lte => "<=",
        Op::All => "all",
    }
}

fn format_expectation_desc(exp: &ExpectResult) -> String {
    let op = format_op_symbol(&exp.op);
    let actual_display = exp
        .actual
        .as_ref()
        .map(|v| v.to_string())
        .unwrap_or_else(|| "?".to_string());

    match exp.op {
        Op::Present | Op::Absent => format!("{} {}", exp.field, op),
        Op::Contains | Op::StartsWith | Op::Matches | Op::MatchesFile | Op::All => {
            format!(
                "{} {} \"{}\" (actual: \"{}\")",
                exp.field, op, exp.expected_display, actual_display
            )
        }
        _ => {
            format!(
                "{} {} {} (expected: {})",
                exp.field, op, actual_display, exp.expected_display
            )
        }
    }
}

fn print_expectations(expectations: &[ExpectResult]) {
    for exp in expectations {
        let desc = format_expectation_desc(exp);
        let symbol = match &exp.status {
            Status::Passed => "✓".green(),
            Status::Failed => "✗".red(),
            _ => "·".dimmed(),
        };
        let gap = right_align_gap(4 + desc.len(), 1);
        println!("    {}{}{}", desc.dimmed(), gap, symbol);
    }
}

fn count_steps(result: &BlueprintResult) -> usize {
    result
        .phases
        .iter()
        .flat_map(|p| &p.steps)
        .filter(|s| s.status != Status::Skipped)
        .count()
}

fn count_passed_steps(result: &BlueprintResult) -> usize {
    result
        .phases
        .iter()
        .flat_map(|p| &p.steps)
        .filter(|s| s.status == Status::Passed)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_passed_result() -> BlueprintResult {
        BlueprintResult {
            name: "Test".to_string(),
            status: Status::Passed,
            phases: vec![PhaseResult {
                name: "test".to_string(),
                status: Status::Passed,
                steps: vec![StepResult {
                    name: "step 1".to_string(),
                    status: Status::Passed,
                    expectations: Vec::new(),
                    captures: vec![("$var".to_string(), Value::String("abc".into()))],
                    input_matched: None,
                    duration_ms: 50,
                    retry_count: 0,
                }],
                duration_ms: 50,
            }],
            duration_ms: 50,
            captured: std::collections::HashMap::new(),
            input_provided: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn test_count_steps() {
        let result = make_passed_result();
        assert_eq!(count_steps(&result), 1);
        assert_eq!(count_passed_steps(&result), 1);
    }

    // printing tests are smoke tests — just ensure they don't panic
    #[test]
    fn test_print_passed_result() {
        let result = make_passed_result();
        CliReporter::print_result(&result, false);
    }

    #[test]
    fn test_print_failed_result() {
        let result = BlueprintResult {
            name: "Test".to_string(),
            status: Status::Failed,
            phases: vec![PhaseResult {
                name: "test".to_string(),
                status: Status::Failed,
                steps: vec![StepResult {
                    name: "step 1".to_string(),
                    status: Status::Failed,
                    expectations: vec![ExpectResult {
                        field: "status".to_string(),
                        op: Op::Eq,
                        status: Status::Failed,
                        actual: Some(Value::Int(404)),
                        expected_display: "200".to_string(),
                        message: Some("expected status == 200, got 404".to_string()),
                    }],
                    captures: Vec::new(),
                    input_matched: None,
                    duration_ms: 100,
                    retry_count: 0,
                }],
                duration_ms: 100,
            }],
            duration_ms: 100,
            captured: std::collections::HashMap::new(),
            input_provided: std::collections::HashMap::new(),
        };
        CliReporter::print_result(&result, false);
    }
}
