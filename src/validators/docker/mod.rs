//! Docker-based validators
//!
//! Docker images must be registered in the registry module for security.
//! Only pre-approved images can be executed on user machines.

mod executor;
pub mod registry;
mod validator;

pub use executor::{is_docker_available, DockerExecutor, ExecutorResult};
pub use registry::{lookup as lookup_image, ImageSource, RegisteredImage};
pub use validator::{DockerValidator, Expectation};
