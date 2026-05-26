/**
 * \file wasmtime/types/arrayref.hh
 */

#ifndef WASMTIME_TYPES_ARRAYREF_HH
#define WASMTIME_TYPES_ARRAYREF_HH

#include <memory>
#include <wasmtime/engine.hh>
#include <wasmtime/types/arrayref.h>
#include <wasmtime/types/structref.hh>

namespace wasmtime {

/**
 * \brief Owned handle to a WebAssembly array type definition.
 */
class ArrayType {
/// Bridge the various naming conventions here.
#define wasmtime_array_type_clone wasmtime_array_type_copy
  WASMTIME_CLONE_WRAPPER(ArrayType, wasmtime_array_type)
#undef wasmtime_array_type_clone

  /// Create a new array type with the given element type.
  ArrayType(const Engine &engine, const FieldType &field)
      : ArrayType(wasmtime_array_type_new(engine.capi(), field.capi())) {}

  FieldType element_type() const {
    wasmtime_field_type_t ty;
    wasmtime_array_type_element(capi(), &ty);
    return FieldType(ty);
  }
};

} // namespace wasmtime

#endif // WASMTIME_TYPES_ARRAYREF_HH
