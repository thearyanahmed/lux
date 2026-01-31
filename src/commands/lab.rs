use color_eyre::eyre::Result;

use crate::api::LighthouseAPIClient;
use crate::config::Config;
use crate::state::LabState;
use crate::ui::UI;

/// handle `luxctl lab start --id <id> --workspace <path> [--runtime <runtime>]`
pub async fn start(slug: &str, workspace: &str, runtime: Option<&str>) -> Result<()> {
    let config = Config::load()?;
    if !config.has_auth_token() {
        UI::error(
            "not authenticated",
            Some("run `luxctl auth --token $token`"),
        );
        return Ok(());
    }

    let client = LighthouseAPIClient::from_config(&config);

    let lab = match client.lab_by_slug(slug).await {
        Ok(l) => l,
        Err(err) => {
            UI::error(
                &format!("lab '{}' not found", slug),
                Some(&format!("{}", err)),
            );
            UI::note("run `luxctl lab list` to see available labs");
            return Ok(());
        }
    };

    let workspace_path = std::path::Path::new(workspace);
    let absolute_workspace = if workspace_path.is_absolute() {
        workspace_path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| color_eyre::eyre::eyre!("cannot get cwd: {}", e))?
            .join(workspace_path)
    };

    let canonical = absolute_workspace
        .canonicalize()
        .unwrap_or(absolute_workspace);

    let workspace_str = canonical.to_string_lossy().to_string();

    let tasks = lab.tasks.as_deref().unwrap_or(&[]);

    let mut state = LabState::load(config.expose_token())?;
    state.set_active(&lab.slug, &lab.name, tasks, &workspace_str, runtime);
    state.save(config.expose_token())?;

    UI::success(&format!("now working on: {}", lab.name));
    UI::kv("workspace", &workspace_str);
    if let Some(rt) = runtime {
        UI::kv("runtime", rt);
    }
    UI::note("run `luxctl task list` to see available tasks");

    Ok(())
}

/// handle `luxctl lab status`
pub fn status() -> Result<()> {
    let config = Config::load()?;
    if !config.has_auth_token() {
        UI::error(
            "not authenticated",
            Some("run `luxctl auth --token $token`"),
        );
        return Ok(());
    }

    let state = LabState::load(config.expose_token())?;

    if let Some(lab) = state.get_active() {
        UI::kv_aligned("active lab", &lab.name, 14);
        UI::kv_aligned("slug", &lab.slug, 14);
        UI::kv_aligned("workspace", &lab.workspace, 14);
        if let Some(ref rt) = lab.runtime {
            UI::kv_aligned("runtime", rt, 14);
        } else {
            UI::kv_aligned("runtime", "not set", 14);
        }
        UI::kv_aligned(
            "progress",
            &format!(
                "{}/{} tasks completed",
                lab.completed_count(),
                lab.tasks.len()
            ),
            14,
        );
        UI::note("run `luxctl task list` for task list");
    } else {
        UI::info("no active lab");
        UI::note("run `luxctl lab start --id <ID>` to start one");
    }

    Ok(())
}

/// handle `luxctl lab stop`
pub fn stop() -> Result<()> {
    let config = Config::load()?;
    if !config.has_auth_token() {
        UI::error(
            "not authenticated",
            Some("run `luxctl auth --token $token`"),
        );
        return Ok(());
    }

    let mut state = LabState::load(config.expose_token())?;

    if state.get_active().is_some() {
        let name = state
            .get_active()
            .map(|l| l.name.clone())
            .unwrap_or_default();
        state.clear_active();
        state.save(config.expose_token())?;
        UI::success(&format!("stopped working on: {}", name));
    } else {
        UI::info("no active lab to stop");
    }

    Ok(())
}

/// handle `luxctl lab set --runtime <runtime>`
pub fn set_runtime(runtime: &str) -> Result<()> {
    let config = Config::load()?;
    if !config.has_auth_token() {
        UI::error(
            "not authenticated",
            Some("run `luxctl auth --token $token`"),
        );
        return Ok(());
    }

    let mut state = LabState::load(config.expose_token())?;

    if state.get_active().is_some() {
        state.set_runtime(runtime);
        state.save(config.expose_token())?;
        UI::success(&format!("runtime set to: {}", runtime));
    } else {
        UI::error("no active lab", None);
        UI::note("run `luxctl lab start --id <ID>` first");
    }

    Ok(())
}

/// handle `luxctl lab set --workspace <path>`
pub fn set_workspace(workspace: &str) -> Result<()> {
    let config = Config::load()?;
    if !config.has_auth_token() {
        UI::error(
            "not authenticated",
            Some("run `luxctl auth --token $token`"),
        );
        return Ok(());
    }

    let mut state = LabState::load(config.expose_token())?;

    if state.get_active().is_none() {
        UI::error("no active lab", None);
        UI::note("run `luxctl lab start --id <ID>` first");
        return Ok(());
    }

    let workspace_path = std::path::Path::new(workspace);
    let absolute_workspace = if workspace_path.is_absolute() {
        workspace_path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| color_eyre::eyre::eyre!("cannot get cwd: {}", e))?
            .join(workspace_path)
    };

    if !absolute_workspace.exists() {
        UI::error(
            "directory does not exist",
            Some(&absolute_workspace.to_string_lossy()),
        );
        return Ok(());
    }

    let canonical = absolute_workspace
        .canonicalize()
        .map_err(|e| color_eyre::eyre::eyre!("cannot resolve path: {}", e))?;

    let workspace_str = canonical.to_string_lossy().to_string();
    state.set_workspace(&workspace_str);
    state.save(config.expose_token())?;
    UI::success(&format!("workspace set to: {}", workspace_str));

    Ok(())
}

/// handle `luxctl lab restart`
pub async fn restart() -> Result<()> {
    let config = Config::load()?;
    if !config.has_auth_token() {
        UI::error(
            "not authenticated",
            Some("run `luxctl auth --token $token`"),
        );
        return Ok(());
    }

    let mut state = LabState::load(config.expose_token())?;

    let lab = match state.get_active() {
        Some(l) => l.clone(),
        None => {
            UI::error("no active lab", None);
            UI::note("run `luxctl lab start --id <ID>` first");
            return Ok(());
        }
    };

    let client = LighthouseAPIClient::from_config(&config);

    match client.restart_lab(&lab.slug).await {
        Ok(response) => {
            // refresh tasks from server
            match client.lab_by_slug(&lab.slug).await {
                Ok(refreshed_lab) => {
                    let tasks = refreshed_lab.tasks.as_deref().unwrap_or(&[]);
                    state.set_active(
                        &lab.slug,
                        &lab.name,
                        tasks,
                        &lab.workspace,
                        lab.runtime.as_deref(),
                    );
                    state.save(config.expose_token())?;
                }
                Err(_) => {
                    // if refresh fails, just clear local progress
                    state.clear_progress();
                    state.save(config.expose_token())?;
                }
            }

            UI::success(&format!("restarted lab: {}", lab.name));
            UI::info(&response.message);
        }
        Err(err) => {
            UI::error("failed to restart lab", Some(&format!("{}", err)));
        }
    }

    Ok(())
}
