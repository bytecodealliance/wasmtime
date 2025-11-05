/**
 * \file wasmtime/component/types/func.hh
 */

#ifndef WASMTIME_COMPONENT_TYPES_FUNC_HH
#define WASMTIME_COMPONENT_TYPES_FUNC_HH

#include <wasmtime/conf.h>

#ifdef WASMTIME_FEATURE_COMPONENT_MODEL

#include <wasmtime/component/types/func.h>
#include <wasmtime/component/types/val.hh>

namespace wasmtime {
namespace component {

/**
 * \brief Type information about a component function.
 */
class FuncType {
  WASMTIME_CLONE_WRAPPER(FuncType, wasmtime_component_func_type);

  /// Returns the number of parameters of this component function type.
  size_t param_count() const {
    return wasmtime_component_func_type_param_count(ptr.get());
  }

  /// Retrieves the nth parameter.
  std::optional<std::pair<std::string_view, ValType>>
  param_nth(size_t nth) const {
    const char *name_ptr = nullptr;
    size_t name_len = 0;
    wasmtime_component_valtype_t type_ret;
    if (wasmtime_component_func_type_param_nth(ptr.get(), nth, &name_ptr,
                                               &name_len, &type_ret)) {
      return std::make_pair(std::string_view(name_ptr, name_len),
                            ValType(std::move(type_ret)));
    }
    return std::nullopt;
  }

  /// Returns the result type of this component function type, if any.
  std::optional<ValType> result() const {
    wasmtime_component_valtype_t type_ret;
    if (wasmtime_component_func_type_result(ptr.get(), &type_ret)) {
      return ValType(std::move(type_ret));
    }
    return std::nullopt;
  }
};

} // namespace component
} // namespace wasmtime

#endif // WASMTIME_FEATURE_COMPONENT_MODEL

#endif // WASMTIME_COMPONENT_TYPES_FUNC_HH
