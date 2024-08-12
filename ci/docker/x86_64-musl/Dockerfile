# Rust binaries need `libgcc_s.so` but ubuntu's musl toolchain does not have it.
# Get it from alpine instead.
FROM alpine:3.16 as libgcc_s_src
RUN apk add libgcc

# Use something glibc-based for the actual compile because the Rust toolchain
# we're using is glibc-based in CI.
FROM ubuntu:24.04
RUN apt-get update -y && apt-get install -y cmake musl-tools
COPY --from=libgcc_s_src /usr/lib/libgcc_s.so.1 /usr/lib/x86_64-linux-musl

# Note that `-crt-feature` is passed here to specifically disable static linking
# with musl. We want a `*.so` to pop out so static linking isn't what we want.
ENV CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_RUSTFLAGS=-Ctarget-feature=-crt-static
ENV CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=musl-gcc
