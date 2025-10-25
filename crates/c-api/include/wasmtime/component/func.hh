/// \file wasmtime/component/func.hh

#ifndef WASMTIME_COMPONENT_FUNC_HH
#define WASMTIME_COMPONENT_FUNC_HH

#include <wasmtime/conf.h>

#ifdef WASMTIME_FEATURE_COMPONENT_MODEL

#include <string_view>
#include <wasmtime/component/func.h>
#include <wasmtime/component/types/val.hh>
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

  /// \brief Returns the number of parameters that this function takes.
  size_t params_count(Store::Context cx) const {
    return wasmtime_component_func_params_count(&func, cx.capi());
  }

  /// \brief Retrieves the parameter name and types for this function.
  std::vector<std::pair<std::string, ValType>> params(Store::Context cx) const {
    size_t count = params_count(cx);
    std::vector<wasm_name_t> names(count);
    std::vector<wasmtime_component_valtype_t> types(count);
    wasmtime_component_func_params_get(&func, cx.capi(), names.data(),
                                       types.data(), count);

    std::vector<std::pair<std::string, ValType>> result;
    result.reserve(count);
    for (size_t i = 0; i < count; ++i) {
      std::string name(names[i].data, names[i].size);
      result.emplace_back(std::move(name), ValType(std::move(types[i])));
      wasm_name_delete(&names[i]);
    }
    return result;
  }

  /// \brief Returns the result type of this function, if any.
  std::optional<ValType> result(Store::Context cx) const {
    wasmtime_component_valtype_t out;
    bool has_result = wasmtime_component_func_result(&func, cx.capi(), &out);
    if (!has_result) {
      return std::nullopt;
    }
    return ValType(std::move(out));
  }
};

} // namespace component
} // namespace wasmtime

#endif // WASMTIME_FEATURE_COMPONENT_MODEL

#endif // WASMTIME_COMPONENT_FUNC_H
