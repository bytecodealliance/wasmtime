FROM almalinux:8@sha256:2d0f2eb2a83cc09dbc2eb92fade4dbe8f709628149e81aa00aec70630406603b

RUN dnf install -y git gcc make cmake git unzip
RUN git config --global --add safe.directory '*'

WORKDIR /usr/local/bin
RUN curl --retry-connrefused --retry 5 -LO https://github.com/ninja-build/ninja/releases/download/v1.12.1/ninja-linux.zip
RUN unzip ./ninja-linux
WORKDIR /
