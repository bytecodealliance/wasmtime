FROM ubuntu:22.04@sha256:076e7bf82319619f28dd0aa9c02ab34b055ad37100395b4f49a2ffba04fb5e11

RUN apt-get update -y && apt-get install -y gcc gcc-riscv64-linux-gnu ca-certificates cmake git ninja-build
RUN git config --global --add safe.directory '*'

ENV CARGO_TARGET_RISCV64GC_UNKNOWN_LINUX_GNU_LINKER=riscv64-linux-gnu-gcc
