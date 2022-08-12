# K8s Development

## Create a kind cluster

```
kind create cluster --config=kind-config.yaml --image kindest/node:v1.22.9@sha256:8135260b959dfe320206eb36b3aeda9cffcb262f4b44cda6b33f7bb73f453105

helm -n minio install --create-namespace minio minio/minio --values minio-values.yaml
kubectl -n minio run --rm --image=minio/mc --command --wait --attach create-bucket -- bash -c "mc alias set minio http://minio.minio.svc.cluster.local:9000 minioadmin minioadmin && mc mb minio/argo-workflows"

kubectl create ns argowf
kubectl -n argowf apply -k argo-workflows

# Create a sample workflow
kubectl -n argowf create -f https://raw.githubusercontent.com/argoproj/argo-workflows/master/examples/hello-world.yaml
```
