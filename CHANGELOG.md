# Changelog

All notable changes to this project will be documented in this file.

## [0.12.0] - 2026-06-05

### Fixed

- HTTP probe now sends step-scope `headers { ... }` block. Transpiler moves `Step.headers` into `HttpProbe.headers` so executor includes them in requests.

## [0.5.5] - 2026-01-18

### Changed

- Rename lux to luxctl in all messages and use $token placeholder

### Fixed

- Handle missing config gracefully in doctor command
- Fix invalid keyword and bump to 0.5.5

## [0.5.4] - 2026-01-18

### Added

- Add curl | bash installation script

### Changed

- Update package metadata: repository URL and description
- Simplify package description

### Other

- Rewrite README with updated installation and usage

## [0.5.3] - 2026-01-17

### Added

- Add optional fields to HealthCheckResponse

### Changed

- Improve release workflow

## [0.5.2] - 2026-01-17

### Added

- Add healthcheck endpoint to doctor command
- Add post-release verification on Ubuntu, macOS, Windows

### Changed

- Rename lux to luxctl in CLI usage message
- Rename config directory from .lux to .luxctl
- Rename env vars from LUX_* to LUXCTL_*

## [0.5.1] - 2026-01-17

### Fixed

- Fix API client to use RELEASE env for release builds

## [0.5.0] - 2026-01-17

### Changed

- Rename package to luxctl v0.5.0

## [0.4.7] - 2026-01-17

### Fixed

- Fix release workflow and bump to 0.4.7

## [0.4.6] - 2026-01-17

### Added

- Add package metadata for crates.io publishing

### Changed

- Update description to use projectlighthouse

## [0.4.5] - 2026-01-17

### Added

- Add version
- Add clap
- Add version
- Add async-trait
- Add run command skeleton
- Add task get logic
- Add linter to prevent unwrap() and expect()
- Create challenge_4_1_tcp_client.md
- Create systems_programming_challenges_for_lux.md
- Create systems_programming_more.md
- Add .env logger
- Add lighthouse api
- Add test for /api
- Add tests for api
- Add coloreyre
- Add build and test github action
- Add boilerplate
- Add generic get()
- Add clippy rules
- Add secrecy
- Add make file to request local backend
- Add macro for printing
- Add projects api
- Add tests for api
- Add validator DSL parser and factory
- Add is_reattempt field to attempt response
- Add --detailed flag and reattempt handling
- Add test HTTP server for E2E testing
- Add hmac, sha2, chrono dependencies for state integrity
- Add state module with HMAC integrity verification
- Add commands module with project, tasks, run, validate handlers
- Add can_compile validator
- Add clippy rules
- Add http_get_file validator
- Add file_contents_match validator
- Add http_get_compressed validator
- Add hints and hint unlock commands
- Add bollard dependency for Docker container management
- Add HTTP JSON, rate limit, and POST validators
- Add process validators for concurrent access and graceful shutdown
- Add Docker-based validators for Go race detection and compilation
- Add multi-step scenario validators for job queue and worker pool
- Add release script and update documentation
- Add release script and update documentation
- Add is_locked
- Add unit tests for task filtering in validate command
- Add doctor command for environment diagnostics
- Add Doctor CLI command and handler
- Add set_workspace method to ProjectState
- Add set_workspace command handler
- Add --workspace option to project set command
- Add unit tests for TokenAuthenticator
- Add auto-tag workflow on master merge
- Add manual release workflow with cargo publish

### Changed

- Update validator docs
- Rename LighthouseAPI to LighthouseAPIClient
- Update param format
- Refactor, colorise user messages
- refactor config, add tests
- Update completed tasks
- Update README.md
- Update macros
- Refactor lighthouse api to accept config
- Update project details output
- Update CLI with project, tasks, run, validate commands
- Update commands for project validation and task display improvements
- Update main entry point and improve compile/parser validators
- Simplify CLI output and add error truncation
- Improve run command with task numbers and context truncation
- Improve CLI help text with user-friendly descriptions

### Fixed

- Fix comment
- Fix imports
- fix param order in TokenAuthenticator test
- Fix config store isuse
- Fix syntax
- Fix clippy warnings and errors
- Fix docker validators workspace and add progress messages
- Fix clippy: avoid unwrap in whoami command
- Handle mutex poisoning in env var test helper

### Other

- Journey to a thousand miles begins with a single commit
- docs: update validator spec with static dispatch architecture
- docs: add concrete validator implementation examples
- feat: add Task trait and validation context
- feat: add validator enum and basic validators (port, endpoint, json)
- deps: add tokio and once_cell dependencies
- feat: declare tasks and validators modules in main
- feat: add http server example task with validators
- Bring version to main.rs
- Housekeeping
- [WIP] run tasks, add notes
- Move to lib.rs
- Impl LighthouseAPIBaseURL and tests
- Default lighthouse api
- Impl fmt::Display for LighthouseAPI
- Housekeeping: use environment instead of env for clarity
- Explicitly allow base url to be projectlighthouse.io
- Use LighthouseAPIBaseURL instead of string
- Drop docs
- Plan for upcoming tasks
- [WIP] auth
- [WIP] TokenAuthenticator validation
- Allow 0.0.0.0 to be part of dev env base url
- Housekeeping - log client initialistion as log level debug
- Apply clippy rules
- Scratch
- Impl Config
- Use &str for token secrecy, write token after authentication
- Drop printer
- Print errors
- Store cfg as key value pair
- Use Path instead of PathBuf, lint
- Format code
- Housekeeping
- [WIP] api module refactor
- Drop unnecessary comments
- Bind lux projects with projects api
- Default paginate to 50
- Housekeeping
- Make url display in a new line
- Change output ui to make slug and url dimmed, description regular color
- Display api error as text instead of json
- [WIP] task descriptions, need to figure out a good ux
- Format
- [WIP]
- Format output with padded brackets
- Ignore test server build artifacts
- Patch lock
- Export state module and add hex dependency
- Auto-refresh task status after submission
- cargo clippy --fix
- Hosuekeeping
- Use Makefile commands in CI for consistency
- Register new validators in factory and module exports
- Enhance API client with project validation and improve message formatting
- Display task numbers with leading 0
- Truncate error output in concurrent validators
- Track points earned and add workspace/runtime to project state
- Use workspace and runtime from project state in compile validator
- Consolidate CLI commands into subcommands and add whoami
- Make clippy happy
- Test release
- Replace string status with TaskStatus enum
- Register doctor module in commands
- Sort imports in validate module
- Make LighthouseAPIClient fields private
- Extract with_active_mut helper for state mutations
- Extract status symbol constants in message module

### Removed

- Delete monolithic api.rs
- Remove unused task registry
- Remove redundant to_string() in SecretString creation


