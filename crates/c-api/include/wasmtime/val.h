/**
 * \file wasmtime/val.h
 *
 * APIs for interacting with WebAssembly values in Wasmtime.
 */

#ifndef WASMTIME_VAL_H
#define WASMTIME_VAL_H

#include <stdalign.h>
#include <wasm.h>
#include <wasmtime/extern.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * \typedef wasmtime_anyref_t
 * \brief Convenience alias for #wasmtime_anyref
 *
 * \struct wasmtime_anyref
 * \brief A host-defined un-forgeable reference to pass into WebAssembly.
 *
 * This structure represents an `anyref` that can be passed to WebAssembly.
 * It cannot be forged by WebAssembly itself and is guaranteed to have been
 * created by the host.
 */
typedef struct wasmtime_anyref wasmtime_anyref_t;

/**
 * \brief Creates a shallow copy of the `anyref` argument, returning a
 * separately owned pointer (depending on the configured collector this might
 * increase a reference count or create a new GC root).
 */
WASM_API_EXTERN wasmtime_anyref_t *
wasmtime_anyref_clone(wasmtime_context_t *context, wasmtime_anyref_t *ref);

/**
 * \brief Drops an owned pointer to `ref`, potentially deleting it if it's the
 * last reference, or allowing it to be collected during the next GC.
 */
WASM_API_EXTERN void wasmtime_anyref_delete(wasmtime_context_t *context,
                                            wasmtime_anyref_t *ref);

/**
 * \brief Converts a raw `anyref` value coming from #wasmtime_val_raw_t into
 * a #wasmtime_anyref_t.
 *
 * Note that the returned #wasmtime_anyref_t is an owned value that must be
 * deleted via #wasmtime_anyref_delete by the caller if it is non-null.
 */
WASM_API_EXTERN wasmtime_anyref_t *
wasmtime_anyref_from_raw(wasmtime_context_t *context, uint32_t raw);

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
WASM_API_EXTERN wasmtime_anyref_t *
wasmtime_anyref_from_i31(wasmtime_context_t *context, uint32_t i31val);

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
                                               wasmtime_anyref_t *anyref,
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
                                               wasmtime_anyref_t *anyref,
                                               int32_t *dst);

/**
 * \typedef wasmtime_externref_t
 * \brief Convenience alias for #wasmtime_externref
 *
 * \struct wasmtime_externref
 * \brief A host-defined un-forgeable reference to pass into WebAssembly.
 *
 * This structure represents an `externref` that can be passed to WebAssembly.
 * It cannot be forged by WebAssembly itself and is guaranteed to have been
 * created by the host.
 */
typedef struct wasmtime_externref wasmtime_externref_t;

/**
 * \brief Create a new `externref` value.
 *
 * Creates a new `externref` value wrapping the provided data, returning the
 * pointer to the externref.
 *
 * \param context the store context to allocate this externref within
 * \param data the host-specific data to wrap
 * \param finalizer an optional finalizer for `data`
 *
 * When the reference is reclaimed, the wrapped data is cleaned up with the
 * provided `finalizer`.
 *
 * The returned value must be deleted with #wasmtime_externref_delete and may
 * not be used after the context is destroyed.
 */
WASM_API_EXTERN wasmtime_externref_t *
wasmtime_externref_new(wasmtime_context_t *context, void *data,
                       void (*finalizer)(void *));

/**
 * \brief Get an `externref`'s wrapped data
 *
 * Returns the original `data` passed to #wasmtime_externref_new. It is required
 * that `data` is not `NULL`.
 */
WASM_API_EXTERN void *wasmtime_externref_data(wasmtime_context_t *context,
                                              wasmtime_externref_t *data);

/**
 * \brief Creates a shallow copy of the `externref` argument, returning a
 * separately owned pointer (depending on the configured collector this might
 * increase a reference count or create a new GC root).
 */
WASM_API_EXTERN wasmtime_externref_t *
wasmtime_externref_clone(wasmtime_context_t *context,
                         wasmtime_externref_t *ref);

/**
 * \brief Drops an owned pointer to `ref`, potentially deleting it if it's the
 * last reference, or allowing it to be collected during the next GC.
 */
