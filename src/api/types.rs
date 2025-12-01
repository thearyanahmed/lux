use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ApiError {
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct ApiResponse<T> {
    pub data: T,
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
}

#[derive(Debug, Deserialize)]
pub struct ProjectStats {
    pub attempted_count: i32,
    pub succeed_count: i32,
    pub failed_count: i32,
}

#[derive(Debug, Deserialize)]
pub struct Project {
    pub id: i32,
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
    pub stats: Option<ProjectStats>,
    #[serde(default)]
    pub published_at: Option<String>,
    #[serde(default)]
    pub tasks_count: Option<i32>,
    #[serde(default)]
    pub runner_image: Option<String>,
    #[serde(default)]
    pub tasks: Option<Vec<Task>>,
}

impl Project {
    pub fn url(&self) -> String {
        format!("https://projectlighthouse.io/projects/{}", self.slug)
    }
}

#[derive(Debug, Deserialize)]
pub struct Task {
    pub id: i32,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub sort_order: i32,
    pub scores: String,
    pub status: String,
    pub abandoned_deduction: i32,
    pub hints: Vec<Hint>,
    pub validators: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Hint {
    pub id: i32,
    pub text: String,
    pub unlock_criteria: String,
    pub points_deduction: i32,
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
        };

        assert_eq!(user.id(), 42);
        assert_eq!(user.name(), "Test User");
    }

    #[test]
    fn test_api_response_deserialize() {
        let json = r#"{"data": {"id": 1, "name": "John", "email": "john@example.com"}}"#;
        let response: ApiResponse<ApiUser> = serde_json::from_str(json).unwrap();

        assert_eq!(response.data.id, 1);
        assert_eq!(response.data.name, "John");
    }

    #[test]
    fn test_project_deserialize() {
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

        let project: Project = serde_json::from_str(json).unwrap();

        assert_eq!(project.id, 2);
        assert_eq!(project.slug, "build-your-own-http-server");
        assert_eq!(project.name, "Build Your Own Server");
        assert_eq!(project.is_published, Some(true));
        assert_eq!(project.is_featured, Some(false));
        assert_eq!(project.show_tasks, Some(true));
        let stats = project.stats.unwrap();
        assert_eq!(stats.attempted_count, 10);
        assert_eq!(stats.succeed_count, 5);
        assert_eq!(stats.failed_count, 3);
        assert_eq!(project.tasks_count, Some(9));
    }

    #[test]
    fn test_project_with_null_published_at() {
        let json = r#"{
            "id": 1,
            "slug": "test-project",
            "name": "Test Project",
            "short_description": "A test project",
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

        let project: Project = serde_json::from_str(json).unwrap();

        assert!(project.published_at.is_none());
    }

    #[test]
    fn test_project_detail_with_tasks() {
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
                    "abandoned_deduction": 5,
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

        let project: Project = serde_json::from_str(json).unwrap();

        assert_eq!(project.id, 1);
        assert_eq!(project.slug, "build-your-own-git");
        assert_eq!(project.runner_image, Some("local|go|rust|c".to_string()));

        let tasks = project.tasks.unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "Initialize a Repository");
        assert_eq!(tasks[0].status, "challenge_awaits");
        assert_eq!(tasks[0].hints.len(), 1);
        assert_eq!(tasks[0].hints[0].text, "Create the .git directory.");
    }

    #[test]
    fn test_paginated_response_deserialize() {
        let json = r#"{
            "data": [
                {
                    "id": 1,
                    "slug": "project-one",
                    "name": "Project One",
                    "short_description": "First project",
                    "is_published": true,
                    "is_featured": true,
                    "show_tasks": true,
                    "stats": {"attempted_count": 0, "succeed_count": 0, "failed_count": 0},
                    "published_at": "2025-01-15T00:00:00+00:00",
                    "tasks_count": 5
                },
                {
                    "id": 2,
                    "slug": "project-two",
                    "name": "Project Two",
                    "short_description": "Second project",
                    "is_published": true,
                    "is_featured": false,
                    "show_tasks": false,
                    "stats": {"attempted_count": 1, "succeed_count": 1, "failed_count": 0},
                    "published_at": null,
                    "tasks_count": 3
                }
            ],
            "links": {
                "first": "http://example.com/api/v1/projects?page=1",
                "last": "http://example.com/api/v1/projects?page=2",
                "prev": null,
                "next": "http://example.com/api/v1/projects?page=2"
            },
            "meta": {
                "current_page": 1,
                "from": 1,
                "last_page": 2,
                "path": "http://example.com/api/v1/projects",
                "per_page": 15,
                "to": 15,
                "total": 21
            }
        }"#;

        let response: PaginatedResponse<Project> = serde_json::from_str(json).unwrap();

        assert_eq!(response.data.len(), 2);
        assert_eq!(response.data[0].id, 1);
        assert_eq!(response.data[0].slug, "project-one");
        assert_eq!(response.data[1].id, 2);
        assert_eq!(response.data[1].slug, "project-two");

        assert_eq!(
            response.links.first,
            Some("http://example.com/api/v1/projects?page=1".to_string())
        );
        assert!(response.links.prev.is_none());
        assert_eq!(
            response.links.next,
            Some("http://example.com/api/v1/projects?page=2".to_string())
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
                "first": "http://example.com/api/v1/projects?page=1",
                "last": "http://example.com/api/v1/projects?page=1",
                "prev": null,
                "next": null
            },
            "meta": {
                "current_page": 1,
                "from": null,
                "last_page": 1,
                "path": "http://example.com/api/v1/projects",
                "per_page": 15,
                "to": null,
                "total": 0
            }
        }"#;

        let response: PaginatedResponse<Project> = serde_json::from_str(json).unwrap();

        assert!(response.data.is_empty());
        assert!(response.meta.from.is_none());
        assert!(response.meta.to.is_none());
        assert_eq!(response.meta.total, 0);
    }
}
