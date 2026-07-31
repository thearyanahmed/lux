use crate::executor::context::Context;
use crate::executor::error::ExecutionError;
use crate::transpiler::ir::{FileProbe, ProbeResult, Value};
use std::collections::HashMap;
use std::time::Instant;
use tokio::process::Command;

/// stat + read a path inside the guest. `%f` is the raw mode in hex, which we
/// re-render as octal so it matches what `PermissionsExt::mode()` reports on the
/// local path — a `probe file` must read the same either side.
const GUEST_STAT_SCRIPT: &str = r#"p="$1"
[ -e "$p" ] || exit 1
stat -c '%s %f' "$p" 2>/dev/null || echo '0 0'
cat "$p" 2>/dev/null || true
"#;

pub async fn execute(probe: &FileProbe, ctx: &Context) -> Result<ProbeResult, ExecutionError> {
    let path = ctx.interpolate(&probe.path);
    let start = Instant::now();

    if !ctx.runner.is_local() {
        return execute_in_guest(&path, ctx, start).await;
    }

    let mut fields = HashMap::new();

    match tokio::fs::metadata(&path).await {
        Ok(metadata) => {
            fields.insert("exists".to_string(), Value::Bool(true));
            fields.insert("size".to_string(), Value::Int(metadata.len() as i64));

            // read file mode on unix
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = format!("{:o}", metadata.permissions().mode());
                fields.insert("mode".to_string(), Value::String(mode));
            }

            // read contents
            match tokio::fs::read_to_string(&path).await {
                Ok(contents) => {
                    fields.insert("contents".to_string(), Value::String(contents.clone()));
                }
                Err(_) => {
                    fields.insert("contents".to_string(), Value::String(String::new()));
                }
            }
        }
        Err(_) => {
            fields.insert("exists".to_string(), Value::Bool(false));
            fields.insert("size".to_string(), Value::Int(0));
            fields.insert("contents".to_string(), Value::String(String::new()));
            fields.insert("mode".to_string(), Value::String(String::new()));
        }
    }

    let duration = start.elapsed();

    Ok(ProbeResult {
        fields,
        raw_stdout: None,
        duration_ms: duration.as_millis() as u64,
    })
}

async fn execute_in_guest(
    path: &str,
    ctx: &Context,
    start: Instant,
) -> Result<ProbeResult, ExecutionError> {
    let (program, args, _) = ctx
        .runner
        .wrap_shell(GUEST_STAT_SCRIPT, &[path.to_string()]);

    let mut cmd = Command::new(&program);
    cmd.args(&args);
    cmd.envs(ctx.runner.env().iter().cloned());

    let output = cmd
        .output()
        .await
        .map_err(|e| ExecutionError::new(format!("failed to stat '{}' in guest: {}", path, e)))?;

    let mut fields = HashMap::new();

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let (header, contents) = match stdout.split_once('\n') {
            Some((h, rest)) => (h, rest),
            None => (stdout.as_ref(), ""),
        };
        let mut parts = header.split_whitespace();
        let size: i64 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        let mode = parts
            .next()
            .and_then(|s| u32::from_str_radix(s, 16).ok())
            .map(|m| format!("{:o}", m))
            .unwrap_or_default();

        fields.insert("exists".to_string(), Value::Bool(true));
        fields.insert("size".to_string(), Value::Int(size));
        fields.insert("mode".to_string(), Value::String(mode));
        fields.insert("contents".to_string(), Value::String(contents.to_string()));
    } else {
        fields.insert("exists".to_string(), Value::Bool(false));
        fields.insert("size".to_string(), Value::Int(0));
        fields.insert("contents".to_string(), Value::String(String::new()));
        fields.insert("mode".to_string(), Value::String(String::new()));
    }

    Ok(ProbeResult {
        fields,
        raw_stdout: None,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::context::ExecutionMode;
    use crate::transpiler::ir::Config;
    use std::io::Write;

    #[tokio::test]
    async fn test_file_exists() {
        let tmp = tempfile::NamedTempFile::new().unwrap_or_else(|e| panic!("{e}"));
        let path = tmp.path().to_string_lossy().to_string();

        let probe = FileProbe { path };
        let ctx = Context::new(Config::default(), ExecutionMode::Validate);

        let result = execute(&probe, &ctx).await;
        assert!(result.is_ok());
        let result = result.unwrap_or_else(|e| panic!("{e}"));
        assert!(matches!(
            result.fields.get("exists"),
            Some(Value::Bool(true))
        ));
    }

    #[tokio::test]
    async fn test_file_not_exists() {
        let probe = FileProbe {
            path: "/tmp/nonexistent_blueprint_test_file_xyz".to_string(),
        };
        let ctx = Context::new(Config::default(), ExecutionMode::Validate);

        let result = execute(&probe, &ctx).await;
        assert!(result.is_ok());
        let result = result.unwrap_or_else(|e| panic!("{e}"));
        assert!(matches!(
            result.fields.get("exists"),
            Some(Value::Bool(false))
        ));
    }

    #[tokio::test]
    async fn test_file_contents() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap_or_else(|e| panic!("{e}"));
        write!(tmp, "hello world").unwrap_or_else(|e| panic!("{e}"));

        let path = tmp.path().to_string_lossy().to_string();
        let probe = FileProbe { path };
        let ctx = Context::new(Config::default(), ExecutionMode::Validate);

        let result = execute(&probe, &ctx).await;
        assert!(result.is_ok());
        let result = result.unwrap_or_else(|e| panic!("{e}"));
        assert!(
            matches!(result.fields.get("contents"), Some(Value::String(s)) if s == "hello world")
        );
    }
}
