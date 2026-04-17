# Stage 1: Build
FROM rust:1.80-slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/althread

# Copy the workspace manifest and the project files
COPY Cargo.toml Cargo.lock ./
COPY cli/Cargo.toml cli/
COPY interpreter/Cargo.toml interpreter/
COPY web/Cargo.toml web/

# Create dummy source files to pre-build dependencies
RUN mkdir -p cli/src interpreter/src web/src interpreter/benches && \
    touch cli/src/main.rs interpreter/src/lib.rs web/src/lib.rs interpreter/benches/bench-examples.rs

# Build dependencies (this layer will be cached)
RUN cargo build --release -p althread-cli

# Copy the actual source code
COPY cli/src cli/src
COPY interpreter/src interpreter/src

# Re-build with actual source
# We need to touch the main.rs to ensure cargo re-builds it
RUN touch cli/src/main.rs && cargo build --release -p althread-cli

# Stage 2: Runtime
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy the binary from the builder
COPY --from=builder /usr/src/althread/target/release/althread-cli /usr/local/bin/althread-cli

ENTRYPOINT ["althread-cli"]
