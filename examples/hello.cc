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
  // First the wasm module needs to be compiled. This is done with a global
  // "compilation environment" within an `Engine`. Note that engines can be
  // further configured through `Config` if desired instead of using the
  // default like this is here.
  std::cout << "Compiling module\n";
  Engine engine;
  auto module =
      Module::compile(engine, readFile("examples/hello.wat")).unwrap();

  // After a module is compiled we create a `Store` which will contain
  // instantiated modules and other items like host functions. A Store
  // contains an arbitrary piece of host information, and we use `MyState`
  // here.
  std::cout << "Initializing...\n";
  Store store(engine);

  // Our wasm module we'll be instantiating requires one imported function.
  // the function takes no parameters and returns no results. We create a host
  // implementation of that function here.
  std::cout << "Creating callback...\n";
  Func host_func =
      Func::wrap(store, []() { std::cout << "Calling back...\n"; });

  // Once we've got that all set up we can then move to the instantiation
  // phase, pairing together a compiled module as well as a set of imports.
  // Note that this is where the wasm `start` function, if any, would run.
  std::cout << "Instantiating module...\n";
  auto instance = Instance::create(store, module, {host_func}).unwrap();

  // Next we poke around a bit to extract the `run` function from the module.
  std::cout << "Extracting export...\n";
  auto run = std::get<Func>(*instance.get(store, "run"));

  // And last but not least we can call it!
  std::cout << "Calling export...\n";
  run.call(store, {}).unwrap();

  std::cout << "Done\n";
}
