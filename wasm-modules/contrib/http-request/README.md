# http-request

This module uses the HTTP capability to make outgoing HTTP requests. Learn more about capabilities in the [main README](/README.md).

The HTTP capability provider allows you to make HTTP requests from your Wasm module. The capability is available in every module mode. Please refer to the [`wasi-experimental-http`](https://github.com/deislabs/wasi-experimental-http) repository for complete information of how to access the HTTP capability from your module. There you will find examples for Rust.

When using the HTTP capability, you need to whitelist the hosts that the module is allowed to connect to. This illustrates the ease-of-use that WebAssembly's capability-oriented security model offers: for you it's very easy to tell if a module should be able to connect outside – and now securing your code got easy.

The module supports the following input parameters:

* `url`: the URL that you want to call
* `method`: HTTP request method (e.g. `GET`, `POST`, etc.) – optional, defaults to `GET`
* `body`: HTTP request body as a string – optional
* `content_type`: HTTP request body content type (e.g. `application/json`) – optional

As a result, you get the following output parameters:

* `status_code`: HTTP response status code as a number (e.g. `200`)
* `body`: HTTP response body as a string
* `content_type`: HTTP response body content type (e.g. `text/plain`)

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

This will create `target/wasm32-wasi/debug/http-request.wasm`.

You can push this module to an OCI registry using [wasm-to-oci](https://github.com/engineerd/wasm-to-oci):

```shell
wasm-to-oci push --use-http target/wasm32-wasi/debug/http-request.wasm 192.168.64.2:32000/demo_mod:latest
```
