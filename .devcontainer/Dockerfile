FROM rust:1.63

RUN apt-get update \
 && apt-get install -y --no-install-recommends zsh sudo openjdk-11-jre pre-commit \
 && adduser --disabled-password --u 1000 --gid 65534 --gecos '' rust \
 && echo "rust ALL = (ALL) NOPASSWD: ALL" >> /etc/sudoers.d/rust

# Install kubectl, k9s, kind, helm & the Argo Workflows CLI
RUN curl -sSL -o /usr/local/bin/kubectl https://storage.googleapis.com/kubernetes-release/release/$(curl -s https://storage.googleapis.com/kubernetes-release/release/stable.txt)/bin/linux/amd64/kubectl \
 && chmod +x /usr/local/bin/kubectl \
 && curl -sSL -o kind https://github.com/kubernetes-sigs/kind/releases/download/v0.14.0/kind-darwin-amd64 \
 && chmod +x kind \
 && curl -sSL https://get.helm.sh/helm-v3.9.3-linux-amd64.tar.gz | tar xz linux-amd64/helm -C /usr/local/bin --strip-components=1 \
 && curl -sSL https://github.com/derailed/k9s/releases/download/v0.26.3/k9s_Linux_x86_64.tar.gz | tar xz k9s -C /usr/local/bin
 && curl -sSL https://github.com/argoproj/argo-workflows/releases/download/v3.3.9/argo-linux-amd64.gz | gunzip > /usr/local/bin/argo-linux-amd64 \
 && mv /usr/local/bin/argo-linux-amd64 /usr/local/bin/argo \
 && chmod +x /usr/local/bin/argo

USER rust
RUN touch ~/.zshrc \
 && rustup component add rustfmt \
 && rustup component add clippy \
 && cargo install cargo-expand \
 && git clone https://github.com/ohmyzsh/ohmyzsh.git ~/.oh-my-zsh \
 && cp ~/.oh-my-zsh/templates/zshrc.zsh-template ~/.zshrc \
 && echo 'ZSH_THEME="simple"' >> ~/.zshrc
