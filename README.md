[![Issues][issues-shield]][issues-url]
[![MIT License][license-shield]][license-url]

<br />
<div align="center">
  <h3 align="center">wasm-workflows-plugin</h3>

  <p align="center">
    Runs WebAssembly in your Argo Workflows! 🚀
    <br />
    <a href="https://github.com/Shark/wasm-workflows-plugin/#about-the-project"><strong>Find out why that's awesome »</strong></a>
    <!--
    <br />
    <br />
    <a href="https://github.com/Shark/wasm-workflows-plugin/doc/demo.md">View Demo</a>
    ·
    <a href="https://github.com/Shark/wasm-workflows-plugin/doc/use-cases.md">All Use Cases</a>
    -->
  </p>
</div>

<!-- TABLE OF CONTENTS -->
<details>
  <summary>Table of Contents</summary>
  <ol>
    <li>
      <a href="#about-the-project">About The Project</a>
      <ul>
        <li><a href="#built-with">Built With</a></li>
      </ul>
    </li>
    <li>
      <a href="#getting-started">Getting Started</a>
      <ul>
        <li><a href="#prerequisites">Prerequisites</a></li>
        <li><a href="#installation">Installation</a></li>
      </ul>
    </li>
    <li><a href="#usage">Usage</a></li>
    <li><a href="#roadmap">Roadmap</a></li>
    <li><a href="#contributing">Contributing</a></li>
    <li><a href="#license">License</a></li>
    <li><a href="#contact">Contact</a></li>
    <li><a href="#acknowledgments">Acknowledgments</a></li>
  </ol>
</details>

## About The Project

This is an <a href="https://github.com/argoproj/argo-workflows/blob/master/docs/executor_plugins.md">Executor Plugin</a> for <a href="https://argoproj.github.io/argo-workflows/">Argo Workflows</a> that runs WebAssembly modules!

These are the benefits of using Wasm instead of Docker containers in your workflows:

* :airplane: **Portability**

  You must build Docker containers individually for every CPU architecture. Working on a Mac with Apple Silicon, but your Kubernetes nodes run on Intel CPUs? You'll cross-compile your container images all day.

  Wasm modules are architecture-independent by design. Build once, run everywhere.

* :runner: **Performance**

  It takes a while for Kubernetes to spin up a container and run your code. The process has quite a few steps: pulling a container image, often 100s of megabytes in size, creating namespaces and virtual network interfaces. Starting the runtime for interpreted languages takes a while, too.

  Wasm does not emulate a complete operating system as containers do. They are a much simpler abstraction. This means that a module executes in a matter of milliseconds.

* :lock: **Security**

  Securing a container runtime [is a challenge](https://cheatsheetseries.owasp.org/cheatsheets/Docker_Security_Cheat_Sheet.html) because [containers are vulnerable in many ways](https://ieeexplore.ieee.org/document/8693491). Containers are powerful by design.

  Wasm is a minimal runtime that is just powerful enough to run a program. Rather than allowing everything by default, its security works more like on a smartphone, where you give apps permissions explicitly.

  [Read more about the benefits here.](doc/benefits.md)

Even though Wasm is a new technology in Cloud Native, incorporating Wasm into your workflow is seamless:

* Containers and Wasm modules co-exist in the same workflow. You can pass artifacts and parameters between them.

* We have included [ready-to-use templates](wasm-modules/templates/), [examples](wasm-modules/examples/), and even [some useful modules for running off-the-shelf](wasm-modules/contrib/).

### Built with

Open Source software stands on the shoulders of giants. It wouldn't have been possible to build this tool without the authors of these projects:

* [Rust](https://rust-lang.org) is used to implement the Argo Executor Plugin API, pull and execute Wasm modules
* [Axum](https://github.com/tokio-rs/axum) is the Rust web framework to handle RPC calls
* [Wasmtime](https://github.com/bytecodealliance/wasmtime) is the WebAssembly Virtual Machine with [WASI support](https://wasi.dev)
* [oci-distribution](https://crates.io/crates/oci-distribution) allows the tool to pull Wasm modules from OCI registries
* [Best README Template](https://github.com/othneildrew/Best-README-Template)

## Getting Started

You must install Argo Workflows (v3.3.0 or newer) and the [`argo` CLI](https://argoproj.github.io/argo-workflows/cli/). `kubectl` needs access to your cluster.

**Install the plugin:**

Go to the [Releases page](https://github.com/Shark/wasm-workflows-plugin.git) and follow the descriptions for installing the plugin through the ConfigMap.

**Submit your first Wasm workflow:**

Run `argo submit --watch https://raw.githubusercontent.com/Shark/wasm-workflows-plugin/main/wasm-modules/examples/ferris-says/workflow.yaml`.

Add `--namespace XYZ` if your Argo installation is not running in the default namespace.

The workflow produces an output parameter `text` with a cool message:

```
 ___________________
/ "Hello World from \
\ WebAssembly"      /
 -------------------
        \
         \
            _~^~^~_
        \) /  o o  \ (/
          '_   -   _'
          / '-----' \
```

### Module Development

Creating a new Wasm module for use with Argo Workflows is described in the [Module Development Guide](doc/module-development.md).

### Advanced Features

* **Distributed Execution**

  The plugin will run Wasm modules within the plugin process by default. This is the recommended mode because it's easy to set up and is powerful enough for most scenarios.

  The distributed mode creates pods for Wasm modules in a workflow task, much like Argo does for Docker containers.

  [Read more in the Distributed Execution Guide.](doc/distributed-mode.md)

## Roadmap

We manage our roadmap on the [*Developing wasm-workflows-plugin* GitHub project board](https://github.com/users/Shark/projects/1/views/1).

## Contributing

Contributions make the open source community such an amazing place to learn, inspire, and create. Any contributions you make are **greatly appreciated**.

If you have a suggestion that would make this better, please fork the repo and create a pull request. You can also simply open an issue with the tag "enhancement".
Don't forget to give the project a star! Thanks again!

1. Fork the Project
2. Create your Feature Branch (`git checkout -b feature/AmazingFeature`)
3. Commit your Changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the Branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

## License

Distributed under the MIT License. See `LICENSE.txt` for more information.

## Contact

Felix Seidel – [@sh4rk](https://twitter.com/sh4rk) – felix@seidel.me

Project Link: [https://github.com/Shark/wasm-workflows-plugin](https://github.com/Shark/wasm-workflows-plugin)

## Acknowledgements

This is a research project as part of my Master Thesis at the [Chair of Prof. Dr. Holger Karl](https://www.hpi.de/karl/people/holger-karl.html) at [Hasso Plattner Institute](https://www.hpi.de), the University of Potsdam (Germany). Thank you for the ongoing support of my thesis!

[issues-shield]: https://img.shields.io/github/issues/Shark/wasm-workflows-plugin.svg?style=for-the-badge
[issues-url]: https://github.com/Shark/wasm-workflows-plugin/issues
[license-shield]: https://img.shields.io/github/license/Shark/wasm-workflows-plugin.svg?style=for-the-badge
[license-url]: https://github.com/Shark/wasm-workflows-plugin/blob/main/LICENSE.txt
