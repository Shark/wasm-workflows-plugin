FROM rust:1.63
RUN apt-get update \
 && apt-get install -y --no-install-recommends zsh sudo openjdk-11-jre pre-commit \
 && adduser --disabled-password --u 1000 --gid 65534 --gecos '' rust \
 && echo "rust ALL = (ALL) NOPASSWD: ALL" >> /etc/sudoers.d/rust
USER rust
RUN touch ~/.zshrc \
 && rustup component add rustfmt \
 && rustup component add clippy \
 && cargo install cargo-expand
