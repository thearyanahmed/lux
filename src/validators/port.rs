use crate::tasks::{TestCase, ValidationContext};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

pub struct PortValidator {
    port: u16,
}

impl PortValidator {
    pub fn new(port: u16) -> Self {
        Self { port }
    }

    pub async fn validate(&self, _context: &ValidationContext) -> Result<TestCase, String> {
        let addr = format!("127.0.0.1:{}", self.port);
        let result = timeout(Duration::from_secs(2), TcpStream::connect(&addr)).await;

        let test_result = match result {
            Ok(Ok(_)) => Ok(format!("successfully connected to port {}", self.port)),
            Ok(Err(e)) => Err(format!("connection failed: {}", e)),
            Err(_) => Err("connection timeout after 2 seconds".to_string()),
        };

        Ok(TestCase {
            name: format!("server listening on port {}", self.port),
            result: test_result,
        })
    }
}
