use crate::executor::context::Context;
use crate::executor::error::ExecutionError;
use crate::transpiler::ir::{FileProbe, ProbeResult, Value};
use std::collections::HashMap;
use std::time::Instant;

pub async fn execute(probe: &FileProbe, ctx: &Context) -> Result<ProbeResult, ExecutionError> {
    let path = ctx.interpolate(&probe.path);
    let start = Instant::now();
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
