# blueprint

An inspection engine for [luxctl](../../README.md). Blueprint parses a custom DSL (`.bp` files), probes the state of the world (ports, HTTP endpoints, processes, files, command output), and reports whether the student's work matches expectations.

**Blueprint inspects, it does not act.** The student does the work — starts servers, creates containers, writes code. Blueprint observes what's there and reports pass/fail.

## Pipeline

```
.bp source text
      │
      ▼ parse
Untyped AST (blocks, properties, raw lines)
      │
      ▼ transpile
Typed IR (Blueprint, Phase, Step, Probe, Expectation, Capture)
      │
      ▼ execute
BlueprintResult (per-step pass/fail, captured values, timing)
      │
      ▼ report
CLI output (colored terminal) or API payload (JSON)
```

Each stage is a separate module with its own error type. The stages are fully decoupled — the parser knows nothing about probes, the executor knows nothing about syntax.

## DSL syntax

```
blueprint "Build Your Own HTTP Server" {

  config {
    host: localhost
    port: 4221
    timeout: 10s
  }

  phase "tcp foundations" {
    step "port is listening" {
      probe tcp 4221
      expect { connected: true }
    }
  }

  phase "basic routing" {
    depends-on: "tcp foundations"

    step "root returns 200" {
      probe http GET /
      expect { status: 200 }
    }

    step "unknown path returns 404" {
      probe http GET /nonexistent-path
      expect { status: 404 }
    }
  }

  phase "echo endpoint" {
    depends-on: "basic routing"

    step "echoes hello" {
      probe http GET /echo/hello
      expect {
        status: 200
        body: "hello"
      }
    }
  }
}
```

### Probes

Every step has one `probe` that observes the world and produces a field bag for `expect` to evaluate against.

| Probe | Syntax | Fields returned |
|-------|--------|-----------------|
| TCP | `probe tcp PORT` | `connected` (bool) |
| UDP | `probe udp HOST:PORT` | `reachable` (bool), `recv` (string) |
| HTTP | `probe http METHOD /path [body]` | `status` (int), `body` (string), `header.*`, `body.json.*`, `latency` |
| Exec | `probe exec COMMAND [ARGS...]` | `stdout` (string), `stderr` (string), `exit` (int), `duration` |
| Docker | `probe docker SUBCOMMAND [ARGS...]` | same as exec (sugar for `probe exec docker ...`) |
| File | `probe file /path` | `contents` (string), `size` (int), `exists` (bool) |
| Process | `probe process NAME` | `running` (bool), `pid` (int), `name` (string) |

HTTP probe supports modes: `concurrent N`, `keepalive N`, `pipelined N`, `burst N window Xs`, `chunked`.

### Expect operators

```
expect {
  status: 200                          # equality (default)
  body contains: "hello"               # substring match
  body starts-with: "OK"               # prefix match
  body matches: /^[a-f0-9]{64}$/       # regex match
  stdout matches-file: expected/10k.txt # compare against file contents
  header.Server present                # field exists
  header.X-Debug absent                # field does not exist
  body.json.count > 5                  # numeric comparison (also <, >=, <=)
  duration < 10s                       # duration comparison
  all status: 200                      # all concurrent responses match

  capture stdout as $container_id      # extract value into variable
  capture body.json.id as $job_id
}
```

### Variables and captures

Captured values are stored in the execution context and can be referenced in later steps:

```
step "submit job" {
  probe http POST /jobs {"type":"test"}
  expect {
    status: 201
    capture body.json.id as $job_id
  }
}

step "check job status" {
  requires: $job_id
  retry: 5 delay 200ms
  probe http GET /jobs/$job_id
  expect { body.json.status: "completed" }
}
```

`requires` skips the step if the variable hasn't been captured yet. Variable interpolation (`$job_id`) happens at execution time in probe arguments.

### Inputs (result mode)

Steps can require user-supplied values for answer confirmation:

```
step "confirm container ID" {
  input { container-id: string matches /^[a-f0-9]{64}$/ }
  probe docker inspect nginx-1 --format '{{.ID}}'
  expect {
    capture stdout as $real_id
    $container-id: $real_id
  }
}
```

Input steps are skipped by `luxctl validate` (probe-only) and only run under `luxctl result` (answer submission).

### Step directives

```
step "name" {
  slug: task-slug                      # URL-safe identifier
  description: |                       # multi-line markdown
    Create a container named nginx-1.
  points: 50                           # points awarded
  scores: "5:10:50"                    # scoring tiers
  is_free: true                        # visible without payment
  timeout: 60s                         # per-step timeout
  retry: 3 delay 1s                    # retry on failure

  hints {
    - text: "Use net.Listen to create a listener"
      points_deduction: 5
  }

  probe ...
  expect { ... }
}
```

