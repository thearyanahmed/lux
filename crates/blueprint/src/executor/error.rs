use std::fmt;

#[derive(Debug, Clone)]
pub struct ExecutionError {
    pub message: String,
    pub step: Option<String>,
}

impl ExecutionError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            step: None,
        }
    }

    pub fn with_step(mut self, step: impl Into<String>) -> Self {
        self.step = Some(step.into());
        self
    }
}

impl fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(step) = &self.step {
            write!(f, "[step: {}] {}", step, self.message)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl std::error::Error for ExecutionError {}
