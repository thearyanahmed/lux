use std::path::PathBuf;

use color_eyre::eyre::Result;

use crate::api::LighthouseAPIClient;
use crate::api::Task;
use crate::commands::run::run_task_validators;
use crate::config::Config;
use crate::state::LabState;
use crate::ui::RunUI;
use crate::{oops, say};

/// result of filtering tasks for validation
#[derive(Debug)]
pub struct FilteredTasks<'a> {
    pub to_run: Vec<&'a Task>,
    pub skipped_completed: usize,
    /// sequential lock: previous task not completed
    pub skipped_locked: usize,
    /// payment lock: requires voyager subscription
    pub skipped_paid: usize,
}

/// filter tasks based on locked status and completion
/// - paid tasks (requires subscription) are always skipped
/// - locked tasks (sequential) are always skipped
/// - completed tasks are skipped unless include_passed is true
pub fn filter_tasks_for_validation<'a>(
    tasks: &'a [Task],
    include_passed: bool,
) -> FilteredTasks<'a> {
    let mut to_run = Vec::new();
    let mut skipped_completed = 0;
    let mut skipped_locked = 0;
    let mut skipped_paid = 0;

    for task in tasks {
        let is_completed = task.status.is_completed();

        // payment lock takes priority (requires subscription)
        if task.is_paid {
            skipped_paid += 1;
            continue;
        }

        // sequential lock (previous task not done)
        if task.is_locked {
            skipped_locked += 1;
            continue;
        }

        if is_completed && !include_passed {
            skipped_completed += 1;
            continue;
        }

        to_run.push(task);
    }

    FilteredTasks {
        to_run,
        skipped_completed,
        skipped_locked,
        skipped_paid,
    }
}

