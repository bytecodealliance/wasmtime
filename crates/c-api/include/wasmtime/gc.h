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

// ============================================================================
// StructRef
// ============================================================================

/**
 * \brief Discriminant for storage types in struct/array field types.
 *
 * Extends #wasmtime_valkind_t with packed storage types #WASMTIME_I8 and
 * #WASMTIME_I16.
 */
typedef uint8_t wasmtime_storage_kind_t;

/// \brief An 8-bit packed integer (only valid inside struct/array fields).
#define WASMTIME_I8 8
/// \brief A 16-bit packed integer (only valid inside struct/array fields).
#define WASMTIME_I16 9

/**
 * \typedef wasmtime_field_type_t
 * \brief Convenience alias for #wasmtime_field_type
 *
 * \struct wasmtime_field_type
 * \brief Describes the type and mutability of a struct field or array element.
 */
typedef struct wasmtime_field_type {
  /// The storage type of this field. Use #WASMTIME_I32, #WASMTIME_I64,
  /// #WASMTIME_F32, etc. for value types, or #WASMTIME_I8 / #WASMTIME_I16
  /// for packed types.
  wasmtime_storage_kind_t kind;
  /// Whether this field is mutable. `true` for mutable, `false` for
  /// immutable.
  bool mutable_;
} wasmtime_field_type_t;

/**
 * \brief An opaque handle to a WebAssembly struct type definition.
 *
 * A struct type describes the fields of a struct. It is used to create a
 * #wasmtime_struct_ref_pre_t, which can then allocate struct instances.
 *
 * Owned. Must be deleted with #wasmtime_struct_type_delete.
 */
typedef struct wasmtime_struct_type wasmtime_struct_type_t;

/**
 * \brief Create a new struct type.
 *
 * \param engine The engine to register the type with.
 * \param fields Pointer to an array of field type descriptors.
 * \param nfields Number of fields.
 *
 * \return Returns a new struct type, or NULL on error (e.g. invalid field
 * types).
 */
WASM_API_EXTERN wasmtime_struct_type_t *
wasmtime_struct_type_new(const wasm_engine_t *engine,
                         const wasmtime_field_type_t *fields, size_t nfields);

/**
 * \brief Delete a struct type.
 */
WASM_API_EXTERN void wasmtime_struct_type_delete(wasmtime_struct_type_t *ty);

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

/**
 * \typedef wasmtime_structref_t
 * \brief Convenience alias for #wasmtime_structref
 *
 * \struct wasmtime_structref
 * \brief A WebAssembly `structref` value.
 *
 * This structure represents a reference to a GC struct. It is a subtype of
 * `eqref` and `anyref`.
 *
 * Values must be explicitly unrooted via #wasmtime_structref_unroot.
 */
typedef struct wasmtime_structref {
  /// Internal metadata.
  uint64_t store_id;
  /// Internal to Wasmtime.
  uint32_t __private1;
  /// Internal to Wasmtime.
  uint32_t __private2;
  /// Internal to Wasmtime.
  void *__private3;
} wasmtime_structref_t;

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

#endif // WASMTIME_GC_H
