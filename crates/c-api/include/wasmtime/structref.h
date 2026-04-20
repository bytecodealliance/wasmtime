/**
 * \file wasmtime/structref.h
 *
 * APIs for interacting with WebAssembly `structref` type in Wasmtime.
 */

#ifndef WASMTIME_STRUCTREF_H
#define WASMTIME_STRUCTREF_H

#include <wasmtime/conf.h>

#ifdef WASMTIME_FEATURE_GC

#include <wasmtime/types/structref.h>
#include <wasmtime/val.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * \brief An opaque, pre-allocated, and registered struct layout for faster
 * allocation.
 *
 * Created from a #wasmtime_struct_type_t and a store context. Reusable for
 * allocating many struct instances of the same type.
 *
 * Owned. Must be deleted with #wasmtime_struct_ref_pre_delete.
 */
typedef struct wasmtime_struct_ref_pre wasmtime_struct_ref_pre_t;

/**
 * \brief Create a new struct pre-allocator.
 *
 * \param context The store context.
 * \param ty The struct type.
 *
 * \return Returns a new struct ref pre-allocator.
 */
WASM_API_EXTERN wasmtime_struct_ref_pre_t *
wasmtime_struct_ref_pre_new(wasmtime_context_t *context,
                            const wasmtime_struct_type_t *ty);

/**
 * \brief Delete a struct pre-allocator.
 */
WASM_API_EXTERN void
wasmtime_struct_ref_pre_delete(wasmtime_struct_ref_pre_t *pre);

/// \brief Initialize the `ref` to a null `structref` value.
static inline void wasmtime_structref_set_null(wasmtime_structref_t *ref) {
  ref->store_id = 0;
}

/// \brief Returns whether the provided `ref` is a null `structref` value.
static inline bool wasmtime_structref_is_null(const wasmtime_structref_t *ref) {
  return ref->store_id == 0;
}

/**
 * \brief Allocate a new struct instance.
 *
 * \param context The store context.
 * \param pre The struct pre-allocator.
 * \param fields Pointer to array of field values (#wasmtime_val_t).
 * \param nfields Number of fields (must match the struct type).
 * \param out Receives the new structref on success.
 *
 * \return NULL on success, or a #wasmtime_error_t on failure.
 */
WASM_API_EXTERN wasmtime_error_t *wasmtime_structref_new(
    wasmtime_context_t *context, const wasmtime_struct_ref_pre_t *pre,
    const wasmtime_val_t *fields, size_t nfields, wasmtime_structref_t *out);

/**
 * \brief Clone a `structref`, creating a new root.
 */
WASM_API_EXTERN void
wasmtime_structref_clone(const wasmtime_structref_t *structref,
                         wasmtime_structref_t *out);

/**
 * \brief Unroot a `structref` to allow garbage collection.
 */
WASM_API_EXTERN void wasmtime_structref_unroot(wasmtime_structref_t *ref);

/**
 * \brief Upcast a `structref` to an `anyref`.
 */
WASM_API_EXTERN void
wasmtime_structref_to_anyref(const wasmtime_structref_t *structref,
                             wasmtime_anyref_t *out);

/**
 * \brief Upcast a `structref` to an `eqref`.
 */
WASM_API_EXTERN void
wasmtime_structref_to_eqref(const wasmtime_structref_t *structref,
                            wasmtime_eqref_t *out);

/**
 * \brief Read a field from a struct.
 *
 * \param context The store context.
 * \param structref The struct to read from (not consumed).
 * \param index The field index.
 * \param out Receives the field value on success.
 *
 * \return NULL on success, or a #wasmtime_error_t on failure.
 */
WASM_API_EXTERN wasmtime_error_t *
wasmtime_structref_field(wasmtime_context_t *context,
                         const wasmtime_structref_t *structref, size_t index,
                         wasmtime_val_t *out);

/**
 * \brief Set a field of a struct.
 *
 * \param context The store context.
 * \param structref The struct to write to (not consumed).
 * \param index The field index.
 * \param val The value to write (not consumed; caller must still unroot if
 *        applicable).
 *
 * \return NULL on success, or a #wasmtime_error_t on failure.
 */
WASM_API_EXTERN wasmtime_error_t *
wasmtime_structref_set_field(wasmtime_context_t *context,
                             const wasmtime_structref_t *structref,
                             size_t index, const wasmtime_val_t *val);

#ifdef __cplusplus
} // extern "C"
#endif

#endif // WASMTIME_FEATURE_GC

#endif // WASMTIME_STRUCTREF_H