/// handle `luxctl validate [--all] [--detailed]`
pub async fn validate_all(include_passed: bool, _detailed: bool) -> Result<()> {
    let config = Config::load()?;
    if !config.has_auth_token() {
        oops!("not authenticated. Run: `luxctl auth --token $token`");
        return Ok(());
    }

    let token = config.expose_token().to_string();
    let mut state = LabState::load(&token)?;

    let active = if let Some(l) = state.get_active() {
        l.clone()
    } else {
        oops!("no active lab");
        say!("run `luxctl lab start --id <ID>` first");
        return Ok(());
    };

    let client = LighthouseAPIClient::from_config(&config);

    // fetch fresh lab data
    let lab = match client.lab_by_slug(&active.slug).await {
        Ok(l) => l,
        Err(err) => {
            oops!("failed to fetch lab: {}", err);
            return Ok(());
        }
    };

    let tasks = if let Some(t) = &lab.tasks {
        t
    } else {
        oops!("lab has no tasks");
        return Ok(());
    };

    // update cache with fresh data
    state.refresh_tasks(tasks);
    state.save(&token)?;

    // filter tasks
    let filtered = filter_tasks_for_validation(tasks, include_passed);

    say!("validating tasks for: {}", lab.name);

    if filtered.skipped_completed > 0 {
        say!(
            "skipping {} completed task(s). Use --all to include them.",
            filtered.skipped_completed
        );
    }

    if filtered.to_run.is_empty() {
        say!("no tasks to validate");
        return Ok(());
    }

    let total_tasks = filtered.to_run.len();

    // run each task
    for (i, task) in filtered.to_run.iter().enumerate() {
        // blueprint tasks have 0 legacy validators; count doesn't affect separator display
        let validator_count = if task.has_blueprint() { 0 } else { task.validators.len() };
        let ui = RunUI::new(&task.slug, validator_count);
        println!();
        ui.task_separator(i + 1, total_tasks, &task.slug);

        // run validators and submit results (pass state for auto-refresh)
        let workspace = state
            .get_active()
            .map(|l| PathBuf::from(&l.workspace));

        run_task_validators(
            &client,
            &lab.slug,
            task,
            Some((&mut state, &token)),
            workspace,
        )
        .await?;
    }

    // print summary
    println!();
    println!("  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    say!("  summary");
    say!("    ran: {} task(s)", filtered.to_run.len());
    if filtered.skipped_completed > 0 {
        say!("    skipped: {} (completed)", filtered.skipped_completed);
    }
    if filtered.skipped_locked > 0 {
        say!(
            "    skipped: {} (previous task incomplete)",
            filtered.skipped_locked
        );
    }
    if filtered.skipped_paid > 0 {
        say!(
            "    skipped: {} (requires subscription)",
            filtered.skipped_paid
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{TaskInputType, TaskStatus};

    fn make_task(id: i32, slug: &str, status: TaskStatus, is_locked: bool) -> Task {
        make_task_full(id, slug, status, is_locked, false)
    }

    fn make_paid_task(id: i32, slug: &str, status: TaskStatus) -> Task {
        make_task_full(id, slug, status, false, true)
    }

    fn make_task_full(
        id: i32,
        slug: &str,
        status: TaskStatus,
        is_locked: bool,
        is_paid: bool,
    ) -> Task {
        Task {
            id,
            uuid: String::new(),
            slug: slug.to_string(),
            title: format!("Task {}", id),
            description: "Test task".to_string(),
            sort_order: id,
            input_type: TaskInputType::None,
            scores: "10:20:50".to_string(),
            status,
            is_free: false,
            is_locked,
            is_paid,
            abandoned_deduction: 5,
            points_earned: 0,
            hints: vec![],
            validators: vec![],
            blueprint: None,
            prologue: vec![],
            epilogue: vec![],
        }
    }

    #[test]
    fn test_filter_skips_locked_tasks() {
        let tasks = vec![
            make_task(1, "task-1", TaskStatus::ChallengeAwaits, false),
            make_task(2, "task-2", TaskStatus::ChallengeAwaits, true), // locked
            make_task(3, "task-3", TaskStatus::ChallengeAwaits, true), // locked
        ];

        let result = filter_tasks_for_validation(&tasks, false);

        assert_eq!(result.to_run.len(), 1);
        assert_eq!(result.to_run[0].slug, "task-1");
        assert_eq!(result.skipped_locked, 2);
        assert_eq!(result.skipped_completed, 0);
    }

    #[test]
    fn test_filter_skips_completed_tasks_by_default() {
        let tasks = vec![
            make_task(1, "task-1", TaskStatus::ChallengeCompleted, false),
            make_task(2, "task-2", TaskStatus::ChallengeAwaits, false),
        ];

        let result = filter_tasks_for_validation(&tasks, false);

        assert_eq!(result.to_run.len(), 1);
        assert_eq!(result.to_run[0].slug, "task-2");
        assert_eq!(result.skipped_completed, 1);
    }

    #[test]
    fn test_filter_includes_completed_when_include_passed_true() {
        let tasks = vec![
            make_task(1, "task-1", TaskStatus::ChallengeCompleted, false),
            make_task(2, "task-2", TaskStatus::ChallengeAwaits, false),
        ];

        let result = filter_tasks_for_validation(&tasks, true);

        assert_eq!(result.to_run.len(), 2);
        assert_eq!(result.skipped_completed, 0);
    }

    #[test]
    fn test_filter_locked_takes_priority_over_completed() {
        // locked task that is also completed should be skipped as locked, not completed
        let tasks = vec![
            make_task(1, "task-1", TaskStatus::ChallengeCompleted, true), // locked AND completed
        ];

        let result = filter_tasks_for_validation(&tasks, false);

        assert_eq!(result.to_run.len(), 0);
        assert_eq!(result.skipped_locked, 1);
        assert_eq!(result.skipped_completed, 0); // not counted as completed skip
    }

    #[test]
    fn test_filter_empty_tasks() {
        let tasks: Vec<Task> = vec![];

        let result = filter_tasks_for_validation(&tasks, false);

        assert!(result.to_run.is_empty());
        assert_eq!(result.skipped_locked, 0);
        assert_eq!(result.skipped_completed, 0);
    }

    #[test]
    fn test_filter_all_unlocked_incomplete() {
        let tasks = vec![
            make_task(1, "task-1", TaskStatus::ChallengeAwaits, false),
            make_task(2, "task-2", TaskStatus::Challenged, false),
            make_task(3, "task-3", TaskStatus::ChallengeFailed, false),
        ];

        let result = filter_tasks_for_validation(&tasks, false);

        assert_eq!(result.to_run.len(), 3);
        assert_eq!(result.skipped_locked, 0);
        assert_eq!(result.skipped_completed, 0);
        assert_eq!(result.skipped_paid, 0);
    }

    #[test]
    fn test_filter_skips_paid_tasks() {
        let tasks = vec![
            make_task(1, "task-1", TaskStatus::ChallengeAwaits, false),
            make_paid_task(2, "task-2", TaskStatus::ChallengeAwaits), // paid
            make_paid_task(3, "task-3", TaskStatus::ChallengeAwaits), // paid
        ];

        let result = filter_tasks_for_validation(&tasks, false);

        assert_eq!(result.to_run.len(), 1);
        assert_eq!(result.to_run[0].slug, "task-1");
        assert_eq!(result.skipped_paid, 2);
        assert_eq!(result.skipped_locked, 0);
        assert_eq!(result.skipped_completed, 0);
    }

    #[test]
    fn test_filter_paid_takes_priority_over_locked() {
        // task that is both paid AND locked should be counted as paid (payment takes priority)
        let mut task = make_paid_task(1, "task-1", TaskStatus::ChallengeAwaits);
        task.is_locked = true;
        let tasks = vec![task];

        let result = filter_tasks_for_validation(&tasks, false);

        assert_eq!(result.to_run.len(), 0);
        assert_eq!(result.skipped_paid, 1);
        assert_eq!(result.skipped_locked, 0); // not counted as locked
    }

    #[test]
    fn test_filter_mixed_paid_locked_completed() {
        let tasks = vec![
            make_task(1, "task-1", TaskStatus::ChallengeCompleted, false), // completed
            make_paid_task(2, "task-2", TaskStatus::ChallengeAwaits),      // paid
            make_task(3, "task-3", TaskStatus::ChallengeAwaits, true),     // locked
            make_task(4, "task-4", TaskStatus::ChallengeAwaits, false),    // available
        ];

        let result = filter_tasks_for_validation(&tasks, false);

        assert_eq!(result.to_run.len(), 1);
        assert_eq!(result.to_run[0].slug, "task-4");
        assert_eq!(result.skipped_paid, 1);
        assert_eq!(result.skipped_locked, 1);
        assert_eq!(result.skipped_completed, 1);
    }
}
