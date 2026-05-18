# Stage 1: Build
FROM rust:1.82-slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/althread

# Copy the entire project
COPY . .

# Build the project
RUN cargo build --release -p althread-cli

# Stage 2: Runtime
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy the binary from the builder
COPY --from=builder /usr/src/althread/target/release/althread-cli /usr/local/bin/althread-cli

ENTRYPOINT ["althread-cli"]
