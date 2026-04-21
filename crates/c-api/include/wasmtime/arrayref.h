/**
 * \file wasmtime/arrayref.h
 *
 * APIs for interacting with WebAssembly `arrayref` type in Wasmtime.
 */

#ifndef WASMTIME_ARRAYREF_H
#define WASMTIME_ARRAYREF_H

#include <wasmtime/conf.h>

#ifdef WASMTIME_FEATURE_GC

#include <wasmtime/types/arrayref.h>
#include <wasmtime/val.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * \brief An opaque pre-allocated array layout for fast allocation.
 *
 * Created from a #wasmtime_array_type_t and a store context. Reusable for
 * allocating many array instances of the same type.
 *
 * Owned. Must be deleted with #wasmtime_array_ref_pre_delete.
 */
typedef struct wasmtime_array_ref_pre wasmtime_array_ref_pre_t;

/**
 * \brief Create a new array pre-allocator.
 *
 * \param context The store context.
 * \param ty The array type (not consumed; caller retains ownership).
 *
 * \return Returns a new array ref pre-allocator.
 */
WASM_API_EXTERN wasmtime_array_ref_pre_t *
wasmtime_array_ref_pre_new(wasmtime_context_t *context,
                           const wasmtime_array_type_t *ty);

/**
 * \brief Delete an array pre-allocator.
 */
WASM_API_EXTERN void
wasmtime_array_ref_pre_delete(wasmtime_array_ref_pre_t *pre);

/// \brief Initialize the `ref` to a null `arrayref` value.
static inline void wasmtime_arrayref_set_null(wasmtime_arrayref_t *ref) {
  ref->store_id = 0;
}

/// \brief Returns whether the provided `ref` is a null `arrayref` value.
static inline bool wasmtime_arrayref_is_null(const wasmtime_arrayref_t *ref) {
  return ref->store_id == 0;
}

/**
 * \brief Allocate a new array instance.
 *
 * All elements are initialized to the same value.
 *
 * \param context The store context.
 * \param pre The array pre-allocator.
 * \param elem The initial element value.
 * \param len The number of elements.
 * \param out Receives the new arrayref on success.
 *
 * \return NULL on success, or a #wasmtime_error_t on failure.
 */
WASM_API_EXTERN wasmtime_error_t *wasmtime_arrayref_new(
    wasmtime_context_t *context, const wasmtime_array_ref_pre_t *pre,
    const wasmtime_val_t *elem, uint32_t len, wasmtime_arrayref_t *out);

/**
 * \brief Clone an `arrayref`, creating a new root.
 */
WASM_API_EXTERN void
wasmtime_arrayref_clone(const wasmtime_arrayref_t *arrayref,
                        wasmtime_arrayref_t *out);

/**
 * \brief Unroot an `arrayref` to allow garbage collection.
 */
WASM_API_EXTERN void wasmtime_arrayref_unroot(wasmtime_arrayref_t *ref);

/**
 * \brief Upcast an `arrayref` to an `anyref`.
 */
WASM_API_EXTERN void
wasmtime_arrayref_to_anyref(const wasmtime_arrayref_t *arrayref,
                            wasmtime_anyref_t *out);

/**
 * \brief Upcast an `arrayref` to an `eqref`.
 */
WASM_API_EXTERN void
wasmtime_arrayref_to_eqref(const wasmtime_arrayref_t *arrayref,
                           wasmtime_eqref_t *out);

/**
 * \brief Get the length of an array.
 *
 * \param context The store context.
 * \param arrayref The array (not consumed).
 * \param out Receives the length on success.
 *
 * \return NULL on success, or a #wasmtime_error_t on failure.
 */
WASM_API_EXTERN wasmtime_error_t *
wasmtime_arrayref_len(wasmtime_context_t *context,
                      const wasmtime_arrayref_t *arrayref, uint32_t *out);

/**
 * \brief Read an element from an array.
 *
 * \param context The store context.
 * \param arrayref The array (not consumed).
 * \param index The element index.
 * \param out Receives the element value on success.
 *
 * \return NULL on success, or a #wasmtime_error_t on failure.
 */
WASM_API_EXTERN wasmtime_error_t *
wasmtime_arrayref_get(wasmtime_context_t *context,
                      const wasmtime_arrayref_t *arrayref, uint32_t index,
                      wasmtime_val_t *out);

/**
 * \brief Set an element of an array.
 *
 * \param context The store context.
 * \param arrayref The array (not consumed).
 * \param index The element index.
 * \param val The value to write.
 *
 * \return NULL on success, or a #wasmtime_error_t on failure.
 */
WASM_API_EXTERN wasmtime_error_t *
wasmtime_arrayref_set(wasmtime_context_t *context,
                      const wasmtime_arrayref_t *arrayref, uint32_t index,
                      const wasmtime_val_t *val);

#ifdef __cplusplus
} // extern "C"
#endif

#endif // WASMTIME_FEATURE_GC

#endif // WASMTIME_ARRAYREF_H
