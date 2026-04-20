/**
 * \file wasmtime/types/arrayref.h
 *
 * APIs for interacting with WebAssembly GC `structref` type in Wasmtime.
 */

#ifndef WASMTIME_TYPES_ARRAYREF_H
#define WASMTIME_TYPES_ARRAYREF_H

#include <wasmtime/conf.h>

#ifdef WASMTIME_FEATURE_GC

#include <wasm.h>
#include <wasmtime/types/structref.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// StructRef
// ============================================================================

/**
 * \brief An opaque handle to a WebAssembly array type definition.
 *
 * An array type describes the element type of an array. It is used to create a
 * #wasmtime_array_ref_pre_t, which can then allocate array instances.
 *
 * Owned. Must be deleted with #wasmtime_array_type_delete.
 */
typedef struct wasmtime_array_type wasmtime_array_type_t;

/**
 * \brief Create a new array type.
 *
 * \param engine The engine to register the type with.
 * \param field The element type descriptor.
 *
 * \return Returns a new array type.
 */
WASM_API_EXTERN wasmtime_array_type_t *
wasmtime_array_type_new(const wasm_engine_t *engine,
                        const wasmtime_field_type_t *field);

/**
 * \brief Delete an array type.
 */
WASM_API_EXTERN void wasmtime_array_type_delete(wasmtime_array_type_t *ty);

#ifdef __cplusplus
} // extern "C"
#endif

#endif // WASMTIME_FEATURE_GC

#endif // WASMTIME_TYPES_ARRAYREF_H
