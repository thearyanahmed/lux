.PHONY: build run test fmt lint clean dev check all local\:me local\:get

# ==============================================================================
# Local API Testing
# ==============================================================================

LOCAL_API_URL := http://0.0.0.0:8000/api/v1
DEV_TOKEN_FILE := dev_token
DEV_TOKEN := $(shell cat $(DEV_TOKEN_FILE) 2>/dev/null)

# Base function for authenticated GET requests
define api_get
	@if [ -z "$(DEV_TOKEN)" ]; then \
		echo "Error: dev_token file not found or empty. See README.md for setup."; \
		exit 1; \
	fi
	@curl -s -H "Authorization: Bearer $(DEV_TOKEN)" \
		-H "Accept: application/json" \
		"$(LOCAL_API_URL)$(1)" | jq
endef

# Get current user info
local\:me:
	$(call api_get,/user)

# Generic GET request: make local:get ENDPOINT=/some/path
local\:get:
	$(call api_get,$(ENDPOINT))

# ==============================================================================
# Build & Development
# ==============================================================================

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
