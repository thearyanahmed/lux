use color_eyre::eyre::Result;
use std::path::PathBuf;

use crate::config::Config;
use crate::projectfiles;
use crate::state::ProjectState;
use crate::ui::UI;

/// handle `luxctl sync`
/// downloads fixture files for the active project into its workspace
pub async fn run() -> Result<()> {
    let config = Config::load()?;
    if !config.has_auth_token() {
        UI::error(
            "not authenticated",
            Some("run `luxctl auth --token $token`"),
        );
        return Ok(());
    }

    let state = ProjectState::load(config.expose_token())?;

    let project = match state.get_active() {
        Some(p) => p.clone(),
        None => {
            UI::error("no active project", None);
            UI::note("run `luxctl project start --id <ID>` first");
            return Ok(());
        }
    };

    let workspace = PathBuf::from(&project.workspace);
    if !workspace.exists() {
        UI::error(
            "workspace directory does not exist",
            Some(&project.workspace),
        );
        return Ok(());
    }

    UI::info(&format!("syncing fixtures for '{}'...", project.slug));

    match projectfiles::download_fixtures(&project.slug, &workspace).await {
        Ok(true) => {
            UI::success("fixtures synced");
        }
        Ok(false) => {
            // download_fixtures already printed why
        }
        Err(e) => {
            UI::error("failed to sync fixtures", Some(&format!("{}", e)));
        }
    }

    Ok(())
}
