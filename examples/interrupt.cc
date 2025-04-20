/*
Example of instantiating of the WebAssembly module and invoking its exported
function.

You can compile and run this example on Linux with:

   cargo build --release -p wasmtime-c-api
   c++ examples/interrupt.cc -std=c++20 \
       -I crates/c-api/include \
       -I crates/c-api/wasm-c-api/include \
       target/release/libwasmtime.a \
       -lpthread -ldl -lm \
       -o interrupt
   ./interrupt

Note that on Windows and macOS the command will be similar, but you'll need
to tweak the `-lpthread` and such annotations as well as the name of the
`libwasmtime.a` file on Windows.
*/

#include <chrono>
#include <fstream>
#include <iostream>
#include <sstream>
#include <thread>
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
  // Enable interruptible code via `Config` and then create an interrupt
  // handle which we'll use later to interrupt running code.
  Config config;
  config.epoch_interruption(true);
  Engine engine(std::move(config));
  Store store(engine);
  store.context().set_epoch_deadline(1);

  // Compile and instantiate a small example with an infinite loop.
  auto wat = readFile("examples/interrupt.wat");
  Module module = Module::compile(engine, wat).unwrap();
  Instance instance = Instance::create(store, module, {}).unwrap();
  Func run = std::get<Func>(*instance.get(store, "run"));

  // Spin up a thread to send us an interrupt in a second
  std::thread t([engine{std::move(engine)}]() {
    std::this_thread::sleep_for(std::chrono::seconds(1));
    std::cout << "Interrupting!\n";
    engine.increment_epoch();
  });

  std::cout << "Entering infinite loop ...\n";
  auto err = run.call(store, {}).err();
  auto &trap = std::get<Trap>(err.data);

  std::cout << "trap: " << trap.message() << "\n";
  t.join();
}
