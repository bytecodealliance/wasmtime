#include <gtest/gtest.h>
#include <wasmtime/instance.hh>

using namespace wasmtime;

TEST(Instance, Smoke) {
  Engine engine;
  Store store(engine);
  Memory m = Memory::create(store, MemoryType(1)).unwrap();
  Global g = Global::create(store, GlobalType(ValKind::I32, false), 1).unwrap();
  Table t = Table::create(store, TableType(ValKind::FuncRef, 1),
                          std::optional<Func>())
                .unwrap();
  Func f(store, FuncType({}, {}),
         [](auto caller, auto params, auto results) -> auto {
           return std::monostate();
         });

  Module mod =
      Module::compile(engine, "(module"
                              "(import \"\" \"\" (func))"
                              "(import \"\" \"\" (global i32))"
                              "(import \"\" \"\" (table 1 funcref))"
                              "(import \"\" \"\" (memory 1))"

                              "(func (export \"f\"))"
                              "(global (export \"g\") i32 (i32.const 0))"
                              "(export \"m\" (memory 0))"
                              "(export \"t\" (table 0))"
                              ")")
          .unwrap();
  Instance::create(store, mod, {}).err();
  Instance i = Instance::create(store, mod, {f, g, t, m}).unwrap();
  EXPECT_FALSE(i.get(store, "not-present"));
  f = std::get<Func>(*i.get(store, "f"));
  m = std::get<Memory>(*i.get(store, "m"));
  t = std::get<Table>(*i.get(store, "t"));
  g = std::get<Global>(*i.get(store, "g"));

  EXPECT_TRUE(i.get(store, 0));
  EXPECT_TRUE(i.get(store, 1));
  EXPECT_TRUE(i.get(store, 2));
  EXPECT_TRUE(i.get(store, 3));
  EXPECT_FALSE(i.get(store, 4));
  auto [name, func] = *i.get(store, 0);
  EXPECT_EQ(name, "f");
}

TEST(Instance, NewClearsTrapPointer) {
  Engine engine;
  Store store(engine);
  auto context = store.context();

  auto ok_module = Module::compile(engine, "(module)").unwrap();
  wasmtime_instance_t instance;
  wasm_trap_t *trap = reinterpret_cast<wasm_trap_t *>(1);
  auto *error = wasmtime_instance_new(context.capi(), ok_module.capi(), nullptr,
                                      0, &instance, &trap);
  EXPECT_EQ(error, nullptr);
  EXPECT_EQ(trap, nullptr);

  auto import_module =
      Module::compile(engine, "(module (import \"\" \"\" (func)))").unwrap();
  trap = reinterpret_cast<wasm_trap_t *>(1);
  error = wasmtime_instance_new(context.capi(), import_module.capi(), nullptr,
                                0, &instance, &trap);
  EXPECT_NE(error, nullptr);
  EXPECT_EQ(trap, nullptr);
  wasmtime_error_delete(error);
}
