FROM ubuntu:22.04@sha256:f64b2ffc2f10ab8f8505aa56b2664bd846cbb9df87c7e2532e30f7667021c0e6

RUN apt-get update -y && apt-get install -y gcc gcc-riscv64-linux-gnu ca-certificates cmake git ninja-build
RUN git config --global --add safe.directory '*'

ENV CARGO_TARGET_RISCV64GC_UNKNOWN_LINUX_GNU_LINKER=riscv64-linux-gnu-gcc
