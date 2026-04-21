/**
 * \file wasmtime/anyref.hh
 */

#ifndef WASMTIME_ANYREF_HH
#define WASMTIME_ANYREF_HH

#include <wasmtime/_anyref_class.hh>

#ifdef WASMTIME_FEATURE_GC

#include <wasmtime/_arrayref_class.hh>
#include <wasmtime/_eqref_class.hh>
#include <wasmtime/_structref_class.hh>

namespace wasmtime {

// AnyRef downcast method definitions (declared in val.hh)
inline bool AnyRef::is_eqref(Store::Context cx) const {
  return wasmtime_anyref_is_eqref(cx.capi(), &raw);
}
inline bool AnyRef::is_struct(Store::Context cx) const {
  return wasmtime_anyref_is_struct(cx.capi(), &raw);
}
inline bool AnyRef::is_array(Store::Context cx) const {
  return wasmtime_anyref_is_array(cx.capi(), &raw);
}
inline std::optional<EqRef> AnyRef::as_eqref(Store::Context cx) const {
  wasmtime_eqref_t out;
  if (wasmtime_anyref_as_eqref(cx.capi(), &raw, &out))
    return EqRef(out);
  return std::nullopt;
}
inline std::optional<StructRef> AnyRef::as_struct(Store::Context cx) const {
  wasmtime_structref_t out;
  if (wasmtime_anyref_as_struct(cx.capi(), &raw, &out))
    return StructRef(out);
  return std::nullopt;
}
inline std::optional<ArrayRef> AnyRef::as_array(Store::Context cx) const {
  wasmtime_arrayref_t out;
  if (wasmtime_anyref_as_array(cx.capi(), &raw, &out))
    return ArrayRef(out);
  return std::nullopt;
}

} // namespace wasmtime

#endif // WASMTIME_FEATURE_GC

#endif // WASMTIME_ANYREF_HH
