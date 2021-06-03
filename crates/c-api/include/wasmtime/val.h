/**
 * \file wasmtime/val.h
 *
 * APIs for interacting with WebAssembly values in Wasmtime.
 */

#ifndef WASMTIME_VAL_H
#define WASMTIME_VAL_H

#include <wasm.h>
#include <wasmtime/extern.h>

#ifdef __cplusplus
extern "C" {
#endif

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
 * \param data the host-specific data to wrap
 * \param finalizer an optional finalizer for `data`
 *
 * When the reference is reclaimed, the wrapped data is cleaned up with the
 * provided `finalizer`.
 *
 * The returned value must be deleted with #wasmtime_externref_delete
 */
WASM_API_EXTERN wasmtime_externref_t *wasmtime_externref_new(void *data, void (*finalizer)(void*));

/**
 * \brief Get an `externref`'s wrapped data
 *
 * Returns the original `data` passed to #wasmtime_externref_new. It is required
 * that `data` is not `NULL`.
 */
WASM_API_EXTERN void *wasmtime_externref_data(wasmtime_externref_t *data);

/**
 * \brief Creates a shallow copy of the `externref` argument, returning a
 * separately owned pointer (increases the reference count).
 */
WASM_API_EXTERN wasmtime_externref_t *wasmtime_externref_clone(wasmtime_externref_t *ref);

/**
 * \brief Decrements the reference count of the `ref`, deleting it if it's the
 * last reference.
 */
WASM_API_EXTERN void wasmtime_externref_delete(wasmtime_externref_t *ref);

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
/// \brief Value of #wasmtime_valkind_t meaning that #wasmtime_val_t is a funcref
#define WASMTIME_FUNCREF 5
/// \brief Value of #wasmtime_valkind_t meaning that #wasmtime_val_t is an externref
#define WASMTIME_EXTERNREF 6

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
  /// Field used if #wasmtime_val_t::kind is #WASMTIME_FUNCREF
  ///
  /// If this value represents a `ref.null func` value then the `store_id` field
  /// is set to zero.
  wasmtime_func_t funcref;
  /// Field used if #wasmtime_val_t::kind is #WASMTIME_EXTERNREF
  ///
  /// If this value represents a `ref.null extern` value then this pointer will
  /// be `NULL`.
  wasmtime_externref_t *externref;
  /// Field used if #wasmtime_val_t::kind is #WASMTIME_V128
  wasmtime_v128 v128;
} wasmtime_valunion_t;

/**
 * \typedef wasmtime_val_t
 * \brief Convenience alias for #wasmtime_val_t
 *
 * \union wasmtime_val
 * \brief Container for different kinds of wasm values.
 *
 * Note that this structure may contain an owned value, namely
 * #wasmtime_externref_t, depending on the context in which this is used. APIs
 * which consume a #wasmtime_val_t do not take ownership, but APIs that return
 * #wasmtime_val_t require that #wasmtime_val_delete is called to deallocate
 * the value.
 */
typedef struct wasmtime_val {
  /// Discriminant of which field of #of is valid.
  wasmtime_valkind_t kind;
  /// Container for the extern item's value.
  wasmtime_valunion_t of;
} wasmtime_val_t;

/**
 * \brief Delets an owned #wasmtime_val_t.
 *
 * Note that this only deletes the contents, not the memory that `val` points to
 * itself (which is owned by the caller).
 */
WASM_API_EXTERN void wasmtime_val_delete(wasmtime_val_t *val);

/**
 * \brief Copies `src` into `dst`.
 */
WASM_API_EXTERN void wasmtime_val_copy(wasmtime_val_t *dst, const wasmtime_val_t *src);

#ifdef __cplusplus
}  // extern "C"
#endif

#endif // WASMTIME_VAL_H

