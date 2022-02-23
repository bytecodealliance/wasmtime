#! /bin/bash

set -e

TMP_DIR=$(mktemp -d -t ci-XXXXXXXXXX)
pushd "$TMP_DIR"

WASMSIGN="${TMP_DIR}/bin/wasmsign2"
WASM_MODULE="${TMP_DIR}/bin/test-module.wasm"
PUBLIC_KEY="${TMP_DIR}/public.key"
SECRET_KEY="${TMP_DIR}/secret.key"

cargo install --version 0.1.4 --root "$TMP_DIR" wasmsign2-cli
"$WASMSIGN" keygen -K "$PUBLIC_KEY" -k "$SECRET_KEY"

rm -fr test-module
cargo new test-module
pushd test-module
cargo install --root "$TMP_DIR" --path . --target=wasm32-wasi
popd

popd

cargo run --features digital-signatures -- run "$WASM_MODULE"

if cargo run --features digital-signatures -- run --experimental-public-keys "$PUBLIC_KEY" "$WASM_MODULE"; then
  echo "Module ran even though signature verification failed" >&2
  exit 1
fi

"$WASMSIGN" sign -k "$SECRET_KEY" -i "$WASM_MODULE" -o "$WASM_MODULE"
cargo run --features digital-signatures -- run --experimental-public-keys "$PUBLIC_KEY" "$WASM_MODULE"

rm -fr "$TMP_DIR"
