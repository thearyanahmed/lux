.PHONY: build run test fmt lint clean dev check all

# Build debug binary
build:
	cargo build

# Build release binary
release:
	cargo build --release

# Run the application
run:
	cargo run

# Run tests
test:
	cargo test

# Format code
fmt:
	cargo fmt

# Check formatting without modifying
fmt-check:
	cargo fmt -- --check

# Run clippy lints
lint:
	cargo clippy

# Run clippy and auto-fix what it can
fix:
	cargo clippy --fix --allow-dirty --allow-staged

# Clean build artifacts
clean:
	cargo clean

# Watch src and rebuild on changes (requires cargo-watch)
dev:
	cargo watch -c -x run

# Watch and run tests on changes
dev-test:
	cargo watch -c -x test

# Run all checks (format, lint, test)
check: fmt-check lint test

# Build and run all quality checks
all: fmt lint test build
