use crate::executor::context::Context;
use crate::executor::error::ExecutionError;
use crate::transpiler::ir::{ProbeResult, ProcessProbe, Value};
use std::collections::HashMap;
use std::time::Instant;
use tokio::process::Command;

pub async fn execute(probe: &ProcessProbe, ctx: &Context) -> Result<ProbeResult, ExecutionError> {
    let name = ctx.interpolate(&probe.name);
    let start = Instant::now();

    // use pgrep to find processes by name
    let output = Command::new("pgrep").args(["-x", &name]).output().await;

    let duration = start.elapsed();
    let mut fields = HashMap::new();

    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let pid_str = stdout.lines().next().unwrap_or("0");
            let pid: i64 = pid_str.parse().unwrap_or(0);

            fields.insert("running".to_string(), Value::Bool(true));
            fields.insert("pid".to_string(), Value::Int(pid));
            fields.insert("name".to_string(), Value::String(name));
        }
        _ => {
            fields.insert("running".to_string(), Value::Bool(false));
            fields.insert("pid".to_string(), Value::Int(0));
            fields.insert("name".to_string(), Value::String(name));
        }
    }

    Ok(ProbeResult {
        fields,
        raw_stdout: None,
        duration_ms: duration.as_millis() as u64,
    })
}
