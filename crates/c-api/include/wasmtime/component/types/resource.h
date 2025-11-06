/// \file wasmtime/component/types/resource.h

#ifndef WASMTIME_COMPONENT_TYPES_RESOURCE_H
#define WASMTIME_COMPONENT_TYPES_RESOURCE_H

#include <wasmtime/conf.h>

#ifdef WASMTIME_FEATURE_COMPONENT_MODEL

#include <wasm.h>

#ifdef __cplusplus
extern "C" {
#endif

/// \brief Represents the type of a component resource.
///
/// This is an opaque structure which represents the type of a resource. This
/// can be used to equate the type of two resources together to see if they are
/// the same.
typedef struct wasmtime_component_resource_type
    wasmtime_component_resource_type_t;

/// \brief Creates a new resource type representing a host-defined resource.
///
/// This function creates a new `wasmtime_component_resource_type_t` which
/// represents a host-defined resource identified by the `ty` integer argument
/// provided. Two host resources with different `ty` arguments are considered
/// not-equal in terms of resource types. Through this the host can create
/// distinct types of resources at runtime to ensure that components are also
/// required to keep resources distinct.
///
/// The pointer returned from this function must be deallocated with
/// `wasmtime_component_resource_type_delete`.
WASM_API_EXTERN
wasmtime_component_resource_type_t *
wasmtime_component_resource_type_new_host(uint32_t ty);

/// \brief Clones a resource type.
///
/// Creates a new owned copy of a resource type.
///
/// The pointer returned from this function must be deallocated with
/// `wasmtime_component_resource_type_delete`.
WASM_API_EXTERN
wasmtime_component_resource_type_t *wasmtime_component_resource_type_clone(
    const wasmtime_component_resource_type_t *ty);

/// \brief Compares two resource types for equality.
///
/// Returns whether `a` and `b` point to logically the same resource type under
/// the hood.
WASM_API_EXTERN
bool wasmtime_component_resource_type_equal(
    const wasmtime_component_resource_type_t *a,
    const wasmtime_component_resource_type_t *b);

/// \brief Deallocates a resource type.
///
/// This will deallocate the pointer `resource` any any memory that it might
/// own.
WASM_API_EXTERN
void wasmtime_component_resource_type_delete(
    wasmtime_component_resource_type_t *resource);

#ifdef __cplusplus
} // extern "C"
#endif

#endif // WASMTIME_FEATURE_COMPONENT_MODEL

#endif // WASMTIME_COMPONENT_TYPES_RESOURCE_H
