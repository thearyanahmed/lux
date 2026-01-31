use colored::Colorize;

use crate::VERSION;

const SYM_STEP: &str = "▸";
const SYM_PASS: &str = "✓";
const SYM_FAIL: &str = "✗";
const SYM_WARN: &str = "!";
const SYM_SKIP: &str = "○";
const INDENT: &str = "";

/// UI output for running validators, matching HeroTerminal visual style
pub struct RunUI {
    task_name: String,
    total_validators: usize,
}

impl RunUI {
    pub fn new(task_name: &str, validator_count: usize) -> Self {
        Self {
            task_name: task_name.to_string(),
            total_validators: validator_count,
        }
    }

    /// print version header: "projectlighthouse CLI v1.2.0"
    pub fn header(&self) {
        println!("{}projectlighthouse CLI v{}", INDENT, VERSION.dimmed());
    }

    /// print progress step: "▸ Compiling project..."
    pub fn step(&self, msg: &str) {
        println!("{}{} {}", INDENT, SYM_STEP.blue(), msg);
    }

    pub fn blank_line(&self) {
        println!();
    }

    /// print passing test: "✓ server listening on port 4221"
    pub fn test_pass(&self, name: &str) {
        println!("{}{} {}", INDENT, SYM_PASS.green(), name);
    }

    /// print failing test with optional detail
    pub fn test_fail(&self, name: &str, detail: Option<&str>) {
        println!("{}{} {}", INDENT, SYM_FAIL.red(), name.red());

        if let Some(d) = detail {
            if !d.is_empty() && d != name {
                // truncate long error messages
                let display = if d.len() > 600 {
                    format!("{}...", &d[..600])
                } else {
                    d.to_string()
                };

                for line in display.lines() {
                    println!("{}  {}", INDENT, line.red());
                }
            }
        }
    }

    /// print success summary: "PASSED  All 3 tests passed!"
    pub fn summary_pass(&self, total: usize) {
        println!(
            "{}{}  All {} tests passed!",
            INDENT,
            "PASSED".green().bold(),
            total
        );
    }

    /// print failure summary: "FAILED  1 of 3 tests failed"
    pub fn summary_fail(&self, passed: usize, total: usize) {
        let failed = total - passed;
        println!(
            "{}{}  {} of {} tests failed",
            INDENT,
            "FAILED".red().bold(),
            failed,
            total
        );
    }

    /// print hint: "Hint: Check that your response includes the comma."
    pub fn hint(&self, text: &str) {
        println!();
        println!("{}{} {}", INDENT, "Hint:".dimmed(), text);
    }

    /// print task separator for multi-task validation
    pub fn task_separator(&self, current: usize, total: usize, task_slug: &str) {
        println!(
            "{}━━━ Task {}/{}: {} ━━━",
            INDENT,
            current,
            total,
            task_slug.bold()
        );
    }

    /// print points earned on success
    pub fn points_earned(&self, points: i32) {
        if points > 0 {
            println!("{}{}", INDENT, format!("+{} XP", points).green().bold());
        }
    }

    /// accessor for task name
    pub fn task_name(&self) -> &str {
        &self.task_name
    }

    /// accessor for validator count
    pub fn validator_count(&self) -> usize {
        self.total_validators
    }
}

/// General-purpose UI output functions (no instance needed)
pub struct UI;

impl UI {
    /// print version header
    pub fn header() {
        println!("{}projectlighthouse CLI v{}", INDENT, VERSION.dimmed());
    }

    /// print a section header
    pub fn section(name: &str) {
        println!();
        println!("{}{}", INDENT, name.bold());
    }

    /// print progress step: "▸ message..."
    pub fn step(msg: &str) {
        println!("{}{} {}", INDENT, SYM_STEP.blue(), msg);
    }

    /// print success item: "✓ name  detail"
    pub fn ok(name: &str, detail: Option<&str>) {
        match detail {
            Some(d) => println!(
                "{}{} {}  {}",
                INDENT,
                SYM_PASS.green(),
                name.green(),
                d.dimmed()
            ),
            None => println!("{}{} {}", INDENT, SYM_PASS.green(), name.green()),
        }
    }

    /// print warning item: "! name  detail"
    pub fn warn(name: &str, detail: Option<&str>) {
        match detail {
            Some(d) => {
                println!(
                    "{}{} {}  {}",
                    INDENT,
                    SYM_WARN.yellow(),
                    name.yellow(),
                    d.dimmed()
                )
            }
            None => println!("{}{} {}", INDENT, SYM_WARN.yellow(), name.yellow()),
        }
    }

    /// print error item: "✗ name  detail"
    pub fn error(name: &str, detail: Option<&str>) {
        match detail {
            Some(d) => println!(
                "{}{} {}  {}",
                INDENT,
                SYM_FAIL.red(),
                name.red(),
                d.dimmed()
            ),
            None => println!("{}{} {}", INDENT, SYM_FAIL.red(), name.red()),
        }
    }

    /// print skipped/not installed item: "○ name"
    pub fn skip(name: &str, detail: Option<&str>) {
        match detail {
            Some(d) => {
                println!(
                    "{}{} {}  {}",
                    INDENT,
                    SYM_SKIP.dimmed(),
                    name.dimmed(),
                    d.dimmed()
                )
            }
            None => println!("{}{} {}", INDENT, SYM_SKIP.dimmed(), name.dimmed()),
        }
    }

    /// print info line with indent
    pub fn info(msg: &str) {
        println!("{}{}", INDENT, msg);
    }

    /// print dimmed info line
    pub fn note(msg: &str) {
        println!("{}{}", INDENT, msg.dimmed());
    }

    /// print success message
    pub fn success(msg: &str) {
        println!("{}{} {}", INDENT, SYM_PASS.green(), msg.green());
    }

    /// blank line
    pub fn blank() {
        println!();
    }

    /// print key-value pair with alignment
    pub fn kv(key: &str, value: &str) {
        println!("{}{}: {}", INDENT, key.dimmed(), value);
    }

    /// print key-value pair with right-aligned key (for status displays)
    pub fn kv_aligned(key: &str, value: &str, width: usize) {
        println!(
            "{}{:>width$}: {}",
            INDENT,
            key.dimmed(),
            value,
            width = width
        );
    }

    /// print labeled status (like [UNLOCKED], [AVAILABLE])
    pub fn status_unlocked(index: usize, text: &str, cost: i32) {
        println!(
            "{}#{} {} {}",
            INDENT,
            index.to_string().dimmed(),
            "unlocked".green(),
            format!("-{} XP", cost).dimmed()
        );
        println!("{}    {}", INDENT, text);
    }

    pub fn status_available(index: usize, cost: i32, unlock_cmd: &str) {
        println!(
            "{}#{} {} {}",
            INDENT,
            index.to_string().dimmed(),
            "available".yellow(),
            format!("-{} XP", cost).dimmed()
        );
        println!("{}    {} {}", INDENT, "unlock:".dimmed(), unlock_cmd);
    }

    pub fn status_locked(index: usize, cost: i32) {
        println!(
            "{}#{} {} {}",
            INDENT,
            index.to_string().dimmed(),
            "locked".dimmed(),
            format!("-{} XP", cost).dimmed()
        );
        println!("{}    {}", INDENT, "not yet available".dimmed());
    }

    /// print separator line
    pub fn separator() {
        println!("{}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━", INDENT);
    }
}
