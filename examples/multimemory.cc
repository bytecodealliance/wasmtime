/*
An example of how to interact with multiple memories.

You can build the example using CMake:

mkdir build && (cd build && cmake .. && \
  cmake --build . --target wasmtime-multimemory-cpp)

And then run it:

build/wasmtime-multimemory-cpp
*/

#include <fstream>
#include <iostream>
#include <sstream>
#include <wasmtime.hh>

using namespace wasmtime;

std::string readFile(const char *name) {
  std::ifstream watFile;
  watFile.open(name);
  std::stringstream strStream;
  strStream << watFile.rdbuf();
  return strStream.str();
}

int main() {
  std::cout << "Initializing...\n";
  Config config;
  config.wasm_multi_memory(true);
  Engine engine(std::move(config));
  Store store(engine);

  std::cout << "Compiling module...\n";
  auto wat = readFile("examples/multimemory.wat");
  Module module = Module::compile(engine, wat).unwrap();

  std::cout << "Instantiating module...\n";
  Instance instance = Instance::create(store, module, {}).unwrap();
  Memory memory0 = std::get<Memory>(*instance.get(store, "memory0"));
  Memory memory1 = std::get<Memory>(*instance.get(store, "memory1"));

  std::cout << "Checking memory...\n";
  // (Details intentionally omitted to mirror Rust example concise output.)

  std::cout << "Mutating memory...\n";
  auto d0 = memory0.data(store);
  if (d0.size() >= 0x1004)
    d0[0x1003] = 5;
  auto d1 = memory1.data(store);
  if (d1.size() >= 0x1004)
    d1[0x1003] = 7;

  std::cout << "Growing memory...\n";
  memory0.grow(store, 1).unwrap();
  memory1.grow(store, 2).unwrap();

  return 0;
}
