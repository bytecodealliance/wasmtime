#include <gtest/gtest.h>
#include <string_view>
#include <wasmtime/component.hh>
#include <wasmtime/store.hh>

using namespace wasmtime::component;

TEST(wasip2, smoke) {
  static constexpr auto component_text = std::string_view{
      R"END(
(component
  (import "wasi:cli/environment@0.2.0" (instance
    (export "get-arguments" (func (result (list string))))
  ))
)
      )END",
  };

  wasmtime::Engine engine;
  wasmtime::Store store(engine);
  auto context = store.context();

  wasmtime::WasiConfig config;

  wasi_config_set_stdout_custom(
      config.capi(),
      [](void *, const unsigned char *buf, size_t len) -> ptrdiff_t {
        std::cout << std::string_view{(const char *)(buf), len};
        return len;
      },
      nullptr, nullptr);
  wasi_config_set_stderr_custom(
      config.capi(),
      [](void *, const unsigned char *buf, size_t len) -> ptrdiff_t {
        std::cerr << std::string_view{(const char *)(buf), len};
        return len;
      },
      nullptr, nullptr);

  context.set_wasi(std::move(config)).unwrap();
  Component component = Component::compile(engine, component_text).unwrap();

  Linker linker(engine);
  linker.add_wasip2().unwrap();
  linker.instantiate(context, component).unwrap();
}

#ifdef WASMTIME_FEATURE_WASI_HTTP
TEST(wasip2, http_limits) {
  wasmtime::Engine engine;
  wasmtime::Store store(engine);
  auto context = store.context();

  // Setting limits before the WASI HTTP context exists is an error.
  auto err = context.set_wasi_http_field_size_limit(1024);
  EXPECT_FALSE(err);
  EXPECT_NE(err.err().message().find("wasmtime_context_set_wasi_http"),
            std::string::npos);
  err = context.set_wasi_http_request_strings_size_limit(1024);
  EXPECT_FALSE(err);
  EXPECT_NE(err.err().message().find("wasmtime_context_set_wasi_http"),
            std::string::npos);

  context.set_wasi(wasmtime::WasiConfig()).unwrap();
  context.set_wasi_http();
  context.set_wasi_http_field_size_limit(1024).unwrap();
  context.set_wasi_http_request_strings_size_limit(1024).unwrap();
}
#endif // WASMTIME_FEATURE_WASI_HTTP
