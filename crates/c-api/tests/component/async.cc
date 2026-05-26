#include <array>
#include <gtest/gtest.h>
#include <wasmtime.h>
#include <wasmtime/component.hh>
#include <wasmtime/store.hh>

#ifdef WASMTIME_FEATURE_COMPONENT_MODEL_ASYNC

using namespace wasmtime::component;
using wasmtime::Config;
using wasmtime::Engine;
using wasmtime::Store;

TEST(component_async, config) {
  Config config;
  config.wasm_component_model_async(true);
  config.wasm_component_model_more_async_builtins(true);
  config.wasm_component_model_async_stackful(true);
  Engine engine(std::move(config));
}

TEST(component_async, call_func_async) {
  static constexpr auto component_text = std::string_view{
      R"END(
(component
    (core module $m
        (func (export "f") (param $x i32) (param $y i32) (result i32)
            (local.get $x)
            (local.get $y)
            (i32.add)
        )
    )
    (core instance $i (instantiate $m))
    (func $f (param "x" u32) (param "y" u32) (result u32) (canon lift (core func $i "f")))
    (export "f" (func $f))
)
      )END",
  };

  Engine engine;
  Store store(engine);
  auto context = store.context();
  auto component = Component::compile(engine, component_text).unwrap();
  auto f = *component.export_index(nullptr, "f");

  Linker linker(engine);

  auto instance = linker.instantiate(context, component).unwrap();
  auto func = *instance.get_func(context, f);

  auto params = std::array<Val, 2>{
      uint32_t(34),
      uint32_t(35),
  };

  auto results = std::array<Val, 1>{false};

  wasmtime_error_t *error = nullptr;
  auto *future = wasmtime_component_func_call_async(
      func.capi(), context.capi(), Val::to_capi(params.data()), params.size(),
      Val::to_capi(results.data()), results.size(), &error);

  while (!wasmtime_call_future_poll(future)) {
  }
  wasmtime_call_future_delete(future);

  EXPECT_EQ(error, nullptr);
  EXPECT_TRUE(results[0].is_u32());
  EXPECT_EQ(results[0].get_u32(), 69);
}

TEST(component_async, instantiate_async) {
  static constexpr auto component_text = std::string_view{
      R"END(
(component
    (core module)
)
      )END",
  };

  Engine engine;
  Store store(engine);
  auto context = store.context();
  auto component = Component::compile(engine, component_text).unwrap();

  Linker linker(engine);

  wasmtime_component_instance_t instance;
  wasmtime_error_t *error = nullptr;
  auto *future = wasmtime_component_linker_instantiate_async(
      linker.capi(), context.capi(), component.capi(), &instance, &error);

  while (!wasmtime_call_future_poll(future)) {
  }
  wasmtime_call_future_delete(future);

  EXPECT_EQ(error, nullptr);
}

TEST(component_async, host_func_async) {
  static constexpr auto component_text = std::string_view{
      R"END(
(component
    (import "add" (func $add (param "x" u32) (param "y" u32) (result u32)))
    (core module $m
        (import "host" "add" (func $add (param i32 i32) (result i32)))
        (func (export "f") (param $x i32) (param $y i32) (result i32)
            (call $add (local.get $x) (local.get $y))
        )
    )
    (core func $add_lower (canon lower (func $add)))
    (core instance $i (instantiate $m
        (with "host" (instance (export "add" (func $add_lower))))
    ))
    (func $f (param "x" u32) (param "y" u32) (result u32) (canon lift (core func $i "f")))
    (export "f" (func $f))
)
      )END",
  };

  Engine engine;
  Store store(engine);
  auto context = store.context();
  auto component = Component::compile(engine, component_text).unwrap();
  auto f = *component.export_index(nullptr, "f");

  Linker linker(engine);

  // Define an async host function that adds two u32s
  auto *root = wasmtime_component_linker_root(linker.capi());

  wasmtime_component_func_async_callback_t callback =
      [](void *, wasmtime_context_t *, const wasmtime_component_func_type_t *,
         wasmtime_component_val_t *args, size_t,
         wasmtime_component_val_t *results, size_t, wasmtime_error_t **,
         wasmtime_async_continuation_t *cont) {
        results[0].kind = WASMTIME_COMPONENT_U32;
        results[0].of.u32 = args[0].of.u32 + args[1].of.u32;
        // Signal immediate completion
        cont->callback = [](void *) -> bool { return true; };
        cont->env = nullptr;
        cont->finalizer = nullptr;
      };

  auto *err = wasmtime_component_linker_instance_add_func_async(
      root, "add", 3, callback, nullptr, nullptr);
  EXPECT_EQ(err, nullptr);
  wasmtime_component_linker_instance_delete(root);

  wasmtime_component_instance_t raw_instance;
  wasmtime_error_t *inst_error = nullptr;
  auto *inst_future = wasmtime_component_linker_instantiate_async(
      linker.capi(), context.capi(), component.capi(), &raw_instance,
      &inst_error);

  while (!wasmtime_call_future_poll(inst_future)) {
  }
  wasmtime_call_future_delete(inst_future);
  EXPECT_EQ(inst_error, nullptr);

  auto instance = Instance(raw_instance);
  auto func = *instance.get_func(context, f);

  auto params = std::array<Val, 2>{
      uint32_t(34),
      uint32_t(35),
  };

  auto results = std::array<Val, 1>{false};

  wasmtime_error_t *error = nullptr;
  auto *future = wasmtime_component_func_call_async(
      func.capi(), context.capi(), Val::to_capi(params.data()), params.size(),
      Val::to_capi(results.data()), results.size(), &error);

  while (!wasmtime_call_future_poll(future)) {
  }
  wasmtime_call_future_delete(future);

  EXPECT_EQ(error, nullptr);
  EXPECT_TRUE(results[0].is_u32());
  EXPECT_EQ(results[0].get_u32(), 69);
}

#endif // WASMTIME_FEATURE_COMPONENT_MODEL_ASYNC
