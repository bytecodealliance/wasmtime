#include <gtest/gtest.h>
#include <wasmtime/linker.hh>

using namespace wasmtime;

TEST(Linker, Smoke) {
  Engine engine;
  Linker linker(engine);
  Store store(engine);
  linker.allow_shadowing(false);
  Global g = Global::create(store, GlobalType(ValKind::I32, false), 1).unwrap();
  linker.define(store, "a", "g", g).unwrap();
  linker.define_wasi().unwrap();
  linker
      .func_new("a", "f", FuncType({}, {}),
                [](auto caller, auto params, auto results) -> auto {
                  return std::monostate();
                })
      .unwrap();
  linker.func_wrap("a", "f2", []() {}).unwrap();
  linker.func_wrap("a", "f3", [](Caller arg) {}).unwrap();
  linker.func_wrap("a", "f4", [](Caller arg, int32_t a) {}).unwrap();
  Module mod = Module::compile(engine, "(module)").unwrap();
  Instance i = Instance::create(store, mod, {}).unwrap();
  linker.define_instance(store, "x", i).unwrap();
  linker.instantiate(store, mod).unwrap();
  linker.module(store, "y", mod).unwrap();
  EXPECT_TRUE(linker.get(store, "a", "g"));
  linker.get_default(store, "g").unwrap();
  EXPECT_TRUE(linker.get(store, "a", "f"));
  EXPECT_TRUE(std::holds_alternative<Func>(*linker.get(store, "a", "f")));
}

TEST(Linker, CallableMove) {
  Engine engine;
  Linker linker(engine);
  Store store(engine);
  linker.allow_shadowing(false);

  struct CallableFunc {
    CallableFunc() = default;
    CallableFunc(const CallableFunc &) = delete;
    CallableFunc(CallableFunc &&) = default;

    Result<std::monostate, Trap>
    operator()(Caller caller, Span<const Val> params, Span<Val> results) {
      return std::monostate();
    }
  };

  CallableFunc cf;
  linker.func_new("a", "f", FuncType({}, {}), std::move(cf)).unwrap();
}

TEST(Linker, CallableCopy) {
  Engine engine;
  Linker linker(engine);
  Store store(engine);
  linker.allow_shadowing(false);

  struct CallableFunc {
    CallableFunc() = default;
    CallableFunc(const CallableFunc &) = default;
    CallableFunc(CallableFunc &&) = default;

    Result<std::monostate, Trap>
    operator()(Caller caller, Span<const Val> params, Span<Val> results) {
      return std::monostate();
    }
  };

  CallableFunc cf;
  linker.func_new("a", "f", FuncType({}, {}), cf).unwrap();
}
