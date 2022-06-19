# Benchmark Setup

## Virtual Machines

On [Hetzner Cloud](https://hetzner.cloud) in region `nbg1`. OS is Ubuntu 22.04 (Hetzner image).

* `k3s-primary`
  * Machine Type: `CCX12` (AMD Epyc, 2 CPUs, 8GB RAM, 80GB SSD, 23.68€/month)
* `k3s-worker1`
  * Machine Type: `CCX12` (AMD Epyc, 2 CPUs, 8GB RAM, 80GB SSD, 41.53€/month)
* `k3s-worker2`
  * Machine Type: `CCX12` (AMD Epyc, 2 CPUs, 8GB RAM, 80GB SSD, 41.53€/month)

## Installation

1. **Install k3s**
   
   Use [quick installation method](https://rancher.com/docs/k3s/latest/en/installation/install-options/#options-for-installation-with-script).

   Install it on the primary VM:

   ```shell
   export INSTALL_K3S_VERSION="v1.22.10%2Bk3s1"
   curl -sfL https://get.k3s.io | sh -
   systemctl edit k3s
   ```
   
   Add:

   ```
   [Service]
   ExecStart=
   ExecStart=/usr/local/bin/k3s \
     server \
     --node-ip=10.0.0.2 \
     --flannel-iface=enp7s0
   ```

   And on the workers:

   ```shell
   export K3S_TOKEN="" # from primary at /var/lib/rancher/k3s/server/node-token
   export K3S_URL="https://10.0.0.2:6443"
   export INSTALL_K3S_VERSION="v1.22.10%2Bk3s1"
   curl -sfL https://get.k3s.io | sh -
   ```
   
   Do the same edit for the `k3s-agent` service.

2. **Deploy Docker Registry**

   From a [Helm chart](https://github.com/twuni/docker-registry.helm).

   ```shell
   helm repo add twuni https://helm.twun.io
   helm install docker-registry twuni/docker-registry -f values.yaml
   ```
   
   `values.yaml`:

   ```yaml
   service:
     type: NodePort
     nodePort: 32000
   persistence:
     enabled: true
     storageClass: local-path
   nodeSelector:
     node-role.kubernetes.io/master: 'true'
   ```

3. **Install Minio**

   ```shell
   helm repo add minio https://charts.min.io/
   helm install minio minio/minio --values values.yaml
   ```
   
   `values.yaml`:

   ```yaml
   mode: standalone
   rootUser: minioadmin
   rootPassword: minioadmin
   persistence:
     enabled: true
     storageClass: local-path
   resources:
     requests:
       memory: 256Mi
   consoleService:
     type: NodePort
     port: "9001"
     nodePort: 31207
   nodeSelector:
     node-role.kubernetes.io/master: 'true'
   ```
   
   Log into Minio Dashboard at `http://node-ip:31207` with credentials `minioadmin`/`minioadmin` and create the bucket `argo-workflows`.

4. **Install Jaeger**

   ```shell
   kubectl apply -f jaeger.yml
   ```

5. **Install Argo**

   ```shell
   kubectl create namespace argo-workflows
   kubectl apply -n argo-workflows -f https://github.com/argoproj/argo-workflows/releases/download/v3.3.6/namespace-install.yaml
   kubectl apply -n argo-workflows -f https://raw.githubusercontent.com/argoproj/argo-workflows/master/manifests/quick-start/base/agent-role.yaml
   kubectl apply -n argo-workflows -f https://raw.githubusercontent.com/argoproj/argo-workflows/master/manifests/quick-start/base/agent-default-rolebinding.yaml
   ```
   
   :warning: Role `agent` must be extended:

   ```yaml
   - apiGroups:
     - argoproj.io
     resources:
     - workflowtaskresults
     verbs:
     - create
   ```
   
   Edit deployments to add `node-role.kubernetes.io/master: 'true'` to `nodeSelector`.

   Edit the `workflow-controller` deployment add add `ARGO_EXECUTOR_PLUGINS=true`.

   Create the ConfigMap `workflow-controller-configmap` (namespace `argo-workflows`):

   ```yaml
   apiVersion: v1
   kind: ConfigMap
   metadata:
     name: workflow-controller-configmap
   namespace: argo-workflows
   data:
     artifactRepository: |
     archiveLogs: true
     s3:
       accessKeySecret:
         name: argo-workflows-s3
         key: access_key
       secretKeySecret:
         name: argo-workflows-s3
         key: secret_key
     insecure: true
     bucket: argo-workflows
     endpoint: minio.default.svc.cluster.local:9000
     region: eu-central-1
     pathStyleEndpoint: true
   ```

   And the Secret `argo-workflows-s3` (namespace `argo-workflows`):

   ```yaml
   apiVersion: v1
   kind: Secret
   metadata:
     name: argo-workflows-s3
     namespace: argo-workflows
   type: Opaque
   data:
     access_key: bWluaW9hZG1pbg==
     secret_key: bWluaW9hZG1pbg==
   ```

6. **Install `wasm-workflows-plugin`**

   Refer to the [installation guide in the README](/README.md#Installation)

7. **Setup Krustlet**

   Build the Docker image in [`krustlet`](krustlet).

   This needs a) a Krustlet binary (compile from source with `just build`) and b) a bootstrap config in [`krustlet/home/.krustlet/config/bootstrap`](krustlet/home/.krustlet/config) which you can find on the primary k3s node at `/etc/rancher/k3s/k3s.yaml`.

   Create the resources:

   ```shell
   kubectl apply -f krustlet1.yaml
   kubectl apply -f krustlet2.yaml
   ```
   
   Accept the CSRs:

   ```shell
   kubectl certificate approve krustlet1-tls
   kubectl certificate approve krustlet1-tls
   ```
