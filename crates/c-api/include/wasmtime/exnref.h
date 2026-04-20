/**
 * \file wasmtime/exnref.h
 *
 * APIs for interacting with WebAssembly `exnref` type in Wasmtime.
 */

#ifndef WASMTIME_EXNREF_H
#define WASMTIME_EXNREF_H

#include <wasmtime/conf.h>

#ifdef WASMTIME_FEATURE_GC

#include <wasmtime/val.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * \brief Creates a new exception object.
 *
 * \param store the store context
 * \param tag the tag to associate with this exception
 * \param fields pointer to an array of field values matching the tag's
 *        payload signature
 * \param nfields the number of elements in `fields`
 * \param exn_ret on success, set to the newly allocated exception.
 *        The caller owns the returned pointer and must free it with
 *        #wasmtime_exnref_unroot.
 *
 * \return NULL on success, or an error on failure.
 */
WASM_API_EXTERN wasmtime_error_t *
wasmtime_exnref_new(wasmtime_context_t *store, const wasmtime_tag_t *tag,
                    const wasmtime_val_t *fields, size_t nfields,
                    wasmtime_exnref_t *exn_ret);

/// \brief Helper function to initialize the `ref` provided to a null exnref
/// value.
static inline void wasmtime_exnref_set_null(wasmtime_exnref_t *ref) {
  ref->store_id = 0;
}

/// \brief Helper function to return whether the provided `ref` points to a null
/// `exnref` value.
static inline bool wasmtime_exnref_is_null(const wasmtime_exnref_t *ref) {
  return ref->store_id == 0;
}

/**
 * \brief Creates a new reference pointing to the same exception that `ref`
 * points to.
 *
 * The returned reference is stored in `out`.
 */
WASM_API_EXTERN void wasmtime_exnref_clone(const wasmtime_exnref_t *ref,
                                           wasmtime_exnref_t *out);

/**
 * \brief Unroots the `ref` provided, enabling future garbage collection.
 *
 * After this call, `ref` is left in an undefined state and should not be used.
 */
WASM_API_EXTERN void wasmtime_exnref_unroot(wasmtime_exnref_t *ref);

/**
 * \brief Converts a raw `exnref` value coming from #wasmtime_val_raw_t into
 * a #wasmtime_exnref_t.
 *
 * The `out` reference is filled in with the non-raw version of this exnref.
 * It must eventually be unrooted with #wasmtime_exnref_unroot.
 */
WASM_API_EXTERN void wasmtime_exnref_from_raw(wasmtime_context_t *context,
                                              uint32_t raw,
                                              wasmtime_exnref_t *out);

/**
 * \brief Converts a #wasmtime_exnref_t to a raw value suitable for storing
 * into a #wasmtime_val_raw_t.
 *
 * Note that the returned underlying value is not tracked by Wasmtime's garbage
 * collector until it enters WebAssembly. This means that a GC may release the
 * context's reference to the raw value, making the raw value invalid within the
 * context of the store. Do not perform a GC between calling this function and
 * passing it to WebAssembly.
 */
WASM_API_EXTERN uint32_t wasmtime_exnref_to_raw(wasmtime_context_t *context,
                                                const wasmtime_exnref_t *ref);

/**
 * \brief Returns the tag associated with this exception.
 *
 * \param store the store context
 * \param exn the exception to query
 * \param tag_ret on success, filled with the exception's tag
 *
 * \return NULL on success, or an error on failure.
 */
WASM_API_EXTERN wasmtime_error_t *
wasmtime_exnref_tag(wasmtime_context_t *store, const wasmtime_exnref_t *exn,
                    wasmtime_tag_t *tag_ret);

/**
 * \brief Returns the number of fields in this exception.
 *
 * \param store the store context
 * \param exn the exception to query
 */
WASM_API_EXTERN size_t wasmtime_exnref_field_count(
    wasmtime_context_t *store, const wasmtime_exnref_t *exn);

/**
 * \brief Reads a field value from this exception by index.
 *
 * \param store the store context
 * \param exn the exception to query
 * \param index the field index (0-based)
 * \param val_ret on success, filled with the field value
 *                (caller-owned on return).
 *
 * \return NULL on success, or an error if the index is out of bounds.
 */
WASM_API_EXTERN wasmtime_error_t *
wasmtime_exnref_field(wasmtime_context_t *store, const wasmtime_exnref_t *exn,
                      size_t index, wasmtime_val_t *val_ret);

/**
 * \brief Sets the pending exception on the store and returns a trap.
 *
 * This transfers ownership of `exn` to the store. After this call,
 * the caller must not use or free `exn`.
 *
 * Returns a `wasm_trap_t` that the host callback MUST return to signal
 * to the Wasm runtime that an exception was thrown. The caller owns
 * the returned trap.
 *
 * \param store the store context
 * \param exn the exception to throw (ownership transferred)
 * \return a trap to return from the host callback (caller-owned)
 */
WASM_API_EXTERN wasm_trap_t *
wasmtime_context_set_exception(wasmtime_context_t *store,
                               wasmtime_exnref_t *exn);

/**
 * \brief Takes the pending exception from the store, if any.
 *
 * If there is a pending exception, removes it from the store and
 * returns it. The caller owns the returned pointer and must free it
 * with #wasmtime_exnref_unroot.
 *
 * \param store the store context
 * \param exn_ret on success, set to the exception (caller-owned)
 *
 * \return true if there was a pending exception, false otherwise.
 */
WASM_API_EXTERN bool
wasmtime_context_take_exception(wasmtime_context_t *store,
                                wasmtime_exnref_t *exn_ret);

/**
 * \brief Tests whether there is a pending exception on the store.
 *
 * \param store the store context
 *
 * \return true if a pending exception is set, false otherwise.
 */
WASM_API_EXTERN bool wasmtime_context_has_exception(wasmtime_context_t *store);

#ifdef __cplusplus
} // extern "C"
#endif

#endif // WASMTIME_FEATURE_GC
#endif // WASMTIME_EXNREF_H
