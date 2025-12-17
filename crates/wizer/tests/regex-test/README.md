Source code used to create `/wizer/tests/regex_test.wasm`.

Rebuild with:

```
$ cargo build --release --target wasm32-wasi -p regex-test
$ cp target/wasm32-wasi/release/regex_test.wasm tests/regex_test.wasm
```
