#!/usr/bin/env bash
kubectl get workflow -n argo-workflows --no-headers | awk '{print $1}' | xargs kubectl delete workflow -n argo-workflows
kubectl get pod -n argo-workflows --no-headers | awk '/wasm-workflow-/{print $1}' | xargs kubectl delete pod -n argo-workflows
kubectl get configmaps -n argo-workflows --no-headers | awk '/wasm-workflow-/{print $1}' | xargs kubectl delete configmap -n argo-workflows