WASM_API_EXTERN void wasmtime_externref_delete(wasmtime_context_t *context,
                                               wasmtime_externref_t *ref);

/**
 * \brief Converts a raw `externref` value coming from #wasmtime_val_raw_t into
 * a #wasmtime_externref_t.
 *
 * Note that the returned #wasmtime_externref_t is an owned value that must be
 * deleted via #wasmtime_externref_delete by the caller if it is non-null.
 */
WASM_API_EXTERN wasmtime_externref_t *
wasmtime_externref_from_raw(wasmtime_context_t *context, uint32_t raw);

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

/// \brief Discriminant stored in #wasmtime_val::kind
typedef uint8_t wasmtime_valkind_t;
/// \brief Value of #wasmtime_valkind_t meaning that #wasmtime_val_t is an i32
#define WASMTIME_I32 0
/// \brief Value of #wasmtime_valkind_t meaning that #wasmtime_val_t is an i64
#define WASMTIME_I64 1
/// \brief Value of #wasmtime_valkind_t meaning that #wasmtime_val_t is a f32
#define WASMTIME_F32 2
/// \brief Value of #wasmtime_valkind_t meaning that #wasmtime_val_t is a f64
#define WASMTIME_F64 3
/// \brief Value of #wasmtime_valkind_t meaning that #wasmtime_val_t is a v128
#define WASMTIME_V128 4
/// \brief Value of #wasmtime_valkind_t meaning that #wasmtime_val_t is a
/// funcref
#define WASMTIME_FUNCREF 5
/// \brief Value of #wasmtime_valkind_t meaning that #wasmtime_val_t is an
/// externref
#define WASMTIME_EXTERNREF 6
/// \brief Value of #wasmtime_valkind_t meaning that #wasmtime_val_t is an
/// anyref
#define WASMTIME_ANYREF 7

/// \brief A 128-bit value representing the WebAssembly `v128` type. Bytes are
/// stored in little-endian order.
typedef uint8_t wasmtime_v128[16];

/**
 * \typedef wasmtime_valunion_t
 * \brief Convenience alias for #wasmtime_valunion
 *
 * \union wasmtime_valunion
 * \brief Container for different kinds of wasm values.
 *
 * This type is contained in #wasmtime_val_t and contains the payload for the
 * various kinds of items a value can be.
 */
typedef union wasmtime_valunion {
  /// Field used if #wasmtime_val_t::kind is #WASMTIME_I32
  int32_t i32;
  /// Field used if #wasmtime_val_t::kind is #WASMTIME_I64
  int64_t i64;
  /// Field used if #wasmtime_val_t::kind is #WASMTIME_F32
  float32_t f32;
  /// Field used if #wasmtime_val_t::kind is #WASMTIME_F64
  float64_t f64;
  /// Field used if #wasmtime_val_t::kind is #WASMTIME_ANYREF
  ///
  /// If this value represents a `ref.null any` value then this pointer will
  /// be `NULL`.
  wasmtime_anyref_t *anyref;
  /// Field used if #wasmtime_val_t::kind is #WASMTIME_EXTERNREF
  ///
  /// If this value represents a `ref.null extern` value then this pointer will
  /// be `NULL`.
  wasmtime_externref_t *externref;
  /// Field used if #wasmtime_val_t::kind is #WASMTIME_FUNCREF
  ///
  /// Use #wasmtime_funcref_is_null to test whether this is a null function
  /// reference.
  wasmtime_func_t funcref;
  /// Field used if #wasmtime_val_t::kind is #WASMTIME_V128
  wasmtime_v128 v128;
} wasmtime_valunion_t;

/// \brief Initialize a `wasmtime_func_t` value as a null function reference.
///
/// This function will initialize the `func` provided to be a null function
/// reference. Used in conjunction with #wasmtime_val_t and
/// #wasmtime_valunion_t.
static inline void wasmtime_funcref_set_null(wasmtime_func_t *func) {
  func->store_id = 0;
}

/// \brief Helper function to test whether the `func` provided is a null
/// function reference.
///
/// This function is used with #wasmtime_val_t and #wasmtime_valunion_t and its
/// `funcref` field. This will test whether the field represents a null funcref.
static inline bool wasmtime_funcref_is_null(const wasmtime_func_t *func) {
  return func->store_id == 0;
}

