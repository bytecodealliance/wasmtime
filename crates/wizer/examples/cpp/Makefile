CXX := /opt/wasi-sdk/bin/clang++
CXXFLAGS := -O2 -I ../../include/
WIZER := ../../target/release/wizer
WASMTIME ?= wasmtime

.PHONY: all
all: main_initialized.wasm

main.wasm: main.cpp
	$(CXX) $(CXXFLAGS) -o $@ $^

main_initialized.wasm: main.wasm
	$(WIZER) --allow-wasi --wasm-bulk-memory=true -r _start=wizer.resume -o $@ $^

.PHONY: test
test: main_initialized.wasm
	$(WASMTIME) run $^

.PHONY: clean
clean:
	rm -f *.wasm
