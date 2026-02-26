# PredictIQ Test Suite Makefile

.PHONY: help test test-unit test-integration test-all bench coverage clean install-tools format lint audit

help:
	@echo "PredictIQ Test Suite Commands:"
	@echo "  make test              - Run all tests"
	@echo "  make test-unit         - Run unit tests only"
	@echo "  make test-integration  - Run integration tests only"
	@echo "  make bench             - Run gas benchmarks"
	@echo "  make coverage          - Generate coverage report"
	@echo "  make format            - Format code"
	@echo "  make lint              - Run clippy lints"
	@echo "  make audit             - Run security audit"
	@echo "  make clean             - Clean build artifacts"
	@echo "  make install-tools     - Install required tools"

# Install required testing tools
install-tools:
	@echo "Installing testing tools..."
	cargo install cargo-llvm-cov
	cargo install cargo-audit
	cargo install cargo-nextest
	cargo install soroban-cli --features opt
	@echo "Tools installed successfully"

# Run all tests
test: test-unit test-integration
	@echo "All tests completed"

# Run unit tests
test-unit:
	@echo "Running unit tests..."
	cargo test --lib --features testutils

# Run integration tests
test-integration:
	@echo "Running integration tests..."
	cargo test --test '*'

# Run all tests including benchmarks
test-all: test bench
	@echo "All tests and benchmarks completed"

# Run gas benchmarks
bench:
	@echo "Running gas benchmarks..."
	cargo test --benches --features testutils -- --nocapture

# Generate coverage report
coverage:
	@echo "Generating coverage report..."
	cargo llvm-cov --workspace --html
	@echo "Coverage report generated in target/llvm-cov/html/index.html"

# Generate coverage with threshold check
coverage-check:
	@echo "Checking coverage threshold..."
	cargo llvm-cov --workspace --fail-under-lines 80

# Run tests with nextest (faster)
test-fast:
	@echo "Running tests with nextest..."
	cargo nextest run --workspace

# Format code
format:
	@echo "Formatting code..."
	cargo fmt --all

# Check formatting
format-check:
	@echo "Checking code formatting..."
	cargo fmt --all -- --check

# Run clippy lints
lint:
	@echo "Running clippy..."
	cargo clippy --all-targets --all-features -- -D warnings

# Run security audit
audit:
	@echo "Running security audit..."
	cargo audit

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	cargo clean
	rm -rf target/
	rm -rf test_snapshots/

# Build optimized contract
build-optimized:
	@echo "Building optimized contract..."
	cargo build --target wasm32-unknown-unknown --release
	soroban contract optimize \
		--wasm target/wasm32-unknown-unknown/release/predict_iq.wasm \
		--wasm-out target/wasm32-unknown-unknown/release/predict_iq_optimized.wasm

# Run specific test by name
test-name:
	@read -p "Enter test name: " name; \
	cargo test $$name -- --nocapture

# Run tests with output
test-verbose:
	@echo "Running tests with verbose output..."
	cargo test --workspace -- --nocapture --test-threads=1

# Watch tests (requires cargo-watch)
test-watch:
	@echo "Watching tests..."
	cargo watch -x "test --lib"

# Generate test documentation
test-docs:
	@echo "Generating test documentation..."
	cargo test --doc

# Run snapshot tests
test-snapshots:
	@echo "Running snapshot tests..."
	cargo test --features testutils
	@echo "Checking for snapshot changes..."
	git diff --exit-code test_snapshots/ || (echo "Snapshots changed - review carefully" && exit 1)

# Update snapshots
update-snapshots:
	@echo "Updating snapshots..."
	cargo test --features testutils
	git add test_snapshots/

# Pre-commit checks
pre-commit: format-check lint test coverage-check
	@echo "Pre-commit checks passed"

# CI simulation
ci: format-check lint audit test coverage-check build-optimized
	@echo "CI checks passed"

# Performance profiling
profile:
	@echo "Running performance profiling..."
	cargo test --benches --features testutils -- --profile-time=5

# Memory leak detection (requires valgrind)
memcheck:
	@echo "Running memory leak detection..."
	cargo test --features testutils
	valgrind --leak-check=full --show-leak-kinds=all target/debug/deps/predict_iq-*

# Stress test
stress-test:
	@echo "Running stress tests..."
	cargo test --release --features testutils -- --ignored

# Generate test report
test-report:
	@echo "Generating test report..."
	cargo test --workspace -- --format=json | tee test-report.json
	@echo "Test report saved to test-report.json"
