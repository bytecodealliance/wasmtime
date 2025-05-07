#include <wasmtime/module.hh>

#include <gtest/gtest.h>

using namespace wasmtime;

TEST(Module, Simple) {
  Engine engine;
  Module m = Module::compile(engine, "(module)").unwrap();
  auto wasm = wat2wasm("(module)").unwrap();
  Module::compile(engine, wasm).unwrap();
  Module::validate(engine, wasm).unwrap();

  auto serialized = m.serialize().unwrap();
  Module::deserialize(engine, serialized).unwrap();
}
