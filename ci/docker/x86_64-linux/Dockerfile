FROM almalinux:8@sha256:768bedc30fe0a536adb0a65d2eddb7caad364e514fbddd0ae9e026975f590342

RUN dnf install -y git gcc make cmake git unzip
RUN git config --global --add safe.directory '*'

WORKDIR /usr/local/bin
RUN curl --retry-connrefused --retry 5 -LO https://github.com/ninja-build/ninja/releases/download/v1.12.1/ninja-linux.zip
RUN unzip ./ninja-linux
WORKDIR /
