#ifndef WASMTIME_STRUCTREF_CLASS_HH
#define WASMTIME_STRUCTREF_CLASS_HH

#include <wasmtime/conf.h>

#ifdef WASMTIME_FEATURE_GC

#include <wasmtime/_anyref_class.hh>
#include <wasmtime/_eqref_class.hh>
#include <wasmtime/_store_class.hh>
#include <wasmtime/helpers.hh>
#include <wasmtime/structref.h>
#include <wasmtime/types/structref.hh>

namespace wasmtime {

class Val;

/**
 * \brief Pre-allocated struct layout for fast allocation of struct instances.
 *
 * Created from a StructType and a store context. Reusable for allocating
 * many struct instances of the same type.
 */
class StructRefPre {
  WASMTIME_OWN_WRAPPER(StructRefPre, wasmtime_struct_ref_pre)

  /// Create a new struct pre-allocator.
  static StructRefPre create(Store::Context cx, const StructType &ty) {
    return StructRefPre(wasmtime_struct_ref_pre_new(cx.capi(), ty.capi()));
  }
};

/**
 * \brief Representation of a WebAssembly `structref` value.
 *
 * A `structref` is a reference to a GC struct instance. It is a subtype
 * of `eqref` and `anyref`.
 */
class StructRef {
  WASMTIME_REF_WRAPPER(StructRef, wasmtime_structref);

  /// Allocate a new struct instance.
  static Result<StructRef> create(Store::Context cx, const StructRefPre &pre,
                                  const std::vector<Val> &fields);

  /// Read a field from the struct.
  Result<Val> field(Store::Context cx, size_t index) const;

  /// Set a field of the struct.
  Result<std::monostate> set_field(Store::Context cx, size_t index,
                                   const Val &value) const;

  /// Upcast to anyref.
  AnyRef to_anyref() const;

  /// Upcast to eqref.
  EqRef to_eqref() const;
};

} // namespace wasmtime

#endif // WASMTIME_FEATURE_GC

#endif // WASMTIME_STRUCTREF_CLASS_HH
