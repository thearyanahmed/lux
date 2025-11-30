use async_trait::async_trait;
use std::path::PathBuf;
mod registry;
use crate::validators::ValidatorStep;
pub use registry::get_task;

pub mod http_server;

pub use http_server::HttpServerTask;

/// Task trait - each task implements this interface
#[async_trait]
pub trait Task: Send + Sync {
    fn new() -> Self
    where
        Self: Sized;
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn hints(&self) -> &'static [&'static str];
    fn validators(&self) -> &[ValidatorStep];

    /// Validate all steps in sequence
    async fn validate(&self, context: &ValidationContext) -> Result<TestResults, String> {
        let mut tests = Vec::new();
        for step in self.validators() {
            tests.push(step.validator.validate(context).await?);
        }
        Ok(TestResults { tests })
    }

    /// Validate a specific step by ID
    async fn validate_step(
        &self,
        step_id: &str,
        context: &ValidationContext,
    ) -> Result<TestCase, String> {
        let step = self
            .validators()
            .iter()
            .find(|s| s.id == step_id)
            .ok_or_else(|| format!("Step '{}' not found in task '{}'", step_id, self.id()))?;

        step.validator.validate(context).await
    }
}

/// ValidationContext provides runtime context for validators
#[derive(Debug, Clone)]
pub struct ValidationContext {
    pub task_id: String,
    pub environment: String, // rust, go, docker, networking, k8s, etc.
    pub project_path: PathBuf,
}

/// TestResults aggregates all test cases for a task
#[derive(Debug)]
pub struct TestResults {
    pub tests: Vec<TestCase>,
}

/// TestCase represents a single validation test result
#[derive(Debug)]
pub struct TestCase {
    pub name: String,
    pub result: Result<String, String>, // Ok(success_msg) or Err(error_msg)
}
