//! Validators for the "Build Your Own Docker" lab
//!
//! These validators test the user's ./your-docker binary implementation.
//! Each validator runs the binary with specific arguments and checks the output
//! to verify correct container behavior (namespaces, cgroups, chroot, etc).

use crate::config::Config;
use crate::state::LabState;
use crate::tasks::TestCase;
use std::path::PathBuf;
use std::process::Stdio;
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

/// command output with exit code
struct CommandOutput {
    stdout: String,
    stderr: String,
    exit_code: i32,
}

/// run ./your-docker with given arguments
/// returns (stdout, stderr, exit_code)
async fn run_docker_command(args: &[&str], workspace: &PathBuf) -> Result<CommandOutput, String> {
    let binary = workspace.join("your-docker");

    if !binary.exists() {
        return Err(format!(
            "./your-docker binary not found at {}",
            binary.display()
        ));
    }

    let output = Command::new(&binary)
        .args(args)
        .current_dir(workspace)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("failed to run ./your-docker: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(-1);

    Ok(CommandOutput {
        stdout,
        stderr,
        exit_code,
    })
}

/// normalize output for comparison
fn normalize(s: &str) -> String {
    s.trim().to_string()
}

/// truncate string for display
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

// ============================================================================
// docker_exec - run command and check stdout matches expected
// DSL: docker_exec:string(cmd),string(expected)
// ============================================================================

pub struct DockerExecValidator {
    pub command: String,
    pub expected: String,
}

impl DockerExecValidator {
    pub fn new(command: impl Into<String>, expected: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            expected: expected.into(),
        }
    }

    pub async fn validate(&self) -> Result<TestCase, String> {
        let workspace = get_workspace();

        // split command into parts for ./your-docker run <parts...>
        let cmd_parts: Vec<&str> = self.command.split_whitespace().collect();
        let mut args = vec!["run"];
        args.extend(cmd_parts.iter());

        let output = run_docker_command(&args, &workspace).await?;

        let actual = normalize(&output.stdout);
        let expected = normalize(&self.expected);

        let result = if actual == expected {
            Ok(format!("output matches: '{}'", truncate(&expected, 50)))
        } else {
            Err(format!(
                "expected '{}', got '{}'",
                truncate(&expected, 50),
                truncate(&actual, 50)
            ))
        };

        Ok(TestCase {
            name: format!("docker_exec: {}", truncate(&self.command, 30)),
            result,
        })
    }
}

// ============================================================================
// docker_exit_code - run command and check exit code
// DSL: docker_exit_code:int(code)
// ============================================================================

pub struct DockerExitCodeValidator {
    pub expected_code: i32,
}

impl DockerExitCodeValidator {
    pub fn new(expected_code: i32) -> Self {
        Self { expected_code }
    }

    pub async fn validate(&self) -> Result<TestCase, String> {
        let workspace = get_workspace();

        // run ls /nonexistent which should return exit code 2
        let args = ["run", "ls", "/nonexistent"];
        let output = run_docker_command(&args, &workspace).await?;

        let result = if output.exit_code == self.expected_code {
            Ok(format!("exit code {} as expected", self.expected_code))
        } else {
            Err(format!(
                "expected exit code {}, got {}",
                self.expected_code, output.exit_code
            ))
        };

        Ok(TestCase {
            name: format!("docker_exit_code: {}", self.expected_code),
            result,
        })
    }
}

// ============================================================================
// docker_pid_namespace - verify process runs as PID 1
// DSL: docker_pid_namespace:int(expected_pid)
// ============================================================================

pub struct DockerPidNamespaceValidator {
    pub expected_pid: i32,
}

impl DockerPidNamespaceValidator {
    pub fn new(expected_pid: i32) -> Self {
        Self { expected_pid }
    }

    pub async fn validate(&self) -> Result<TestCase, String> {
        let workspace = get_workspace();

        // run sh -c 'echo $$' to get the PID
        let args = ["run", "sh", "-c", "echo $$"];
        let output = run_docker_command(&args, &workspace).await?;

        if output.exit_code != 0 {
            return Ok(TestCase {
                name: "docker_pid_namespace".to_string(),
                result: Err(format!(
                    "command failed with exit code {}: {}",
                    output.exit_code,
                    truncate(&output.stderr, 100)
                )),
            });
        }

        let pid_str = normalize(&output.stdout);
        let pid: i32 = pid_str
            .parse()
            .map_err(|_| format!("failed to parse PID from output: '{}'", pid_str))?;

        let result = if pid == self.expected_pid {
            Ok(format!(
                "PID is {} as expected (namespace isolation working)",
                pid
            ))
        } else {
            Err(format!(
                "expected PID {}, got {} (PID namespace not isolated)",
                self.expected_pid, pid
            ))
        };

        Ok(TestCase {
            name: "docker_pid_namespace".to_string(),
            result,
        })
    }
}

