/// \file wasmtime/component/types/func.h

#ifndef WASMTIME_COMPONENT_TYPES_FUNC_H
#define WASMTIME_COMPONENT_TYPES_FUNC_H

#include <wasmtime/conf.h>

#ifdef WASMTIME_FEATURE_COMPONENT_MODEL

#include <wasm.h>
#include <wasmtime/component/types/val.h>

#ifdef __cplusplus
extern "C" {
#endif

/// \brief Opaque type representing a component function type.
typedef struct wasmtime_component_func_type_t wasmtime_component_func_type_t;

/// \brief Clones a component function type.
///
/// The pointer returned from this function must be deallocated with
/// `wasmtime_component_func_type_delete`.
WASM_API_EXTERN
wasmtime_component_func_type_t *
wasmtime_component_func_type_clone(const wasmtime_component_func_type_t *ty);

/// \brief Deallocates a component instance type.
WASM_API_EXTERN
void wasmtime_component_func_type_delete(wasmtime_component_func_type_t *ty);

/// \brief Returns the number of parameters of a component function type.
WASM_API_EXTERN
size_t wasmtime_component_func_type_param_count(
    const wasmtime_component_func_type_t *ty);

/// \brief Retrieves the nth parameter.
///
/// The `type_ret` return value must be deallocated with
/// `wasmtime_component_valtype_delete`.
WASM_API_EXTERN
bool wasmtime_component_func_type_param_nth(
    const wasmtime_component_func_type_t *ty, size_t nth, const char **name_ret,
    size_t *name_len_ret, wasmtime_component_valtype_t *type_ret);

/// \brief Returns the result, if any, of this component function type.
///
/// The `type_ret` return value must be deallocated with
/// `wasmtime_component_valtype_delete`.
WASM_API_EXTERN
bool wasmtime_component_func_type_result(
    const wasmtime_component_func_type_t *ty,
    wasmtime_component_valtype_t *type_ret);

#ifdef __cplusplus
} // extern "C"
#endif

#endif // WASMTIME_FEATURE_COMPONENT_MODEL

#endif // WASMTIME_COMPONENT_TYPES_FUNC_H
