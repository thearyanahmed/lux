use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use blueprint::reporter::CliReporter;
use color_eyre::eyre::{Result, WrapErr};

use crate::api::{LighthouseAPIClient, SubmitAttemptRequest, Task, TaskStatus, Terminal};
use crate::commands::blueprint_runner::{self, TaskSystem};
use crate::config::Config;
use crate::shell;
use crate::state::ProjectState;
use crate::ui::RunUI;
use crate::{complain, oops, say};

/// handle `luxctl run --task <slug|number> [--project <slug>]`
/// task can be specified by slug or by number (1, 01, 2, 02, etc.)
pub async fn run(task_id: &str, project_slug: Option<&str>, detailed: bool) -> Result<()> {
    let config = Config::load()?;
    if !config.has_auth_token() {
        oops!("not authenticated. Run: `luxctl auth --token $token`");
        return Ok(());
    }

    let token = config.expose_token().to_string();
    let mut state = ProjectState::load(&token)?;
    let client = LighthouseAPIClient::from_config(&config);

    // determine project slug (from arg or active project)
    let project_slug = match project_slug {
        Some(s) => s.to_string(),
        None => {
            if let Some(l) = state.get_active() {
                l.slug.clone()
            } else {
                oops!("no project specified and no active project");
                say!("use `--project <ID>` or run `luxctl project start --id <ID>` first");
                return Ok(());
            }
        }
    };

    // fetch project with tasks
    let project_data = match client.project_by_slug(&project_slug).await {
        Ok(l) => l,
        Err(err) => {
            oops!("failed to fetch project '{}': {}", project_slug, err);
            return Ok(());
        }
    };

    // get tasks list
    let tasks = if let Some(t) = &project_data.tasks {
        t
    } else {
        oops!("project '{}' has no tasks", project_slug);
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
            oops!("task '{}' not found in project '{}'", task_id, project_slug);
            say!("use task number (1, 2, 3...) or slug:");
            for (i, t) in tasks.iter().enumerate() {
                say!("  {:02}. {}", i + 1, t.slug);
            }
            return Ok(());
        }
    };

    let active = state.get_active();
    let workspace = active.map(|l| PathBuf::from(&l.workspace));
    let runtime = active.and_then(|l| l.runtime.clone());

    // collect slugs of previously completed tasks so the reporter
    // can distinguish "already passed" from "never attempted"
    let completed_slugs: HashSet<String> = tasks
        .iter()
        .filter(|t| t.status.is_completed())
        .map(|t| t.slug.clone())
        .collect();

    run_task_validators(
        &client,
        &project_data.slug,
        task_data,
        Some((&mut state, &token)),
        workspace,
        detailed,
        &completed_slugs,
        runtime.as_deref(),
    )
    .await
}

