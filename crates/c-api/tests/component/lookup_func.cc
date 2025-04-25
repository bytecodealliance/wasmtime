#include <gtest/gtest.h>
#include <wasmtime.h>

#define CHECK(err)                                                             \
  do {                                                                         \
    if (err) {                                                                 \
      auto msg = wasm_name_t{};                                                \
      wasmtime_error_message(err, &msg);                                       \
      EXPECT_EQ(err, nullptr) << std::string_view{msg.data, msg.size};         \
    }                                                                          \
  } while (false)

TEST(component, lookup_func) {
  static constexpr auto component_text = std::string_view{
      R"END(
(component
    (core module $m
        (func (export "f"))
    )
    (core instance $i (instantiate $m))
    (func (export "f")
        (canon lift (core func $i "f")))
)
      )END",
  };
  const auto engine = wasm_engine_new();
  EXPECT_NE(engine, nullptr);

  const auto store = wasmtime_store_new(engine, nullptr, nullptr);
  const auto context = wasmtime_store_context(store);

  wasmtime_component_t *component = nullptr;

  auto err = wasmtime_component_new(
      engine, reinterpret_cast<const uint8_t *>(component_text.data()),
      component_text.size(), &component);

  CHECK(err);

  const auto linker = wasmtime_component_linker_new(engine);

  wasmtime_component_instance_t instance = {};
  err = wasmtime_component_linker_instantiate(linker, context, component,
                                              &instance);
  CHECK(err);

  wasmtime_component_func_t func = {};
  auto found =
      wasmtime_component_instance_get_func(&instance, context, "f", &func);
  EXPECT_TRUE(found);
  EXPECT_NE(func.store_id, 0);

  func = {};
  found = wasmtime_component_instance_get_func(&instance, context, "ff", &func);
  EXPECT_FALSE(found);

  wasmtime_component_linker_delete(linker);

  wasmtime_store_delete(store);
  wasm_engine_delete(engine);
}
