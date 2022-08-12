# :test_tube: Distributed Mode

This mode is more advanced because it will orchestrate Wasm module invocations in a Kubernetes cluster. Much like Argo itself creates a Pod for each workflow task, the distributed mode will create Pods for Wasm modules.

What makes this possible is [Krustlet](https://docs.krustlet.dev). Krustlet shows up as a new node in your Kubernetes cluster, and it will execute Wasm modules that Kubernetes schedules to this node. I [forked Krustlet](https://github.com/Shark/krustlet) since there are some changes necessary to make Krustlet aware of the particularities of running workflow tasks as Wasm modules (inputs, outputs, parameters, artifacts, etc.).

For the distributed mode, you need to do a bit more:

* Create a service account and proper credentials for this plugin. See [`argo-plugin/rbac.yaml`](/argo-plugin/rbac.yaml) for details.
* Inject the service account credentials into the plugin container. For this, there is a special [`plugin-distributed.yaml`](/argo-plugin/plugin-distributed.yaml) showing you how.
