/// \file wasmtime/component/types/module.h

#ifndef WASMTIME_COMPONENT_TYPES_MODULE_H
#define WASMTIME_COMPONENT_TYPES_MODULE_H

#include <wasmtime/conf.h>

#ifdef WASMTIME_FEATURE_COMPONENT_MODEL

#include <wasm.h>

#ifdef __cplusplus
extern "C" {
#endif

/// \brief Represents the type of a module.
typedef struct wasmtime_module_type wasmtime_module_type_t;

/// \brief Clones a module type.
///
/// The pointer returned from this function must be deallocated with
/// `wasmtime_module_type_delete`.
WASM_API_EXTERN
wasmtime_module_type_t *
wasmtime_module_type_clone(const wasmtime_module_type_t *ty);

/// \brief Deallocates a component instance type.
WASM_API_EXTERN
void wasmtime_module_type_delete(wasmtime_module_type_t *ty);

/// \brief Returns the number of imports of a module type.
WASM_API_EXTERN
size_t wasmtime_module_type_import_count(const wasmtime_module_type_t *ty,
                                         const wasm_engine_t *engine);

/// \brief Retrieves the nth import.
///
/// The returned `type_ret` pointer must be deallocated with
/// `wasm_importtype_delete`.
WASM_API_EXTERN
wasm_importtype_t *
wasmtime_module_type_import_nth(const wasmtime_module_type_t *ty,
                                const wasm_engine_t *engine, size_t nth);

/// \brief Returns the number of exports of a module type.
WASM_API_EXTERN
size_t wasmtime_module_type_export_count(const wasmtime_module_type_t *ty,
                                         const wasm_engine_t *engine);

/// \brief Retrieves the nth export.
///
/// The returned `type_ret` pointer must be deallocated with
/// `wasm_exporttype_delete`.
WASM_API_EXTERN
wasm_exporttype_t *
wasmtime_module_type_export_nth(const wasmtime_module_type_t *ty,
                                const wasm_engine_t *engine, size_t nth);

#ifdef __cplusplus
} // extern "C"
#endif

#endif // WASMTIME_FEATURE_COMPONENT_MODEL

#endif // WASMTIME_COMPONENT_TYPES_MODULE_H
