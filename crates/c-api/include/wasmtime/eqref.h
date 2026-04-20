/**
 * \file wasmtime/gc.h
 *
 * APIs for interacting with WebAssembly `eqref` type in Wasmtime.
 */

#ifndef WASMTIME_EQREF_H
#define WASMTIME_EQREF_H

#include <wasmtime/conf.h>

#ifdef WASMTIME_FEATURE_GC

#include <wasmtime/val.h>

#ifdef __cplusplus
extern "C" {
#endif

/// \brief Initialize the `ref` to a null `eqref` value.
static inline void wasmtime_eqref_set_null(wasmtime_eqref_t *ref) {
  ref->store_id = 0;
}

/// \brief Returns whether the provided `ref` is a null `eqref` value.
static inline bool wasmtime_eqref_is_null(const wasmtime_eqref_t *ref) {
  return ref->store_id == 0;
}

/**
 * \brief Clone an `eqref`, creating a new root.
 *
 * The cloned reference is stored in `out`.
 */
WASM_API_EXTERN void wasmtime_eqref_clone(const wasmtime_eqref_t *eqref,
                                          wasmtime_eqref_t *out);

/**
 * \brief Unroot an `eqref` to allow garbage collection.
 *
 * After calling this, `ref` is left in an undefined state and should not be
 * used again.
 */
WASM_API_EXTERN void wasmtime_eqref_unroot(wasmtime_eqref_t *ref);

/**
 * \brief Upcast an `eqref` to an `anyref`.
 *
 * The original `eqref` is not consumed; `out` receives a new cloned root
 * pointing to the same GC object as `anyref`.
 */
WASM_API_EXTERN void wasmtime_eqref_to_anyref(const wasmtime_eqref_t *eqref,
                                              wasmtime_anyref_t *out);

/**
 * \brief Create a new `i31ref` value.
 *
 * Creates a new `i31ref` value (which is a subtype of `eqref`) and returns a
 * pointer to it.
 *
 * If `i31val` does not fit in 31 bits, it is wrapped.
 */
WASM_API_EXTERN void wasmtime_eqref_from_i31(wasmtime_context_t *context,
                                             uint32_t i31val,
                                             wasmtime_eqref_t *out);

/**
 * \brief Test whether this `eqref` is an `i31ref`.
 *
 * Returns `true` if the given `eqref` is an `i31ref`, `false` otherwise.
 * Returns `false` for null references.
 */
WASM_API_EXTERN bool wasmtime_eqref_is_i31(wasmtime_context_t *context,
                                           const wasmtime_eqref_t *eqref);

/**
 * \brief Get the `eqref`'s underlying `i31ref` value, zero extended.
 *
 * If the given `eqref` is an instance of `i31ref`, then its value is zero
 * extended to 32 bits, written to `dst`, and `true` is returned.
 *
 * If the given `eqref` is not an instance of `i31ref`, then `false` is
 * returned and `dst` is left unmodified.
 */
WASM_API_EXTERN bool wasmtime_eqref_i31_get_u(wasmtime_context_t *context,
                                              const wasmtime_eqref_t *eqref,
                                              uint32_t *dst);

/**
 * \brief Get the `eqref`'s underlying `i31ref` value, sign extended.
 *
 * If the given `eqref` is an instance of `i31ref`, then its value is sign
 * extended to 32 bits, written to `dst`, and `true` is returned.
 *
 * If the given `eqref` is not an instance of `i31ref`, then `false` is
 * returned and `dst` is left unmodified.
 */
WASM_API_EXTERN bool wasmtime_eqref_i31_get_s(wasmtime_context_t *context,
                                              const wasmtime_eqref_t *eqref,
                                              int32_t *dst);

/**
 * \brief Test whether an `eqref` is an `arrayref`.
 *
 * Returns `false` for null references.
 */
WASM_API_EXTERN bool wasmtime_eqref_is_array(wasmtime_context_t *context,
                                             const wasmtime_eqref_t *eqref);

/**
 * \brief Downcast an `eqref` to an `arrayref`.
 *
 * If the given `eqref` is an `arrayref`, a new root for it is stored in `out`
 * and `true` is returned. Otherwise `false` is returned and `out` is set to
 * null.
 */
WASM_API_EXTERN bool wasmtime_eqref_as_array(wasmtime_context_t *context,
                                             const wasmtime_eqref_t *eqref,
                                             wasmtime_arrayref_t *out);

/**
 * \brief Test whether an `eqref` is a `structref`.
 *
 * Returns `false` for null references.
 */
WASM_API_EXTERN bool wasmtime_eqref_is_struct(wasmtime_context_t *context,
                                              const wasmtime_eqref_t *eqref);

/**
 * \brief Downcast an `eqref` to a `structref`.
 *
 * If the given `eqref` is a `structref`, a new root for it is stored in `out`
 * and `true` is returned. Otherwise `false` is returned and `out` is set to
 * null.
 */
WASM_API_EXTERN bool wasmtime_eqref_as_struct(wasmtime_context_t *context,
                                              const wasmtime_eqref_t *eqref,
                                              wasmtime_structref_t *out);

#ifdef __cplusplus
} // extern "C"
#endif

#endif // WASMTIME_FEATURE_GC

#endif // WASMTIME_EQREF_H