/// run blueprint for a single task and submit results.
/// optionally updates cached state when state_ctx is provided.
pub async fn run_task_validators(
    client: &LighthouseAPIClient,
    project_slug: &str,
    task: &Task,
    state_ctx: Option<(&mut ProjectState, &str)>,
    workspace: Option<PathBuf>,
    detailed: bool,
    completed_slugs: &HashSet<String>,
    runtime: Option<&str>,
) -> Result<()> {
    match blueprint_runner::detect_system(task) {
        TaskSystem::Blueprint(source) => {
            run_blueprint_task(
                client,
                project_slug,
                task,
                source,
                state_ctx,
                workspace,
                detailed,
                completed_slugs,
                runtime,
            )
            .await
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
    project_slug: &str,
    task: &Task,
    bp_source: &str,
    state_ctx: Option<(&mut ProjectState, &str)>,
    workspace: Option<PathBuf>,
    detailed: bool,
    completed_slugs: &HashSet<String>,
    runtime: Option<&str>,
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

    let bp_result =
        match blueprint_runner::run_validate(bp_source, &task.slug, workspace, runtime).await {
            Ok(r) => r,
            Err(err) => {
                oops!("blueprint failed: {}", err);
                run_epilogue(&ui, &task.epilogue).await;
                return Ok(());
            }
        };

    // submit before printing so we can show XP on the summary line
    let attempt_request = blueprint_runner::to_attempt_request(&bp_result, project_slug, task.id);
    let points = submit_and_update(client, &attempt_request, task, state_ctx).await;

    CliReporter::print_result_with_context(&bp_result, detailed, completed_slugs, points);

    run_epilogue(&ui, &task.epilogue).await;
    Ok(())
}

/// submit attempt to API and update local state cache.
/// returns points earned on first-time pass, None otherwise.
pub async fn submit_and_update(
    client: &LighthouseAPIClient,
    attempt_request: &SubmitAttemptRequest,
    task: &Task,
    state_ctx: Option<(&mut ProjectState, &str)>,
) -> Option<i32> {
    match client.submit_attempt(attempt_request).await {
        Ok(response) => {
            log::debug!("attempt recorded: {:?}", response);

            let points = if response.data.is_reattempt {
                log::debug!("re-attempt recorded (no additional points)");
                None
            } else if response.data.task_outcome == "passed" {
                Some(response.data.points_achieved)
            } else {
                None
            };

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

            points
        }
        Err(err) => {
            log::error!("failed to submit attempt: {}", err);
            oops!("failed to submit results: {}", err);
            None
        }
    }
}

/// run a terminal: inject test_files → run blueprint → clean up.
/// test files are written to workspace just before execution and removed after,
/// preventing users from reading them to cheat.
pub async fn run_terminal(
    terminal: &Terminal,
    workspace: &Path,
    lang: Option<&str>,
    detailed: bool,
) -> Result<()> {
    let ui = RunUI::new(&terminal.slug, 0);
    ui.header();
    ui.blank_line();

    let bp_source = match &terminal.blueprint {
        Some(bp) if !bp.is_empty() => bp,
        _ => {
            oops!("no blueprint defined for terminal '{}'", terminal.slug);
            return Ok(());
        }
    };

    let test_files = terminal.test_files_for_lang(lang);
    let written_paths = write_test_files(workspace, &test_files)?;

    ui.step("Running blueprint...");

    let bp_result = blueprint_runner::run_validate(
        bp_source,
        &terminal.slug,
        Some(workspace.to_path_buf()),
        None,
    )
    .await;

    // always clean up test files, even if blueprint failed
    cleanup_test_files(&written_paths);

    let bp_result = bp_result?;

    CliReporter::print_result(&bp_result, detailed);

    Ok(())
}

/// write test files to workspace. returns the absolute paths of files written
/// so they can be cleaned up after execution.
fn write_test_files(workspace: &Path, files: &HashMap<String, String>) -> Result<Vec<PathBuf>> {
    let mut written = Vec::new();

    for (relative_path, content) in files {
        let target = workspace.join(relative_path);

        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).wrap_err_with(|| {
                format!(
                    "failed to create directory for test file: {}",
                    parent.display()
                )
            })?;
        }

        std::fs::write(&target, content)
            .wrap_err_with(|| format!("failed to write test file: {}", target.display()))?;

        written.push(target);
    }

    Ok(written)
}

/// remove injected test files after blueprint execution
fn cleanup_test_files(paths: &[PathBuf]) {
    for path in paths {
        if let Err(e) = std::fs::remove_file(path) {
            log::warn!("failed to clean up test file {}: {}", path.display(), e);
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

    #[test]
    fn test_write_test_files_creates_files() {
        let dir = tempfile::tempdir().unwrap();
        let mut files = HashMap::new();
        files.insert(
            "lru-cache/lru_cache_test.go".to_string(),
            "package lru_cache\n".to_string(),
        );

        let written = write_test_files(dir.path(), &files).unwrap();

        assert_eq!(written.len(), 1);
        let content = std::fs::read_to_string(&written[0]).unwrap();
        assert_eq!(content, "package lru_cache\n");
    }

    #[test]
    fn test_write_test_files_creates_nested_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let mut files = HashMap::new();
        files.insert("deep/nested/dir/test.go".to_string(), "test\n".to_string());

        let written = write_test_files(dir.path(), &files).unwrap();

        assert!(dir.path().join("deep/nested/dir/test.go").exists());
        assert_eq!(written.len(), 1);
    }

    #[test]
    fn test_cleanup_test_files_removes_files() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.go");
        std::fs::write(&file_path, "content").unwrap();
        assert!(file_path.exists());

        cleanup_test_files(&[file_path.clone()]);

        assert!(!file_path.exists());
    }

    #[test]
    fn test_cleanup_test_files_handles_missing_files() {
        // should not panic when file doesn't exist
        let path = PathBuf::from("/tmp/luxctl-nonexistent-cleanup-test");
        cleanup_test_files(&[path]);
    }

    #[test]
    fn test_write_empty_test_files() {
        let dir = tempfile::tempdir().unwrap();
        let files = HashMap::new();

        let written = write_test_files(dir.path(), &files).unwrap();
        assert!(written.is_empty());
    }
}