// ============================================================================
// docker_chroot - verify filesystem is isolated
// DSL: docker_chroot:bool(expected)
// ============================================================================

pub struct DockerChrootValidator {
    pub expected_isolated: bool,
}

impl DockerChrootValidator {
    pub fn new(expected_isolated: bool) -> Self {
        Self { expected_isolated }
    }

    pub async fn validate(&self) -> Result<TestCase, String> {
        let workspace = get_workspace();

        // run ls / and check that we see container fs, not host fs
        // container fs should have typical minimal directories
        let args = ["run", "ls", "/"];
        let output = run_docker_command(&args, &workspace).await?;

        if output.exit_code != 0 {
            return Ok(TestCase {
                name: "docker_chroot".to_string(),
                result: Err(format!(
                    "command failed with exit code {}: {}",
                    output.exit_code,
                    truncate(&output.stderr, 100)
                )),
            });
        }

        let ls_output = normalize(&output.stdout);
        let entries: Vec<&str> = ls_output.split_whitespace().collect();

        // check for typical container fs markers
        // a properly chrooted container should have /bin, /etc, /proc, etc.
        // and should NOT have host-specific dirs like /Users, /System (macOS)
        // or specific host paths

        let has_bin = entries.contains(&"bin");
        let has_etc = entries.contains(&"etc");
        let has_host_markers = entries.contains(&"Users")
            || entries.contains(&"System")
            || entries.contains(&"Library");

        let is_isolated = has_bin && has_etc && !has_host_markers;

        let result = if is_isolated == self.expected_isolated {
            if is_isolated {
                Ok("filesystem is isolated (chroot working)".to_string())
            } else {
                Ok("filesystem is not isolated as expected".to_string())
            }
        } else if self.expected_isolated {
            Err(format!(
                "filesystem not isolated: saw host markers in output: {}",
                truncate(&ls_output, 100)
            ))
        } else {
            Err("filesystem is unexpectedly isolated".to_string())
        };

        Ok(TestCase {
            name: "docker_chroot".to_string(),
            result,
        })
    }
}

// ============================================================================
// docker_pull - verify image can be pulled from registry
// DSL: docker_pull:string(image),bool(success)
// ============================================================================

pub struct DockerPullValidator {
    pub image: String,
    pub expected_success: bool,
}

impl DockerPullValidator {
    pub fn new(image: impl Into<String>, expected_success: bool) -> Self {
        Self {
            image: image.into(),
            expected_success,
        }
    }

    pub async fn validate(&self) -> Result<TestCase, String> {
        let workspace = get_workspace();

        let args = ["pull", &self.image];
        let output = run_docker_command(&args, &workspace).await?;

        let success = output.exit_code == 0;

        let result = if success == self.expected_success {
            if success {
                Ok(format!("successfully pulled image: {}", self.image))
            } else {
                Ok(format!("pull failed as expected for: {}", self.image))
            }
        } else if self.expected_success {
            Err(format!(
                "failed to pull image {}: {}",
                self.image,
                truncate(&output.stderr, 100)
            ))
        } else {
            Err(format!("pull unexpectedly succeeded for: {}", self.image))
        };

        Ok(TestCase {
            name: format!("docker_pull: {}", self.image),
            result,
        })
    }
}

// ============================================================================
// docker_mount_namespace - verify mount namespace isolation
// DSL: docker_mount_namespace:bool(expected)
// ============================================================================

pub struct DockerMountNamespaceValidator {
    pub expected_isolated: bool,
}

impl DockerMountNamespaceValidator {
    pub fn new(expected_isolated: bool) -> Self {
        Self { expected_isolated }
    }

    pub async fn validate(&self) -> Result<TestCase, String> {
        let workspace = get_workspace();

        // check /proc/mounts to verify:
        // 1. /proc is mounted (proc filesystem)
        // 2. host mounts are not visible
        let args = ["run", "cat", "/proc/mounts"];
        let output = run_docker_command(&args, &workspace).await?;

        if output.exit_code != 0 {
            return Ok(TestCase {
                name: "docker_mount_namespace".to_string(),
                result: Err(format!(
                    "failed to read /proc/mounts: {}",
                    truncate(&output.stderr, 100)
                )),
            });
        }

        let mounts = normalize(&output.stdout);

        // check for proc mount
        let has_proc = mounts.contains("proc /proc proc");

        // check for absence of host mounts (common host paths)
        let has_host_mounts = mounts.contains("/dev/disk")
            || mounts.contains("/Users")
            || mounts.contains("/System")
            || mounts.contains("/home");

        let is_isolated = has_proc && !has_host_mounts;

        let result = if is_isolated == self.expected_isolated {
            if is_isolated {
                Ok("mount namespace is isolated".to_string())
            } else {
                Ok("mount namespace not isolated as expected".to_string())
            }
        } else if self.expected_isolated {
            Err(format!(
                "mount namespace not properly isolated. has_proc={}, has_host_mounts={}",
                has_proc, has_host_mounts
            ))
        } else {
            Err("mount namespace is unexpectedly isolated".to_string())
        };

        Ok(TestCase {
            name: "docker_mount_namespace".to_string(),
            result,
        })
    }
}

