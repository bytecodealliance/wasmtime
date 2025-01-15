FROM almalinux:8

RUN dnf install -y git gcc make cmake git unzip glibc-devel.i686
RUN git config --global --add safe.directory '*'

WORKDIR /usr/local/bin
RUN curl -LO https://github.com/ninja-build/ninja/releases/download/v1.12.1/ninja-linux.zip
RUN unzip ./ninja-linux
WORKDIR /

ENV CC_i686-unknown-linux-gnu=gcc
