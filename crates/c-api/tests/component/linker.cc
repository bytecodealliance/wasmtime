#include <gtest/gtest.h>
#include <wasmtime.h>
#include <wasmtime/component.hh>

using namespace wasmtime::component;

static wasmtime_error_t *func_callback(void *, wasmtime_context_t *,
                                       const wasmtime_component_func_type_t *,
                                       wasmtime_component_val_t *, size_t,
                                       wasmtime_component_val_t *, size_t) {
  return nullptr;
}

static void async_func_callback(void *, wasmtime_context_t *,
                                const wasmtime_component_func_type_t *,
                                wasmtime_component_val_t *, size_t,
                                wasmtime_component_val_t *, size_t,
                                wasmtime_error_t **,
                                wasmtime_async_continuation_t *) {}

static wasmtime_error_t *resource_destructor(void *, wasmtime_context_t *, uint32_t) {
  return nullptr;
}

static void finalize(void *data) { *static_cast<bool *>(data) = true; }

TEST(Linker, finalizes_callbacks_when_name_parsing_fails) {
  wasmtime::Engine engine;
  auto *raw = wasmtime_component_linker_new(engine.capi());
  auto *root = wasmtime_component_linker_root(raw);
  const char invalid_utf8[] = {static_cast<char>(0xff)};

  bool finalized = false;
  auto *error = wasmtime_component_linker_instance_add_func(
      root, invalid_utf8, sizeof(invalid_utf8), func_callback, &finalized,
      finalize);
  ASSERT_NE(error, nullptr);
  wasmtime_error_delete(error);
  EXPECT_TRUE(finalized);

  finalized = false;
  error = wasmtime_component_linker_instance_add_func_async(
      root, invalid_utf8, sizeof(invalid_utf8), async_func_callback, &finalized,
      finalize);
  ASSERT_NE(error, nullptr);
  wasmtime_error_delete(error);
  EXPECT_TRUE(finalized);

  finalized = false;
  auto *ty = wasmtime_component_resource_type_new_host(0);
  error = wasmtime_component_linker_instance_add_resource(
      root, invalid_utf8, sizeof(invalid_utf8), ty, resource_destructor,
      &finalized, finalize);
  ASSERT_NE(error, nullptr);
  wasmtime_error_delete(error);
  EXPECT_TRUE(finalized);
  wasmtime_component_resource_type_delete(ty);

  wasmtime_component_linker_instance_delete(root);
  wasmtime_component_linker_delete(raw);
}

TEST(Linker, allow_shadowing) {
  wasmtime::Engine engine;
  Linker linker(engine);
  auto m = wasmtime::Module::compile(engine, "(module)").unwrap();

  linker.root().add_module("x", m).unwrap();
  linker.root().add_module("x", m).err();
  linker.allow_shadowing(true);
  linker.root().add_module("x", m).unwrap();
}

TEST(Linker, unknown_imports_trap) {
  wasmtime::Engine engine;
  Linker linker(engine);
  wasmtime::Store store(engine);

  auto c = Component::compile(engine, R"(
    (component
      (import "a" (func))
    )
  )")
               .unwrap();

  EXPECT_FALSE(linker.instantiate(store, c));
  EXPECT_TRUE(linker.define_unknown_imports_as_traps(c));
  EXPECT_TRUE(linker.instantiate(store, c));
}
