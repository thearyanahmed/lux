.PHONY: build run test e2e fmt lint clean dev check all local\:me local\:get release\:build dev-build

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
# --workspace is required: without it cargo only tests the root package, and the
# blueprint crate's parser/transpiler/executor tests never run.
test:
	cargo test --workspace

# Run E2E tests (requires local API running)
e2e:
	cargo test --test e2e -- --ignored --nocapture

# Format code
fmt:
	cargo fmt

# Check formatting without modifying
fmt-check:
	cargo fmt -- --check

# Run clippy lints
lint:
	cargo clippy --workspace

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

# Build a linux debug binary for local dev containers
dev-build:
	docker run --rm -v .:/src -w /src rust:1.83-alpine sh -c "apk add musl-dev && cargo build"
	cp target/debug/luxctl luxctl_dev

# Build a release: make release:build VERSION=0.2.0
release\:build:
ifndef VERSION
	$(error VERSION is required. Usage: make release:build VERSION=0.2.0)
endif
	./scripts/release.sh $(VERSION)