// ============================================================================
// docker_cgroup_memory - verify memory cgroup limits
// DSL: docker_cgroup_memory:int(bytes),bool(oom_expected)
// ============================================================================

pub struct DockerCgroupMemoryValidator {
    pub memory_limit_bytes: u64,
    pub oom_expected: bool,
}

impl DockerCgroupMemoryValidator {
    pub fn new(memory_limit_bytes: u64, oom_expected: bool) -> Self {
        Self {
            memory_limit_bytes,
            oom_expected,
        }
    }

    pub async fn validate(&self) -> Result<TestCase, String> {
        let workspace = get_workspace();

        // convert bytes to human readable for --memory flag
        let memory_mb = self.memory_limit_bytes / (1024 * 1024);
        let memory_flag = format!("--memory={}m", memory_mb);

        // run a memory stress test
        // allocate 2x the limit to trigger OOM if limits work
        let stress_bytes = self.memory_limit_bytes * 2;
        let stress_mb = stress_bytes / (1024 * 1024);

        // use a simple approach: try to allocate memory with dd or a simple program
        // for simplicity, we'll check if the process gets killed
        let args = [
            "run",
            &memory_flag,
            "sh",
            "-c",
            &format!("head -c {} /dev/zero | tail -c 1", stress_mb * 1024 * 1024),
        ];

        let output = run_docker_command(&args, &workspace).await?;

        // OOM kills typically result in exit code 137 (128 + SIGKILL)
        // or the command may fail with specific error
        let was_killed = output.exit_code == 137
            || output.exit_code == 9
            || output.stderr.to_lowercase().contains("killed")
            || output.stderr.to_lowercase().contains("oom");

        let result = if was_killed == self.oom_expected {
            if was_killed {
                Ok(format!(
                    "process was OOM killed as expected (memory limit: {}MB)",
                    memory_mb
                ))
            } else {
                Ok(format!(
                    "process completed without OOM (memory limit: {}MB)",
                    memory_mb
                ))
            }
        } else if self.oom_expected {
            Err(format!(
                "expected OOM kill but process completed (exit code: {})",
                output.exit_code
            ))
        } else {
            Err(format!(
                "unexpected OOM kill (exit code: {})",
                output.exit_code
            ))
        };

        Ok(TestCase {
            name: format!("docker_cgroup_memory: {}MB", memory_mb),
            result,
        })
    }
}

// ============================================================================
// docker_network_namespace - verify network namespace isolation
// DSL: docker_network_namespace:bool(expected)
// ============================================================================

pub struct DockerNetworkNamespaceValidator {
    pub expected_isolated: bool,
}

impl DockerNetworkNamespaceValidator {
    pub fn new(expected_isolated: bool) -> Self {
        Self { expected_isolated }
    }

    pub async fn validate(&self) -> Result<TestCase, String> {
        let workspace = get_workspace();

        // run ip addr to check network interfaces
        let args = ["run", "ip", "addr"];
        let output = run_docker_command(&args, &workspace).await?;

        if output.exit_code != 0 {
            // try with ifconfig as fallback
            let args_fallback = ["run", "ifconfig", "-a"];
            let output_fallback = run_docker_command(&args_fallback, &workspace).await?;

            if output_fallback.exit_code != 0 {
                return Ok(TestCase {
                    name: "docker_network_namespace".to_string(),
                    result: Err("neither 'ip addr' nor 'ifconfig' available".to_string()),
                });
            }

            return self.check_isolation(&output_fallback.stdout);
        }

        self.check_isolation(&output.stdout)
    }

