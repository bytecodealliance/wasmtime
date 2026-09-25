/*
Example of instantiating a WebAssembly component which uses WASIp2 imports.

You can build and run the example using CMake:

  cargo build --target wasm32-wasip2 -p example-wasi-wasm
  cmake -B build -S examples
  cmake --build build --target wasmtime-wasip2-cpp
  ./build/wasmtime-wasip2-cpp
*/

#include <array>
#include <fstream>
#include <iostream>
#include <vector>
#include <wasmtime.hh>
#include <wasmtime/component.hh>

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
  // Create a component linker with the WASIp2 functions defined.
  Engine engine;
  component::Linker linker(engine);
  linker.add_wasip2().unwrap();

  // Create a WASI context and put it in a Store; all instances in the store
  // share this context. `WasiConfig` provides a number of ways to
  // configure what the target program will have access to.
  WasiConfig wasi;
  wasi.inherit_argv();
  wasi.inherit_env();
  wasi.inherit_stdin();
  wasi.inherit_stdout();
  wasi.inherit_stderr();

  Store store(engine);
  auto context = store.context();
  context.set_wasi(std::move(wasi)).unwrap();

  // Load and compile the wasm component.
  auto bytes = read_binary_file("target/wasm32-wasip2/debug/wasi.wasm");
  auto component =
      component::Component::compile(engine,
                                    Span<uint8_t>(bytes.data(), bytes.size()))
          .unwrap();

  // Instantiate the component with the imports we've defined in the linker.
  auto instance = linker.instantiate(context, component).unwrap();

  // Look up the `run` function in the exported `wasi:cli/run@0.2.0` interface.
  // First look up the interface itself, then the function within it.
  auto interface_idx = instance.get_export_index(context, nullptr,
                                                 "wasi:cli/run@0.2.0");
  if (!interface_idx) {
    std::cerr << "error: cannot find `wasi:cli/run@0.2.0` interface\n";
    return 1;
  }
  auto func_idx = instance.get_export_index(context, &*interface_idx, "run");
  if (!func_idx) {
    std::cerr << "error: cannot find `run` function in "
                 "`wasi:cli/run@0.2.0` interface\n";
    return 1;
  }
  auto func = *instance.get_func(context, *func_idx);

  // Run it. The `run` function takes no arguments and returns a single
  // `result<(), ()>` value indicating whether the program succeeded.
  auto results = std::array<component::Val, 1>{false};
  func.call(context, Span<const component::Val>(nullptr, 0), results)
      .unwrap();
  if (!results[0].get_result().is_ok()) {
    std::cerr << "error: program returned an error\n";
    return 1;
  }

  return 0;
}
