FROM ubuntu:16.04@sha256:1f1a2d56de1d604801a9671f301190704c25d604a416f59e03c04f5c6ffee0d6

RUN apt-get update -y && apt-get install -y gcc gcc-s390x-linux-gnu ca-certificates curl make git ninja-build
RUN git config --global --add safe.directory '*'

# The CMake in Ubuntu 16.04 was a bit too old for us to use so download one from
# CMake's own releases and use that instead.
RUN curl --retry 5 -L -o cmake.tar.gz \
    https://github.com/Kitware/CMake/releases/download/v3.29.3/cmake-3.29.3-linux-x86_64.tar.gz && \
  tar xf cmake.tar.gz && \
  rm cmake.tar.gz
ENV PATH=$PATH:/cmake-3.29.3-linux-x86_64/bin

ENV CARGO_TARGET_S390X_UNKNOWN_LINUX_GNU_LINKER=s390x-linux-gnu-gcc
