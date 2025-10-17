/// \file wasmtime/component/func.hh

#ifndef WASMTIME_COMPONENT_FUNC_HH
#define WASMTIME_COMPONENT_FUNC_HH

#include <wasmtime/conf.h>

#ifdef WASMTIME_FEATURE_COMPONENT_MODEL

#include <string_view>
#include <wasmtime/component/func.h>
#include <wasmtime/error.hh>
#include <wasmtime/store.hh>

namespace wasmtime {
namespace component {

/**
 * \brief Class representing an instantiated WebAssembly component.
 */
class Func {
  wasmtime_component_func_t func;

public:
  /// \brief Constructs an Func from the underlying C API struct.
  explicit Func(const wasmtime_component_func_t &func) : func(func) {}

  /// \brief Returns the underlying C API pointer.
  const wasmtime_component_func_t *capi() const { return &func; }

  // TODO: call with `wasmtime_component_func_call`

  /**
   * \brief Invokes the `post-return` canonical ABI option, if specified.
   */
  Result<std::monostate> post_return(Store::Context cx) const {
    wasmtime_error_t *error =
        wasmtime_component_func_post_return(&func, cx.capi());
    if (error != nullptr) {
      return Error(error);
    }
    return std::monostate();
  }
};

} // namespace component
} // namespace wasmtime

#endif // WASMTIME_FEATURE_COMPONENT_MODEL

#endif // WASMTIME_COMPONENT_FUNC_H
