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

# Like CI, turn off incremental compilation to reduce image size. Line tables still
# give file and line numbers in backtraces and gdb. These stay set in the final
# image so that later `cargo` invocations reuse the prebuilt artifacts.
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH \
    CARGO_INCREMENTAL=0 \
    CARGO_PROFILE_DEV_DEBUG=line-tables-only \
    CARGO_PROFILE_TEST_DEBUG=line-tables-only

# Use the nightly that CI pins to match OSS-Fuzz (`wasmtime-ci-oss-fuzz-pin`
# in `.github/actions/install-rust/action.yml`) for everything: the fuzz
# targets need a nightly, and a single toolchain keeps things simple.
RUN toolchain=$(sed -n '/wasmtime-ci-oss-fuzz-pin/,/version=/s/.*version=\(nightly-[0-9-]*\).*/\1/p' \
                  .github/actions/install-rust/action.yml) \
    && test -n "$toolchain" \
    && curl -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path \
         --profile minimal --default-toolchain "$toolchain" \
    && rustup target add wasm32-wasip1 wasm32-wasip2 wasm32-unknown-unknown \
    && cargo install cargo-fuzz --version 0.13.2 \
    && cargo install wasm-tools

# Download every crate the workspace (including the fuzz targets and the test
# programs compiled to wasm) could need, so offline builds work later.
RUN cargo fetch

# Everything below is built in the dev profile, so debug assertions are on.
#
# Cargo rebuilds crates whenever the set of enabled features changes, so build
# the configurations that are most likely to be used later:
#
# 1. The `wasmtime` CLI at `target/debug/wasmtime`, and the tests at the
#    repository root (`tests/all`, `tests/wast.rs`, `tests/disas.rs`, ...) with
#    default features, as used by `cargo run` and `cargo test`.
RUN cargo build && cargo test --no-run
# 2. `clif-util` at `target/debug/clif-util`, for Cranelift's `.clif` tests.
RUN cargo build -p cranelift-tools
# 3. The full test suite as CI runs it, with all features enabled.
RUN ./ci/run-tests.py --no-run

# A release build of the CLI at `target/release/wasmtime`, to check whether a
# bug also reproduces without debug assertions.
RUN cargo build --release

# Build the fuzz targets the same way OSS-Fuzz does: optimized, with
# AddressSanitizer and debug assertions. They end up in
# `target/x86_64-unknown-linux-gnu/release/`. 
RUN cargo fuzz build --debug-assertions

# Smoke-test the build: run the spec test suite on the CLI's test harness. A
# failure here should be visible in the build log, but not abort the build.
RUN cargo test --locked --test wast -- --quiet \
    || echo "WARNING: wast tests reported failures"
