use crate::executor::context::Context;
use crate::executor::error::ExecutionError;
use crate::transpiler::ir::{ProbeResult, TcpProbe, Value};
use std::collections::HashMap;
use std::time::Instant;
use tokio::net::TcpStream;

pub async fn execute(probe: &TcpProbe, ctx: &Context) -> Result<ProbeResult, ExecutionError> {
    let host = &ctx.config.host;
    let addr = format!("{host}:{}", probe.port);
    let start = Instant::now();

    let connected =
        match tokio::time::timeout(std::time::Duration::from_secs(5), TcpStream::connect(&addr))
            .await
        {
            Ok(Ok(_stream)) => true,
            _ => false,
        };

    let duration = start.elapsed();
    let mut fields = HashMap::new();
    fields.insert("connected".to_string(), Value::Bool(connected));

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
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn test_tcp_probe_connects() {
        // bind to a random port
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .unwrap_or_else(|e| panic!("{e}"));
        let port = listener
            .local_addr()
            .unwrap_or_else(|e| panic!("{e}"))
            .port();

        let probe = TcpProbe { port };
        let mut config = Config::default();
        config.host = "127.0.0.1".to_string();
        let ctx = Context::new(config, ExecutionMode::Validate);

        let result = execute(&probe, &ctx).await;
        assert!(result.is_ok());
        let result = result.unwrap_or_else(|e| panic!("{e}"));
        assert!(matches!(
            result.fields.get("connected"),
            Some(Value::Bool(true))
        ));
    }

    #[tokio::test]
    async fn test_tcp_probe_fails() {
        // use a port that's almost certainly not listening
        let probe = TcpProbe { port: 59999 };
        let mut config = Config::default();
        config.host = "127.0.0.1".to_string();
        let ctx = Context::new(config, ExecutionMode::Validate);

        let result = execute(&probe, &ctx).await;
        assert!(result.is_ok());
        let result = result.unwrap_or_else(|e| panic!("{e}"));
        assert!(matches!(
            result.fields.get("connected"),
            Some(Value::Bool(false))
        ));
    }
}
