cc := /wasi-sdk-19.0/bin/clang

.phony: gen clean run

default: target/debug/rust.wasm

gen:
	wit-bindgen rust ../../wit

target/wasm32-wasi/debug/rust.wasm: gen
	cargo build --target=wasm32-wasi

clean:
	rm -rf proxy.rs target

run: target/wasm32-wasi/debug/rust.wasm
	../../../../target/debug/wasmtime --wasi-modules=experimental-wasi-http target/wasm32-wasi/debug/rust.wasm