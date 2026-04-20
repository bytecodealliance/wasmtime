/**
 * \file wasmtime/exnref.hh
 */

#ifndef WASMTIME_EXNREF_HH
#define WASMTIME_EXNREF_HH

#include <wasmtime/_exnref_class.hh>

#ifdef WASMTIME_FEATURE_GC

#include <wasmtime/_val_class.hh>

namespace wasmtime {

inline Result<ExnRef> ExnRef::create(Store::Context cx, const Tag &tag,
                                     const std::vector<Val> &fields) {
  wasmtime_exnref_t exn;
  auto *error = wasmtime_exnref_new(
      cx.capi(), &tag.capi(),
      reinterpret_cast<const wasmtime_val_t *>(fields.data()), fields.size(),
      &exn);
  if (error != nullptr) {
    return Error(error);
  }
  return ExnRef(exn);
}

/// Reads a field value by index.
inline Result<Val> ExnRef::field(Store::Context cx, size_t index) const {
  wasmtime_val_t val;
  auto *error = wasmtime_exnref_field(cx.capi(), &raw, index, &val);
  if (error != nullptr) {
    return Error(error);
  }
  return Val(val);
}

} // namespace wasmtime

#endif // WASMTIME_FEATURE_GC

#endif // WASMTIME_EXNREF_HH