### Phase dependencies

Phases form a DAG via `depends-on`. The executor runs a topological sort — if a phase fails, all downstream phases are skipped.

```
phase "basics" { ... }
phase "advanced" {
  depends-on: "basics"
  ...
}
```

## Crate structure

```
src/
  lib.rs                    # re-exports: parser, transpiler, executor, reporter

  parser/
    lexer.rs                # tokenize .bp text (context-aware: / is regex vs path)
    ast.rs                  # untyped AST: BlueprintBlock, Block, Property, RawLine
    grammar.rs              # recursive descent parser: tokens → AST
    error.rs                # ParseError with line/column

  transpiler/
    ir.rs                   # typed IR: Blueprint, Phase, Step, Probe, Expectation, etc.
    resolve.rs              # AST → IR conversion (probe line patterns, expect parsing)
    validate.rs             # IR validation (cycle detection via Kahn's algorithm)
    error.rs                # TranspileError

  executor/
    engine.rs               # main loop: phase ordering → step execution → result collection
    context.rs              # runtime state: variables, config, user inputs, execution mode
    expect.rs               # expectation evaluation + capture extraction + input matching
    probes/
      tcp.rs                # TCP connect check
      udp.rs                # UDP reachability
      http.rs               # HTTP request (reqwest)
      exec.rs               # command execution (also covers `probe docker`)
      file.rs               # file contents/metadata
      process.rs            # running process inspection
    error.rs                # ExecutionError

  reporter/
    cli.rs                  # colored terminal output (pass/fail per step, --detailed mode)
    api.rs                  # BlueprintResult → JSON payload for API submission
```

## Two execution modes

| Mode | Command | Behavior |
|------|---------|----------|
| **Validate** | `luxctl validate --task <slug>` | Probes the world. Skips steps with `input`. |
| **Result** | `luxctl result --task <slug> --flag value` | Runs input steps, compares user value against probed reality. Skips steps without `input`. |

The same `.bp` file contains both kinds of steps. The executor's `ExecutionMode` (Validate or Result) determines which steps run.

## Usage

```rust
use blueprint::parser;
use blueprint::transpiler;
use blueprint::executor::{Engine, Context};
use blueprint::reporter::CliReporter;

// parse
let ast = parser::parse(bp_source)?;

// transpile
let blueprint = transpiler::transpile(&ast)?;

// execute
let ctx = Context::new(blueprint.config.clone(), ExecutionMode::Validate);
let mut engine = Engine::new(ctx);
let result = engine.execute(&blueprint).await?;

// report
CliReporter::print_result(&result, detailed);
```

## Result types

```
BlueprintResult
  status: Passed | Failed | Skipped | Error
  phases: Vec<PhaseResult>
  duration_ms: u64
  captured: HashMap<String, Value>         # all captured values
  input_provided: HashMap<String, String>  # user's CLI flag values

PhaseResult
  name, status, steps: Vec<StepResult>, duration_ms

StepResult
  name, status
  expectations: Vec<ExpectResult>          # per-field pass/fail
  captures: Vec<(String, Value)>
  input_matched: Option<bool>
  duration_ms, retry_count

ExpectResult
  field, op, status
  actual: Option<Value>
  expected_display: String
  message: Option<String>                  # human-readable failure
```

## Tests

```bash
cargo test -p blueprint        # 94 tests across all modules
```

| Module | Tests |
|--------|-------|
| parser::lexer | 10 |
| parser::grammar | 13 |
| transpiler::ir | 6 |
| transpiler::resolve | 13 |
| transpiler::validate | 5 |
| executor::context | 4 |
| executor::expect | 12 |
| executor::probes | 8 |
| executor::engine | 10 |
| reporter::cli | 3 |
| reporter::api | 3 |

## Design principles

1. **Blueprint inspects, it does not act.** The keyword is `probe`, not `do`. Probes are read-only.
2. **Single source of truth.** The `.bp` file IS the project definition — metadata for the website and validation logic for luxctl in one file.
3. **Captures flow to the API.** Values extracted by probes are reported upstream so the website can display them.
4. **`validate` probes, `result` confirms.** Two modes, one file. The executor filters steps based on which command was invoked.
5. **`probe docker` is just `probe exec docker`.** No special Docker probe type — it's syntax sugar that transpiles to `ExecProbe`.
6. **Retry is for eventual consistency.** When probing async systems (job queues, containers starting), retry lets the probe wait for the world to catch up.
7. **New probes don't change the architecture.** Adding a new probe type (Redis, gRPC) means one new file in `probes/`. Everything else stays the same.
