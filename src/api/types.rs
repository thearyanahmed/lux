use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct ApiError {
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct HealthCheckResponse {
    pub status: String,
    pub app: Option<String>,
    pub version: Option<String>,
    pub environment: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub links: PaginationLinks,
    pub meta: PaginationMeta,
}

#[derive(Debug, Deserialize)]
pub struct PaginationLinks {
    pub first: Option<String>,
    pub last: Option<String>,
    pub prev: Option<String>,
    pub next: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PaginationMeta {
    pub current_page: i32,
    pub from: Option<i32>,
    pub last_page: i32,
    pub path: String,
    pub per_page: i32,
    pub to: Option<i32>,
    pub total: i32,
}

#[derive(Debug, Deserialize)]
pub struct ApiUser {
    pub id: i32,
    pub name: String,
    pub email: String,
    #[serde(default)]
    pub stats: Option<UserStats>,
}

#[derive(Debug, Deserialize)]
pub struct UserStats {
    pub projects_attempted: i32,
    pub tasks_completed: i32,
    pub total_xp: i32,
}

#[derive(Debug, Deserialize)]
pub struct LabStats {
    pub attempted_count: i32,
    pub succeed_count: i32,
    pub failed_count: i32,
}

#[derive(Debug, Deserialize)]
pub struct Lab {
    pub id: i32,
    #[serde(default)]
    pub uuid: String,
    pub slug: String,
    pub name: String,
    #[serde(default)]
    pub short_description: Option<String>,
    #[serde(default)]
    pub is_published: Option<bool>,
    #[serde(default)]
    pub is_featured: Option<bool>,
    #[serde(default)]
    pub show_tasks: Option<bool>,
    #[serde(default)]
    pub stats: Option<LabStats>,
    #[serde(default)]
    pub published_at: Option<String>,
    #[serde(default)]
    pub tasks_count: Option<i32>,
    #[serde(default)]
    pub runner_image: Option<String>,
    #[serde(default)]
    pub tasks: Option<Vec<Task>>,
}

impl Lab {
    pub fn url(&self) -> String {
        format!("https://projectlighthouse.io/labs/{}", self.slug)
    }
}

/// task input type (matches Laravel TaskInputType enum)
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskInputType {
    #[default]
    None,
    Text,
    Number,
    Select,
    Code,
    MultiSelect,
}

/// task progress status (matches Laravel TaskStatus enum)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    ChallengeAwaits,
    Challenged,
    ChallengeCompleted,
    ChallengeFailed,
    ChallengeAbandoned,
}

impl TaskStatus {
    pub fn is_completed(self) -> bool {
        self == TaskStatus::ChallengeCompleted
    }
}

#[derive(Debug, Deserialize)]
pub struct Task {
    pub id: i32,
    #[serde(default)]
    pub uuid: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub sort_order: i32,
    #[serde(default)]
    pub input_type: TaskInputType,
    pub scores: String,
    pub status: TaskStatus,
    #[serde(default)]
    pub is_free: bool,
    pub is_locked: bool,
    pub abandoned_deduction: i32,
    pub points_earned: i32,
    pub hints: Vec<Hint>,
    pub validators: Vec<String>,
    /// commands to run before validators (e.g., docker compose up)
    #[serde(default)]
    pub prologue: Vec<String>,
    /// commands to run after validators (e.g., docker compose down)
    #[serde(default)]
    pub epilogue: Vec<String>,
}

impl Task {
    /// check if this task accepts user input
    pub fn accepts_input(&self) -> bool {
        self.input_type != TaskInputType::None
    }
}

#[derive(Debug, Deserialize)]
pub struct Hint {
    pub id: i32,
    #[serde(default)]
    pub uuid: String,
    pub text: String,
    pub unlock_criteria: String,
    pub points_deduction: i32,
}

/// hint data from the hints API (includes unlock status)
#[derive(Debug, Deserialize)]
pub struct TaskHint {
    pub id: i32,
    pub uuid: String,
    pub text: Option<String>, // only present if unlocked
    pub points_deduction: i32,
    pub sort_order: i32,
    pub is_unlocked: bool,
    pub is_available: bool,
}

/// response wrapper for hints list
#[derive(Debug, Deserialize)]
pub struct HintsResponse {
    pub data: Vec<TaskHint>,
}

/// response from unlocking a hint
#[derive(Debug, Deserialize)]
pub struct UnlockHintResponse {
    pub data: UnlockedHintData,
    pub unlocked_at: String,
    pub points_deducted: i32,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct UnlockedHintData {
    pub id: i32,
    pub uuid: String,
    pub text: String,
    pub points_deduction: i32,
}

/// outcome values for task attempts
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskOutcome {
    Attempted,
    Passed,
    Failed,
}

impl std::fmt::Display for TaskOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskOutcome::Attempted => write!(f, "attempted"),
            TaskOutcome::Passed => write!(f, "passed"),
            TaskOutcome::Failed => write!(f, "failed"),
        }
    }
}

