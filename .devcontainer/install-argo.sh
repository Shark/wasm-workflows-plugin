#!/usr/bin/env bash
set -euo pipefail

main() {
  kind create cluster \
    --config=doc/kind/kind-config.yaml \
    --image kindest/node:v1.22.9@sha256:8135260b959dfe320206eb36b3aeda9cffcb262f4b44cda6b33f7bb73f453105

  helm repo add minio https://charts.min.io/
  helm -n minio install --create-namespace minio minio/minio --values doc/kind/minio-values.yaml
  kubectl -n minio run \
    --rm --image=minio/mc --command --wait --attach --restart=Never create-bucket \
    -- bash -c "mc alias set minio http://minio.minio.svc.cluster.local:9000 minioadmin minioadmin && mc mb --ignore-existing minio/argo-workflows"

  kubectl create ns argowf
  kubectl -n argowf apply -k doc/kind/argo-workflows

  kubectl wait deployment -n argowf workflow-controller --for condition=Available=True --timeout=90s
}

main "$@"

#kubectl -n argowf create -f https://raw.githubusercontent.com/Shark/wasm-workflows-plugin/67ae607ec65e1ac93bc1749c5f7a9bde4b673cfd/wasm-modules/examples/ferris-says/workflow.yaml
#kubectl -n argowf wait workflow.argoproj.io/wasm-m786c --for condition=Completed=True --timeout=90s
#kubectl -n argowf get workflow.argoproj.io/wasm-m786c -o jsonpath={.status.phase}
