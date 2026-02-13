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

fn current_dir_fallback() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// get workspace from active lab state
fn get_workspace() -> PathBuf {
    let config = match Config::load() {
        Ok(c) => c,
        Err(_) => return current_dir_fallback(),
    };
    if !config.has_auth_token() {
        return current_dir_fallback();
    }
    let state = match LabState::load(config.expose_token()) {
        Ok(s) => s,
        Err(_) => return current_dir_fallback(),
    };
    match state.get_active() {
        Some(lab) => PathBuf::from(&lab.workspace),
        None => current_dir_fallback(),
    }
}

/// run a command and capture output
/// note: uses split_whitespace for parsing, so paths with spaces are not supported
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

    std::fs::read_to_string(&full_path).map_err(|e| {
        format!(
            "failed to read expected file '{}': {}",
            full_path.display(),
            e
        )
    })
}

/// normalize output for comparison (trim trailing whitespace)
fn normalize(s: &str) -> String {
    s.trim_end().to_string()
}

/// generate a diff preview message for mismatched output
fn diff_preview(actual: &str, expected: &str) -> String {
    let actual_lines: Vec<&str> = actual.lines().collect();
    let expected_lines: Vec<&str> = expected.lines().collect();

    for (i, (a, e)) in actual_lines.iter().zip(expected_lines.iter()).enumerate() {
        if a != e {
            return format!(
                "line {}: expected '{}', got '{}'",
                i + 1,
                truncate(e, 50),
                truncate(a, 50)
            );
        }
    }

    if actual_lines.len() != expected_lines.len() {
        return format!(
            "line count mismatch: expected {}, got {}",
            expected_lines.len(),
            actual_lines.len()
        );
    }

    String::new()
}

