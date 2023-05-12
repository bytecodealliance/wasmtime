FROM ubuntu:16.04

RUN apt-get update -y && apt-get install -y gcc gcc-s390x-linux-gnu ca-certificates

ENV PATH=$PATH:/rust/bin
ENV CARGO_BUILD_TARGET=s390x-unknown-linux-gnu
ENV CARGO_TARGET_S390X_UNKNOWN_LINUX_GNU_LINKER=s390x-linux-gnu-gcc
