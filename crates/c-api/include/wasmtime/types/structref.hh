/**
 * \file wasmtime/types/structref.hh
 */

#ifndef WASMTIME_TYPES_STRUCTREF_HH
#define WASMTIME_TYPES_STRUCTREF_HH

#include <wasmtime/conf.h>

#ifdef WASMTIME_FEATURE_GC

#include <memory>
#include <vector>
#include <wasmtime/engine.hh>
#include <wasmtime/types/structref.h>

namespace wasmtime {

/**
 * \brief Describes the storage type and mutability of a struct field or array
 * element.
 */
struct FieldType {
  /// The field's storage kind.
  wasmtime_storage_kind_t kind;
  /// Whether the field is mutable or not.
  bool mutable_;

  /// Create a mutable field type.
  static FieldType mut_(wasmtime_storage_kind_t k) { return {k, true}; }
  /// Create an immutable field type.
  static FieldType const_(wasmtime_storage_kind_t k) { return {k, false}; }
};

/**
 * \brief Owned handle to a WebAssembly struct type definition.
 *
 * Create with StructType::create, then use with StructRefPre to allocate
 * instances.
 */
class StructType {
  WASMTIME_OWN_WRAPPER(StructType, wasmtime_struct_type)

  /// Create a new struct type with the given fields.
  static StructType create(const Engine &engine,
                           const std::vector<FieldType> &fields) {
    static_assert(sizeof(FieldType) == sizeof(wasmtime_field_type_t));
    auto *raw = wasmtime_struct_type_new(
        engine.capi(),
        reinterpret_cast<const wasmtime_field_type_t *>(fields.data()),
        fields.size());
    return StructType(raw);
  }
};

} // namespace wasmtime

#endif // WASMTIME_FEATURE_GC

#endif // WASMTIME_TYPES_STRUCTREF_HH
