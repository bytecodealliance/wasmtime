/**
 * \file wasmtime/eqref.hh
 */

#ifndef WASMTIME_EQREF_HH
#define WASMTIME_EQREF_HH

#include <wasmtime/_eqref_class.hh>

#ifdef WASMTIME_FEATURE_GC

#include <wasmtime/_anyref_class.hh>
#include <wasmtime/_arrayref_class.hh>
#include <wasmtime/_structref_class.hh>

namespace wasmtime {

inline std::optional<StructRef> EqRef::as_struct(Store::Context cx) const {
  wasmtime_structref_t out;
  if (wasmtime_eqref_as_struct(cx.capi(), &raw, &out))
    return StructRef(out);
  return std::nullopt;
}

inline std::optional<ArrayRef> EqRef::as_array(Store::Context cx) const {
  wasmtime_arrayref_t out;
  if (wasmtime_eqref_as_array(cx.capi(), &raw, &out))
    return ArrayRef(out);
  return std::nullopt;
}

inline AnyRef EqRef::to_anyref() const {
  wasmtime_anyref_t out;
  wasmtime_eqref_to_anyref(&raw, &out);
  return AnyRef(out);
}

} // namespace wasmtime

#endif // WASMTIME_FEATURE_GC

#endif // WASMTIME_EQREF_HH
