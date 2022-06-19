FROM ubuntu:jammy
RUN apt-get update \
 && apt-get install -y ca-certificates curl \
 && adduser --system --home /work --disabled-login work
COPY krustlet-wasi /usr/local/bin/
WORKDIR /work
ADD home /work
ENV KUBECONFIG=/work/.krustlet/config/kubeconfig RUST_LOG="info,regalloc=warn,workflow_model=debug,cranelift_codegen=info,wasi_provider=debug"
VOLUME /work
ENTRYPOINT ["/usr/local/bin/krustlet-wasi"]

