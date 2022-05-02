# syntax=docker/dockerfile:1.3-labs
FROM rust:1.58 AS build
RUN cargo new app
COPY Cargo.toml Cargo.lock /app/
WORKDIR /app
RUN --mount=type=cache,target=/usr/local/cargo/registry cargo build --release
COPY ./src/ /app/src/
RUN --mount=type=cache,target=/usr/local/cargo/registry <<EOF
  set -e
  # update timestamps to force a new build
  touch /app/src/main.rs
  cargo build --release
  strip target/release/wasm-workflows-plugin
EOF

FROM ubuntu:latest
RUN apt-get update \
 && apt-get install -y libssl1.1 ca-certificates curl
COPY --from=build /app/target/release/wasm-workflows-plugin /wasm-workflows-plugin
ENTRYPOINT ["/wasm-workflows-plugin", "--bind", "0.0.0.0"]
