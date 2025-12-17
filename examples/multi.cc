/*
Example of instantiating of the WebAssembly module and invoking its exported
function.

You can build the example using CMake:

mkdir build && (cd build && cmake .. && \
  cmake --build . --target wasmtime-multi-cpp)

And then run it:

build/wasmtime-multi-cpp
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
  Engine engine;
  Store store(engine);

  std::cout << "Compiling module...\n";
  auto wat = readFile("examples/multi.wat");
  Module module = Module::compile(engine, wat).unwrap();

  std::cout << "Creating callback...\n";
  Func callback_func = Func::wrap(
      store, [](int32_t a, int64_t b) -> std::tuple<int64_t, int32_t> {
        // Rust example adds 1 to each argument but flips order.
        return std::make_tuple(b + 1, a + 1);
      });

  std::cout << "Instantiating module...\n";
  Instance instance = Instance::create(store, module, {callback_func}).unwrap();

  std::cout << "Extracting export...\n";
  Func g = std::get<Func>(*instance.get(store, "g"));

  std::cout << "Calling export \"g\"...\n";
  // Provide (i32=1, i64=3) like the Rust example
  auto results = g.call(store, {Val(int32_t(1)), Val(int64_t(3))}).unwrap();

  std::cout << "Printing result...\n";
  std::cout << "> " << results[0].i64() << " " << results[1].i32() << "\n";

  std::cout << "Calling export \"round_trip_many\"...\n";
  Func round_trip_many =
      std::get<Func>(*instance.get(store, "round_trip_many"));
  auto many_results =
      round_trip_many
          .call(store, {Val(int64_t(0)), Val(int64_t(1)), Val(int64_t(2)),
                        Val(int64_t(3)), Val(int64_t(4)), Val(int64_t(5)),
                        Val(int64_t(6)), Val(int64_t(7)), Val(int64_t(8)),
                        Val(int64_t(9))})
          .unwrap();
  std::cout << "Printing result...\n";
  std::cout << "> (";
  for (size_t i = 0; i < many_results.size(); i++) {
    if (i)
      std::cout << ", ";
    std::cout << many_results[i].i64();
  }
  std::cout << ")\n";
  return 0;
}
