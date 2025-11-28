use crate::tasks::{TestCase, ValidationContext};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub struct JsonResponseValidator {
    endpoint: String,
    port: u16,
}

impl Default for JsonResponseValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl JsonResponseValidator {
    pub fn new() -> Self {
        Self {
            endpoint: "/api/v1/hello".to_string(),
            port: 8000,
        }
    }

    pub async fn validate(&self, _context: &ValidationContext) -> Result<TestCase, String> {
        let addr = format!("127.0.0.1:{}", self.port);
        let mut stream = TcpStream::connect(&addr)
            .await
            .map_err(|e| format!("failed to connect: {}", e))?;

        let request = format!(
            "GET {} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
            self.endpoint
        );

        stream
            .write_all(request.as_bytes())
            .await
            .map_err(|e| format!("failed to send request: {}", e))?;

        let mut response = Vec::new();
        stream
            .read_to_end(&mut response)
            .await
            .map_err(|e| format!("failed to read response: {}", e))?;

        let response_str = String::from_utf8_lossy(&response);

        // Check for JSON content type
        let has_json_header = response_str
            .lines()
            .any(|line| line.to_lowercase().contains("content-type") && line.to_lowercase().contains("application/json"));

        let test_result = if has_json_header {
            Ok("response has json content-type header".to_string())
        } else {
            Err("missing or incorrect content-type header".to_string())
        };

        Ok(TestCase {
            name: "response has json content-type header".to_string(),
            result: test_result,
        })
    }
}
