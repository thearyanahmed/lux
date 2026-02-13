use crate::transpiler::ir::*;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
pub struct ApiPayload {
    pub task_id: Option<String>,
    pub status: String,
    pub captured: HashMap<String, String>,
    pub input_provided: HashMap<String, String>,
    pub steps: Vec<ApiStepPayload>,
    pub duration_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct ApiStepPayload {
    pub name: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_matched: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    pub captured: HashMap<String, String>,
    pub duration_ms: u64,
}

/// format a BlueprintResult into the API submission payload
pub fn format_api_payload(result: &BlueprintResult, task_id: Option<&str>) -> ApiPayload {
    let mut all_steps = Vec::new();

    for phase in &result.phases {
        for step in &phase.steps {
            if step.status == Status::Skipped {
                continue;
            }

            let mut step_captured = HashMap::new();
            for (k, v) in &step.captures {
                step_captured.insert(k.clone(), v.to_string());
            }

            let message = step
                .expectations
                .iter()
                .find(|e| e.status == Status::Failed)
                .and_then(|e| e.message.clone());

            all_steps.push(ApiStepPayload {
                name: step.name.clone(),
                status: status_to_string(&step.status),
                input_matched: step.input_matched,
                message,
                captured: step_captured,
                duration_ms: step.duration_ms,
            });
        }
    }

    let captured: HashMap<String, String> = result
        .captured
        .iter()
        .map(|(k, v)| (k.clone(), v.to_string()))
        .collect();

    ApiPayload {
        task_id: task_id.map(|s| s.to_string()),
        status: status_to_string(&result.status),
        captured,
        input_provided: result.input_provided.clone(),
        steps: all_steps,
        duration_ms: result.duration_ms,
    }
}

fn status_to_string(status: &Status) -> String {
    match status {
        Status::Passed => "passed".to_string(),
        Status::Failed => "failed".to_string(),
        Status::Skipped => "skipped".to_string(),
        Status::Error(msg) => format!("error: {msg}"),
    }
}

/// serialize the payload as JSON string
pub fn to_json(payload: &ApiPayload) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_passed_payload() {
        let result = BlueprintResult {
            name: "Test".to_string(),
            status: Status::Passed,
            phases: vec![PhaseResult {
                name: "test".to_string(),
                status: Status::Passed,
                steps: vec![StepResult {
                    name: "check".to_string(),
                    status: Status::Passed,
                    expectations: Vec::new(),
                    captures: vec![("$id".to_string(), Value::String("abc".into()))],
                    input_matched: None,
                    duration_ms: 50,
                    retry_count: 0,
                }],
                duration_ms: 50,
            }],
            duration_ms: 50,
            captured: {
                let mut m = HashMap::new();
                m.insert("$id".to_string(), Value::String("abc".into()));
                m
            },
            input_provided: HashMap::new(),
        };

        let payload = format_api_payload(&result, Some("task-123"));
        assert_eq!(payload.status, "passed");
        assert_eq!(payload.task_id, Some("task-123".to_string()));
        assert_eq!(payload.steps.len(), 1);
        assert_eq!(payload.steps[0].name, "check");

        // should serialize to JSON without error
        let json = to_json(&payload);
        assert!(json.is_ok());
    }

    #[test]
    fn test_format_failed_with_input() {
        let result = BlueprintResult {
            name: "Test".to_string(),
            status: Status::Failed,
            phases: vec![PhaseResult {
                name: "test".to_string(),
                status: Status::Failed,
                steps: vec![StepResult {
                    name: "confirm id".to_string(),
                    status: Status::Failed,
                    expectations: vec![ExpectResult {
                        field: "$container-id".to_string(),
                        op: Op::Eq,
                        status: Status::Failed,
                        actual: Some(Value::String("wrong".into())),
                        expected_display: "abc123".to_string(),
                        message: Some("input mismatch".to_string()),
                    }],
                    captures: Vec::new(),
                    input_matched: Some(false),
                    duration_ms: 80,
                    retry_count: 0,
                }],
                duration_ms: 80,
            }],
            duration_ms: 80,
            captured: HashMap::new(),
            input_provided: {
                let mut m = HashMap::new();
                m.insert("container-id".to_string(), "wrong".to_string());
                m
            },
        };

        let payload = format_api_payload(&result, None);
        assert_eq!(payload.status, "failed");
        assert_eq!(payload.steps[0].input_matched, Some(false));
        assert!(payload.steps[0].message.is_some());
    }

    #[test]
    fn test_skipped_steps_excluded() {
        let result = BlueprintResult {
            name: "Test".to_string(),
            status: Status::Passed,
            phases: vec![PhaseResult {
                name: "test".to_string(),
                status: Status::Passed,
                steps: vec![
                    StepResult {
                        name: "run".to_string(),
                        status: Status::Passed,
                        expectations: Vec::new(),
                        captures: Vec::new(),
                        input_matched: None,
                        duration_ms: 50,
                        retry_count: 0,
                    },
                    StepResult {
                        name: "skipped".to_string(),
                        status: Status::Skipped,
                        expectations: Vec::new(),
                        captures: Vec::new(),
                        input_matched: None,
                        duration_ms: 0,
                        retry_count: 0,
                    },
                ],
                duration_ms: 50,
            }],
            duration_ms: 50,
            captured: HashMap::new(),
            input_provided: HashMap::new(),
        };

        let payload = format_api_payload(&result, None);
        assert_eq!(payload.steps.len(), 1);
    }
}
