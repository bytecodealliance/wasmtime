#undef NDEBUG

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
  // Create our `store` context and then compile a module and create an
  // instance from the compiled module all in one go.
  Engine engine;
  Module module =
      Module::compile(engine, readFile("examples/memory.wat")).unwrap();
  Store store(engine);
  Instance instance = Instance::create(store, module, {}).unwrap();

  // load_fn up our exports from the instance
  auto memory = std::get<Memory>(*instance.get(store, "memory"));
  auto size = std::get<Func>(*instance.get(store, "size"));
  auto load_fn = std::get<Func>(*instance.get(store, "load"));
  auto store_fn = std::get<Func>(*instance.get(store, "store"));

  std::cout << "Checking memory...\n";
  assert(memory.size(store) == 2);
  auto data = memory.data(store);
  assert(data.size() == 0x20000);
  assert(data[0] == 0);
  assert(data[0x1000] == 1);
  assert(data[0x1003] == 4);

  assert(size.call(store, {}).unwrap()[0].i32() == 2);
  assert(load_fn.call(store, {0}).unwrap()[0].i32() == 0);
  assert(load_fn.call(store, {0x1000}).unwrap()[0].i32() == 1);
  assert(load_fn.call(store, {0x1003}).unwrap()[0].i32() == 4);
  assert(load_fn.call(store, {0x1ffff}).unwrap()[0].i32() == 0);
  load_fn.call(store, {0x20000}).err(); // out of bounds trap

  std::cout << "Mutating memory...\n";
  memory.data(store)[0x1003] = 5;

  store_fn.call(store, {0x1002, 6}).unwrap();
  store_fn.call(store, {0x20000, 0}).err(); // out of bounds trap

  assert(memory.data(store)[0x1002] == 6);
  assert(memory.data(store)[0x1003] == 5);
  assert(load_fn.call(store, {0x1002}).unwrap()[0].i32() == 6);
  assert(load_fn.call(store, {0x1003}).unwrap()[0].i32() == 5);

  // Grow memory.
  std::cout << "Growing memory...\n";
  memory.grow(store, 1).unwrap();
  assert(memory.size(store) == 3);
  assert(memory.data(store).size() == 0x30000);

  assert(load_fn.call(store, {0x20000}).unwrap()[0].i32() == 0);
  store_fn.call(store, {0x20000, 0}).unwrap();
  load_fn.call(store, {0x30000}).err();
  store_fn.call(store, {0x30000, 0}).err();

  memory.grow(store, 1).err();
  memory.grow(store, 0).ok();

  std::cout << "Creating stand-alone memory...\n";
  MemoryType ty(5, 5);
  Memory memory2 = Memory::create(store, ty).unwrap();
  assert(memory2.size(store) == 5);
  memory2.grow(store, 1).err();
  memory2.grow(store, 0).ok();
}
