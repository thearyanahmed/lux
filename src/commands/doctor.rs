//! `luxctl doctor` - diagnose environment and check tool availability

use color_eyre::eyre::Result;
use std::process::Command;

use crate::api::LighthouseAPIClient;
use crate::config::Config;
use crate::state::LabState;
use crate::ui::UI;

/// run all diagnostic checks
pub async fn run() -> Result<()> {
    UI::header();

    // system info
    UI::section("System");
    check_system_info();

    // authentication
    UI::section("Authentication");
    let config = check_auth();

    // network connectivity
    UI::section("Network");
    check_network(&config).await;

    // development tools
    UI::section("Development Tools");
    check_dev_tools();

    // active lab
    UI::section("Lab State");
    check_lab_state(&config);

    UI::blank();
    UI::note("run `luxctl doctor` after installing missing tools to verify");

    Ok(())
}

fn check_system_info() {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    UI::ok("os", Some(os));
    UI::ok("arch", Some(arch));

    match dirs::home_dir() {
        Some(home) => {
            let luxctl_dir = home.join(".luxctl");
            if luxctl_dir.exists() {
                UI::ok("config dir", Some(&luxctl_dir.to_string_lossy()));
            } else {
                UI::warn(
                    "config dir",
                    Some(&format!(
                        "{} (will be created)",
                        luxctl_dir.to_string_lossy()
                    )),
                );
            }
        }
        None => {
            UI::error("home dir", Some("could not determine home directory"));
        }
    }
}

fn check_auth() -> Option<Config> {
    match Config::exists() {
        Ok(false) => {
            UI::warn(
                "not configured",
                Some("run `luxctl auth --token $token` to get started"),
            );
            return None;
        }
        Err(e) => {
            UI::error("config", Some(&format!("could not check config: {}", e)));
            return None;
        }
        Ok(true) => {}
    }

    match Config::load() {
        Ok(config) if config.has_auth_token() => {
            UI::ok("authenticated", Some("token configured"));
            Some(config)
        }
        Ok(_) => {
            UI::warn("token empty", Some("run `luxctl auth --token $token`"));
            None
        }
        Err(e) => {
            UI::error("config", Some(&format!("failed to load: {}", e)));
            None
        }
    }
}

async fn check_network(config: &Option<Config>) {
    let client = LighthouseAPIClient::default();
    match client.healthcheck().await {
        Ok(response) => {
            UI::ok("healthcheck", Some(&response.status));
        }
        Err(e) => {
            let msg = format!("{}", e);
            if msg.contains("timeout") || msg.contains("connect") {
                UI::error(
                    "healthcheck",
                    Some("could not connect to projectlighthouse.io"),
                );
            } else {
                UI::error("healthcheck", Some(&msg));
            }
            return;
        }
    }

    let Some(config) = config else {
        UI::warn("api", Some("skipped (not authenticated)"));
        return;
    };

    let client = LighthouseAPIClient::from_config(config);
    match client.me().await {
        Ok(user) => {
            UI::ok("api", Some(&format!("connected as {}", user.email)));
        }
        Err(e) => {
            UI::error("api", Some(&format!("{}", e)));
        }
    }
}

fn check_dev_tools() {
    let tools = vec![
        ToolCheck::new("git", &["--version"], true),
        ToolCheck::new("go", &["version"], false),
        ToolCheck::new("cargo", &["--version"], false),
        ToolCheck::new("rustc", &["--version"], false),
        ToolCheck::new("docker", &["--version"], false),
    ];

    for tool in tools {
        tool.check();
    }

    UI::blank();
    UI::note("supported runtimes: go, rust");
}

struct ToolCheck {
    name: &'static str,
    args: &'static [&'static str],
    required: bool,
}

impl ToolCheck {
    fn new(name: &'static str, args: &'static [&'static str], required: bool) -> Self {
        Self {
            name,
            args,
            required,
        }
    }

    fn check(&self) {
        match Command::new(self.name).args(self.args).output() {
            Ok(output) if output.status.success() => {
                let version = extract_version(&output.stdout);
                UI::ok(self.name, version.as_deref());
            }
            Ok(_) => {
                if self.required {
                    UI::error(self.name, Some("installed but returned error"));
                } else {
                    UI::warn(self.name, Some("installed but returned error"));
                }
            }
            Err(_) => {
                if self.required {
                    UI::error(self.name, Some("required but not found"));
                } else {
                    UI::skip(self.name, None);
                }
            }
        }
    }
}

/// extract version string from command output
fn extract_version(output: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(output);
    let first_line = text.lines().next()?;

    for word in first_line.split_whitespace() {
        let cleaned = word
            .trim_start_matches('v')
            .trim_start_matches("go")
            .trim_end_matches(',');
        if cleaned.contains('.') && cleaned.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            return Some(cleaned.to_string());
        }
    }

    Some(first_line.trim().to_string())
}

fn check_lab_state(config: &Option<Config>) {
    let Some(config) = config else {
        UI::warn("lab", Some("skipped (not authenticated)"));
        return;
    };

    let state = match LabState::load(config.expose_token()) {
        Ok(s) => s,
        Err(e) => {
            UI::error("state", Some(&format!("failed to load: {}", e)));
            return;
        }
    };

    if let Some(lab) = state.get_active() {
        UI::ok("active lab", Some(&lab.name));

        let workspace_path = std::path::Path::new(&lab.workspace);
        if workspace_path.exists() {
            UI::ok("workspace", Some(&lab.workspace));
        } else {
            UI::error("workspace", Some(&format!("{} (not found)", lab.workspace)));
        }

        if let Some(rt) = &lab.runtime {
            UI::ok("runtime", Some(rt));
        } else {
            UI::warn("runtime", Some("not set"));
        }

        let progress = format!(
            "{}/{} tasks completed",
            lab.completed_count(),
            lab.tasks.len()
        );
        UI::ok("progress", Some(&progress));
    } else {
        UI::ok("lab", Some("none active"));
        UI::note("run `luxctl lab start --id <ID>` to begin");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_version_git() {
        let output = b"git version 2.39.0";
        assert_eq!(extract_version(output), Some("2.39.0".to_string()));
    }

    #[test]
    fn test_extract_version_go() {
        let output = b"go version go1.22.0 darwin/arm64";
        assert_eq!(extract_version(output), Some("1.22.0".to_string()));
    }

    #[test]
    fn test_extract_version_cargo() {
        let output = b"cargo 1.75.0 (abc123 2024-01-01)";
        assert_eq!(extract_version(output), Some("1.75.0".to_string()));
    }

    #[test]
    fn test_extract_version_docker() {
        let output = b"Docker version 24.0.7, build abcd123";
        assert_eq!(extract_version(output), Some("24.0.7".to_string()));
    }

    #[test]
    fn test_extract_version_fallback() {
        let output = b"some unknown format";
        assert_eq!(
            extract_version(output),
            Some("some unknown format".to_string())
        );
    }
}
