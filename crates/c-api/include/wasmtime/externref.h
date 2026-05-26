/**
 * \file wasmtime/externref.h
 *
 * APIs for interacting with WebAssembly `externref` types in Wasmtime.
 */

#ifndef WASMTIME_EXTERNREF_H
#define WASMTIME_EXTERNREF_H

#include <wasmtime/conf.h>

#ifdef WASMTIME_FEATURE_GC

#include <wasmtime/val.h>

#ifdef __cplusplus
extern "C" {
#endif

/// \brief Helper function to initialize the `ref` provided to a null externref
/// value.
static inline void wasmtime_externref_set_null(wasmtime_externref_t *ref) {
  ref->store_id = 0;
}

/// \brief Helper function to return whether the provided `ref` points to a null
/// `externref` value.
///
/// Note that `ref` itself should not be null as null is represented internally
/// within a #wasmtime_externref_t value.
static inline bool wasmtime_externref_is_null(const wasmtime_externref_t *ref) {
  return ref->store_id == 0;
}

/**
 * \brief Create a new `externref` value.
 *
 * Creates a new `externref` value wrapping the provided data, returning whether
 * it was created or not.
 *
 * \param context the store context to allocate this externref within
 * \param data the host-specific data to wrap
 * \param finalizer an optional finalizer for `data`
 * \param out where to store the created value.
 *
 * When the reference is reclaimed, the wrapped data is cleaned up with the
 * provided `finalizer`.
 *
 * If `true` is returned then `out` has been filled in and must be unrooted
 * in the future with #wasmtime_externref_unroot. If `false` is returned then
 * the host wasn't able to create more GC values at this time. Performing a GC
 * may free up enough space to try again.
 *
 * If you do not unroot the value, *even if you free the corresponding
 * Store*, there will be some memory leaked, because GC roots use a
 * separate allocation to track liveness.
 */
WASM_API_EXTERN bool wasmtime_externref_new(wasmtime_context_t *context,
                                            void *data,
                                            void (*finalizer)(void *),
                                            wasmtime_externref_t *out);

/**
 * \brief Get an `externref`'s wrapped data
 *
 * Returns the original `data` passed to #wasmtime_externref_new. It is required
 * that `data` is not `NULL`.
 */
WASM_API_EXTERN void *wasmtime_externref_data(wasmtime_context_t *context,
                                              const wasmtime_externref_t *data);

/**
 * \brief Creates a new reference pointing to the same data that `ref` points
 * to (depending on the configured collector this might increase a reference
 * count or create a new GC root).
 *
 * The `out` parameter stores the cloned reference. This reference must
 * eventually be unrooted with #wasmtime_externref_unroot in the future to
 * enable GC'ing it.
 */
WASM_API_EXTERN void wasmtime_externref_clone(const wasmtime_externref_t *ref,
                                              wasmtime_externref_t *out);

/**
 * \brief Unroots the pointer `ref` from the `context` provided.
 *
 * This function will enable future garbage collection of the value pointed to
 * by `ref` once there are no more references. The `ref` value may be mutated in
 * place by this function and its contents are undefined after this function
 * returns. It should not be used until after re-initializing it.
 *
 * Note that null externref values do not need to be unrooted but are still
 * valid to pass to this function.
 */
WASM_API_EXTERN void wasmtime_externref_unroot(wasmtime_externref_t *ref);

/**
 * \brief Converts a raw `externref` value coming from #wasmtime_val_raw_t into
 * a #wasmtime_externref_t.
 *
 * The `out` reference is filled in with the non-raw version of this externref.
 * It must eventually be unrooted with #wasmtime_externref_unroot.
 */
WASM_API_EXTERN void wasmtime_externref_from_raw(wasmtime_context_t *context,
                                                 uint32_t raw,
                                                 wasmtime_externref_t *out);

/**
 * \brief Converts a #wasmtime_externref_t to a raw value suitable for storing
 * into a #wasmtime_val_raw_t.
 *
 * Note that the returned underlying value is not tracked by Wasmtime's garbage
 * collector until it enters WebAssembly. This means that a GC may release the
 * context's reference to the raw value, making the raw value invalid within the
 * context of the store. Do not perform a GC between calling this function and
 * passing it to WebAssembly.
 */
WASM_API_EXTERN uint32_t wasmtime_externref_to_raw(
    wasmtime_context_t *context, const wasmtime_externref_t *ref);

#ifdef __cplusplus
} // extern "C"
#endif

#endif // WASMTIME_FEATURE_GC
#endif // WASMTIME_EXTERNREF_H
