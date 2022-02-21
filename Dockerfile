FROM rust:1.58 AS build
WORKDIR /usr/src
ENV CARGO_HOME=/cache/cargo
COPY . /usr/src/
RUN --mount=type=cache,target=/cache/cargo cargo build --release \
 && strip target/release/wasm-workflow-executor
RUN mkdir /wasm-cache

FROM gcr.io/distroless/base-debian11
ENV FS_CACHE_DIR=/wasm-cache
# nonroot user in distroless
USER 65532:65532
COPY --from=build --chown=65532:65532 /wasm-cache /wasm-cache
COPY --from=build /usr/src/target/release/wasm-workflow-executor /wasm-workflow-executor
COPY --from=build /lib/x86_64-linux-gnu/libgcc_s.so.1 /lib/x86_64-linux-gnu/
ENTRYPOINT ["/wasm-workflow-executor", "--bind", "0.0.0.0"]
