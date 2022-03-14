# text2qr

You provide a string to this module, and it returns a QR code rendered as text.

* Input parameter: `text` (example: `https://github.com/Shark/wasm-workflows-plugin`)
* Output parameter: `qrcode` (example: see below)

```
█████████████████████████████████████████
█████████████████████████████████████████
████ ▄▄▄▄▄ █▄█▄▄ █▄▄  ▄▀█▀ ▄▀█ ▄▄▄▄▄ ████
████ █   █ █▀▄▀▄▄▄▀▀▄▀▀▄▀▄█▄██ █   █ ████
████ █▄▄▄█ █ ▀ ██▀▄█▀▄  ▄█▀▄██ █▄▄▄█ ████
████▄▄▄▄▄▄▄█ █ █▄█ ▀ ▀▄█▄▀▄▀ █▄▄▄▄▄▄▄████
████▄▀██▄▀▄▄▄█▀ ▄▀ ▀ ▄▀ █ ▄▀▄ ▄▄▄▄██▄████
████ ▄ ▀▄▀▄▄▄▄█  █▀ ▀ ▀▀  ▄ ▀ ▀ █▄█ ▀████
████▀█  ▀▀▄▄▄▀ ▀ ▄▄█▀█▄▀▄▄▄▀██▄ ▄▄▀▀█████
████ ▀███▄▄▀█ ▀█ █▄█▄ ▀ ▀▀  ▀  ██▄▄▄▀████
████▀█▄▄▀█▄▄█▀▄▀█ ▀ ▀ ██  █ █ ▄▄▄▀▀ ▄████
████▄ ▀ ▀█▄ ██▀▀▄▀▀▄██▄█ ▀▀ ▄▀▀▀▀▄█ ▀████
████ ▀█▄█ ▄██▀▀█▀ ▀█▄█▀▀▀ █ ▄ ▄▀▄▄█▀▄████
██████▄▀▄▄▄ ▄▄ ▄▄ ▄ ▀▀██▄▀▀ █  ▄▄█▄██████
████▄▄▄▄▄█▄▄▀▀█▄ ▀▀▀ ▄▀▄▄▄▄█ ▄▄▄ ▀▀▄▄████
████ ▄▄▄▄▄ █▄█▄▀███▄█ ▀▀ ▀ █ █▄█ ▄█ █████
████ █   █ █▄██ ▀ ▀▄▀▀▄▀▄██▀▄▄    ▀██████
████ █▄▄▄█ ██▀  ▀  █▄▀▀ ▀▀▄█▀▀ ▀ ▄▄██████
████▄▄▄▄▄▄▄█▄▄▄██▄▄▄█▄▄█▄▄▄▄▄█▄█████▄████
█████████████████████████████████████████
▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀
```

You'll find a sample workflow in [`workflow.yaml`](workflow.yaml). The module expects an input parameter `text` and produces an output parameter named `qrcode`.

The Wasm module is a public package on GitHub's Container Registry (ghcr.io), so you don't need to build or publish anything.

## Developing

**Prerequisites:**
* A working Rust toolchain
* cargo-wasi is required: `cargo install cargo-wasi`

**Building:**

```shell
cargo wasi build
```

This will create `target/wasm32-wasi/debug/text2qr.wasm`.

You can push this module to an OCI registry using [wasm-to-oci](https://github.com/engineerd/wasm-to-oci):

```shell
wasm-to-oci push --use-http target/wasm32-wasi/debug/demo_mod.wasm 192.168.64.2:32000/demo_mod:latest
```
