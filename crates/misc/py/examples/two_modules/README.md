# Build example's file

To build `one.wasm` use rustc (nightly) for wasm32 target with debug information:

```
rustc +nightly --target=wasm32-unknown-unknown one.rs --crate-type=cdylib
```

To build `two.wasm` use wabt.
```
wat2wasm two.wat -o two.wasm
```

# Run example

Point path to the built wasmtime_py library location when running python, e.g.

```
PYTHONPATH=../../target/debug python3 run.py
```
