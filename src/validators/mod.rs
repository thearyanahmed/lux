pub mod port;
pub mod endpoint;
pub mod json_response;

use crate::tasks::{TestCase, ValidationContext};

pub use port::PortValidator;
pub use endpoint::EndpointValidator;
pub use json_response::JsonResponseValidator;

/// ValidatorStep combines a validator with metadata
pub struct ValidatorStep {
    pub id: &'static str,
    pub name: &'static str,
    pub hints: &'static [&'static str],
    pub validator: Validator,
}

/// Validator enum - static dispatch for all validator types
pub enum Validator {
    // Runtime validators (test running processes)
    Port(PortValidator),
    Endpoint(EndpointValidator),
    JsonResponse(JsonResponseValidator),

    // Code validators (parse source files) - TODO
    // CodeStructure(CodeValidator),
    // FunctionExists(FunctionValidator),

    // Infrastructure validators - TODO
    // Docker(DockerValidator),
    // Kubernetes(K8sValidator),
}

impl Validator {
    pub async fn validate(&self, context: &ValidationContext) -> Result<TestCase, String> {
        match self {
            Validator::Port(v) => v.validate(context).await,
            Validator::Endpoint(v) => v.validate(context).await,
            Validator::JsonResponse(v) => v.validate(context).await,
        }
    }
}
