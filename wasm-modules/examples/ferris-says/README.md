# ferris-says

This is a toy example which mimics the one of the [official examples in the Argo Workflows docs: whalesay](https://github.com/argoproj/argo-workflows/blob/master/examples/README.md#parameters). But rather than using a container with the `docker/whalesay` image, this does something way cooler and prints the text with the official Rust mascot ["Ferris"](https://rustacean.net) entirely from WebAssembly:

```
 _______________
< "Hello World" >
 ---------------
        \
         \
            _~^~^~_
        \) /  o o  \ (/
          '_   -   _'
          / '-----' \
```

You'll find a sample workflow in [`workflow.yaml`](workflow.yaml). The module expects an input parameter `text` and produces an output parameter of the same name.

The Wasm module is a public package on GitHub's Container Registry (ghcr.io), so you don't need to build or publish anything.

## Developing

**Prerequisites:**
* A working Rust toolchain
* cargo-wasi is required: `cargo install cargo-wasi`

**Building:**

```shell
cargo wasi build
```

This will create `target/wasm32-wasi/debug/ferris_says.wasm`.

You can push this module to an OCI registry using [wasm-to-oci](https://github.com/engineerd/wasm-to-oci):

```shell
wasm-to-oci push --use-http target/wasm32-wasi/debug/demo_mod.wasm 192.168.64.2:32000/demo_mod:latest
```
