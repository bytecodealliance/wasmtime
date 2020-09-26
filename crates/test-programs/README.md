This is the `test-programs` crate, which builds and runs whole programs
compiled to wasm32-wasi.

To actually run these tests, the test-programs feature must be enabled, e.g.:
```
cargo test --features test-programs/test_programs --package test-programs
```
