# demo-mod

cargo-wasi is required: `cargo install cargo-wasi`.

Building: `cargo wasi build` creates `target/wasm32-wasi/debug/demo_mod.wasm`.

Push to local registry: `wasm-to-oci push --use-http target/wasm32-wasi/debug/demo_mod.wasm 192.168.64.2:32000/demo_mod:latest`
