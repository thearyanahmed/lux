# lighthouse CLI - Technical Specification

## Overview
A Rust-based CLI tool that validates local systems programming exercises without requiring cloud infrastructure.

## Why Rust
- Cross-platform single binary distribution
- Strong async support (tokio) for network testing
- Excellent process management
- Fast validation = better UX
- Dogfooding (teaching Rust by using Rust)

## Core Commands

```bash
lux auth --token <token>                       # Authenticate with Project Lighthouse
lux init <task-id> [--lang rust|go]            # Download starter code for a task
lux validate --task-id <task-uuid>             # Run validation tests for a task
lux hints --task-id <task-uuid> [--progressive] # Show progressive hints
lux doctor                                      # Check local environment
lux submit --task-id <task-uuid>               # (future) Record completion
```

### Command Details

**`lux auth --token <token>`**
- Stores authentication token locally (e.g., in `~/.config/lux/auth.json`)
- Token used for fetching tasks and submitting results
- Required before running other commands

**`lux validate --task-id <task-uuid>`**
- Looks up task in local task registry (`tasks.json`)
- Locates user's implementation in `exercises/<task-uuid>/<lang>/`
- Runs task-specific validator
- Reports test results

## Project Structure

```
lighthouse/
├── src/
│   ├── main.rs              # CLI entry point (clap)
│   ├── commands/
│   │   ├── init.rs          # Download starter code
│   │   ├── test.rs          # Run validators
│   │   ├── hints.rs         # Show hints
│   │   └── doctor.rs        # Check setup
│   ├── exercises/
│   │   ├── mod.rs
│   │   ├── registry.rs      # Exercise definitions
│   │   └── validators/
│   │       ├── tcp_echo.rs
│   │       ├── http_parser.rs
│   │       └── ...
│   ├── runner.rs            # Spawn/manage user binaries
│   └── output.rs            # Pretty test results
└── exercises/               # Starter code templates
    ├── tcp-echo-server/
    │   ├── rust/
    │   │   ├── Cargo.toml
    │   │   └── src/main.rs  # TODOs
    │   └── go/
    │       ├── go.mod
    │       └── main.go
    └── ...
```

## Key Dependencies
- `clap` - CLI argument parsing
- `tokio` - Async runtime for validators
- `colored` - Terminal output coloring
- `indicatif` - Progress bars
- `serde` / `serde_json` - Configuration
- `reqwest` - Fetch templates (optional)

## Task & Validator Architecture

### Core Design: Static Dispatch with Enums

All validators use static dispatch via enums (no `Box<dyn Trait>`). Each task UUID maps to a `Task` struct containing metadata and validators.

### Task Structure

```rust
pub struct Task {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub hints: &'static [&'static str],  // Compile-time strings
    pub validators: Vec<Validator>,
}

impl Task {
    pub fn description(&self) -> &'static str {
        self.description
    }

    pub fn hints(&self) -> &'static [&'static str] {
        self.hints
    }

    pub async fn validate(&self, context: &ValidationContext) -> Result<TestResults> {
        let mut tests = Vec::new();
        for validator in &self.validators {
            tests.push(validator.validate(context).await?);
        }
        Ok(TestResults { tests })
    }
}
```

### Validator Enum (Static Dispatch)

```rust
pub enum Validator {
    // Runtime validators (test running processes)
    Port(PortValidator),
    Endpoint(EndpointValidator),
    JsonResponse(JsonResponseValidator),
    StatusCode(StatusCodeValidator),
    TcpEcho(TcpEchoValidator),

    // Code validators (parse source files)
    CodeStructure(CodeValidator),
    FunctionExists(FunctionValidator),

    // Infrastructure validators (future)
    Docker(DockerValidator),
    Kubernetes(K8sValidator),
}

impl Validator {
    pub async fn validate(&self, context: &ValidationContext) -> Result<TestCase> {
        match self {
            Validator::Port(v) => v.validate(context).await,
            Validator::Endpoint(v) => v.validate(context).await,
            Validator::JsonResponse(v) => v.validate(context).await,
            Validator::StatusCode(v) => v.validate(context).await,
            Validator::TcpEcho(v) => v.validate(context).await,
            Validator::CodeStructure(v) => v.validate(context).await,
            Validator::FunctionExists(v) => v.validate(context).await,
            Validator::Docker(v) => v.validate(context).await,
            Validator::Kubernetes(v) => v.validate(context).await,
        }
    }
}
```

