# Build environment for Anthropic's OSS Scanner (https://red.anthropic.com/oss-scanner).
#
# The scanner builds this image with network access, using a checkout of this
# repository as the build context, and then analyzes Wasmtime inside it with no
# network access. Everything needed to build, test, and fuzz Wasmtime therefore
# has to be fetched here. See `threat_model.md` next to this file for what the
# scanner should focus on.
# Ubuntu 24.04 for its OCaml 4.14: the spec interpreter used by the
# `differential` fuzz target doesn't build against OCaml 5.
FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
      ca-certificates curl git build-essential clang lld cmake pkg-config \
      libssl-dev python3 gdb \
      ocaml-nox ocamlbuild ocaml-findlib libzarith-ocaml-dev \
    && rm -rf /var/lib/apt/lists/*

COPY . /src
WORKDIR /src

# The spec, WASI, and component-model test suites are git submodules.
RUN git submodule update --init --recursive

# Like CI, turn off incremental compilation and reduce debug info: with full
# debug info, the builds below need well over 100GB of disk. Line tables still
# give file and line numbers in backtraces and gdb. These stay set in the final
# image so that later `cargo` invocations reuse the prebuilt artifacts.
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH \
    CARGO_INCREMENTAL=0 \
    CARGO_PROFILE_DEV_DEBUG=line-tables-only \
    CARGO_PROFILE_TEST_DEBUG=line-tables-only

# Install the same toolchains as CI (see `.github/actions/install-rust`): stable
# is MSRV + 2, and the nightly is the one pinned to match OSS-Fuzz, used for the
# fuzz targets.
ARG FUZZ_NIGHTLY=nightly-2026-07-09
RUN msrv=$(grep 'rust-version.*1' Cargo.toml | sed 's/.*\.\([0-9]*\)\..*/\1/') \
    && curl -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path \
         --profile minimal --default-toolchain "1.$((msrv + 2)).0" \
    && rustup target add wasm32-wasip1 wasm32-wasip2 wasm32-unknown-unknown \
    && rustup toolchain install "$FUZZ_NIGHTLY" --profile minimal \
    && cargo install --locked cargo-fuzz --version 0.13.2 \
    && cargo install --locked wasm-tools

# Download every crate the workspace (including the fuzz targets and the test
# programs compiled to wasm) could need, so offline builds work later.
RUN cargo fetch --locked

# Everything below is built in the dev profile, so debug assertions are on.
#
# Cargo rebuilds crates whenever the set of enabled features changes, so build
# the configurations that are most likely to be used later:
#
# 1. The `wasmtime` CLI at `target/debug/wasmtime`, and the tests at the
#    repository root (`tests/all`, `tests/wast.rs`, `tests/disas.rs`, ...) with
#    default features, as used by `cargo run` and `cargo test`.
RUN cargo build --locked && cargo test --locked --no-run
# 2. `clif-util` at `target/debug/clif-util`, for Cranelift's `.clif` tests.
RUN cargo build --locked -p cranelift-tools
# 3. The full test suite as CI runs it, with all features enabled.
RUN ./ci/run-tests.py --locked --no-run

# A release build of the CLI at `target/release/wasmtime`, to check whether a
# bug also reproduces without debug assertions.
RUN cargo build --locked --release

# Build the fuzz targets the same way OSS-Fuzz does: optimized, with
# AddressSanitizer and debug assertions. They end up in
# `target/x86_64-unknown-linux-gnu/release/`. A failure here should not make
# the rest of the image unusable, so only warn about it.
RUN cargo "+$FUZZ_NIGHTLY" fuzz build --debug-assertions \
    || echo "WARNING: building the fuzz targets failed"

# Smoke-test the build: run the spec test suite on the CLI's test harness. A
# failure here should be visible in the build log, but not abort the build.
RUN cargo test --locked --test wast -- --quiet \
    || echo "WARNING: wast tests reported failures"

# No network from here on.
ENV CARGO_NET_OFFLINE=true
