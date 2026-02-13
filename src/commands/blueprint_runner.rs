use std::collections::HashMap;

use color_eyre::eyre::{Result, WrapErr};

use blueprint::executor::context::{Context, ExecutionMode};
use blueprint::executor::Engine;
use blueprint::parser::parse;
use blueprint::reporter::format_api_payload;
use blueprint::transpiler::{transpile, BlueprintResult, Status};

use crate::api::{SubmitAttemptRequest, Task, TaskOutcome};

/// which validation system a task uses
pub enum TaskSystem<'a> {
    Blueprint(&'a str),
    Legacy(&'a [String]),
    None,
}

/// inspect task fields to decide which system to use.
/// blueprint takes priority — if both are present, blueprint wins.
pub fn detect_system(task: &Task) -> TaskSystem<'_> {
    if let Some(ref bp) = task.blueprint {
        if !bp.is_empty() {
            return TaskSystem::Blueprint(bp);
        }
    }
    if !task.validators.is_empty() {
        return TaskSystem::Legacy(&task.validators);
    }
    TaskSystem::None
}

/// run a blueprint in Validate mode (probe-only, no user inputs)
pub async fn run_validate(bp_source: &str, task_slug: &str) -> Result<BlueprintResult> {
    let ast = parse(bp_source)
        .map_err(|e| color_eyre::eyre::eyre!("parse error at line {}: {}", e.line, e.message))?;

    let bp = transpile(&ast).map_err(|e| {
        let msg = match e.context {
            Some(ctx) => format!("{}: {}", ctx, e.message),
            None => e.message,
        };
        color_eyre::eyre::eyre!("transpile error: {}", msg)
    })?;

    let ctx = Context::new(bp.config.clone(), ExecutionMode::Validate);
    let mut engine = Engine::new(ctx).with_task(task_slug);

    let result = engine
        .execute(&bp)
        .await
        .wrap_err("blueprint execution failed")?;

    Ok(result)
}

/// run a blueprint in Result mode (with user inputs for input-matching steps)
pub async fn run_result(
    bp_source: &str,
    task_slug: &str,
    user_inputs: &HashMap<String, String>,
) -> Result<BlueprintResult> {
    let ast = parse(bp_source)
        .map_err(|e| color_eyre::eyre::eyre!("parse error at line {}: {}", e.line, e.message))?;

    let bp = transpile(&ast).map_err(|e| {
        let msg = match e.context {
            Some(ctx) => format!("{}: {}", ctx, e.message),
            None => e.message,
        };
        color_eyre::eyre::eyre!("transpile error: {}", msg)
    })?;

    let mut ctx = Context::new(bp.config.clone(), ExecutionMode::Result);
    for (key, value) in user_inputs {
        ctx.set_user_input(key, value);
    }

    let mut engine = Engine::new(ctx).with_task(task_slug);

    let result = engine
        .execute(&bp)
        .await
        .wrap_err("blueprint execution failed")?;

    Ok(result)
}

/// convert a BlueprintResult into a SubmitAttemptRequest for the API
pub fn to_attempt_request(
    result: &BlueprintResult,
    lab_slug: &str,
    task_id: i32,
) -> SubmitAttemptRequest {
    let outcome = match &result.status {
        Status::Passed => TaskOutcome::Passed,
        Status::Failed => TaskOutcome::Failed,
        _ => TaskOutcome::Failed,
    };

    let payload = format_api_payload(result, None);
    // truncate if too long (API limit ~5000 chars)
    let json = serde_json::to_string(&payload).unwrap_or_default();
    let context = if json.len() > 4900 {
        format!("{}...[truncated]", &json[..4900])
    } else {
        json
    };

    SubmitAttemptRequest {
        lab_slug: lab_slug.to_string(),
        task_id,
        task_outcome: outcome,
        points_achieved: None,
        task_outcome_context: Some(context),
    }
}

