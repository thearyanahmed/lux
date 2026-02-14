# luxctl

CLI for [projectlighthouse.io](https://projectlighthouse.io) — learn by building real systems.

![luxctl](./images/luxctl-dev-env.png)

## Install

### Pre-built Binaries (Recommended)

Download the latest binary for your platform from the [releases page](https://github.com/thearyanahmed/luxctl/releases):


### Via Install Script

```bash
# one-liner (installs Rust if needed)
curl -fsSL https://raw.githubusercontent.com/thearyanahmed/luxctl/master/install.sh | bash
```

### Via Cargo

```bash
# latest version
cargo install luxctl

# specific version
cargo install luxctl --version 0.7.0
```

## Quick Start

```bash
# authenticate with your token from projectlighthouse.io
luxctl auth --token $token

# verify setup
luxctl doctor

# see who you are
luxctl whoami
```

## Usage

```bash
# list available projects
luxctl project list

# start a project
luxctl project start --slug tcp-echo-server --runtime go

# list tasks for current project
luxctl task list

# show task details
luxctl task show --task 1

# run validation
luxctl run --task 1

# run with verbose per-expectation output
luxctl run --task 1 --detailed

# validate all tasks
luxctl validate

# validate all with verbose output
luxctl validate --detailed

# get hints (costs points)
luxctl hint list --task 1
luxctl hint unlock --task 1 --hint $hint_uuid
```

## Development

```bash
cargo build           # debug build
cargo test            # run tests
cargo fmt             # format
cargo clippy          # lint
```

## Supported Runtimes

- **Go** - detects `go.mod`, builds with `go build .`
- **Rust** - detects `Cargo.toml`, builds with `cargo check`

## Contributing

Contributions are welcome! Here's how to get started:

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/my-feature`
3. Make your changes
4. Run checks: `cargo fmt && cargo clippy && cargo test`
5. Commit with a clear message
6. Push and open a pull request

### Guidelines

- Follow existing code style
- Add tests for new functionality
- Keep commits focused and atomic
- Update documentation as needed

### Reporting Issues

- Check existing issues before creating a new one
- Include luxctl version (`luxctl --version`)
- Include OS and architecture
- Provide steps to reproduce

## Release

Releases are automated via GitHub Actions. To create a new release:

1. Update version in both `Cargo.toml` and `src/lib.rs`
2. Run `cargo build` to update `Cargo.lock`
3. Commit and push to master
4. Wait for Auto Tag workflow to create the version tag
5. Trigger the Release workflow:
   ```bash
   gh workflow run Release --field tag=v0.6.2
   ```

The Release workflow will:
- Run tests
- Verify version matches tag
- Publish to crates.io
- Generate changelog
- Create GitHub release

## License

AGPL-3.0 - See [LICENSE](LICENSE) for details.

This means you can use, modify, and distribute this software, but if you modify it and provide it as a service (even over a network), you must release your source code under the same license.
