use color_eyre::eyre::Result;

use crate::api::LighthouseAPIClient;
use crate::config::Config;
use crate::message::Message;
use crate::state::LabState;
use crate::ui::UI;

/// handle `luxctl tasks [--refresh]`
pub async fn list(refresh: bool) -> Result<()> {
    let config = Config::load()?;
    if !config.has_auth_token() {
        UI::error(
            "not authenticated",
            Some("run `luxctl auth --token $token`"),
        );
        return Ok(());
    }

    let mut state = LabState::load(config.expose_token())?;

    let lab = if let Some(l) = state.get_active() {
        l.clone()
    } else {
        UI::error("no active lab", None);
        UI::note("run `luxctl lab start --id <ID>` first");
        return Ok(());
    };

    // refresh from API if requested or no cached tasks
    if refresh || lab.tasks.is_empty() {
        let client = LighthouseAPIClient::from_config(&config);

        let fresh_lab = match client.lab_by_slug(&lab.slug).await {
            Ok(l) => l,
            Err(err) => {
                UI::error(
                    &format!("failed to fetch lab '{}'", lab.slug),
                    Some(&format!("{}", err)),
                );
                return Ok(());
            }
        };

        if let Some(tasks) = &fresh_lab.tasks {
            state.refresh_tasks(tasks);
            state.save(config.expose_token())?;
        }
    }

    // print task list
    if let Some(active) = state.get_active() {
        Message::print_task_list(active);
    }

    Ok(())
}