/**
 * \typedef wasmtime_val_raw_t
 * \brief Convenience alias for #wasmtime_val_raw
 *
 * \union wasmtime_val_raw
 * \brief Container for possible wasm values.
 *
 * This type is used on conjunction with #wasmtime_func_new_unchecked as well
 * as #wasmtime_func_call_unchecked. Instances of this type do not have type
 * information associated with them, it's up to the embedder to figure out
 * how to interpret the bits contained within, often using some other channel
 * to determine the type.
 */
typedef union wasmtime_val_raw {
  /// Field for when this val is a WebAssembly `i32` value.
  ///
  /// Note that this field is always stored in a little-endian format.
  int32_t i32;
  /// Field for when this val is a WebAssembly `i64` value.
  ///
  /// Note that this field is always stored in a little-endian format.
  int64_t i64;
  /// Field for when this val is a WebAssembly `f32` value.
  ///
  /// Note that this field is always stored in a little-endian format.
  float32_t f32;
  /// Field for when this val is a WebAssembly `f64` value.
  ///
  /// Note that this field is always stored in a little-endian format.
  float64_t f64;
  /// Field for when this val is a WebAssembly `v128` value.
  ///
  /// Note that this field is always stored in a little-endian format.
  wasmtime_v128 v128;
  /// Field for when this val is a WebAssembly `anyref` value.
  ///
  /// If this is set to 0 then it's a null anyref, otherwise this must be
  /// passed to `wasmtime_anyref_from_raw` to determine the
  /// `wasmtime_anyref_t`.
  ///
  /// Note that this field is always stored in a little-endian format.
  uint32_t anyref;
  /// Field for when this val is a WebAssembly `externref` value.
  ///
  /// If this is set to 0 then it's a null externref, otherwise this must be
  /// passed to `wasmtime_externref_from_raw` to determine the
  /// `wasmtime_externref_t`.
  ///
  /// Note that this field is always stored in a little-endian format.
  uint32_t externref;
  /// Field for when this val is a WebAssembly `funcref` value.
  ///
  /// If this is set to 0 then it's a null funcref, otherwise this must be
  /// passed to `wasmtime_func_from_raw` to determine the `wasmtime_func_t`.
  ///
  /// Note that this field is always stored in a little-endian format.
  void *funcref;
} wasmtime_val_raw_t;

// Assert that the shape of this type is as expected since it needs to match
// Rust.
static inline void __wasmtime_val_assertions() {
  static_assert(sizeof(wasmtime_valunion_t) == 16, "should be 16-bytes large");
  static_assert(__alignof(wasmtime_valunion_t) == 8,
                "should be 8-byte aligned");
  static_assert(sizeof(wasmtime_val_raw_t) == 16, "should be 16 bytes large");
  static_assert(__alignof(wasmtime_val_raw_t) == 8, "should be 8-byte aligned");
}

/**
 * \typedef wasmtime_val_t
 * \brief Convenience alias for #wasmtime_val_t
 *
 * \union wasmtime_val
 * \brief Container for different kinds of wasm values.
 *
 * Note that this structure may contain an owned value, namely rooted GC
 * references, depending on the context in which this is used. APIs which
 * consume a #wasmtime_val_t do not take ownership, but APIs that return
 * #wasmtime_val_t require that #wasmtime_val_delete is called to deallocate the
 * value.
 */
typedef struct wasmtime_val {
  /// Discriminant of which field of #of is valid.
  wasmtime_valkind_t kind;
  /// Container for the extern item's value.
  wasmtime_valunion_t of;
} wasmtime_val_t;

/**
 * \brief Deletes an owned #wasmtime_val_t.
 *
 * Note that this only deletes the contents, not the memory that `val` points to
 * itself (which is owned by the caller).
 */
WASM_API_EXTERN void wasmtime_val_delete(wasmtime_context_t *context,
                                         wasmtime_val_t *val);

/**
 * \brief Copies `src` into `dst`.
 */
WASM_API_EXTERN void wasmtime_val_copy(wasmtime_context_t *context,
                                       wasmtime_val_t *dst,
                                       const wasmtime_val_t *src);

#ifdef __cplusplus
} // extern "C"
#endif

#endif // WASMTIME_VAL_H
