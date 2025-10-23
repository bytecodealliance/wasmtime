/// \file wasmtime/component/types/instance.h

#ifndef WASMTIME_COMPONENT_TYPES_INSTANCE_H
#define WASMTIME_COMPONENT_TYPES_INSTANCE_H

#include <wasmtime/conf.h>

#ifdef WASMTIME_FEATURE_COMPONENT_MODEL

#include <wasm.h>

#ifdef __cplusplus
extern "C" {
#endif

struct wasmtime_component_item_t;

/// \brief Represents the type of a component instance.
typedef struct wasmtime_component_instance_type
    wasmtime_component_instance_type_t;

/// \brief Clones a component instance type.
///
/// The pointer returned from this function must be deallocated with
/// `wasmtime_component_instance_type_delete`.
WASM_API_EXTERN
wasmtime_component_instance_type_t *wasmtime_component_instance_type_clone(
    const wasmtime_component_instance_type_t *ty);

/// \brief Deallocates a component instance type.
WASM_API_EXTERN
void wasmtime_component_instance_type_delete(
    wasmtime_component_instance_type_t *ty);

/// \brief Returns the number of exports of a component instance type.
WASM_API_EXTERN
size_t wasmtime_component_instance_type_export_count(
    const wasmtime_component_instance_type_t *ty, const wasm_engine_t *engine);

/// \brief Retrieves the export with the specified name.
///
/// The returned `wasmtime_component_item_t` must be deallocated with
/// `wasmtime_component_item_delete`.
WASM_API_EXTERN
bool wasmtime_component_instance_type_export_get(
    const wasmtime_component_instance_type_t *ty, const wasm_engine_t *engine,
    const char *name, size_t name_len, struct wasmtime_component_item_t *ret);

/// \brief Retrieves the nth export.
///
/// The returned `wasmtime_component_item_t` must be deallocated with
/// `wasmtime_component_item_delete`.
WASM_API_EXTERN
bool wasmtime_component_instance_type_export_nth(
    const wasmtime_component_instance_type_t *ty, const wasm_engine_t *engine,
    size_t nth, const char **name_ret, size_t *name_len_ret,
    struct wasmtime_component_item_t *type_ret);

#ifdef __cplusplus
} // extern "C"
#endif

#endif // WASMTIME_FEATURE_COMPONENT_MODEL

#endif // WASMTIME_COMPONENT_TYPES_INSTANCE_H
