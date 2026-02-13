use crate::transpiler::ir::*;
use colored::Colorize;

pub struct CliReporter;

impl CliReporter {
    pub fn print_result(result: &BlueprintResult) {
        println!();

        for phase in &result.phases {
            if phase.status == Status::Skipped {
                println!("  {} {}", "⊘".dimmed(), phase.name.dimmed());
                continue;
            }

            for step in &phase.steps {
                match &step.status {
                    Status::Passed => {
                        println!("  {} {}", "✓".green(), step.name);
                    }
                    Status::Failed => {
                        println!("  {} {}", "✗".red(), step.name.red());
                        for exp in &step.expectations {
                            if exp.status == Status::Failed {
                                if let Some(msg) = &exp.message {
                                    println!("    {}", msg.dimmed());
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

                // show captured values
                for (var, val) in &step.captures {
                    println!("    {} = {}", var.cyan(), val);
                }
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
    pub fn print_step_result(step: &StepResult) {
        match &step.status {
            Status::Passed => {
                println!("  {} {}", "✓".green(), step.name);
            }
            Status::Failed => {
                println!("  {} {}", "✗".red(), step.name.red());
                for exp in &step.expectations {
                    if exp.status == Status::Failed {
                        if let Some(msg) = &exp.message {
                            println!("    {}", msg.dimmed());
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
        CliReporter::print_result(&result);
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
        CliReporter::print_result(&result);
    }
}