/// truncate string for display
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
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
            Ok(format!("output matches ({}ms)", elapsed_ms))
        } else {
            Err(format!(
                "output mismatch: {}",
                diff_preview(&actual, &expected)
            ))
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
            return Ok(TestCase {
                name: format!("benchmark < {}ms", self.max_time_ms),
                result: Err(format!(
                    "output mismatch: {}",
                    diff_preview(&actual, &expected)
                )),
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

/// Validator: run 1BRC solution and compute expected output on-the-fly
/// DSL: brc_validate:string(./solution),string(data/measurements.txt)
pub struct BrcValidator {
    pub solution: String,
    pub measurements_file: String,
}

impl BrcValidator {
    pub fn new(solution: impl Into<String>, measurements_file: impl Into<String>) -> Self {
        Self {
            solution: solution.into(),
            measurements_file: measurements_file.into(),
        }
    }

    pub async fn validate(&self) -> Result<TestCase, String> {
        use std::collections::BTreeMap;
        use std::io::{BufRead, BufReader};

        let workspace = get_workspace();

        // build the command: ./solution <measurements_file>
        let cmd = format!("{} {}", self.solution, self.measurements_file);

        // run user's solution
        let (stdout, _stderr, elapsed_ms) = run_command(&cmd, &workspace).await?;

        // compute expected output by reading measurements file
        let measurements_path = if self.measurements_file.starts_with('/') {
            PathBuf::from(&self.measurements_file)
        } else {
            workspace.join(&self.measurements_file)
        };

        let file = std::fs::File::open(&measurements_path)
            .map_err(|e| format!("failed to open {}: {}", measurements_path.display(), e))?;

        struct Stats {
            min: f64,
            max: f64,
            sum: f64,
            count: u64,
        }

        let mut stats: BTreeMap<String, Stats> = BTreeMap::new();
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line.map_err(|e| format!("read error: {}", e))?;
            if let Some(pos) = line.rfind(';') {
                let name = &line[..pos];
                let temp_str = &line[pos + 1..];
                if let Ok(temp) = temp_str.parse::<f64>() {
                    stats
                        .entry(name.to_string())
                        .and_modify(|s| {
                            if temp < s.min {
                                s.min = temp;
                            }
                            if temp > s.max {
                                s.max = temp;
                            }
                            s.sum += temp;
                            s.count += 1;
                        })
                        .or_insert(Stats {
                            min: temp,
                            max: temp,
                            sum: temp,
                            count: 1,
                        });
                }
            }
        }

        // format expected output
        let mut expected = String::from("{");
        let mut first = true;
        for (name, s) in &stats {
            if !first {
                expected.push_str(", ");
            }
            first = false;
            let mean = s.sum / s.count as f64;
            expected.push_str(&format!("{}={:.1}/{:.1}/{:.1}", name, s.min, mean, s.max));
        }
        expected.push('}');

        let actual = normalize(&stdout);
        let expected = normalize(&expected);

        let result = if actual == expected {
            Ok(format!("output correct ({}ms)", elapsed_ms))
        } else {
            Err(format!(
                "output mismatch: {}",
                diff_preview(&actual, &expected)
            ))
        };

        Ok(TestCase {
            name: format!("brc validate {}", self.measurements_file),
            result,
        })
    }
}

/// Validator: run 1BRC solution, validate correctness, and check time limit
/// DSL: brc_benchmark:string(./solution),string(data/measurements.txt),int(max_ms)
pub struct BrcBenchmarkValidator {
    pub solution: String,
    pub measurements_file: String,
    pub max_time_ms: u64,
}

impl BrcBenchmarkValidator {
    pub fn new(
        solution: impl Into<String>,
        measurements_file: impl Into<String>,
        max_time_ms: u64,
    ) -> Self {
        Self {
            solution: solution.into(),
            measurements_file: measurements_file.into(),
            max_time_ms,
        }
    }

    pub async fn validate(&self) -> Result<TestCase, String> {
        use std::collections::BTreeMap;
        use std::io::{BufRead, BufReader};

        let workspace = get_workspace();

        let cmd = format!("{} {}", self.solution, self.measurements_file);
        let (stdout, _stderr, elapsed_ms) = run_command(&cmd, &workspace).await?;

        // compute expected
        let measurements_path = if self.measurements_file.starts_with('/') {
            PathBuf::from(&self.measurements_file)
        } else {
            workspace.join(&self.measurements_file)
        };

        let file = std::fs::File::open(&measurements_path)
            .map_err(|e| format!("failed to open {}: {}", measurements_path.display(), e))?;

        struct Stats {
            min: f64,
            max: f64,
            sum: f64,
            count: u64,
        }

        let mut stats: BTreeMap<String, Stats> = BTreeMap::new();
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line.map_err(|e| format!("read error: {}", e))?;
            if let Some(pos) = line.rfind(';') {
                let name = &line[..pos];
                let temp_str = &line[pos + 1..];
                if let Ok(temp) = temp_str.parse::<f64>() {
                    stats
                        .entry(name.to_string())
                        .and_modify(|s| {
                            if temp < s.min {
                                s.min = temp;
                            }
                            if temp > s.max {
                                s.max = temp;
                            }
                            s.sum += temp;
                            s.count += 1;
                        })
                        .or_insert(Stats {
                            min: temp,
                            max: temp,
                            sum: temp,
                            count: 1,
                        });
                }
            }
        }

        let mut expected = String::from("{");
        let mut first = true;
        for (name, s) in &stats {
            if !first {
                expected.push_str(", ");
            }
            first = false;
            let mean = s.sum / s.count as f64;
            expected.push_str(&format!("{}={:.1}/{:.1}/{:.1}", name, s.min, mean, s.max));
        }
        expected.push('}');

        let actual = normalize(&stdout);
        let expected = normalize(&expected);

        // check correctness first
        if actual != expected {
            return Ok(TestCase {
                name: format!("brc benchmark < {}ms", self.max_time_ms),
                result: Err(format!(
                    "output mismatch: {}",
                    diff_preview(&actual, &expected)
                )),
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
            name: format!("brc benchmark < {}ms", self.max_time_ms),
            result,
        })
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

    #[test]
    fn test_diff_preview_line_mismatch() {
        let actual = "line1\nline2\nline3";
        let expected = "line1\nLINE2\nline3";
        let diff = diff_preview(actual, expected);
        assert!(diff.contains("line 2"));
    }

    #[test]
    fn test_diff_preview_count_mismatch() {
        let actual = "line1\nline2";
        let expected = "line1\nline2\nline3";
        let diff = diff_preview(actual, expected);
        assert!(diff.contains("line count mismatch"));
    }

    #[test]
    fn test_diff_preview_identical() {
        let actual = "line1\nline2";
        let expected = "line1\nline2";
        let diff = diff_preview(actual, expected);
        assert!(diff.is_empty());
    }
}
