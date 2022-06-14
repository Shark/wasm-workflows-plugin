# syntax=docker/dockerfile:1.3-labs
FROM rust:1.60 AS chef
RUN cargo install cargo-chef --version "0.1.35"
WORKDIR app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release --bin image-processor

# We do not need the Rust toolchain to run the binary!
FROM ubuntu:jammy AS runtime
WORKDIR app
RUN apt-get update \
 && apt-get install -y ca-certificates curl \
 && mkdir -p /work/artifacts-in /work/artifacts-out
WORKDIR /work
COPY --from=builder /app/target/release/image-processor /usr/local/bin
ENTRYPOINT ["/usr/local/bin/image-processor"]