/// request body for submitting a task attempt
#[derive(Debug, Serialize)]
pub struct SubmitAttemptRequest {
    pub lab_slug: String,
    pub task_id: i32,
    pub task_outcome: TaskOutcome,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub points_achieved: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_outcome_context: Option<String>,
}

/// response from submitting a task attempt
#[derive(Debug, Deserialize)]
pub struct SubmitAttemptResponse {
    pub message: String,
    pub data: AttemptData,
}

#[derive(Debug, Deserialize)]
pub struct AttemptData {
    pub id: i32,
    pub task_id: i32,
    pub lab_id: i32,
    pub task_outcome: String,
    pub points_achieved: i32,
    pub is_reattempt: bool,
    pub created_at: String,
}

/// request body for submitting a task answer
#[derive(Debug, Serialize)]
pub struct SubmitAnswerRequest {
    pub answer: serde_json::Value,
}

impl SubmitAnswerRequest {
    pub fn text(answer: &str) -> Self {
        Self {
            answer: serde_json::Value::String(answer.to_string()),
        }
    }

    pub fn number(answer: f64) -> Self {
        Self {
            answer: serde_json::json!(answer),
        }
    }

    pub fn choices(answers: Vec<&str>) -> Self {
        Self {
            answer: serde_json::json!(answers),
        }
    }
}

/// response from submitting a task answer
#[derive(Debug, Deserialize)]
pub struct SubmitAnswerResponse {
    pub success: bool,
    pub valid: bool,
    pub message: String,
    #[serde(default)]
    pub points_earned: Option<i32>,
    #[serde(default)]
    pub attempts: Option<i32>,
    #[serde(default)]
    pub already_completed: Option<bool>,
}

/// response from restarting a lab
#[derive(Debug, Deserialize)]
pub struct RestartLabResponse {
    pub message: String,
    pub data: RestartLabData,
}

#[derive(Debug, Deserialize)]
pub struct RestartLabData {
    pub attempt_group_id: i32,
    pub created_at: String,
}

impl ApiUser {
    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_user_accessors() {
        let user = ApiUser {
            id: 42,
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            stats: None,
        };

        assert_eq!(user.id(), 42);
        assert_eq!(user.name(), "Test User");
    }

    #[test]
    fn test_lab_deserialize() {
        let json = r#"{
            "id": 2,
            "slug": "build-your-own-http-server",
            "name": "Build Your Own Server",
            "short_description": "Learn the fundamentals of web servers.",
            "is_published": true,
            "is_featured": false,
            "show_tasks": true,
            "stats": {
                "attempted_count": 10,
                "succeed_count": 5,
                "failed_count": 3
            },
            "published_at": "2025-01-15T00:00:00+00:00",
            "tasks_count": 9
        }"#;

        let lab: Lab = serde_json::from_str(json).unwrap();

