/*
Example of instantiating a wasm module which uses WASI imports.

You can build and run the example using CMake:

  cargo build --target wasm32-wasip1 -p example-wasi-wasm
  cmake -B build -S examples
  cmake --build build --target wasmtime-wasip1-cpp
  ./build/wasmtime-wasip1-cpp
*/

#include <fstream>
#include <iostream>
#include <sstream>
#include <vector>
#include <wasmtime.hh>

using namespace wasmtime;

static std::vector<uint8_t> read_binary_file(const char *path) {
  std::ifstream file(path, std::ios::in | std::ios::binary);
  if (!file.is_open()) {
    throw std::runtime_error(std::string("failed to open wasm file: ") + path);
  }
  std::vector<uint8_t> data((std::istreambuf_iterator<char>(file)),
                            std::istreambuf_iterator<char>());
  return data;
}

int main() {
  // Define the WASI functions globally on the `Config`.
  Engine engine;
  Linker linker(engine);
  linker.define_wasi().unwrap();

  // Create a WASI context and put it in a Store; all instances in the store
  // share this context. `WasiConfig` provides a number of ways to
  // configure what the target program will have access to.
  WasiConfig wasi;
  wasi.inherit_argv();
  wasi.inherit_stdin();
  wasi.inherit_stdout();
  wasi.inherit_stderr();

  Store store(engine);
  store.context().set_wasi(std::move(wasi)).unwrap();

  // Load and compile the wasm module.
  auto bytes = read_binary_file("target/wasm32-wasip1/debug/wasi.wasm");
  auto module =
      Module::compile(engine, Span<uint8_t>(bytes.data(), bytes.size()))
          .unwrap();

  // Define the module in the linker (anonymous name matches Rust example
  // usage).
  linker.module(store.context(), "", module).unwrap();

  // Get the default export (command entrypoint) and invoke it.
  Func default_func = linker.get_default(store.context(), "").unwrap();
  default_func.call(store, {}).unwrap();

  return 0;
}
