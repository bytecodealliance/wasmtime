/*
Example of using `anyref` values.

You can build the example using CMake:

mkdir build && (cd build && cmake .. && \
  cmake --build . --target wasmtime-anyref-cpp)

And then run it:

build/wasmtime-anyref-cpp
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
  config.wasm_reference_types(true);
  config.wasm_function_references(true);
  config.wasm_gc(true);
  Engine engine(std::move(config));
  Store store(engine);

  std::cout << "Compiling module...\n";
  auto wat = readFile("examples/anyref.wat");
  Module module = Module::compile(engine, wat).unwrap();

  std::cout << "Instantiating module...\n";
  Instance instance = Instance::create(store, module, {}).unwrap();

  std::cout << "Creating new `anyref` from i31...\n";
  // Create an i31ref wrapping 1234
  auto cx = store.context();
  AnyRef i31 = AnyRef::i31(cx, 1234);
  Val anyref_val(i31);
  auto opt_any = anyref_val.anyref();
  if (!opt_any || !opt_any->u31(cx) || *opt_any->u31(cx) != 1234) {
    std::cerr << "> Error creating i31 anyref\n";
    return 1;
  }

  std::cout << "Touching `anyref` table...\n";
  Table table = std::get<Table>(*instance.get(store, "table"));
  table.set(store, 3, anyref_val).unwrap();
  auto elem_opt = table.get(store, 3);
  if (!elem_opt) {
    std::cerr << "> Error getting table element\n";
    return 1;
  }
  auto elem_any = elem_opt->anyref();
  if (!elem_any || !elem_any->u31(cx) || *elem_any->u31(cx) != 1234) {
    std::cerr << "> Error verifying table element\n";
    return 1;
  }

  std::cout << "Touching `anyref` global...\n";
  Global global = std::get<Global>(*instance.get(store, "global"));
  global.set(store, anyref_val).unwrap();
  Val global_val = global.get(store);
  auto global_any = global_val.anyref();
  if (!global_any || !global_any->u31(cx) || *global_any->u31(cx) != 1234) {
    std::cerr << "> Error verifying global value\n";
    return 1;
  }

  std::cout << "Passing `anyref` into func...\n";
  Func take_anyref = std::get<Func>(*instance.get(store, "take_anyref"));
  take_anyref.call(store, {anyref_val}).unwrap();

  std::cout << "Getting `anyref` from func...\n";
  Func return_anyref = std::get<Func>(*instance.get(store, "return_anyref"));
  auto results = return_anyref.call(store, {}).unwrap();
  if (results.size() != 1) {
    std::cerr << "> Unexpected number of results\n";
    return 1;
  }
  auto ret_any = results[0].anyref();
  if (!ret_any || !ret_any->u31(cx) || *ret_any->u31(cx) != 42) {
    std::cerr << "> Error verifying returned anyref\n";
    return 1;
  }

  std::cout << "GCing within the store...\n";
  if (!store.context().gc()) {
    std::cerr << "> Error while collecting garbage\n";
    return 1;
  }

  std::cout << "Done.\n";
  return 0;
}
