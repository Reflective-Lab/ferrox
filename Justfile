# ferrox development commands
# Install: brew install just  |  cargo install just
# Usage:   just --list

set dotenv-load := true

# Show available recipes
default:
    @just --list

# Build without native solver features
build:
    cargo build --workspace

# Build with all solvers. Requires `just deps`.
build-full:
    cargo build --workspace --features ferrox/full

# Build release artifacts with all solvers
build-release:
    cargo build --workspace --release --features ferrox/full

# Check all supported feature combinations
check:
    cargo check --workspace
    cargo check --workspace --features ferrox/ortools
    cargo check --workspace --features ferrox/highs
    cargo check --workspace --features ferrox/full

# Run pure Rust tests
test:
    cargo test --workspace

# Run tests with OR-Tools linked. Requires `just deps-ortools`.
test-ortools:
    cargo test --workspace --features ferrox/ortools

# Run tests with HiGHS linked. Requires `just deps-highs`.
test-highs:
    cargo test --workspace --features ferrox/highs

# Run tests with both native solver stacks. Requires `just deps`.
test-full:
    cargo test --workspace --features ferrox/full

# Alias for the full test gate
test-all: test-full

# Check formatting
fmt-check:
    cargo fmt --all -- --check

# Format code
fmt:
    cargo fmt --all

# Run clippy for default and full solver configurations
clippy:
    cargo clippy --workspace --all-targets -- -D warnings
    cargo clippy --workspace --all-targets --features ferrox/full -- -D warnings

# Formatting plus clippy
lint: fmt-check clippy

# Build all native dependencies
deps:
    make all

# Build OR-Tools native dependency
deps-ortools:
    make ortools

# Build HiGHS native dependency
deps-highs:
    make highs

# Remove native dependency build artifacts
deps-clean:
    make clean

# Generate docs
doc:
    cargo doc --no-deps --workspace --features ferrox/full

# Generate and open docs
doc-open:
    cargo doc --no-deps --workspace --features ferrox/full --open

# Run CP-SAT Sudoku example. Requires `just deps-ortools`.
example-cp:
    DYLD_LIBRARY_PATH="$(pwd)/vendor/ortools/build/lib:${DYLD_LIBRARY_PATH:-}" \
        cargo run --manifest-path examples/cp_sudoku/Cargo.toml --features ferrox/ortools

# Run HiGHS MIP example. Requires `just deps-highs`.
example-mip:
    DYLD_LIBRARY_PATH="$(pwd)/vendor/highs/build/lib:${DYLD_LIBRARY_PATH:-}" \
        cargo run --manifest-path examples/highs_mip/Cargo.toml --features ferrox/highs

# Run multi-agent assignment example. Requires `just deps-ortools`.
example-maatw:
    DYLD_LIBRARY_PATH="$(pwd)/vendor/ortools/build/lib:${DYLD_LIBRARY_PATH:-}" \
        cargo run --release --manifest-path examples/maatw/Cargo.toml

# Run job-shop benchmark example. Requires `just deps-ortools`.
example-jspbench:
    DYLD_LIBRARY_PATH="$(pwd)/vendor/ortools/build/lib:${DYLD_LIBRARY_PATH:-}" \
        cargo run --release --manifest-path examples/jspbench/Cargo.toml

# Run VRPTW example. Requires `just deps-ortools`.
example-vrptw:
    DYLD_LIBRARY_PATH="$(pwd)/vendor/ortools/build/lib:${DYLD_LIBRARY_PATH:-}" \
        cargo run --release --manifest-path examples/vrptw/Cargo.toml

# Run Criterion benchmarks
bench:
    cargo bench --workspace --features ferrox/full

# Run gRPC server locally without TLS
server:
    cargo run --package ferrox-server --features ferrox-server/full

# Build the Docker image
docker-build:
    docker build -f Dockerfile -t ferrox-server:latest ..

# Run the Docker image with certs from ./tls
docker-run:
    docker run --rm -p 50051:50051 -v "$(pwd)/tls:/tls:ro" ferrox-server:latest

# Bring up the docker-compose stack
up:
    docker compose up --build

# Tear down the docker-compose stack
down:
    docker compose down

# Generate self-signed development certs for localhost testing
tls-dev-certs:
    mkdir -p tls
    openssl req -x509 -newkey rsa:4096 -keyout tls/server.key \
      -out tls/server.crt -days 365 -nodes \
      -subj "/CN=ferrox-server" \
      -addext "subjectAltName=DNS:localhost,IP:127.0.0.1"

# Session opener
focus: status check test

# Git status and recent commits
status:
    git status --short --branch
    git log --oneline -5

# Remove Rust build artifacts
clean:
    cargo clean
