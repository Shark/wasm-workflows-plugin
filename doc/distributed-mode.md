# :test_tube: Distributed Mode

This is provided as a technical prototype and not considered production-ready. Much like Argo itself creates a Pod for each workflow task, the distributed mode will create Pods for Wasm modules. It creates a pod for each workflow task. The pod is executed by a virtual Kubernetes node that is provided by Krustlet.

What makes this possible is [Krustlet](https://docs.krustlet.dev). Krustlet shows up as a new node in your Kubernetes cluster, and it will execute Wasm modules that Kubernetes schedules to this node. I [forked Krustlet](https://github.com/Shark/krustlet) since there are some changes necessary to make Krustlet aware of the particularities of running workflow tasks as Wasm modules (inputs, outputs, parameters, artifacts, etc.).

For the distributed mode, you need to do a bit more:

* Create a service account and proper credentials for this plugin. See [`argo-plugin/rbac.yaml`](/argo-plugin/rbac.yaml) for details.
* Inject the service account credentials into the plugin container. For this, there is a special [`distributed-mode-plugin.yaml`](distributed-mode-plugin.yaml) showing you how.
