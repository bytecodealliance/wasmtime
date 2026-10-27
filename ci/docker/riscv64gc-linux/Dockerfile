FROM ubuntu:22.04@sha256:e0f80c829509ef4df9e76bd8944eb7b8ec25cbb379b112a7f9448b2c77a77c9b

RUN apt-get update -y && apt-get install -y gcc gcc-riscv64-linux-gnu ca-certificates cmake git ninja-build
RUN git config --global --add safe.directory '*'

ENV CARGO_TARGET_RISCV64GC_UNKNOWN_LINUX_GNU_LINKER=riscv64-linux-gnu-gcc
