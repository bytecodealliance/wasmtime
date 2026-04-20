/**
 * \file wasmtime/types/arrayref.hh
 */

#ifndef WASMTIME_TYPES_ARRAYREF_HH
#define WASMTIME_TYPES_ARRAYREF_HH

#include <wasmtime/conf.h>

#ifdef WASMTIME_FEATURE_GC

#include <memory>
#include <wasmtime/engine.hh>
#include <wasmtime/types/arrayref.h>
#include <wasmtime/types/structref.hh>

namespace wasmtime {

/**
 * \brief Owned handle to a WebAssembly array type definition.
 */
class ArrayType {
  WASMTIME_OWN_WRAPPER(ArrayType, wasmtime_array_type)

  /// Create a new array type with the given element type.
  static ArrayType create(const Engine &engine, const FieldType &field) {
    static_assert(sizeof(FieldType) == sizeof(wasmtime_field_type_t));
    auto *raw = wasmtime_array_type_new(
        engine.capi(), reinterpret_cast<const wasmtime_field_type_t *>(&field));
    return ArrayType(raw);
  }
};

} // namespace wasmtime

#endif // WASMTIME_FEATURE_GC

#endif // WASMTIME_TYPES_ARRAYREF_HH
