Source code used to create `/wizer/benches/regex_bench.{control,wizer}.wasm`.

From within this directory, rebuild via:

```
$ cargo build --release --target wasm32-wasi
$ cp ../../target/wasm32-wasi/release/regex_bench.wasm ../regex_bench.control.wasm
$ cargo build --release --target wasm32-wasi --features wizer
$ cd ../..
$ cargo run --all-features -- --allow-wasi target/wasm32-wasi/release/regex_bench.wasm -o benches/regex_bench.wizer.wasm
```
