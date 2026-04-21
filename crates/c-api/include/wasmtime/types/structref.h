/**
 * \file wasmtime/types/structref.h
 *
 * APIs for interacting with WebAssembly GC `structref` type in Wasmtime.
 */

#ifndef WASMTIME_TYPES_STRUCTREF_H
#define WASMTIME_TYPES_STRUCTREF_H

#include <wasmtime/conf.h>

#ifdef WASMTIME_FEATURE_GC

#include <wasm.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * \brief Discriminant for storage types in struct/array field types.
 *
 * Extends #wasmtime_valkind_t with packed storage types
 * #WASMTIME_STORAGE_KIND_I8 and #WASMTIME_STORAGE_KIND_I16.
 */
typedef uint8_t wasmtime_storage_kind_t;

/// \brief An 8-bit packed integer (only valid inside struct/array fields).
#define WASMTIME_STORAGE_KIND_I8 9
/// \brief A 16-bit packed integer (only valid inside struct/array fields).
#define WASMTIME_STORAGE_KIND_I16 10

/**
 * \typedef wasmtime_field_type_t
 * \brief Convenience alias for #wasmtime_field_type
 *
 * \struct wasmtime_field_type
 * \brief Describes the type and mutability of a struct field or array element.
 */
typedef struct wasmtime_field_type {
  /// The storage type of this field. Use #WASMTIME_I32, #WASMTIME_I64,
  /// #WASMTIME_F32, etc. for value types, or #WASMTIME_STORAGE_KIND_I8 /
  /// #WASMTIME_STORAGE_KIND_I16 for packed types.
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

#ifdef __cplusplus
} // extern "C"
#endif

#endif // WASMTIME_FEATURE_GC

#endif // WASMTIME_TYPES_STRUCTREF_H