/// parse `key=value` CLI args into a HashMap.
/// returns error if any arg is missing the `=` separator.
pub fn parse_inputs(raw: &[String]) -> Result<HashMap<String, String>> {
    let mut map = HashMap::new();
    for item in raw {
        let (key, value) = item
            .split_once('=')
            .ok_or_else(|| color_eyre::eyre::eyre!("invalid input '{}': expected key=value", item))?;
        map.insert(key.to_string(), value.to_string());
    }
    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{TaskInputType, TaskStatus};

    fn make_task(validators: Vec<String>, blueprint: Option<String>) -> Task {
        Task {
            id: 1,
            uuid: String::new(),
            slug: "test-task".to_string(),
            title: "Test".to_string(),
            description: "test".to_string(),
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
            blueprint,
            prologue: vec![],
            epilogue: vec![],
        }
    }

    #[test]
    fn test_detect_blueprint() {
        let task = make_task(vec![], Some("blueprint \"test\" {}".to_string()));
        assert!(matches!(detect_system(&task), TaskSystem::Blueprint(_)));
    }

    #[test]
    fn test_detect_legacy() {
        let task = make_task(vec!["tcp_listening:int(8080)".to_string()], None);
        assert!(matches!(detect_system(&task), TaskSystem::Legacy(_)));
    }

    #[test]
    fn test_detect_none() {
        let task = make_task(vec![], None);
        assert!(matches!(detect_system(&task), TaskSystem::None));
    }

    #[test]
    fn test_blueprint_takes_priority_over_legacy() {
        let task = make_task(
            vec!["tcp_listening:int(8080)".to_string()],
            Some("blueprint \"test\" {}".to_string()),
        );
        assert!(matches!(detect_system(&task), TaskSystem::Blueprint(_)));
    }

    #[test]
    fn test_empty_blueprint_falls_through_to_legacy() {
        let task = make_task(vec!["tcp_listening:int(8080)".to_string()], Some(String::new()));
        assert!(matches!(detect_system(&task), TaskSystem::Legacy(_)));
    }

    #[test]
    fn test_parse_inputs_valid() {
        let raw = vec!["key=value".to_string(), "foo=bar".to_string()];
        let result = parse_inputs(&raw).unwrap();
        assert_eq!(result.get("key").unwrap(), "value");
        assert_eq!(result.get("foo").unwrap(), "bar");
    }

    #[test]
    fn test_parse_inputs_with_equals_in_value() {
        let raw = vec!["key=value=extra".to_string()];
        let result = parse_inputs(&raw).unwrap();
        assert_eq!(result.get("key").unwrap(), "value=extra");
    }

    #[test]
    fn test_parse_inputs_missing_equals() {
        let raw = vec!["noequalssign".to_string()];
        let result = parse_inputs(&raw);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_inputs_empty() {
        let raw: Vec<String> = vec![];
        let result = parse_inputs(&raw).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_to_attempt_request_passed() {
        let bp_result = BlueprintResult {
            name: "test".to_string(),
            status: Status::Passed,
            phases: vec![],
            duration_ms: 100,
            captured: HashMap::new(),
            input_provided: HashMap::new(),
        };

        let req = to_attempt_request(&bp_result, "my-lab", 42);
        assert_eq!(req.lab_slug, "my-lab");
        assert_eq!(req.task_id, 42);
        assert!(matches!(req.task_outcome, TaskOutcome::Passed));
        assert!(req.task_outcome_context.is_some());
    }

    #[test]
    fn test_to_attempt_request_failed() {
        let bp_result = BlueprintResult {
            name: "test".to_string(),
            status: Status::Failed,
            phases: vec![],
            duration_ms: 50,
            captured: HashMap::new(),
            input_provided: HashMap::new(),
        };

        let req = to_attempt_request(&bp_result, "my-lab", 1);
        assert!(matches!(req.task_outcome, TaskOutcome::Failed));
    }

    #[tokio::test]
    async fn test_run_validate_with_file_probe() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("hello.txt");
        std::fs::write(&file_path, "hello world").unwrap();

        // file probe path is unquoted in the DSL
        let bp_source = format!(
            r#"blueprint "file test" {{
    phase "check" {{
        step "file exists" {{
            probe file {}
            expect {{ exists: true }}
        }}
    }}
}}"#,
            file_path.display()
        );

        let result = run_validate(&bp_source, "test-task").await.unwrap();
        assert!(matches!(result.status, Status::Passed));
    }

    #[tokio::test]
    async fn test_run_validate_fails_on_missing_file() {
        let bp_source = r#"blueprint "missing" {
    phase "check" {
        step "file exists" {
            probe file "/tmp/luxctl-nonexistent-file-test"
            expect { exists: true }
        }
    }
}"#;

        let result = run_validate(bp_source, "test-task").await.unwrap();
        assert!(matches!(result.status, Status::Failed));
    }

    #[tokio::test]
    async fn test_run_validate_parse_error() {
        let result = run_validate("not valid blueprint syntax {{{", "test-task").await;
        assert!(result.is_err());
    }
}
