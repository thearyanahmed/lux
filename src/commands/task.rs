use color_eyre::eyre::Result;

use crate::api::LighthouseAPIClient;
use crate::config::Config;
use crate::message::Message;
use crate::state::LabState;
use crate::ui::UI;

/// handle `luxctl task --task <slug|number> [--detailed]`
pub async fn show(task_id: &str, detailed: bool) -> Result<()> {
    let config = Config::load()?;
    if !config.has_auth_token() {
        UI::error(
            "not authenticated",
            Some("run `luxctl auth --token $token`"),
        );
        return Ok(());
    }

    let state = LabState::load(config.expose_token())?;
    let client = LighthouseAPIClient::from_config(&config);

    let lab_slug = if let Some(l) = state.get_active() {
        l.slug.clone()
    } else {
        UI::error("no active lab", None);
        UI::note("run `luxctl lab start --id <ID>` first");
        return Ok(());
    };

    let lab_data = match client.lab_by_slug(&lab_slug).await {
        Ok(l) => l,
        Err(err) => {
            UI::error(
                &format!("failed to fetch lab '{}'", lab_slug),
                Some(&format!("{}", err)),
            );
            return Ok(());
        }
    };

    let tasks = if let Some(t) = &lab_data.tasks {
        t
    } else {
        UI::error(&format!("lab '{}' has no tasks", lab_slug), None);
        return Ok(());
    };

    let task_data = if let Ok(task_num) = task_id.parse::<usize>() {
        if task_num == 0 || task_num > tasks.len() {
            UI::error(
                &format!("task #{} not found", task_num),
                Some(&format!("valid range: 1-{}", tasks.len())),
            );
            return Ok(());
        }
        &tasks[task_num - 1]
    } else if let Some(t) = tasks.iter().find(|t| t.slug == task_id) {
        t
    } else {
        UI::error(
            &format!("task '{}' not found in lab '{}'", task_id, lab_slug),
            None,
        );
        return Ok(());
    };

    Message::print_task_detail(task_data, detailed);

    Ok(())
}
