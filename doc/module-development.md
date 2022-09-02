# Module Development

Creating a new Wasm module for use with Argo Workflows is easy and works with every language.

There are ready-to-use templates for:

* [Rust](wasm-modules/templates/rust/)
* [TinyGo](wasm-modules/templates/tinygo/)

You implement a [WASI](https://wasi.dev) module. WASI is a modular system interface for Wasm. The principle is easy: the module is given its input in a file at `/work/input.json`. It is expected to write its results to a file at `/work/result.json` and exit.

We created an easy-to-use wrapper for Rust. The wrapper abstracts all the file handling magic and lets you implement a function with a signature like this:

```rust
fn run(invocation: PluginInvocation) -> anyhow::Result<PluginResult> {
    // This is where your code goes

    PluginResult {
        phase: Phase::Succeeded,
        message: "Done".to_string(),
        outputs: Default::default(),
    }
}
```

For any other language you can easily parse the JSON yourself:

* PluginInvocation: [Example](crates/workflow-model/doc/plugin-invocation.example.json), [Schema](crates/workflow-model/doc/plugin-invocation.schema.json)
* PluginResult: [Example](crates/workflow-model/doc/plugin-result.example.json), [Schema](crates/workflow-model/doc/plugin-result.schema.json)

## Capabilities

Capabilities expand what modules can do. Out of the box, modules can take input parameters and artifacts and produce some output. Take a look at the [capabilities for wasmCloud](https://wasmcloud.dev/reference/host-runtime/capabilities/) for a more complete list of useful capabilities. The capabilities that this plugin offers will be extended in the future.

### HTTP Capability

The HTTP capability provider allows you to make HTTP requests from your Wasm module. The capability is available in every module mode. Please refer to the [`wasi-experimental-http`](https://github.com/deislabs/wasi-experimental-http) repository for complete information of how to access the HTTP capability from your module. There you will find examples for Rust.

When using the HTTP capability, you need to whitelist the hosts that the module is allowed to connect to. This illustrates the ease-of-use that WebAssembly's capability-oriented security model offers: for you, it's very easy to tell if a module should be able to connect outside – and now securing your code got easy.

You can find a full-featured module at [`wasm-modules/contrib/http-request`](wasm-modules/contrib/http-request/).

The module supports the following input parameters:

* `url`: the URL that you want to call
* `method`: HTTP request method (e.g. `GET`, `POST`, etc.) – optional, defaults to `GET`
* `body`: HTTP request body as a string – optional
* `content_type`: HTTP request body content type (e.g. `application/json`) – optional

As a result, you get the following output parameters:

* `status_code`: HTTP response status code as a number (e.g. `200`)
* `body`: HTTP response body as a string
* `content_type`: HTTP response body content type (e.g. `text/plain`)

The `http-request` module can be used in a workflow like so:

```yaml
- name: wasm
  inputs:
    parameters:
    - name: url
      value: https://httpbin.org/post
    - name: method
      value: POST
    - name: body
      value: Hello World
    - name: content_type
      value: text/plain
  plugin:
    wasm:
      module:
        oci: ghcr.io/shark/wasm-workflows-plugin-http-request:latest
      permissions:
        http:
          allowed_hosts:
          - https://httpbin.org
```
