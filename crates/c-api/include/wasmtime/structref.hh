/**
 * \file wasmtime/structref.hh
 */

#ifndef WASMTIME_STRUCTREF_HH
#define WASMTIME_STRUCTREF_HH

#include <wasmtime/_structref_class.hh>

#ifdef WASMTIME_FEATURE_GC

#include <wasmtime/_anyref_class.hh>
#include <wasmtime/_eqref_class.hh>
#include <wasmtime/_val_class.hh>

namespace wasmtime {

inline Result<StructRef> StructRef::create(Store::Context cx,
                                           const StructRefPre &pre,
                                           const std::vector<Val> &fields) {
  wasmtime_structref_t out;
  auto *err = wasmtime_structref_new(
      cx.capi(), pre.capi(),
      reinterpret_cast<const wasmtime_val_t *>(fields.data()), fields.size(),
      &out);
  if (err)
    return Result<StructRef>(Error(err));
  return Result<StructRef>(StructRef(out));
}

inline Result<Val> StructRef::field(Store::Context cx, size_t index) const {
  wasmtime_val_t out;
  auto *err = wasmtime_structref_field(cx.capi(), &raw, index, &out);
  if (err)
    return Result<Val>(Error(err));
  return Result<Val>(Val(out));
}

inline Result<std::monostate>
StructRef::set_field(Store::Context cx, size_t index, const Val &value) const {
  auto *err =
      wasmtime_structref_set_field(cx.capi(), &raw, index, value.capi());
  if (err)
    return Result<std::monostate>(Error(err));
  return Result<std::monostate>(std::monostate{});
}

inline AnyRef StructRef::to_anyref() const {
  wasmtime_anyref_t out;
  wasmtime_structref_to_anyref(&raw, &out);
  return AnyRef(out);
}

inline EqRef StructRef::to_eqref() const {
  wasmtime_eqref_t out;
  wasmtime_structref_to_eqref(&raw, &out);
  return EqRef(out);
}

} // namespace wasmtime

#endif // WASMTIME_FEATURE_GC

#endif // WASMTIME_STRUCTREF_HH
