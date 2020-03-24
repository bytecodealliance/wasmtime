# Build example's file

To build `demo.wasm` use rustc (nightly) for wasm32 target with debug information:

```
rustc +nightly --target=wasm32-unknown-unknown demo.rs --crate-type=cdylib
```

# Run example

Point path to the built `wasmtime_py` library location when running python, e.g.

```
PYTHONPATH=../../target/debug python3 run.py
```
