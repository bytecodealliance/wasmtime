/// \file wasmtime/component/func.hh

#ifndef WASMTIME_COMPONENT_FUNC_HH
#define WASMTIME_COMPONENT_FUNC_HH

#include <wasmtime/conf.h>

#ifdef WASMTIME_FEATURE_COMPONENT_MODEL

#include <string_view>
#include <wasmtime/component/func.h>
#include <wasmtime/component/types/func.hh>
#include <wasmtime/component/val.hh>
#include <wasmtime/error.hh>
#include <wasmtime/span.hh>
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

  /// \brief Invokes this component function with the provided `args` and the
  /// results are placed in `results`.
  Result<std::monostate> call(Store::Context cx, Span<const Val> args,
                              Span<Val> results) const {
    wasmtime_error_t *error = wasmtime_component_func_call(
        &func, cx.capi(), Val::to_capi(args.data()), args.size(),
        Val::to_capi(results.data()), results.size());
    if (error != nullptr) {
      return Error(error);
    }
    return std::monostate();
  }

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

  /// \brief Returns the type of this function.
  FuncType type(Store::Context cx) const {
    return FuncType(wasmtime_component_func_type(&func, cx.capi()));
  }
};

} // namespace component
} // namespace wasmtime

#endif // WASMTIME_FEATURE_COMPONENT_MODEL

#endif // WASMTIME_COMPONENT_FUNC_H
