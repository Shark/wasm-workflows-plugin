# image-processor

You provide an image to this module, and it applies an effect and/or a watermark on it.

* Inputs
  * Artifacts
    * `input` (must be a JPEG image)
    * `watermark` (must be a JPEG image)
  * Parameter `effect`:
      * optional
      * allowed values: `saturate`, `desaturate`, `shift_hue`, `darken`, `lighten`
* Output Artifact: `output` (a JPEG image)

You'll find a sample workflow in [`workflow.yaml`](workflow.yaml).

The Wasm module is a public package on GitHub's Container Registry (ghcr.io), so you don't need to build or publish anything.

## Developing

**Prerequisites:**
* A working Rust toolchain
* cargo-wasi is required: `cargo install cargo-wasi`

**Building:**

```shell
cargo wasi build
```

This will create `target/wasm32-wasi/debug/image-processor.wasm`.

You can push this module to an OCI registry using [wasm-to-oci](https://github.com/engineerd/wasm-to-oci):

```shell
wasm-to-oci push --use-http target/wasm32-wasi/debug/image-processor.wasm 192.168.64.2:32000/image-processor:latest
```
