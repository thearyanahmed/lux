use super::Task;
use crate::validators::{
    EndpointValidator, JsonResponseValidator, PortValidator, Validator, ValidatorStep,
};
use async_trait::async_trait;

pub struct HttpServerTask {
    validators: Vec<ValidatorStep>,
}

#[async_trait]
impl Task for HttpServerTask {
    fn new() -> Self {
        Self {
            validators: vec![
                ValidatorStep {
                    id: "port-check",
                    name: "server binds to port 8000",
                    hints: &[
                        "use tcplistener::bind to listen on port 8000",
                        "bind to 127.0.0.1:8000",
                        "make sure to handle the result properly",
                    ],
                    validator: Validator::Port(PortValidator::new(8000)),
                },
                ValidatorStep {
                    id: "hello-endpoint",
                    name: "implements /api/v1/hello endpoint",
                    hints: &[
                        "parse the request path from the http request",
                        "return a 200 ok status for this endpoint",
                        "make sure to handle the http/1.1 protocol correctly",
                    ],
                    validator: Validator::Endpoint(EndpointValidator::new("/api/v1/hello")),
                },
                ValidatorStep {
                    id: "json-response",
                    name: "returns json content-type header",
                    hints: &[
                        "add content-type: application/json header to response",
                        "headers should come before the response body",
                        "use \\r\\n for line endings in http headers",
                    ],
                    validator: Validator::JsonResponse(JsonResponseValidator::new()),
                },
            ],
        }
    }

    fn id(&self) -> &'static str {
        "550e8400-e29b-41d4-a716-446655440000"
    }

    fn name(&self) -> &'static str {
        "http server"
    }

    fn description(&self) -> &'static str {
        "build an http server that handles json api requests on port 8000"
    }

    fn hints(&self) -> &'static [&'static str] {
        &[
            "start by creating a tcp listener on port 8000",
            "parse incoming http requests to extract the method and path",
            "implement the /api/v1/hello endpoint first",
            "return proper http headers including content-type",
        ]
    }

    fn validators(&self) -> &[ValidatorStep] {
        &self.validators
    }
}
