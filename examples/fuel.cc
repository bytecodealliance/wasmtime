/*
Example of instantiating of the WebAssembly module and invoking its exported
function.

You can compile and run this example on Linux with:

   cargo build --release -p wasmtime-c-api
   c++ examples/fuel.cc -std=c++20 \
       -I crates/c-api/include \
       -I crates/c-api/wasm-c-api/include \
       target/release/libwasmtime.a \
       -lpthread -ldl -lm \
       -o fuel
   ./fuel

Note that on Windows and macOS the command will be similar, but you'll need
to tweak the `-lpthread` and such annotations.
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

const size_t kStoreFuel = 10000;

int main() {
  Config config;
  config.consume_fuel(true);
  Engine engine(std::move(config));
  Store store(engine);
  store.context().set_fuel(kStoreFuel).unwrap();

  auto wat = readFile("examples/fuel.wat");
  Module module = Module::compile(engine, wat).unwrap();
  Instance instance = Instance::create(store, module, {}).unwrap();
  Func fib = std::get<Func>(*instance.get(store, "fibonacci"));

  // Call it repeatedly until it fails
  for (int32_t n = 1;; n++) {
    auto result = fib.call(store, {n});
    if (!result) {
      std::cout << "Exhausted fuel computing fib(" << n << ")\n";
      break;
    }
    uint64_t consumed = kStoreFuel - store.context().get_fuel().unwrap();
    auto fib_result = std::move(result).unwrap()[0].i32();

    std::cout << "fib(" << n << ") = " << fib_result << " [consumed "
              << consumed << " fuel]\n";
    store.context().set_fuel(kStoreFuel).unwrap();
  }
}
