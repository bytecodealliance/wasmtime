#include <gtest/gtest.h>
#include <wasmtime.h>
#include <wasmtime/func.hh>
#include <wasmtime/linker.hh>

using namespace wasmtime;

namespace {

wasm_trap_t *callback(void *, wasmtime_caller_t *, const wasmtime_val_t *,
                      size_t, wasmtime_val_t *, size_t) {
  return nullptr;
}

wasm_trap_t *unchecked_callback(void *, wasmtime_caller_t *,
                                wasmtime_val_raw_t *, size_t) {
  return nullptr;
}

void finalize(void *data) { *static_cast<bool *>(data) = true; }

} // namespace

TEST(Linker, Smoke) {
  Engine engine;
  Linker linker(engine);
  Store store(engine);
  linker.allow_shadowing(false);
  Global g =
      Global::create(store, GlobalType(ValType::i32(), false), 1).unwrap();
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

TEST(Linker, FinalizesCallbacksWhenNameParsingFails) {
  Engine engine;
  Linker linker(engine);
  auto *ty = wasm_functype_new_0_0();
  const char invalid_utf8[] = {static_cast<char>(0xff)};

  bool finalized = false;
  auto *error = wasmtime_linker_define_func(linker.capi(), invalid_utf8,
                                            sizeof(invalid_utf8), "name", 4, ty,
                                            callback, &finalized, finalize);
  ASSERT_NE(error, nullptr);
  wasmtime_error_delete(error);
  EXPECT_TRUE(finalized);

  finalized = false;
  error = wasmtime_linker_define_func_unchecked(
      linker.capi(), "module", 6, invalid_utf8, sizeof(invalid_utf8), ty,
      unchecked_callback, &finalized, finalize);
  ASSERT_NE(error, nullptr);
  wasmtime_error_delete(error);
  EXPECT_TRUE(finalized);

  wasm_functype_delete(ty);
}

TEST(Linker, DefineUnknownImportsAsTraps) {
  Engine engine;
  Linker linker(engine);
  Store store(engine);
  Module mod =
      Module::compile(
          engine,
          "(module (import \"\" \"\" (func)) (func (export \"x\") call 0))")
          .unwrap();
  linker.define_unknown_imports_as_traps(mod).unwrap();

  auto instance = linker.instantiate(store.context(), mod).unwrap();
  Func f = std::get<Func>(*instance.get(store.context(), "x"));
  TrapError err = f.call(store.context(), {}).err();
  std::get<Error>(err.data);
}

TEST(Linker, DefineUnknownImportsAsDefaultValues) {
  Engine engine;
  Linker linker(engine);
  Store store(engine);
  Module mod =
      Module::compile(
          engine,
          "(module (import \"\" \"\" (func)) (func (export \"x\") call 0))")
          .unwrap();
  linker.define_unknown_imports_as_default_values(store.context(), mod)
      .unwrap();

  auto instance = linker.instantiate(store.context(), mod).unwrap();
  Func f = std::get<Func>(*instance.get(store.context(), "x"));
  f.call(store.context(), {}).unwrap();
}
