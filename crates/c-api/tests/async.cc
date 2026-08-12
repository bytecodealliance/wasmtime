#include <gtest/gtest.h>
#include <wasmtime.h>
#include <wasmtime.hh>

#ifdef WASMTIME_FEATURE_ASYNC

using namespace wasmtime;

namespace {

void async_callback(void *, wasmtime_caller_t *, const wasmtime_val_t *, size_t,
                    wasmtime_val_t *, size_t, wasm_trap_t **,
                    wasmtime_async_continuation_t *) {}

void finalize(void *data) { *static_cast<bool *>(data) = true; }

} // namespace

TEST(async, finalizes_callback_when_name_parsing_fails) {
  Engine engine;
  Linker linker(engine);
  auto *ty = wasm_functype_new_0_0();
  const char invalid_utf8[] = {static_cast<char>(0xff)};
  bool finalized = false;

  auto *error = wasmtime_linker_define_async_func(
      linker.capi(), invalid_utf8, sizeof(invalid_utf8), "name", 4, ty,
      async_callback, &finalized, finalize);

  ASSERT_NE(error, nullptr);
  wasmtime_error_delete(error);
  EXPECT_TRUE(finalized);
  wasm_functype_delete(ty);
}

TEST(async, call_func_async) {
  Engine engine;
  Store store(engine);
  auto context = store.context();

  Module m = Module::compile(
                 engine, "(module"
                         "  (func (export \"f\") (param i32 i32) (result i32)"
                         "    local.get 0"
                         "    local.get 1"
                         "    i32.add))")
                 .unwrap();

  Instance instance = Instance::create(store, m, {}).unwrap();
  auto f = std::get<Func>(*instance.get(store, "f"));

  wasmtime_val_t params[2] = {{.kind = WASMTIME_I32, .of = {.i32 = 34}},
                              {.kind = WASMTIME_I32, .of = {.i32 = 35}}};
  wasmtime_val_t results[1] = {};
  wasm_trap_t *trap = nullptr;
  wasmtime_error_t *error = nullptr;

  auto *future = wasmtime_func_call_async(context.capi(), &f.capi(), params, 2,
                                          results, 1, &trap, &error);

  while (!wasmtime_call_future_poll(future)) {
  }
  wasmtime_call_future_delete(future);

  EXPECT_EQ(trap, nullptr);
  EXPECT_EQ(error, nullptr);
  EXPECT_EQ(results[0].kind, WASMTIME_I32);
  EXPECT_EQ(results[0].of.i32, 69);
}

TEST(async, instantiate_async) {
  Engine engine;
  Store store(engine);

  Module m = Module::compile(engine, "(module)").unwrap();

  Linker linker(engine);

  wasmtime_instance_t instance;
  wasm_trap_t *trap = nullptr;
  wasmtime_error_t *error = nullptr;

  auto *future =
      wasmtime_linker_instantiate_async(linker.capi(), store.context().capi(),
                                        m.capi(), &instance, &trap, &error);

  while (!wasmtime_call_future_poll(future)) {
  }
  wasmtime_call_future_delete(future);

  EXPECT_EQ(trap, nullptr);
  EXPECT_EQ(error, nullptr);
}

#endif // WASMTIME_FEATURE_ASYNC
