#!/bin/bash

# The following script demonstrates how to execute a machine learning inference using the wasi-nn module optionally
# compiled into Wasmtime. Calling it will download the necessary model and tensor files into $TMP_DIR (optionally pass
# a directory with existing files as the first argument to re-try the script). Then, it will compile the example code
# in crates/wasi-nn/tests/example into a Wasm file that is subsequently executed with the Wasmtime CLI.
# NOTE: This requires installing the TensorFlow C libraries. Instructions found here
# https://www.tensorflow.org/install/lang_c
set -e
WASMTIME_DIR=$(dirname "$0" | xargs dirname)
if [ -z "${1+x}" ]; then
    # If no temporary directory is specified, create one.
    TMP_DIR=$(mktemp -d -t ci-XXXXXXXXXX)
    REMOVE_TMP_DIR=1
else
    # If a directory was specified, use it and avoid removing it.
    TMP_DIR=$(realpath $1)
    REMOVE_TMP_DIR=0
fi

# Download all necessary test fixtures to the temporary directory.
wget --no-clobber https://github.com/bytecodealliance/wasi-nn/raw/performance/rust/examples/classification-example/models/mobilenet_v2/saved_model.pb --output-document=$TMP_DIR/saved_model.pb
mkdir $TMP_DIR/variables
wget --no-clobber https://github.com/bytecodealliance/wasi-nn/raw/performance/rust/examples/classification-example/models/mobilenet_v2/variables/variables.data-00000-of-00001 --output-document=$TMP_DIR/variables/variables.data-00000-of-00001
wget --no-clobber https://github.com/bytecodealliance/wasi-nn/raw/performance/rust/examples/classification-example/models/mobilenet_v2/variables/variables.index --output-document=$TMP_DIR/variables/variables.index
export BACKEND="tensorflow"
export MAPDIR="fixture"

# Now build an example that uses the wasi-nn API.
pushd $WASMTIME_DIR/crates/wasi-nn/examples/classification-example

cp src/train.jpg $TMP_DIR/train.jpg
cargo build --release --target=wasm32-wasi
cp target/wasm32-wasi/release/wasi-nn-example.wasm $TMP_DIR
popd

# Run the example in Wasmtime (note that the example uses `fixture` as the expected location of the model/tensor files).
cargo run -- run --mapdir fixture::$TMP_DIR $TMP_DIR/wasi-nn-example.wasm --wasi-modules=experimental-wasi-nn

# Clean up the temporary directory only if it was not specified (users may want to keep the directory around).
if [[ $REMOVE_TMP_DIR -eq 1 ]]; then
    rm -rf $TMP_DIR
fi
