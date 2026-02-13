use crate::transpiler::ir::{Config, Value};
use std::collections::HashMap;

/// runtime execution context — holds captured variables, config, and user inputs
#[derive(Debug, Clone)]
pub struct Context {
    pub config: Config,
    pub variables: HashMap<String, Value>,
    pub user_inputs: HashMap<String, String>,
    pub mode: ExecutionMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionMode {
    /// `luxctl validate` — probe-only, skip steps with input
    Validate,
    /// `luxctl result` — run steps with input, compare user values
    Result,
}

impl Context {
    pub fn new(config: Config, mode: ExecutionMode) -> Self {
        Self {
            config,
            variables: HashMap::new(),
            user_inputs: HashMap::new(),
            mode,
        }
    }

    pub fn set_variable(&mut self, name: &str, value: Value) {
        self.variables.insert(name.to_string(), value);
    }

    pub fn get_variable(&self, name: &str) -> Option<&Value> {
        self.variables.get(name)
    }

    pub fn has_variable(&self, name: &str) -> bool {
        self.variables.contains_key(name)
    }

    pub fn set_user_input(&mut self, name: &str, value: &str) {
        self.user_inputs.insert(name.to_string(), value.to_string());
    }

    pub fn get_user_input(&self, name: &str) -> Option<&str> {
        self.user_inputs.get(name).map(|s| s.as_str())
    }

    /// interpolate $variable references in a string
    pub fn interpolate(&self, s: &str) -> String {
        let mut result = s.to_string();
        for (key, value) in &self.variables {
            let var_ref = if key.starts_with('$') {
                key.clone()
            } else {
                format!("${key}")
            };
            result = result.replace(&var_ref, &value.to_string());
        }
        // also interpolate user inputs
        for (key, value) in &self.user_inputs {
            let var_ref = format!("${key}");
            result = result.replace(&var_ref, value);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variable_storage() {
        let mut ctx = Context::new(Config::default(), ExecutionMode::Validate);
        ctx.set_variable("$container_id", Value::String("abc123".into()));
        assert!(ctx.has_variable("$container_id"));
        assert!(
            matches!(ctx.get_variable("$container_id"), Some(Value::String(s)) if s == "abc123")
        );
    }

    #[test]
    fn test_interpolation() {
        let mut ctx = Context::new(Config::default(), ExecutionMode::Validate);
        ctx.set_variable("$job_id", Value::String("abc-123".into()));

        let result = ctx.interpolate("/jobs/$job_id");
        assert_eq!(result, "/jobs/abc-123");
    }

    #[test]
    fn test_interpolation_multiple() {
        let mut ctx = Context::new(Config::default(), ExecutionMode::Validate);
        ctx.set_variable("$host", Value::String("localhost".into()));
        ctx.set_variable("$port", Value::Int(8080));

        let result = ctx.interpolate("http://$host:$port/api");
        assert_eq!(result, "http://localhost:8080/api");
    }

    #[test]
    fn test_user_input() {
        let mut ctx = Context::new(Config::default(), ExecutionMode::Result);
        ctx.set_user_input("container-id", "abc123");
        assert_eq!(ctx.get_user_input("container-id"), Some("abc123"));
    }
}
