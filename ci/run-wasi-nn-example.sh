#!/bin/bash

# The following script demonstrates how to execute a machine learning inference using the wasi-nn module optionally
# compiled into Wasmtime. Calling it will download the necessary model and tensor files stored separately in $FIXTURE
# into $TMP_DIR (optionally pass a directory with existing files as the first argument to re-try the script). Then,
# it will compile the example code in crates/wasi-nn/tests/example into a Wasm file that is subsequently
# executed with the Wasmtime CLI.
set -e
WASMTIME_DIR=$(dirname "$0" | xargs dirname)
FIXTURE=https://gist.github.com/abrown/c7847bf3701f9efbb2070da1878542c1/raw/07a9f163994b0ff8f0d7c5a5c9645ec3d8b24024

# Inform the environment of OpenVINO library locations. Then we use OPENVINO_INSTALL_DIR below to avoid building all of
# OpenVINO from source (quite slow).
source /opt/intel/openvino/bin/setupvars.sh

# Build Wasmtime with wasi-nn enabled; we attempt this first to avoid extra work if the build fails.
OPENVINO_INSTALL_DIR=/opt/intel/openvino cargo build -p wasmtime-cli --features wasi-nn

# Download all necessary test fixtures to the temporary directory.
TMP_DIR=${1:-$(mktemp -d -t ci-XXXXXXXXXX)}
wget --no-clobber --directory-prefix=$TMP_DIR $FIXTURE/frozen_inference_graph.bin
wget --no-clobber --directory-prefix=$TMP_DIR $FIXTURE/frozen_inference_graph.xml
wget --no-clobber --directory-prefix=$TMP_DIR $FIXTURE/tensor-1x3x300x300-f32.bgr

# Now build an example that uses the wasi-nn API.
pushd $WASMTIME_DIR/crates/wasi-nn/examples/classification-example
cargo build --release --target=wasm32-wasi
cp target/wasm32-wasi/release/wasi-nn-example.wasm $TMP_DIR
popd

# Run the example in Wasmtime (note that the example uses `fixture` as the expected location of the model/tensor files).
OPENVINO_INSTALL_DIR=/opt/intel/openvino cargo run --features wasi-nn -- run --mapdir fixture::$TMP_DIR $TMP_DIR/wasi-nn-example.wasm

# Clean up.
rm -rf $TMP_DIR