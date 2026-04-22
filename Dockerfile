# ─── Stage 1: Build C++ libraries ────────────────────────────────────────────
FROM debian:bookworm-slim AS cpp-builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential cmake git ca-certificates python3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

ARG ORTOOLS_TAG=v9.15
RUN git clone --depth 1 --branch ${ORTOOLS_TAG} \
      https://github.com/google/or-tools /build/ortools && \
    cmake -S /build/ortools -B /build/ortools-build \
      -DCMAKE_BUILD_TYPE=Release \
      -DBUILD_DEPS=ON \
      -DBUILD_SHARED_LIBS=ON \
      -DBUILD_EXAMPLES=OFF \
      -DBUILD_TESTS=OFF \
      -DUSE_GLOP=ON \
      -DUSE_CP_SAT=ON \
      -DUSE_SCIP=OFF \
      -DUSE_COINOR=OFF && \
    cmake --build /build/ortools-build --target ortools -j"$(nproc)"

ARG HIGHS_TAG=v1.14.0
RUN git clone --depth 1 --branch ${HIGHS_TAG} \
      https://github.com/ERGO-Code/HiGHS /build/highs && \
    cmake -S /build/highs -B /build/highs-build \
      -DCMAKE_BUILD_TYPE=Release \
      -DBUILD_SHARED_LIBS=ON \
      -DFAST_BUILD=ON && \
    cmake --build /build/highs-build -j"$(nproc)"

# ─── Stage 2: Compile Rust server ────────────────────────────────────────────
FROM rust:1.85-bookworm AS rust-builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential cmake clang libclang-dev protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

COPY --from=cpp-builder /build/ortools-build /opt/ortools-build
COPY --from=cpp-builder /build/ortools/include /opt/ortools/include
COPY --from=cpp-builder /build/highs-build   /opt/highs-build
COPY --from=cpp-builder /build/highs/src      /opt/highs/src

WORKDIR /workspace

# Copy the ferrox workspace (relative to the build context root, which is
# the parent dir of ferrox/ so that converge/ is also accessible).
COPY ferrox/  /workspace/ferrox/
COPY converge/crates/pack          /workspace/converge/crates/pack/
COPY converge/crates/model         /workspace/converge/crates/model/
COPY converge/crates/provider-api  /workspace/converge/crates/provider-api/

ENV FERROX_ORTOOLS_ROOT=/opt/ortools-build
ENV FERROX_HIGHS_ROOT=/opt/highs-build

RUN cargo build --release --manifest-path /workspace/ferrox/Cargo.toml \
      --package ferrox-server --features ferrox-server/full

# ─── Stage 3: Minimal runtime ────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    libstdc++6 ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# OR-Tools shared library
COPY --from=cpp-builder /build/ortools-build/lib/libortools.so* /usr/local/lib/
# HiGHS shared library
COPY --from=cpp-builder /build/highs-build/lib/libhighs.so*     /usr/local/lib/

RUN ldconfig

COPY --from=rust-builder \
     /workspace/ferrox/target/release/ferrox-server \
     /usr/local/bin/ferrox-server

VOLUME ["/tls"]
EXPOSE 50051

ENV FERROX_ADDR=0.0.0.0:50051
ENV FERROX_TLS_CERT=/tls/server.crt
ENV FERROX_TLS_KEY=/tls/server.key

ENTRYPOINT ["/usr/local/bin/ferrox-server"]
