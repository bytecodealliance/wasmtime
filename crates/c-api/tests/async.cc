#include <cstring>
#include <gtest/gtest.h>
#include <wasmtime.h>
#include <wasmtime.hh>

#ifdef WASMTIME_FEATURE_ASYNC

using namespace wasmtime;

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

TEST(async, call_func_poll_with_notify) {
  Engine engine;
  Store store(engine);

  wasmtime_func_async_callback_t callback =
      [](void *, wasmtime_caller_t *, const wasmtime_val_t *args, size_t,
         wasmtime_val_t *results, size_t, wasm_trap_t **,
         wasmtime_async_continuation_t *cont) {
        results[0].kind = WASMTIME_I32;
        results[0].of.i32 = args[0].of.i32 + args[1].of.i32;
        auto *count = new int(0);
        cont->env = count;
        cont->callback = [](void *env) -> bool {
          return ++(*static_cast<int *>(env)) > 1;
        };
        cont->finalizer = [](void *env) { delete static_cast<int *>(env); };
      };

  wasm_functype_t *ty = wasm_functype_new_2_1(
      wasm_valtype_new_i32(), wasm_valtype_new_i32(), wasm_valtype_new_i32());
  Linker linker(engine);
  wasmtime_linker_define_async_func(linker.capi(), "host", 4, "add", 3, ty,
                                    callback, nullptr, nullptr);
  wasm_functype_delete(ty);

  Module m =
      Module::compile(
          engine,
          "(module"
          "  (import \"host\" \"add\" (func $add (param i32 i32) (result i32)))"
          "  (func (export \"f\") (param i32 i32) (result i32)"
          "    local.get 0 local.get 1 call $add))")
          .unwrap();

  wasmtime_instance_t instance;
  auto *inst_future =
      wasmtime_linker_instantiate_async(linker.capi(), store.context().capi(),
                                        m.capi(), &instance, nullptr, nullptr);
  while (!wasmtime_call_future_poll(inst_future)) {
  }
  wasmtime_call_future_delete(inst_future);

  wasmtime_extern_t ext;
  wasmtime_instance_export_get(store.context().capi(), &instance, "f", 1, &ext);

  wasmtime_val_t params[2] = {{.kind = WASMTIME_I32, .of = {.i32 = 7}},
                              {.kind = WASMTIME_I32, .of = {.i32 = 8}}};
  wasmtime_val_t results[1] = {};
  wasm_trap_t *trap = nullptr;
  wasmtime_error_t *error = nullptr;
  auto *future = wasmtime_func_call_async(store.context().capi(), &ext.of.func,
                                          params, 2, results, 1, &trap, &error);

  EXPECT_FALSE(wasmtime_call_future_poll_with_notify(future, -1));
  EXPECT_TRUE(wasmtime_call_future_poll_with_notify(future, -1));

  wasmtime_call_future_delete(future);

  EXPECT_EQ(error, nullptr);
  EXPECT_EQ(results[0].of.i32, 15);
}

TEST(async, async_allow_sync) {
  Config config;
  wasmtime_config_async_allow_sync_set(config.capi(), true);
  Engine engine(std::move(config));
  Store store(engine);

  wasmtime_func_async_callback_t callback =
      [](void *, wasmtime_caller_t *, const wasmtime_val_t *, size_t,
         wasmtime_val_t *results, size_t, wasm_trap_t **,
         wasmtime_async_continuation_t *cont) {
        results[0].kind = WASMTIME_I32;
        results[0].of.i32 = 99;
        cont->callback = [](void *) -> bool { return true; };
        cont->env = nullptr;
        cont->finalizer = nullptr;
      };

  wasm_functype_t *ty = wasm_functype_new_0_1(wasm_valtype_new_i32());
  Linker linker(engine);
  wasmtime_linker_define_async_func(linker.capi(), "host", 4, "f", 1, ty,
                                    callback, nullptr, nullptr);
  wasm_functype_delete(ty);

  Module m = Module::compile(
                 engine, "(module"
                         "  (import \"host\" \"f\" (func (result i32)))"
                         "  (func (export \"g\") (result i32) i32.const 42))")
                 .unwrap();

  wasmtime_instance_t instance;
  auto *inst_future =
      wasmtime_linker_instantiate_async(linker.capi(), store.context().capi(),
                                        m.capi(), &instance, nullptr, nullptr);
  while (!wasmtime_call_future_poll(inst_future)) {
  }
  wasmtime_call_future_delete(inst_future);

  wasmtime_extern_t ext;
  wasmtime_instance_export_get(store.context().capi(), &instance, "g", 1, &ext);

  wasmtime_val_t results[1] = {};
  wasm_trap_t *trap = nullptr;
  auto *error = wasmtime_func_call(store.context().capi(), &ext.of.func,
                                   nullptr, 0, results, 1, &trap);
  EXPECT_EQ(error, nullptr);
  EXPECT_EQ(results[0].of.i32, 42);
}

#endif // WASMTIME_FEATURE_ASYNC
