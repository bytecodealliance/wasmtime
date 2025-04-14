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
  // Load our WebAssembly (parsed WAT in our case), and then load it into a
  // `Module` which is attached to a `Store`. After we've got that we
  // can instantiate it.
  Engine engine;
  Store store(engine);
  auto module = Module::compile(engine, readFile("examples/gcd.wat")).unwrap();
  auto instance = Instance::create(store, module, {}).unwrap();

  // Invoke `gcd` export
  auto gcd = std::get<Func>(*instance.get(store, "gcd"));
  auto results = gcd.call(store, {6, 27}).unwrap();

  std::cout << "gcd(6, 27) = " << results[0].i32() << "\n";
}
