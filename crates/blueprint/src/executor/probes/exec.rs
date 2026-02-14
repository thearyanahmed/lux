use crate::executor::context::Context;
use crate::executor::error::ExecutionError;
use crate::transpiler::ir::{ExecProbe, ProbeResult, Value};
use std::collections::HashMap;
use std::time::Instant;
use tokio::process::Command;

pub async fn execute(probe: &ExecProbe, ctx: &Context) -> Result<ProbeResult, ExecutionError> {
    let command = ctx.interpolate(&probe.command);
    let args: Vec<String> = probe.args.iter().map(|a| ctx.interpolate(a)).collect();

    let start = Instant::now();

    let mut cmd = Command::new(&command);
    cmd.args(&args);
    if let Some(ref ws) = ctx.workspace {
        cmd.current_dir(ws);
    }
    let output = cmd
        .output()
        .await
        .map_err(|e| ExecutionError::new(format!("failed to execute '{}': {}", command, e)))?;

    let duration = start.elapsed();

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let exit_code = output.status.code().unwrap_or(-1);

    let mut fields = HashMap::new();
    fields.insert("stdout".to_string(), Value::String(stdout.clone()));
    fields.insert("stderr".to_string(), Value::String(stderr));
    fields.insert("exit".to_string(), Value::Int(exit_code as i64));
    fields.insert(
        "duration".to_string(),
        Value::Int(duration.as_millis() as i64),
    );

    Ok(ProbeResult {
        fields,
        raw_stdout: Some(stdout),
        duration_ms: duration.as_millis() as u64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::context::ExecutionMode;
    use crate::transpiler::ir::Config;

    #[tokio::test]
    async fn test_exec_echo() {
        let probe = ExecProbe {
            command: "echo".to_string(),
            args: vec!["hello".to_string()],
        };
        let ctx = Context::new(Config::default(), ExecutionMode::Validate);

        let result = execute(&probe, &ctx).await;
        assert!(result.is_ok());
        let result = result.unwrap_or_else(|e| panic!("{e}"));

        assert!(matches!(result.fields.get("stdout"), Some(Value::String(s)) if s == "hello"));
        assert!(matches!(result.fields.get("exit"), Some(Value::Int(0))));
    }

    #[tokio::test]
    async fn test_exec_exit_code() {
        let probe = ExecProbe {
            command: "sh".to_string(),
            args: vec!["-c".to_string(), "exit 42".to_string()],
        };
        let ctx = Context::new(Config::default(), ExecutionMode::Validate);

        let result = execute(&probe, &ctx).await;
        assert!(result.is_ok());
        let result = result.unwrap_or_else(|e| panic!("{e}"));

        assert!(matches!(result.fields.get("exit"), Some(Value::Int(42))));
    }

    #[tokio::test]
    async fn test_exec_with_variable_interpolation() {
        let probe = ExecProbe {
            command: "echo".to_string(),
            args: vec!["$name".to_string()],
        };
        let mut ctx = Context::new(Config::default(), ExecutionMode::Validate);
        ctx.set_variable("$name", Value::String("world".into()));

        let result = execute(&probe, &ctx).await;
        assert!(result.is_ok());
        let result = result.unwrap_or_else(|e| panic!("{e}"));

        assert!(matches!(result.fields.get("stdout"), Some(Value::String(s)) if s == "world"));
    }

    #[tokio::test]
    async fn test_exec_stderr() {
        let probe = ExecProbe {
            command: "sh".to_string(),
            args: vec!["-c".to_string(), "echo error >&2".to_string()],
        };
        let ctx = Context::new(Config::default(), ExecutionMode::Validate);

        let result = execute(&probe, &ctx).await;
        assert!(result.is_ok());
        let result = result.unwrap_or_else(|e| panic!("{e}"));

        assert!(matches!(result.fields.get("stderr"), Some(Value::String(s)) if s == "error"));
    }
}
