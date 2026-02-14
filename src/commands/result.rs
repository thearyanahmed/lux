use std::path::PathBuf;

use blueprint::reporter::CliReporter;
use color_eyre::eyre::Result;

use crate::api::LighthouseAPIClient;
use crate::commands::blueprint_runner;
use crate::commands::run::submit_and_update;
use crate::config::Config;
use crate::shell;
use crate::state::LabState;
use crate::ui::RunUI;
use crate::{oops, say};

/// handle `luxctl result --task <slug|number> --input key=value`
/// runs blueprint in Result mode, matching user-provided inputs against captured values
pub async fn result(task_id: &str, inputs: &[String], lab_slug: Option<&str>) -> Result<()> {
    let config = Config::load()?;
    if !config.has_auth_token() {
        oops!("not authenticated. Run: `luxctl auth --token $token`");
        return Ok(());
    }

    let token = config.expose_token().to_string();
    let mut state = LabState::load(&token)?;
    let client = LighthouseAPIClient::from_config(&config);

    let lab_slug = match lab_slug {
        Some(s) => s.to_string(),
        None => {
            if let Some(l) = state.get_active() {
                l.slug.clone()
            } else {
                oops!("no lab specified and no active lab");
                say!("use `--lab <ID>` or run `luxctl lab start --id <ID>` first");
                return Ok(());
            }
        }
    };

    let lab_data = match client.lab_by_slug(&lab_slug).await {
        Ok(l) => l,
        Err(err) => {
            oops!("failed to fetch lab '{}': {}", lab_slug, err);
            return Ok(());
        }
    };

    let tasks = if let Some(t) = &lab_data.tasks {
        t
    } else {
        oops!("lab '{}' has no tasks", lab_slug);
        return Ok(());
    };

    let task_data = if let Ok(task_num) = task_id.parse::<usize>() {
        if task_num == 0 || task_num > tasks.len() {
            oops!(
                "task #{} not found. valid range: 1-{}",
                task_num,
                tasks.len()
            );
            return Ok(());
        }
        &tasks[task_num - 1]
    } else if let Some(t) = tasks.iter().find(|t| t.slug == task_id) {
        t
    } else {
        oops!("task '{}' not found in lab '{}'", task_id, lab_slug);
        return Ok(());
    };

    if !task_data.has_blueprint() {
        oops!("task '{}' has no blueprint — `result` only works with blueprint tasks", task_data.slug);
        say!("use `luxctl run --task {}` for legacy validator tasks", task_data.slug);
        return Ok(());
    }

    let bp_source = task_data.blueprint.as_deref().unwrap_or_default();
    let user_inputs = blueprint_runner::parse_inputs(inputs)?;
    let workspace = state
        .get_active()
        .map(|l| PathBuf::from(&l.workspace));

    let ui = RunUI::new(&task_data.slug, 0);
    ui.header();
    ui.blank_line();

    // prologue
    if !task_data.prologue.is_empty() {
        ui.step(&format!(
            "Running {} setup commands...",
            task_data.prologue.len()
        ));
        if let Err((cmd, shell_result)) = shell::run_commands(&task_data.prologue).await {
            oops!("setup command failed: {}", cmd);
            if !shell_result.stderr.is_empty() {
                say!("stderr: {}", shell_result.stderr.trim());
            }
            return Ok(());
        }
        ui.blank_line();
    }

    ui.step("Running blueprint (result mode)...");

    let bp_result =
        match blueprint_runner::run_result(bp_source, &task_data.slug, &user_inputs, workspace)
            .await
        {
            Ok(r) => r,
            Err(err) => {
                oops!("blueprint failed: {}", err);
                return Ok(());
            }
        };

    CliReporter::print_result(&bp_result, false);

    // submit attempt
    let attempt_request =
        blueprint_runner::to_attempt_request(&bp_result, &lab_slug, task_data.id);
    submit_and_update(
        &client,
        &attempt_request,
        &ui,
        task_data,
        Some((&mut state, &token)),
    )
    .await;

    // epilogue
    if !task_data.epilogue.is_empty() {
        ui.step(&format!(
            "Running {} cleanup commands...",
            task_data.epilogue.len()
        ));
        let failures = shell::run_commands_best_effort(&task_data.epilogue).await;
        for (cmd, r) in failures {
            log::warn!("cleanup failed: {} (exit {})", cmd, r.exit_code);
        }
    }

    Ok(())
}
