# ─── Stage 1: Build C++ libraries ────────────────────────────────────────────
FROM debian:trixie-slim AS cpp-builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential cmake git ca-certificates python3 \
    && rm -rf /var/lib/apt/lists/*

# OR-Tools and HiGHS are built with the build dir as a subdir of the source
# tree (e.g. /build/ortools/build). The Rust sys crates rely on this layout:
# their build.rs sets ortools_src = ortools_build.parent().
ARG ORTOOLS_TAG=v9.15
RUN git clone --depth 1 --branch ${ORTOOLS_TAG} \
      https://github.com/google/or-tools /build/ortools && \
    cmake -S /build/ortools -B /build/ortools/build \
      -DCMAKE_BUILD_TYPE=Release \
      -DBUILD_DEPS=ON \
      -DBUILD_SHARED_LIBS=ON \
      -DBUILD_EXAMPLES=OFF \
      -DBUILD_TESTS=OFF \
      -DUSE_GLOP=ON \
      -DUSE_CP_SAT=ON \
      -DUSE_SCIP=OFF \
      -DUSE_COINOR=OFF && \
    cmake --build /build/ortools/build --target ortools -j"$(nproc)"

ARG HIGHS_TAG=v1.14.0
RUN git clone --depth 1 --branch ${HIGHS_TAG} \
      https://github.com/ERGO-Code/HiGHS /build/highs && \
    cmake -S /build/highs -B /build/highs/build \
      -DCMAKE_BUILD_TYPE=Release \
      -DBUILD_SHARED_LIBS=ON \
      -DFAST_BUILD=ON && \
    cmake --build /build/highs/build -j"$(nproc)"

# ─── Stage 2: Compile Rust server ────────────────────────────────────────────
FROM rust:1.94-trixie AS rust-builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential cmake clang libclang-dev protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Bring full source + build trees so the sys crates' build.rs finds headers
# under the source root and libs/_deps under the build dir.
COPY --from=cpp-builder /build/ortools /opt/ortools
COPY --from=cpp-builder /build/highs   /opt/highs

WORKDIR /workspace

# Build context is the ferrox repo root.
COPY . /workspace/

# Strip path = "../converge/..." so cargo resolves converge crates from crates.io.
RUN sed -i -E 's|path = "\.\./converge/[^"]*",[[:space:]]*||g' /workspace/Cargo.toml && \
    rm -f /workspace/Cargo.lock

ENV FERROX_ORTOOLS_ROOT=/opt/ortools/build
ENV FERROX_HIGHS_ROOT=/opt/highs/build

RUN echo "=== /opt/highs (top) ===" && ls /opt/highs/ | head -20 && \
    echo "=== /opt/highs/src ===" && ls /opt/highs/src/ 2>&1 | head -20 && \
    echo "=== Highs.h locations ===" && find /opt/highs -name "Highs.h" 2>/dev/null | head -5

RUN cargo build --release \
      --package ferrox-server --features ferrox-server/full

# ─── Stage 3: Minimal runtime ────────────────────────────────────────────────
FROM debian:trixie-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    libstdc++6 ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=cpp-builder /build/ortools/build/lib/libortools.so* /usr/local/lib/
COPY --from=cpp-builder /build/highs/build/lib/libhighs.so*     /usr/local/lib/

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
