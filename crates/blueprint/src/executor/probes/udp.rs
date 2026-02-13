use crate::executor::context::Context;
use crate::executor::error::ExecutionError;
use crate::transpiler::ir::{ProbeResult, UdpProbe, Value};
use std::collections::HashMap;
use std::time::Instant;
use tokio::net::UdpSocket;

pub async fn execute(probe: &UdpProbe, _ctx: &Context) -> Result<ProbeResult, ExecutionError> {
    let addr = format!("{}:{}", probe.host, probe.port);
    let start = Instant::now();

    let result = tokio::time::timeout(std::time::Duration::from_secs(3), async {
        let socket = UdpSocket::bind("0.0.0.0:0")
            .await
            .map_err(|e| ExecutionError::new(format!("failed to bind UDP socket: {e}")))?;
        socket
            .connect(&addr)
            .await
            .map_err(|e| ExecutionError::new(format!("failed to connect UDP: {e}")))?;

        // send a small probe packet
        socket
            .send(b"\x00")
            .await
            .map_err(|e| ExecutionError::new(format!("failed to send UDP: {e}")))?;

        let mut buf = [0u8; 4096];
        match tokio::time::timeout(std::time::Duration::from_secs(2), socket.recv(&mut buf)).await {
            Ok(Ok(n)) => {
                let recv = String::from_utf8_lossy(&buf[..n]).to_string();
                Ok::<_, ExecutionError>((true, recv))
            }
            _ => Ok::<_, ExecutionError>((true, String::new())),
        }
    })
    .await;

    let duration = start.elapsed();
    let mut fields = HashMap::new();

    match result {
        Ok(Ok((reachable, recv))) => {
            fields.insert("reachable".to_string(), Value::Bool(reachable));
            fields.insert("recv".to_string(), Value::String(recv));
        }
        _ => {
            fields.insert("reachable".to_string(), Value::Bool(false));
            fields.insert("recv".to_string(), Value::String(String::new()));
        }
    }

    Ok(ProbeResult {
        fields,
        raw_stdout: None,
        duration_ms: duration.as_millis() as u64,
    })
}
