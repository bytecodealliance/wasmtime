FROM ubuntu:16.04

RUN apt-get update -y && apt-get install -y gcc gcc-aarch64-linux-gnu ca-certificates

ENV PATH=$PATH:/rust/bin
ENV CARGO_BUILD_TARGET=aarch64-unknown-linux-gnu
ENV CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc
