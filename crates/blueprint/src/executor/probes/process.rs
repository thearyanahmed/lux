use crate::executor::context::Context;
use crate::executor::error::ExecutionError;
use crate::transpiler::ir::{ProbeResult, ProcessProbe, Value};
use std::collections::HashMap;
use std::time::Instant;
use tokio::process::Command;

pub async fn execute(probe: &ProcessProbe, ctx: &Context) -> Result<ProbeResult, ExecutionError> {
    let name = ctx.interpolate(&probe.name);
    let start = Instant::now();

    // use pgrep to find processes by name — inside the guest in linux| mode,
    // where the process the learner started actually lives
    let (program, args, _) = ctx.runner.wrap("pgrep", &["-x".to_string(), name.clone()]);
    let output = Command::new(&program)
        .args(&args)
        .envs(ctx.runner.env().iter().cloned())
        .output()
        .await;

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
