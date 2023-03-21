cc := /wasi-sdk-19.0/bin/clang

.phony: gen clean run

default: main.wasm

gen:
	wit-bindgen c ../wit

main.wasm: gen
	${cc} proxy.c proxy_component_type.o main.c -o main.wasm

clean:
	rm -f proxy.c proxy_component_type.o proxy.h main.wasm

run: main.wasm
	../../../target/debug/wasmtime --wasi-modules=experimental-wasi-http main.wasm