use crate::tasks::{TestCase, ValidationContext};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub struct EndpointValidator {
    endpoint: String,
    port: u16,
}

impl EndpointValidator {
    pub fn new(endpoint: &str) -> Self {
        Self {
            endpoint: endpoint.to_string(),
            port: 8000,
        }
    }

    pub async fn validate(&self, _context: &ValidationContext) -> Result<TestCase, String> {
        let addr = format!("127.0.0.1:{}", self.port);
        let mut stream = TcpStream::connect(&addr)
            .await
            .map_err(|e| format!("failed to connect: {}", e))?;

        // Send HTTP GET request
        let request = format!(
            "GET {} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
            self.endpoint
        );

        stream
            .write_all(request.as_bytes())
            .await
            .map_err(|e| format!("failed to send request: {}", e))?;

        // Read response
        let mut response = Vec::new();
        stream
            .read_to_end(&mut response)
            .await
            .map_err(|e| format!("failed to read response: {}", e))?;

        let response_str = String::from_utf8_lossy(&response);

        // Check for 200 OK status
        let test_result =
            if response_str.contains("HTTP/1.1 200") || response_str.contains("HTTP/1.0 200") {
                Ok(format!("endpoint {} returned 200 ok", self.endpoint))
            } else {
                Err(format!(
                    "expected 200 ok, got: {}",
                    response_str.lines().next().unwrap_or("no response")
                ))
            };

        Ok(TestCase {
            name: format!("endpoint {} returns 200 ok", self.endpoint),
            result: test_result,
        })
    }
}
