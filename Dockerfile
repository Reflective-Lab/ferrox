# ─── Stage 1: Build C++ libraries ────────────────────────────────────────────
FROM debian:trixie-slim AS cpp-builder

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
FROM rust:1.94-trixie AS rust-builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential cmake clang libclang-dev protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

COPY --from=cpp-builder /build/ortools-build       /opt/ortools-build
COPY --from=cpp-builder /build/ortools/include     /opt/ortools/include
COPY --from=cpp-builder /build/highs-build         /opt/highs-build
COPY --from=cpp-builder /build/highs/src           /opt/highs/src

WORKDIR /workspace

# Build context is the ferrox repo root.
COPY . /workspace/

# Strip path = "../converge/..." so cargo resolves converge crates from crates.io.
RUN sed -i -E 's|path = "\.\./converge/[^"]*",[[:space:]]*||g' /workspace/Cargo.toml && \
    rm -f /workspace/Cargo.lock

ENV FERROX_ORTOOLS_ROOT=/opt/ortools-build
ENV FERROX_HIGHS_ROOT=/opt/highs-build

RUN cargo build --release \
      --package ferrox-server --features ferrox-server/full

# ─── Stage 3: Minimal runtime ────────────────────────────────────────────────
FROM debian:trixie-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    libstdc++6 ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=cpp-builder /build/ortools-build/lib/libortools.so* /usr/local/lib/
COPY --from=cpp-builder /build/highs-build/lib/libhighs.so*     /usr/local/lib/

RUN ldconfig

COPY --from=rust-builder \
     /workspace/target/release/ferrox-server \
     /usr/local/bin/ferrox-server

VOLUME ["/tls"]
EXPOSE 50051

ENV FERROX_ADDR=0.0.0.0:50051
ENV FERROX_TLS_CERT=/tls/server.crt
ENV FERROX_TLS_KEY=/tls/server.key

ENTRYPOINT ["/usr/local/bin/ferrox-server"]
