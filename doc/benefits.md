# Benefits

* :lock: **Security**

  The [list of things to do](https://cheatsheetseries.owasp.org/cheatsheets/Docker_Security_Cheat_Sheet.html) when you want to run containers securely is long and the topic is more complex than even ambitious users care about. Containers are [vulnerable in many ways](https://ieeexplore.ieee.org/document/8693491) because of their denylist approach to security: they're allowed to do many things by default.

  WebAssembly's security model is the opposite. As with smartphone apps, they must be given permission for potentially infringing tasks. For example, you might want to give a module the permission to read and write files but not communicate over the internet.

  Container images from third parties you don't know are usually a security nightmare. With WebAssembly, you can run code you don't fully trust with more confidence. Say you have a workflow step that renders Markdown. When the author of your Markdown parser container image decides to deliver a crypto miner instead, most Kubernetes setups will happily run it. If you were using this project and a WebAssembly module: zero chance, since it's easy for you to know that the step doesn't need the network but only takes an input parameter and produces some output. [This example is not made up](https://www.trendmicro.com/vinfo/fr/security/news/virtualization-and-cloud/malicious-docker-hub-container-images-cryptocurrency-mining).

  <details>
    <summary>More about the difference between containers and Wasm modules</summary>
    <img src="doc/container-vs-wasm.png" style="max-width: 700px">
    <p>Linux processes use more than 300 system calls for any task that involves sharing data with outside of a process. Containers are a combination of different Linux Kernel technologies (namespaces, cgroups etc.) that segment one computer into many seemlingly independent containers. But this very much depends on a) the secure implementation of all syscalls not to leak anything and b) trust in the application inside the container to do what the user intends it to.</p>
    <p>Wasm modules are very restricted by default. We use application-level capabilities to allow them to access external resources like the network, S3 object stores, or the filesystem. The modules are the capability consumers, the Wasm runtime is the capability provider. The capability provider translates the requests from the Wasm module and acts as a secure proxy to the outside.</p>
  </details>

* :runner: **Performance**

  Containers have some overhead: for each workflow step, Argo creates new Kubernetes Pod. This Pod has several containers to enable all the Argo features, your code is just one of them. All the containers must execute, then results are gathered and sent back to Argo. This all takes time: container images are often towards 100s of megabytes, they may rely on interpreted languages like Python or have huge dependencies leading to a slow start time. You may know the [Cold Start issue](https://aws.amazon.com/blogs/compute/operating-lambda-performance-optimization-part-1/) with Function-as-a-Service. In Argo, every workflow step is a cold start.

  Because WebAssembly modules don't have to bring a whole operating system, they're much smaller. And there is less setup work to do, even for interpreted languages. This means that a module can be run in a matter of milliseconds rather than tens of seconds.
