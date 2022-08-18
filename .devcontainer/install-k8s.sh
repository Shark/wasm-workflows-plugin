#!/usr/bin/env bash
set -euo pipefail

install_kubectl() {
  if command -v kubectl &> /dev/null; then
    echo "kubectl already installed"
    return 0
  fi

  curl -sSL -o /usr/local/bin/kubectl https://storage.googleapis.com/kubernetes-release/release/$(curl -s https://storage.googleapis.com/kubernetes-release/release/stable.txt)/bin/linux/amd64/kubectl
  chmod +x /usr/local/bin/kubectl
}

install_kind() {
  if command -v kind &> /dev/null; then
    echo "kind already installed"
    return 0
  fi

  curl -sSL -o /usr/local/bin/kind https://github.com/kubernetes-sigs/kind/releases/download/v0.14.0/kind-linux-amd64
  chmod +x /usr/local/bin/kind
}

install_helm() {
  if command -v helm &> /dev/null; then
    echo "helm already installed"
    return 0
  fi

  curl -sSL https://get.helm.sh/helm-v3.9.3-linux-amd64.tar.gz | tar xvz -C /usr/local/bin --strip-components=1 linux-amd64/helm
  chmod +x /usr/local/bin/helm
}

install_k9s() {
  if command -v k9s &> /dev/null; then
    echo "k9s already installed"
    return 0
  fi

  curl -sSL https://github.com/derailed/k9s/releases/download/v0.26.3/k9s_Linux_x86_64.tar.gz | tar xvz -C /usr/local/bin k9s
}

install_argo() {
  if command -v argo &> /dev/null; then
    echo "argo already installed"
    return 0
  fi

  curl -sSL https://github.com/argoproj/argo-workflows/releases/download/v3.3.9/argo-linux-amd64.gz | gunzip > /usr/local/bin/argo-linux-amd64
    mv /usr/local/bin/argo-linux-amd64 /usr/local/bin/argo
    chmod +x /usr/local/bin/argo
}

main() {
  install_kubectl
  install_kind
  install_helm
  install_k9s
  install_argo
}

main "$@"
