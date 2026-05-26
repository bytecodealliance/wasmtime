/**
 * \file wasmtime/types/structref.h
 *
 * APIs for interacting with WebAssembly GC `structref` type in Wasmtime.
 */

#ifndef WASMTIME_TYPES_STRUCTREF_H
#define WASMTIME_TYPES_STRUCTREF_H

#include <wasm.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * \brief Discriminant for storage types in struct/array field types.
 */
typedef uint8_t wasmtime_storage_type_kind_t;

/// \brief An 8-bit packed integer
#define WASMTIME_STORAGE_TYPE_KIND_I8 0
/// \brief A 16-bit packed integer
#define WASMTIME_STORAGE_TYPE_KIND_I16 1
/// \brief A regular value type (i32, f64, funcref, etc).
#define WASMTIME_STORAGE_TYPE_KIND_VALTYPE 2

/// \brief A storage type descriptor for struct/array fields.
typedef struct wasmtime_storage_type {
  /// The kind of storage type this is.
  wasmtime_storage_type_kind_t kind;
  /// if `kind` is `WASMTIME_STORAGE_TYPE_KIND_VALTYPE`, then this is
  /// set.
  wasm_valtype_t *valtype;
} wasmtime_storage_type_t;

/// \brief Clone a storage type into `out`.
WASM_API_EXTERN void
wasmtime_storage_type_clone(const wasmtime_storage_type_t *storage,
                            wasmtime_storage_type_t *out);

/// \brief Delete a storage type.
///
/// Only necessary for `WASMTIME_STORAGE_TYPE_KIND_VALTYPE` when the value
/// type is a concrete reference type.
WASM_API_EXTERN void
wasmtime_storage_type_delete(wasmtime_storage_type_t *storage);

/**
 * \typedef wasmtime_field_type_t
 * \brief Convenience alias for #wasmtime_field_type
 *
 * \struct wasmtime_field_type
 * \brief Describes the type and mutability of a struct field or array element.
 */
typedef struct wasmtime_field_type {
  /// Whether this field is mutable. `true` for mutable, `false` for
  /// immutable.
  bool mutable_;
  /// The type stored in this field.
  wasmtime_storage_type_t storage;
} wasmtime_field_type_t;

/// \brief Clone a field type into `out`.
WASM_API_EXTERN void
wasmtime_field_type_clone(const wasmtime_field_type_t *field,
                          wasmtime_field_type_t *out);

/// \brief Delete a field type.
WASM_API_EXTERN void wasmtime_field_type_delete(wasmtime_field_type_t *field);

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
 * \brief Clone a struct type.
 */
WASM_API_EXTERN wasmtime_struct_type_t *
wasmtime_struct_type_copy(const wasmtime_struct_type_t *ty);

/**
 * \brief Delete a struct type.
 */
WASM_API_EXTERN void wasmtime_struct_type_delete(wasmtime_struct_type_t *ty);

/// \brief Get the number of fields in a struct type.
WASM_API_EXTERN size_t
wasmtime_struct_type_num_fields(const wasmtime_struct_type_t *ty);

/// \brief Get the field type of a struct type's field by index.
///
/// Returns `true` if `index` is in-bounds and `out` is filled in. Returns
/// `false` if `index` is out-of-bounds and `out` is not modified.
WASM_API_EXTERN bool
wasmtime_struct_type_field(const wasmtime_struct_type_t *ty, size_t index,
                           wasmtime_field_type_t *out);

#ifdef __cplusplus
} // extern "C"
#endif

#endif // WASMTIME_TYPES_STRUCTREF_H
