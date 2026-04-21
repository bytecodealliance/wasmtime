/**
 * \file wasmtime/anyref.h
 *
 * APIs for interacting with WebAssembly GC `anyref` type in Wasmtime.
 */

#ifndef WASMTIME_ANYREF_H
#define WASMTIME_ANYREF_H

#ifdef WASMTIME_FEATURE_GC

#include <wasmtime/val.h>

#ifdef __cplusplus
extern "C" {
#endif

/// \brief Helper function to initialize the `ref` provided to a null anyref
/// value.
static inline void wasmtime_anyref_set_null(wasmtime_anyref_t *ref) {
  ref->store_id = 0;
}

/// \brief Helper function to return whether the provided `ref` points to a null
/// `anyref` value.
///
/// Note that `ref` itself should not be null as null is represented internally
/// within a #wasmtime_anyref_t value.
static inline bool wasmtime_anyref_is_null(const wasmtime_anyref_t *ref) {
  return ref->store_id == 0;
}

/**
 * \brief Creates a new reference pointing to the same data that `anyref`
 * points to (depending on the configured collector this might increase a
 * reference count or create a new GC root).
 *
 * The returned reference is stored in `out`.
 */
WASM_API_EXTERN void wasmtime_anyref_clone(const wasmtime_anyref_t *anyref,
                                           wasmtime_anyref_t *out);

/**
 * \brief Unroots the `ref` provided within the `context`.
 *
 * This API is required to enable the `ref` value provided to be
 * garbage-collected. This API itself does not necessarily garbage-collect the
 * value, but it's possible to collect it in the future after this.
 *
 * This may modify `ref` and the contents of `ref` are left in an undefined
 * state after this API is called and it should no longer be used.
 *
 * Note that null or i32 anyref values do not need to be unrooted but are still
 * valid to pass to this function.
 */
WASM_API_EXTERN void wasmtime_anyref_unroot(wasmtime_anyref_t *ref);

/**
 * \brief Converts a raw `anyref` value coming from #wasmtime_val_raw_t into
 * a #wasmtime_anyref_t.
 *
 * The provided `out` pointer is filled in with a reference converted from
 * `raw`.
 */
WASM_API_EXTERN void wasmtime_anyref_from_raw(wasmtime_context_t *context,
                                              uint32_t raw,
                                              wasmtime_anyref_t *out);

/**
 * \brief Converts a #wasmtime_anyref_t to a raw value suitable for storing
 * into a #wasmtime_val_raw_t.
 *
 * Note that the returned underlying value is not tracked by Wasmtime's garbage
 * collector until it enters WebAssembly. This means that a GC may release the
 * context's reference to the raw value, making the raw value invalid within the
 * context of the store. Do not perform a GC between calling this function and
 * passing it to WebAssembly.
 */
WASM_API_EXTERN uint32_t wasmtime_anyref_to_raw(wasmtime_context_t *context,
                                                const wasmtime_anyref_t *ref);

/**
 * \brief Create a new `i31ref` value.
 *
 * Creates a new `i31ref` value (which is a subtype of `anyref`) and returns a
 * pointer to it.
 *
 * If `i31val` does not fit in 31 bits, it is wrapped.
 */
WASM_API_EXTERN void wasmtime_anyref_from_i31(wasmtime_context_t *context,
                                              uint32_t i31val,
                                              wasmtime_anyref_t *out);

/**
 * \brief Test whether an `anyref` is an `i31ref`.
 *
 * Returns `true` if the given `anyref` is an `i31ref`, `false` otherwise.
 * Returns `false` for null references.
 */
WASM_API_EXTERN bool wasmtime_anyref_is_i31(wasmtime_context_t *context,
                                            const wasmtime_anyref_t *anyref);

/**
 * \brief Get the `anyref`'s underlying `i31ref` value, zero extended, if any.
 *
 * If the given `anyref` is an instance of `i31ref`, then its value is zero
 * extended to 32 bits, written to `dst`, and `true` is returned.
 *
 * If the given `anyref` is not an instance of `i31ref`, then `false` is
 * returned and `dst` is left unmodified.
 */
WASM_API_EXTERN bool wasmtime_anyref_i31_get_u(wasmtime_context_t *context,
                                               const wasmtime_anyref_t *anyref,
                                               uint32_t *dst);

/**
 * \brief Get the `anyref`'s underlying `i31ref` value, sign extended, if any.
 *
 * If the given `anyref` is an instance of `i31ref`, then its value is sign
 * extended to 32 bits, written to `dst`, and `true` is returned.
 *
 * If the given `anyref` is not an instance of `i31ref`, then `false` is
 * returned and `dst` is left unmodified.
 */
WASM_API_EXTERN bool wasmtime_anyref_i31_get_s(wasmtime_context_t *context,
                                               const wasmtime_anyref_t *anyref,
                                               int32_t *dst);

/**
 * \brief Test whether an `anyref` is an `eqref`.
 *
 * Returns `false` for null references.
 */
WASM_API_EXTERN bool wasmtime_anyref_is_eqref(wasmtime_context_t *context,
                                              const wasmtime_anyref_t *anyref);

/**
 * \brief Downcast an `anyref` to an `eqref`.
 *
 * If the given `anyref` is an `eqref`, a new root is stored in `out` and
 * `true` is returned. Otherwise `false` is returned and `out` is set to null.
 */
WASM_API_EXTERN bool wasmtime_anyref_as_eqref(wasmtime_context_t *context,
                                              const wasmtime_anyref_t *anyref,
                                              wasmtime_eqref_t *out);

/**
 * \brief Test whether an `anyref` is a `structref`.
 *
 * Returns `false` for null references.
 */
WASM_API_EXTERN bool wasmtime_anyref_is_struct(wasmtime_context_t *context,
                                               const wasmtime_anyref_t *anyref);

/**
 * \brief Downcast an `anyref` to a `structref`.
 *
 * If the given `anyref` is a `structref`, a new root is stored in `out` and
 * `true` is returned. Otherwise `false` is returned and `out` is set to null.
 */
WASM_API_EXTERN bool wasmtime_anyref_as_struct(wasmtime_context_t *context,
                                               const wasmtime_anyref_t *anyref,
                                               wasmtime_structref_t *out);

/**
 * \brief Test whether an `anyref` is an `arrayref`.
 *
 * Returns `false` for null references.
 */
WASM_API_EXTERN bool wasmtime_anyref_is_array(wasmtime_context_t *context,
                                              const wasmtime_anyref_t *anyref);

/**
 * \brief Downcast an `anyref` to an `arrayref`.
 *
 * If the given `anyref` is an `arrayref`, a new root is stored in `out` and
 * `true` is returned. Otherwise `false` is returned and `out` is set to null.
 */
WASM_API_EXTERN bool wasmtime_anyref_as_array(wasmtime_context_t *context,
                                              const wasmtime_anyref_t *anyref,
                                              wasmtime_arrayref_t *out);

#ifdef __cplusplus
} // extern "C"
#endif

#endif // WASMTIME_FEATURE_GC

#endif // WASMTIME_ANYREF_H