        assert_eq!(lab.id, 2);
        assert_eq!(lab.slug, "build-your-own-http-server");
        assert_eq!(lab.name, "Build Your Own Server");
        assert_eq!(lab.is_published, Some(true));
        assert_eq!(lab.is_featured, Some(false));
        assert_eq!(lab.show_tasks, Some(true));
        let stats = lab.stats.unwrap();
        assert_eq!(stats.attempted_count, 10);
        assert_eq!(stats.succeed_count, 5);
        assert_eq!(stats.failed_count, 3);
        assert_eq!(lab.tasks_count, Some(9));
    }

    #[test]
    fn test_lab_with_null_published_at() {
        let json = r#"{
            "id": 1,
            "slug": "test-lab",
            "name": "Test Lab",
            "short_description": "A test lab",
            "is_published": false,
            "is_featured": false,
            "show_tasks": false,
            "stats": {
                "attempted_count": 0,
                "succeed_count": 0,
                "failed_count": 0
            },
            "published_at": null,
            "tasks_count": 0
        }"#;

        let lab: Lab = serde_json::from_str(json).unwrap();

        assert!(lab.published_at.is_none());
    }

    #[test]
    fn test_lab_detail_with_tasks() {
        let json = r#"{
            "id": 1,
            "slug": "build-your-own-git",
            "name": "Build Your Own Git",
            "runner_image": "local|go|rust|c",
            "tasks": [
                {
                    "id": 1,
                    "slug": "initialize-a-repository",
                    "title": "Initialize a Repository",
                    "description": "Create the .git directory structure.",
                    "sort_order": 1,
                    "scores": "5:10:50|10:20:35|20:30:20",
                    "status": "challenge_awaits",
                    "is_locked": false,
                    "abandoned_deduction": 5,
                    "points_earned": 0,
                    "hints": [
                        {
                            "id": 15,
                            "text": "Create the .git directory.",
                            "unlock_criteria": "10:3:A",
                            "points_deduction": 5
                        }
                    ],
                    "validators": ["can_compile:bool(true)"]
                }
            ]
        }"#;

        let lab: Lab = serde_json::from_str(json).unwrap();

        assert_eq!(lab.id, 1);
        assert_eq!(lab.slug, "build-your-own-git");
        assert_eq!(lab.runner_image, Some("local|go|rust|c".to_string()));

        let tasks = lab.tasks.unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "Initialize a Repository");
        assert_eq!(tasks[0].status, TaskStatus::ChallengeAwaits);
        assert_eq!(tasks[0].hints.len(), 1);
        assert_eq!(tasks[0].hints[0].text, "Create the .git directory.");
    }

    #[test]
    fn test_paginated_response_deserialize() {
        let json = r#"{
            "data": [
                {
                    "id": 1,
                    "slug": "lab-one",
                    "name": "Lab One",
                    "short_description": "First lab",
                    "is_published": true,
                    "is_featured": true,
                    "show_tasks": true,
                    "stats": {"attempted_count": 0, "succeed_count": 0, "failed_count": 0},
                    "published_at": "2025-01-15T00:00:00+00:00",
                    "tasks_count": 5
                },
                {
                    "id": 2,
                    "slug": "lab-two",
                    "name": "Lab Two",
                    "short_description": "Second lab",
                    "is_published": true,
                    "is_featured": false,
                    "show_tasks": false,
                    "stats": {"attempted_count": 1, "succeed_count": 1, "failed_count": 0},
                    "published_at": null,
                    "tasks_count": 3
                }
            ],
            "links": {
                "first": "http://example.com/api/v1/labs?page=1",
                "last": "http://example.com/api/v1/labs?page=2",
                "prev": null,
                "next": "http://example.com/api/v1/labs?page=2"
            },
            "meta": {
                "current_page": 1,
                "from": 1,
                "last_page": 2,
                "path": "http://example.com/api/v1/labs",
                "per_page": 15,
                "to": 15,
                "total": 21
            }
        }"#;

        let response: PaginatedResponse<Lab> = serde_json::from_str(json).unwrap();

        assert_eq!(response.data.len(), 2);
        assert_eq!(response.data[0].id, 1);
        assert_eq!(response.data[0].slug, "lab-one");
        assert_eq!(response.data[1].id, 2);
        assert_eq!(response.data[1].slug, "lab-two");

        assert_eq!(
            response.links.first,
            Some("http://example.com/api/v1/labs?page=1".to_string())
        );
        assert!(response.links.prev.is_none());
        assert_eq!(
            response.links.next,
            Some("http://example.com/api/v1/labs?page=2".to_string())
        );

        assert_eq!(response.meta.current_page, 1);
        assert_eq!(response.meta.last_page, 2);
        assert_eq!(response.meta.per_page, 15);
        assert_eq!(response.meta.total, 21);
    }

    #[test]
    fn test_paginated_response_empty_data() {
        let json = r#"{
            "data": [],
            "links": {
                "first": "http://example.com/api/v1/labs?page=1",
                "last": "http://example.com/api/v1/labs?page=1",
                "prev": null,
                "next": null
            },
            "meta": {
                "current_page": 1,
                "from": null,
                "last_page": 1,
                "path": "http://example.com/api/v1/labs",
                "per_page": 15,
                "to": null,
                "total": 0
            }
        }"#;

        let response: PaginatedResponse<Lab> = serde_json::from_str(json).unwrap();

        assert!(response.data.is_empty());
        assert!(response.meta.from.is_none());
        assert!(response.meta.to.is_none());
        assert_eq!(response.meta.total, 0);
    }

    #[test]
    fn test_task_with_prologue_and_epilogue() {
        let json = r#"{
            "id": 1,
            "slug": "api-client-test",
            "title": "API Client Basics",
            "description": "Test your API client implementation",
            "sort_order": 1,
            "scores": "5:10:50",
            "status": "challenge_awaits",
            "is_locked": false,
            "abandoned_deduction": 5,
            "points_earned": 0,
            "hints": [],
            "validators": ["tcp_listening:int(8080)"],
            "prologue": ["docker compose up -d", "sleep 2"],
            "epilogue": ["docker compose down"]
        }"#;

        let task: Task = serde_json::from_str(json).unwrap();

        assert_eq!(task.prologue.len(), 2);
        assert_eq!(task.prologue[0], "docker compose up -d");
        assert_eq!(task.prologue[1], "sleep 2");
        assert_eq!(task.epilogue.len(), 1);
        assert_eq!(task.epilogue[0], "docker compose down");
    }

    #[test]
    fn test_task_without_prologue_epilogue_defaults_to_empty() {
        // when prologue/epilogue are not present in JSON, they should default to empty
        let json = r#"{
            "id": 1,
            "slug": "simple-task",
            "title": "Simple Task",
            "description": "No hooks",
            "sort_order": 1,
            "scores": "5:10:50",
            "status": "challenge_awaits",
            "is_locked": false,
            "abandoned_deduction": 5,
            "points_earned": 0,
            "hints": [],
            "validators": []
        }"#;

        let task: Task = serde_json::from_str(json).unwrap();

        assert!(task.prologue.is_empty());
        assert!(task.epilogue.is_empty());
    }

    #[test]
    fn test_task_with_input_type() {
        let json = r#"{
            "id": 1,
            "slug": "text-input-task",
            "title": "Text Input Task",
            "description": "Enter a text answer",
            "sort_order": 1,
            "input_type": "text",
            "scores": "5:10:50",
            "status": "challenge_awaits",
            "is_locked": false,
            "abandoned_deduction": 5,
            "points_earned": 0,
            "hints": [],
            "validators": []
        }"#;

        let task: Task = serde_json::from_str(json).unwrap();

        assert_eq!(task.input_type, TaskInputType::Text);
        assert!(task.accepts_input());
    }

    #[test]
    fn test_task_without_input_type_defaults_to_none() {
        let json = r#"{
            "id": 1,
            "slug": "no-input-task",
            "title": "No Input Task",
            "description": "No input needed",
            "sort_order": 1,
            "scores": "5:10:50",
            "status": "challenge_awaits",
            "is_locked": false,
            "abandoned_deduction": 5,
            "points_earned": 0,
            "hints": [],
            "validators": []
        }"#;

        let task: Task = serde_json::from_str(json).unwrap();

        assert_eq!(task.input_type, TaskInputType::None);
        assert!(!task.accepts_input());
    }

    #[test]
    fn test_submit_answer_request_text() {
        let request = SubmitAnswerRequest::text("my answer");
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("my answer"));
    }

    #[test]
    fn test_submit_answer_request_number() {
        let request = SubmitAnswerRequest::number(42.0);
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("42"));
    }

    #[test]
    fn test_submit_answer_request_choices() {
        let request = SubmitAnswerRequest::choices(vec!["a", "b"]);
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("[\"a\",\"b\"]"));
    }

    #[test]
    fn test_submit_answer_response_deserialize() {
        let json = r#"{
            "success": true,
            "valid": true,
            "message": "Correct!",
            "points_earned": 50,
            "attempts": 2
        }"#;

        let response: SubmitAnswerResponse = serde_json::from_str(json).unwrap();

        assert!(response.success);
        assert!(response.valid);
        assert_eq!(response.message, "Correct!");
        assert_eq!(response.points_earned, Some(50));
        assert_eq!(response.attempts, Some(2));
    }

    #[test]
    fn test_submit_answer_response_incorrect() {
        let json = r#"{
            "success": true,
            "valid": false,
            "message": "Incorrect answer.",
            "attempts": 3
        }"#;

        let response: SubmitAnswerResponse = serde_json::from_str(json).unwrap();

        assert!(response.success);
        assert!(!response.valid);
        assert_eq!(response.message, "Incorrect answer.");
        assert!(response.points_earned.is_none());
        assert_eq!(response.attempts, Some(3));
    }
}