    fn check_isolation(&self, network_output: &str) -> Result<TestCase, String> {
        let output = normalize(network_output);

        // in an isolated network namespace:
        // 1. should have loopback (lo)
        // 2. should have a container interface (eth0 or similar)
        // 3. should NOT have host interfaces (en0, wlan0, eth0 with host IP, etc.)

        let has_loopback = output.contains("lo") || output.contains("127.0.0.1");

        // check for container-specific interface patterns
        let has_container_interface = output.contains("eth0@")
            || output.contains("veth")
            || (output.contains("eth0") && output.contains("172.17."));

        // check for host interface patterns (this is a heuristic)
        let has_host_interface = output.contains("en0")
            || output.contains("wlan0")
            || output.contains("192.168.")
            || output.contains("10.0.");

        let is_isolated = has_loopback && (has_container_interface || !has_host_interface);

        let result = if is_isolated == self.expected_isolated {
            if is_isolated {
                Ok("network namespace is isolated".to_string())
            } else {
                Ok("network namespace not isolated as expected".to_string())
            }
        } else if self.expected_isolated {
            Err(format!(
                "network namespace not isolated. loopback={}, container_if={}, host_if={}",
                has_loopback, has_container_interface, has_host_interface
            ))
        } else {
            Err("network namespace is unexpectedly isolated".to_string())
        };

        Ok(TestCase {
            name: "docker_network_namespace".to_string(),
            result,
        })
    }
}

// ============================================================================
// docker_veth_pair - verify veth pair connectivity
// DSL: docker_veth_pair:bool(expected)
// ============================================================================

pub struct DockerVethPairValidator {
    pub expected_connected: bool,
}

impl DockerVethPairValidator {
    pub fn new(expected_connected: bool) -> Self {
        Self { expected_connected }
    }

    pub async fn validate(&self) -> Result<TestCase, String> {
        let workspace = get_workspace();

        // first get the gateway IP (typically the host end of the veth pair)
        // common patterns: 172.17.0.1 for docker0 bridge
        let args = ["run", "ip", "route"];
        let route_output = run_docker_command(&args, &workspace).await?;

        let gateway = if route_output.exit_code == 0 {
            // parse gateway from "default via X.X.X.X"
            route_output
                .stdout
                .lines()
                .find(|l| l.contains("default via"))
                .and_then(|l| l.split_whitespace().nth(2))
                .unwrap_or("172.17.0.1")
                .to_string()
        } else {
            "172.17.0.1".to_string()
        };

        // try to ping the gateway
        let ping_args = ["run", "ping", "-c", "1", "-W", "2", &gateway];
        let ping_output = run_docker_command(&ping_args, &workspace).await?;

        let is_connected = ping_output.exit_code == 0
            || ping_output.stdout.contains("1 received")
            || ping_output.stdout.contains("1 packets received");

        let result = if is_connected == self.expected_connected {
            if is_connected {
                Ok(format!(
                    "veth pair connected (can reach gateway {})",
                    gateway
                ))
            } else {
                Ok("veth pair not connected as expected".to_string())
            }
        } else if self.expected_connected {
            Err(format!("cannot reach gateway {} via veth pair", gateway))
        } else {
            Err(format!("unexpectedly connected to gateway {}", gateway))
        };

        Ok(TestCase {
            name: "docker_veth_pair".to_string(),
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
        assert_eq!(normalize("  hello  "), "hello");
        assert_eq!(normalize("\n\nhello\n\n"), "hello");
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 5), "hello...");
    }

    #[test]
    fn test_docker_exec_validator_new() {
        let v = DockerExecValidator::new("echo hello", "hello");
        assert_eq!(v.command, "echo hello");
        assert_eq!(v.expected, "hello");
    }

    #[test]
    fn test_docker_exit_code_validator_new() {
        let v = DockerExitCodeValidator::new(2);
        assert_eq!(v.expected_code, 2);
    }

    #[test]
    fn test_docker_pid_namespace_validator_new() {
        let v = DockerPidNamespaceValidator::new(1);
        assert_eq!(v.expected_pid, 1);
    }

    #[test]
    fn test_docker_chroot_validator_new() {
        let v = DockerChrootValidator::new(true);
        assert!(v.expected_isolated);
    }

    #[test]
    fn test_docker_pull_validator_new() {
        let v = DockerPullValidator::new("alpine:latest", true);
        assert_eq!(v.image, "alpine:latest");
        assert!(v.expected_success);
    }

    #[test]
    fn test_docker_mount_namespace_validator_new() {
        let v = DockerMountNamespaceValidator::new(true);
        assert!(v.expected_isolated);
    }

    #[test]
    fn test_docker_cgroup_memory_validator_new() {
        let v = DockerCgroupMemoryValidator::new(10_485_760, true);
        assert_eq!(v.memory_limit_bytes, 10_485_760);
        assert!(v.oom_expected);
    }

    #[test]
    fn test_docker_network_namespace_validator_new() {
        let v = DockerNetworkNamespaceValidator::new(true);
        assert!(v.expected_isolated);
    }

    #[test]
    fn test_docker_veth_pair_validator_new() {
        let v = DockerVethPairValidator::new(true);
        assert!(v.expected_connected);
    }
}
