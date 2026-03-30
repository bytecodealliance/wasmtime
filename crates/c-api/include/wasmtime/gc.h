/**
 * \file wasmtime/gc.h
 *
 * APIs for interacting with WebAssembly GC types in Wasmtime.
 *
 * This header provides types and functions for GC reference types beyond
 * the basic `anyref` and `externref` in val.h: `eqref`, `structref`,
 * and `arrayref`.
 */

#ifndef WASMTIME_GC_H
#define WASMTIME_GC_H

#include <wasmtime/val.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * \typedef wasmtime_eqref_t
 * \brief Convenience alias for #wasmtime_eqref
 *
 * \struct wasmtime_eqref
 * \brief A WebAssembly `eqref` value.
 *
 * This structure represents a reference to a GC object that can be tested for
 * equality. The subtypes of `eqref` include `structref`, `arrayref`, and
 * `i31ref`.
 *
 * This type has the same representation and ownership semantics as
 * #wasmtime_anyref_t. Values must be explicitly unrooted via
 * #wasmtime_eqref_unroot to enable garbage collection.
 */
typedef struct wasmtime_eqref {
  /// Internal metadata tracking within the store, embedders should not
  /// configure or modify these fields.
  uint64_t store_id;
  /// Internal to Wasmtime.
  uint32_t __private1;
  /// Internal to Wasmtime.
  uint32_t __private2;
  /// Internal to Wasmtime.
  void *__private3;
} wasmtime_eqref_t;

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

#ifdef __cplusplus
} // extern "C"
#endif

#endif // WASMTIME_GC_H
