FROM ubuntu:16.04

RUN apt-get update -y && apt-get install -y gcc gcc-s390x-linux-gnu ca-certificates curl make

# The CMake in Ubuntu 16.04 was a bit too old for us to use so download one from
# CMake's own releases and use that instead.
RUN curl -L https://github.com/Kitware/CMake/releases/download/v3.29.3/cmake-3.29.3-linux-x86_64.tar.gz | tar xzf -
ENV PATH=$PATH:/cmake-3.29.3-linux-x86_64/bin

ENV CARGO_TARGET_S390X_UNKNOWN_LINUX_GNU_LINKER=s390x-linux-gnu-gcc
