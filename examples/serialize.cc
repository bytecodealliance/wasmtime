/*
Small example of how to serialize compiled wasm module to the disk,
and then instantiate it from the compilation artifacts.

You can build the example using CMake:

mkdir build && (cd build && cmake .. && \
  cmake --build . --target wasmtime-serialize-cpp)

And then run it:

build/wasmtime-serialize-cpp
*/

#include <fstream>
#include <iostream>
#include <sstream>
#include <vector>
#include <wasmtime.hh>

using namespace wasmtime;

std::string readFile(const char *name) {
  std::ifstream watFile;
  watFile.open(name);
  std::stringstream strStream;
  strStream << watFile.rdbuf();
  return strStream.str();
}

std::vector<uint8_t> serialize() {
  std::cout << "Initializing...\n";
  Engine engine;

  std::cout << "Compiling module...\n";
  auto wat = readFile("examples/hello.wat");
  Module module = Module::compile(engine, wat).unwrap();

  auto serialized = module.serialize().unwrap();

  std::cout << "Serialized.\n";
  return serialized;
}

void deserialize(std::vector<uint8_t> buffer) {
  std::cout << "Initializing...\n";
  Engine engine;
  Store store(engine);

  std::cout << "Deserialize module...\n";
  Module module =
      Module::deserialize(engine, Span<uint8_t>(buffer.data(), buffer.size()))
          .unwrap();

  std::cout << "Creating callback...\n";
  Func hello_func = Func::wrap(store, []() {
    std::cout << "Calling back...\n";
    std::cout << "> Hello World!\n";
  });

  std::cout << "Instantiating module...\n";
  Instance instance = Instance::create(store, module, {hello_func}).unwrap();

  std::cout << "Extracting export...\n";
  Func run = std::get<Func>(*instance.get(store, "run"));

  std::cout << "Calling export...\n";
  run.call(store, {}).unwrap();

  std::cout << "Done.\n";
}

int main() {
  auto buffer = serialize();
  deserialize(buffer);
  return 0;
}
