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

# Run a named example (examples are standalone; build C++ deps first with `just deps`)
# DYLD_LIBRARY_PATH is needed because rpath from build.rs doesn't propagate to standalone examples.
example-cp:
    DYLD_LIBRARY_PATH="$(pwd)/vendor/ortools/build/lib:${DYLD_LIBRARY_PATH:-}" \
        cargo run --manifest-path examples/cp_sudoku/Cargo.toml --features ferrox/ortools

example-mip:
    DYLD_LIBRARY_PATH="$(pwd)/vendor/highs/build/lib:${DYLD_LIBRARY_PATH:-}" \
        cargo run --manifest-path examples/highs_mip/Cargo.toml --features ferrox/highs

example-maatw:
    DYLD_LIBRARY_PATH="$(pwd)/vendor/ortools/build/lib:${DYLD_LIBRARY_PATH:-}" \
        cargo run --release --manifest-path examples/maatw/Cargo.toml

example-jspbench:
    DYLD_LIBRARY_PATH="$(pwd)/vendor/ortools/build/lib:${DYLD_LIBRARY_PATH:-}" \
        cargo run --release --manifest-path examples/jspbench/Cargo.toml

example-vrptw:
    DYLD_LIBRARY_PATH="$(pwd)/vendor/ortools/build/lib:${DYLD_LIBRARY_PATH:-}" \
        cargo run --release --manifest-path examples/vrptw/Cargo.toml

# Benchmarks
bench:
    cargo bench --workspace --features ferrox/full

# ── gRPC server ───────────────────────────────────────────────────────────────

# Run gRPC server locally (no TLS)
server:
    cargo run --package ferrox-server --features ferrox-server/full

# ── Docker ───────────────────────────────────────────────────────────────────

# Build the Docker image (context = parent dir so converge/ is reachable)
docker-build:
    docker build -f Dockerfile -t ferrox-server:latest ..

# Run the container with certs from ./tls
docker-run:
    docker run --rm -p 50051:50051 -v "$(pwd)/tls:/tls:ro" ferrox-server:latest

# Bring up with docker compose
up:
    docker compose up --build

# Tear down
down:
    docker compose down

# Generate self-signed dev certs for localhost testing
tls-dev-certs:
    mkdir -p tls
    openssl req -x509 -newkey rsa:4096 -keyout tls/server.key \
      -out tls/server.crt -days 365 -nodes \
      -subj "/CN=ferrox-server" \
      -addext "subjectAltName=DNS:localhost,IP:127.0.0.1"