### Validation Context & Results

```rust
pub struct ValidationContext {
    pub task_id: String,
    pub language: String,
    pub project_path: PathBuf,  // e.g., exercises/<task-uuid>/rust/
}

pub struct TestResults {
    pub tests: Vec<TestCase>,
}

pub struct TestCase {
    pub name: String,
    pub passed: bool,
    pub error: Option<String>,
}
```

### Task Registry (In-Memory)

```rust
use std::collections::HashMap;
use once_cell::sync::Lazy;

type TaskRegistry = HashMap<&'static str, Task>;

static TASK_REGISTRY: Lazy<TaskRegistry> = Lazy::new(|| {
    let mut registry = HashMap::new();

    // HTTP Server task
    registry.insert(
        "550e8400-e29b-41d4-a716-446655440000",
        Task {
            id: "550e8400-e29b-41d4-a716-446655440000",
            name: "HTTP Server",
            description: "Build an HTTP server that handles JSON API requests on port 8000",
            hints: &[
                "Start by binding a TCP listener to port 8000",
                "Implement the /api/v1/hello endpoint first",
                "Return proper JSON Content-Type headers",
                "Handle 404 errors for unknown endpoints",
            ],
            validators: vec![
                Validator::Port(PortValidator::new(8000)),
                Validator::Endpoint(EndpointValidator::new("/api/v1/hello")),
                Validator::JsonResponse(JsonResponseValidator::new()),
                Validator::StatusCode(StatusCodeValidator::new(404)),
            ],
        }
    );

    registry
});
```

## Example: TCP Echo Validator

**What it tests:**
1. Server starts and binds to port
2. Accepts connections
3. Echoes input correctly
4. Handles concurrent connections
5. Graceful shutdown

**How it works:**
- Spawn user's binary as subprocess
- Connect via tokio TcpStream
- Send test payloads
- Validate responses
- Test concurrent connections
- Kill process and verify cleanup

## Exercise Definition

```rust
pub struct Exercise {
    pub id: String,
    pub name: String,
    pub description: String,
    pub validator: Box<dyn Validator>,
    pub templates: HashMap<String, String>, // lang -> source
    pub hints: Vec<String>,
}
```

## Output Format

```
Running tests...
✓ Server starts on port 8080
✓ Accepts connections
✓ Echoes input correctly
✗ Handles concurrent connections (expected: 10, got: 1)

3/4 tests passed

Hints: lighthouse hints --progressive
```

## Distribution
- Build for Linux (x86_64), macOS (x86_64, ARM64), Windows
- GitHub Releases with pre-built binaries
- Use `cargo-dist` for automated releases
- Single binary, no dependencies

## Development Timeline

**Week 1-2:** Core CLI + test runner framework
**Week 3:** First exercise validator (TCP echo server)
**Week 4:** Add 2-3 more exercises, beta release

## First Exercise: TCP Echo Server

**Why this one first:**
- Simple concept, actually useful
- Tests networking + concurrency fundamentals
- Easy to validate programmatically
- Good learning progression

**Starter code includes:**
- TODO comments marking implementation points
- Basic structure (main, connection handling)
- Error handling skeleton
- Tests they can run locally

## Future Considerations
- Cache validated exercises locally
- Submit results to projectlighthouse.io (track progress)
- Leaderboards / completion stats
- Exercise difficulty ratings
- Community-contributed exercises
