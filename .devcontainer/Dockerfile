FROM rust:1.63

RUN apt-get update \
 && apt-get install -y --no-install-recommends zsh sudo openjdk-11-jre pre-commit lld less \
 && adduser --disabled-password --u 1000 --gid 65534 --gecos '' rust \
 && echo "rust ALL = (ALL) NOPASSWD: ALL" >> /etc/sudoers.d/rust

COPY install-k8s.sh /tmp/
RUN /tmp/install-k8s.sh \
 && rm /tmp/install-k8s.sh

USER rust
RUN touch ~/.zshrc \
 && rustup component add rustfmt \
 && rustup component add clippy \
 && cargo install cargo-expand \
 && cargo install cargo-wasi \
 && mkdir ~/.cargo \
 && git clone https://github.com/ohmyzsh/ohmyzsh.git ~/.oh-my-zsh \
 && cp ~/.oh-my-zsh/templates/zshrc.zsh-template ~/.zshrc \
 && echo 'ZSH_THEME="simple"' >> ~/.zshrc

COPY <<EOF /home/rust/.cargo/config.toml
[target.x86_64-unknown-linux-gnu]
rustflags = ["-C", "link-arg=-fuse-ld=lld"]
EOF

ENV EDITOR="code --wait"
