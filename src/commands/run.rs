use std::path::PathBuf;

use blueprint::reporter::CliReporter;
use color_eyre::eyre::Result;

use crate::api::{LighthouseAPIClient, SubmitAttemptRequest, Task, TaskStatus};
use crate::commands::blueprint_runner::{self, TaskSystem};
use crate::config::Config;
use crate::shell;
use crate::state::LabState;
use crate::ui::RunUI;
use crate::{complain, oops, say};

/// handle `luxctl run --task <slug|number> [--lab <slug>]`
/// task can be specified by slug or by number (1, 01, 2, 02, etc.)
pub async fn run(task_id: &str, lab_slug: Option<&str>, _detailed: bool) -> Result<()> {
    let config = Config::load()?;
    if !config.has_auth_token() {
        oops!("not authenticated. Run: `luxctl auth --token $token`");
        return Ok(());
    }

    let token = config.expose_token().to_string();
    let mut state = LabState::load(&token)?;
    let client = LighthouseAPIClient::from_config(&config);

    // determine lab slug (from arg or active lab)
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

    // fetch lab with tasks
    let lab_data = match client.lab_by_slug(&lab_slug).await {
        Ok(l) => l,
        Err(err) => {
            oops!("failed to fetch lab '{}': {}", lab_slug, err);
            return Ok(());
        }
    };

    // get tasks list
    let tasks = if let Some(t) = &lab_data.tasks {
        t
    } else {
        oops!("lab '{}' has no tasks", lab_slug);
        return Ok(());
    };

    // find task by number or slug
    let task_data = if let Ok(task_num) = task_id.parse::<usize>() {
        // task specified by number (1-based index)
        if task_num == 0 || task_num > tasks.len() {
            oops!(
                "task #{} not found. valid range: 1-{}",
                task_num,
                tasks.len()
            );
            return Ok(());
        }
        &tasks[task_num - 1]
    } else {
        // task specified by slug
        if let Some(t) = tasks.iter().find(|t| t.slug == task_id) {
            t
        } else {
            oops!("task '{}' not found in lab '{}'", task_id, lab_slug);
            say!("use task number (1, 2, 3...) or slug:");
            for (i, t) in tasks.iter().enumerate() {
                say!("  {:02}. {}", i + 1, t.slug);
            }
            return Ok(());
        }
    };

    let workspace = state
        .get_active()
        .map(|l| PathBuf::from(&l.workspace));

    run_task_validators(
        &client,
        &lab_data.slug,
        task_data,
        Some((&mut state, &token)),
        workspace,
    )
    .await
}

/// run blueprint for a single task and submit results.
/// optionally updates cached state when state_ctx is provided.
pub async fn run_task_validators(
    client: &LighthouseAPIClient,
    lab_slug: &str,
    task: &Task,
    state_ctx: Option<(&mut LabState, &str)>,
    workspace: Option<PathBuf>,
) -> Result<()> {
    match blueprint_runner::detect_system(task) {
        TaskSystem::Blueprint(source) => {
            run_blueprint_task(client, lab_slug, task, source, state_ctx, workspace).await
        }
        TaskSystem::None => {
            let ui = RunUI::new(&task.slug, 0);
            ui.header();
            ui.blank_line();
            ui.step("no blueprint defined for this task");
            Ok(())
        }
    }
}

/// run a task using the blueprint engine (parse → transpile → execute)
async fn run_blueprint_task(
    client: &LighthouseAPIClient,
    lab_slug: &str,
    task: &Task,
    bp_source: &str,
    state_ctx: Option<(&mut LabState, &str)>,
    workspace: Option<PathBuf>,
) -> Result<()> {
    let ui = RunUI::new(&task.slug, 0);

    if task.status.is_completed() {
        complain!("you've already passed this task");
        say!("running blueprint anyway for verification...");
    }

    ui.header();
    ui.blank_line();

    // prologue
    if !task.prologue.is_empty() {
        ui.step(&format!(
            "Running {} setup commands...",
            task.prologue.len()
        ));
        if let Err((cmd, result)) = shell::run_commands(&task.prologue).await {
            oops!("setup command failed: {}", cmd);
            if !result.stderr.is_empty() {
                say!("stderr: {}", result.stderr.trim());
            }
            run_epilogue(&ui, &task.epilogue).await;
            return Ok(());
        }
        ui.blank_line();
    }

    ui.step("Running blueprint...");

    let bp_result = match blueprint_runner::run_validate(bp_source, &task.slug, workspace).await {
        Ok(r) => r,
        Err(err) => {
            oops!("blueprint failed: {}", err);
            run_epilogue(&ui, &task.epilogue).await;
            return Ok(());
        }
    };

    CliReporter::print_result(&bp_result);

    // submit attempt
    let attempt_request = blueprint_runner::to_attempt_request(&bp_result, lab_slug, task.id);
    submit_and_update(client, &attempt_request, &ui, task, state_ctx).await;

    run_epilogue(&ui, &task.epilogue).await;
    Ok(())
}

