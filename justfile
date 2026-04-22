set dotenv-load := true

# Build (no C++ features)
build:
    cargo build --workspace

# Build with all solvers (requires: make all)
build-full:
    cargo build --workspace --features ferrox/full

# Check all feature combinations
check:
    cargo check --workspace
    cargo check --workspace --features ferrox/ortools
    cargo check --workspace --features ferrox/highs
    cargo check --workspace --features ferrox/full

# Tests (pure Rust, no FFI)
test:
    cargo test --workspace

# Test with OR-Tools linked (requires: make ortools)
test-ortools:
    cargo test --workspace --features ferrox/ortools

# Test with HiGHS linked (requires: make highs)
test-highs:
    cargo test --workspace --features ferrox/highs

# Test with both (requires: make all)
test-full:
    cargo test --workspace --features ferrox/full

# Lint (must be clean before shipping)
lint:
    cargo fmt --check
    cargo clippy --all-targets -- -D warnings
    cargo clippy --all-targets --features ferrox/full -- -D warnings

# Format
fmt:
    cargo fmt

# Build C++ deps
deps:
    make all

deps-clean:
    make clean

# Docs
doc:
    cargo doc --no-deps --workspace --features ferrox/full --open

# Run a named example
example name:
    cargo run --example {{name}} --features ferrox/full

# Benchmarks
bench:
    cargo bench --workspace --features ferrox/full
