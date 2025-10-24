/**
 * \file wasmtime/engine.hh
 */

#ifndef WASMTIME_ENGINE_HH
#define WASMTIME_ENGINE_HH

#include <memory>
#include <wasmtime/config.hh>
#include <wasmtime/engine.h>
#include <wasmtime/helpers.hh>

namespace wasmtime {

/**
 * \brief Global compilation state in Wasmtime.
 *
 * Created with either default configuration or with a specified instance of
 * configuration, an `Engine` is used as an umbrella "session" for all other
 * operations in Wasmtime.
 */
class Engine {
/// bridging wasm.h vs wasmtime.h conventions
#define wasm_engine_clone wasmtime_engine_clone
  WASMTIME_CLONE_WRAPPER(Engine, wasm_engine);
#undef wasm_engine_clone

  /// \brief Creates an engine with default compilation settings.
  Engine() : ptr(wasm_engine_new()) {}
  /// \brief Creates an engine with the specified compilation settings.
  explicit Engine(Config config)
      : ptr(wasm_engine_new_with_config(config.capi_release())) {}

  /// \brief Increments the current epoch which may result in interrupting
  /// currently executing WebAssembly in connected stores if the epoch is now
  /// beyond the configured threshold.
  void increment_epoch() const { wasmtime_engine_increment_epoch(ptr.get()); }

  /// \brief Returns whether this engine is using Pulley for execution.
  void is_pulley() const { wasmtime_engine_is_pulley(ptr.get()); }
};

} // namespace wasmtime

#endif // WASMTIME_ENGINE_HH
