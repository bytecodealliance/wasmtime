#ifndef WASMTIME_ARRAYREF_CLASS_HH
#define WASMTIME_ARRAYREF_CLASS_HH

#include <wasmtime/conf.h>

#ifdef WASMTIME_FEATURE_GC

#include <wasmtime/_store_class.hh>
#include <wasmtime/arrayref.h>
#include <wasmtime/types/arrayref.hh>

namespace wasmtime {

class Val;
class AnyRef;
class EqRef;

/**
 * \brief Pre-allocated array layout for fast allocation of array instances.
 *
 * Created from a ArrayType and a store context. Reusable for allocating
 * many array instances of the same type.
 */
class ArrayRefPre {
  friend class ArrayRef;
  WASMTIME_OWN_WRAPPER(ArrayRefPre, wasmtime_array_ref_pre)

public:
  /// Create a new array pre-allocator.
  static ArrayRefPre create(Store::Context cx, const ArrayType &ty) {
    auto *raw = wasmtime_array_ref_pre_new(cx.capi(), ty.capi());
    ArrayRefPre pre(raw);
    return pre;
  }
};

/**
 * \brief Representation of a WebAssembly `arrayref` value.
 *
 * An `arrayref` is a reference to a GC array instance. It is a subtype
 * of `eqref` and `anyref`.
 */
class ArrayRef {
  WASMTIME_REF_WRAPPER(ArrayRef, wasmtime_arrayref)

public:
  /// Allocate a new array with all elements set to the same value.
  static Result<ArrayRef> create(Store::Context cx, const ArrayRefPre &pre,
                                 const Val &elem, uint32_t len);

  /// Get the length of the array.
  Result<uint32_t> len(Store::Context cx) const {
    uint32_t out;
    auto *err = wasmtime_arrayref_len(cx.capi(), &raw, &out);
    if (err)
      return Result<uint32_t>(Error(err));
    return Result<uint32_t>(out);
  }

  /// Read an element from the array.
  Result<Val> get(Store::Context cx, uint32_t index) const;

  /// Set an element of the array.
  Result<std::monostate> set(Store::Context cx, uint32_t index,
                             const Val &value) const;

  /// Upcast to anyref.
  AnyRef to_anyref() const;

  /// Upcast to eqref.
  EqRef to_eqref() const;
};

} // namespace wasmtime

#endif // WASMTIME_FEATURE_GC

#endif // WASMTIME_ARRAYREF_CLASS_HH