/// submit attempt to API and update local state cache
pub async fn submit_and_update(
    client: &LighthouseAPIClient,
    attempt_request: &SubmitAttemptRequest,
    ui: &RunUI,
    task: &Task,
    state_ctx: Option<(&mut LabState, &str)>,
) {
    match client.submit_attempt(attempt_request).await {
        Ok(response) => {
            log::debug!("attempt recorded: {:?}", response);
            if response.data.is_reattempt {
                log::debug!("re-attempt recorded (no additional points)");
            } else if response.data.task_outcome == "passed" {
                ui.points_earned(response.data.points_achieved);
            }

            if let Some((state, token)) = state_ctx {
                let new_status = if response.data.task_outcome == "passed" {
                    TaskStatus::ChallengeCompleted
                } else {
                    TaskStatus::Challenged
                };
                state.update_task_status(task.id, new_status);
                if let Err(e) = state.save(token) {
                    log::warn!("failed to save state: {}", e);
                }
            }
        }
        Err(err) => {
            log::error!("failed to submit attempt: {}", err);
            oops!("failed to submit results: {}", err);
        }
    }
}

/// run epilogue commands with best-effort (continues even on failure)
async fn run_epilogue(ui: &RunUI, commands: &[String]) {
    if commands.is_empty() {
        return;
    }

    ui.blank_line();
    ui.step(&format!("Running {} cleanup commands...", commands.len()));

    let failures = shell::run_commands_best_effort(commands).await;
    for (cmd, result) in failures {
        log::warn!(
            "cleanup command failed: {} (exit {})",
            cmd,
            result.exit_code
        );
        if !result.stderr.is_empty() {
            log::debug!("stderr: {}", result.stderr.trim());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{TaskInputType, TaskStatus};

    fn make_task_with_hooks(
        prologue: Vec<String>,
        epilogue: Vec<String>,
        validators: Vec<String>,
    ) -> Task {
        Task {
            id: 1,
            uuid: String::new(),
            slug: "test-task".to_string(),
            title: "Test Task".to_string(),
            description: "A test task".to_string(),
            sort_order: 1,
            input_type: TaskInputType::None,
            scores: "10:20:50".to_string(),
            status: TaskStatus::ChallengeAwaits,
            is_free: false,
            is_locked: false,
            is_paid: false,
            abandoned_deduction: 5,
            points_earned: 0,
            hints: vec![],
            validators,
            blueprint: None,
            prologue,
            epilogue,
        }
    }

    #[test]
    fn test_task_with_empty_hooks() {
        let task = make_task_with_hooks(vec![], vec![], vec![]);
        assert!(task.prologue.is_empty());
        assert!(task.epilogue.is_empty());
    }

    #[test]
    fn test_task_with_prologue_and_epilogue() {
        let task = make_task_with_hooks(
            vec!["docker compose up -d".to_string()],
            vec!["docker compose down".to_string()],
            vec!["tcp_listening:int(8080)".to_string()],
        );

        assert_eq!(task.prologue.len(), 1);
        assert_eq!(task.epilogue.len(), 1);
        assert_eq!(task.prologue[0], "docker compose up -d");
        assert_eq!(task.epilogue[0], "docker compose down");
    }

    #[tokio::test]
    async fn test_prologue_stops_on_failure() {
        let commands = vec![
            "echo starting".to_string(),
            "exit 1".to_string(),
            "echo should not run".to_string(),
        ];

        let result = shell::run_commands(&commands).await;
        assert!(result.is_err());

        let (failed_cmd, _) = result.unwrap_err();
        assert_eq!(failed_cmd, "exit 1");
    }

    #[tokio::test]
    async fn test_epilogue_continues_on_failure() {
        let commands = vec![
            "exit 1".to_string(),
            "exit 2".to_string(),
            "echo still runs".to_string(),
        ];

        // best_effort continues even when commands fail
        let failures = shell::run_commands_best_effort(&commands).await;

        // should have 2 failures (exit 1 and exit 2)
        assert_eq!(failures.len(), 2);
    }

    #[tokio::test]
    async fn test_prologue_success_allows_continuation() {
        let commands = vec!["echo one".to_string(), "echo two".to_string()];

        let result = shell::run_commands(&commands).await;
        assert!(result.is_ok());
    }
}
