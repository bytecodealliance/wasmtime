FROM almalinux:8

RUN dnf install -y git gcc make

# The CMake in AlmaLinux 8 was a bit too old for us to use so download one from
# CMake's own releases and use that instead.
RUN curl -L https://github.com/Kitware/CMake/releases/download/v3.29.3/cmake-3.29.3-linux-x86_64.tar.gz | tar xzf -
ENV PATH=$PATH:/cmake-3.29.3-linux-x86_64/bin

ENV PATH=$PATH:/rust/bin
