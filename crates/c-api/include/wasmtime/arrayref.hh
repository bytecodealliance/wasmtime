/**
 * \file wasmtime/arrayref.hh
 */

#ifndef WASMTIME_ARRAYREF_HH
#define WASMTIME_ARRAYREF_HH

#include <wasmtime/_arrayref_class.hh>

#ifdef WASMTIME_FEATURE_GC

#include <wasmtime/_anyref_class.hh>
#include <wasmtime/_eqref_class.hh>
#include <wasmtime/_val_class.hh>

namespace wasmtime {

inline Result<ArrayRef> ArrayRef::create(Store::Context cx,
                                         const ArrayRefPre &pre,
                                         const Val &elem, uint32_t len) {
  wasmtime_arrayref_t out;
  auto *err =
      wasmtime_arrayref_new(cx.capi(), pre.capi(), elem.capi(), len, &out);
  if (err)
    return Result<ArrayRef>(Error(err));
  return Result<ArrayRef>(ArrayRef(out));
}

inline Result<Val> ArrayRef::get(Store::Context cx, uint32_t index) const {
  wasmtime_val_t out;
  auto *err = wasmtime_arrayref_get(cx.capi(), &raw, index, &out);
  if (err)
    return Result<Val>(Error(err));
  return Result<Val>(Val(out));
}

inline Result<std::monostate> ArrayRef::set(Store::Context cx, uint32_t index,
                                            const Val &value) const {
  auto *err = wasmtime_arrayref_set(cx.capi(), &raw, index, value.capi());
  if (err)
    return Result<std::monostate>(Error(err));
  return Result<std::monostate>(std::monostate{});
}

/// Upcast to anyref.
inline AnyRef ArrayRef::to_anyref() const {
  wasmtime_anyref_t out;
  wasmtime_arrayref_to_anyref(&raw, &out);
  return AnyRef(out);
}

/// Upcast to eqref.
inline EqRef ArrayRef::to_eqref() const {
  wasmtime_eqref_t out;
  wasmtime_arrayref_to_eqref(&raw, &out);
  return EqRef(out);
}

} // namespace wasmtime

#endif // WASMTIME_FEATURE_GC

#endif // WASMTIME_ARRAYREF_HH
