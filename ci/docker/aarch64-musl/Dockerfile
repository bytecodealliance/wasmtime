FROM ghcr.io/cross-rs/aarch64-unknown-linux-musl:0.2.5@sha256:702154f52b2d8091671aa2c84d5582d849f949977228c735ff8462f93cc0e1e4

RUN apt-get update -y && apt-get install -y ninja-build
RUN git config --global --add safe.directory '*'
ENV EXTRA_RUSTFLAGS=-Ctarget-feature=-crt-static
