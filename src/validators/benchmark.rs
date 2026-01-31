//! Validators for output matching and benchmarking
//!
//! Used by performance-focused challenges like 1BRC

use crate::config::Config;
use crate::state::LabState;
use crate::tasks::TestCase;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Instant;
use tokio::process::Command;

/// get workspace from active lab state
fn get_workspace() -> PathBuf {
    let config = match Config::load() {
        Ok(c) => c,
        Err(_) => return std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    };
    if !config.has_auth_token() {
        return std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    }
    let state = match LabState::load(config.expose_token()) {
        Ok(s) => s,
        Err(_) => return std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    };
    match state.get_active() {
        Some(lab) => PathBuf::from(&lab.workspace),
        None => std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    }
}

/// run a command and capture output
async fn run_command(cmd_str: &str, workspace: &PathBuf) -> Result<(String, String, u64), String> {
    let parts: Vec<&str> = cmd_str.split_whitespace().collect();
    if parts.is_empty() {
        return Err("empty command".to_string());
    }

    let program = parts[0];
    let args = &parts[1..];

    let start = Instant::now();

    let output = Command::new(program)
        .args(args)
        .current_dir(workspace)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("failed to run '{}': {}", cmd_str, e))?;

    let elapsed_ms = start.elapsed().as_millis() as u64;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        let err_preview = stderr.lines().take(5).collect::<Vec<_>>().join("\n");
        return Err(format!(
            "command exited with status {}: {}",
            output.status.code().unwrap_or(-1),
            err_preview
        ));
    }

    Ok((stdout, stderr, elapsed_ms))
}

/// read expected output from file
fn read_expected(path: &str, workspace: &PathBuf) -> Result<String, String> {
    let full_path = if path.starts_with('/') {
        PathBuf::from(path)
    } else {
        workspace.join(path)
    };

    std::fs::read_to_string(&full_path)
        .map_err(|e| format!("failed to read expected file '{}': {}", full_path.display(), e))
}

/// normalize output for comparison (trim trailing whitespace)
fn normalize(s: &str) -> String {
    s.trim_end().to_string()
}

/// Validator: run command and compare output to expected file
pub struct OutputMatchValidator {
    pub command: String,
    pub expected_file: String,
}

impl OutputMatchValidator {
    pub fn new(command: impl Into<String>, expected_file: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            expected_file: expected_file.into(),
        }
    }

    pub async fn validate(&self) -> Result<TestCase, String> {
        let workspace = get_workspace();

        let expected = read_expected(&self.expected_file, &workspace)?;
        let (stdout, _stderr, elapsed_ms) = run_command(&self.command, &workspace).await?;

        let actual = normalize(&stdout);
        let expected = normalize(&expected);

        let result = if actual == expected {
            Ok(format!(
                "output matches ({}ms)",
                elapsed_ms
            ))
        } else {
            // show diff preview
            let actual_lines: Vec<&str> = actual.lines().collect();
            let expected_lines: Vec<&str> = expected.lines().collect();

            let mut diff_msg = String::new();
            for (i, (a, e)) in actual_lines.iter().zip(expected_lines.iter()).enumerate() {
                if a != e {
                    diff_msg = format!(
                        "line {}: expected '{}', got '{}'",
                        i + 1,
                        truncate(e, 50),
                        truncate(a, 50)
                    );
                    break;
                }
            }

            if diff_msg.is_empty() {
                if actual_lines.len() != expected_lines.len() {
                    diff_msg = format!(
                        "line count mismatch: expected {}, got {}",
                        expected_lines.len(),
                        actual_lines.len()
                    );
                }
            }

            Err(format!("output mismatch: {}", diff_msg))
        };

        Ok(TestCase {
            name: format!("output matches {}", self.expected_file),
            result,
        })
    }
}

/// Validator: run command, compare output, and verify time limit
pub struct BenchmarkValidator {
    pub command: String,
    pub expected_file: String,
    pub max_time_ms: u64,
}

impl BenchmarkValidator {
    pub fn new(
        command: impl Into<String>,
        expected_file: impl Into<String>,
        max_time_ms: u64,
    ) -> Self {
        Self {
            command: command.into(),
            expected_file: expected_file.into(),
            max_time_ms,
        }
    }

    pub async fn validate(&self) -> Result<TestCase, String> {
        let workspace = get_workspace();

        let expected = read_expected(&self.expected_file, &workspace)?;
        let (stdout, _stderr, elapsed_ms) = run_command(&self.command, &workspace).await?;

        let actual = normalize(&stdout);
        let expected = normalize(&expected);

        // first check correctness
        if actual != expected {
            let actual_lines: Vec<&str> = actual.lines().collect();
            let expected_lines: Vec<&str> = expected.lines().collect();

            let mut diff_msg = String::new();
            for (i, (a, e)) in actual_lines.iter().zip(expected_lines.iter()).enumerate() {
                if a != e {
                    diff_msg = format!(
                        "line {}: expected '{}', got '{}'",
                        i + 1,
                        truncate(e, 50),
                        truncate(a, 50)
                    );
                    break;
                }
            }

            if diff_msg.is_empty() && actual_lines.len() != expected_lines.len() {
                diff_msg = format!(
                    "line count mismatch: expected {}, got {}",
                    expected_lines.len(),
                    actual_lines.len()
                );
            }

            return Ok(TestCase {
                name: format!("benchmark < {}ms", self.max_time_ms),
                result: Err(format!("output mismatch: {}", diff_msg)),
            });
        }

        // then check timing
        let result = if elapsed_ms <= self.max_time_ms {
            Ok(format!(
                "completed in {}ms (limit: {}ms)",
                elapsed_ms, self.max_time_ms
            ))
        } else {
            Err(format!(
                "too slow: {}ms (limit: {}ms)",
                elapsed_ms, self.max_time_ms
            ))
        };

        Ok(TestCase {
            name: format!("benchmark < {}ms", self.max_time_ms),
            result,
        })
    }
}

/// truncate string for display
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize() {
        assert_eq!(normalize("hello\n"), "hello");
        assert_eq!(normalize("hello\n\n\n"), "hello");
        assert_eq!(normalize("hello  \n"), "hello");
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 5), "hello...");
    }
}
