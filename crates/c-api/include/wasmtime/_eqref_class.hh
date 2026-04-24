#ifndef WASMTIME_EQREF_CLASS_HH
#define WASMTIME_EQREF_CLASS_HH

#include <optional>
#include <wasmtime/conf.h>

#ifdef WASMTIME_FEATURE_GC

#include <wasmtime/_store_class.hh>
#include <wasmtime/eqref.h>
#include <wasmtime/helpers.hh>

namespace wasmtime {

class AnyRef;
class StructRef;
class ArrayRef;

/**
 * \brief Representation of a WebAssembly `eqref` value.
 *
 * An `eqref` is a reference to a GC object that supports equality testing.
 * Subtypes include `structref`, `arrayref`, and `i31ref`.
 *
 * Like all GC references, `EqRef` values are rooted in a `Store` and must be
 * unrooted (by destruction or move) to allow garbage collection.
 */
class EqRef {
  WASMTIME_REF_WRAPPER(EqRef, wasmtime_eqref);

public:
  /// Create an `eqref` from an i31 value.
  static EqRef from_i31(Store::Context cx, uint32_t val) {
    wasmtime_eqref_t out;
    wasmtime_eqref_from_i31(cx.capi(), val, &out);
    return EqRef(out);
  }

  /// Returns `true` if this eqref is an i31ref.
  bool is_i31(Store::Context cx) const {
    return wasmtime_eqref_is_i31(cx.capi(), &raw);
  }

  /// Get the i31 value as an unsigned 32-bit integer.
  /// Returns `std::nullopt` if this eqref is not an i31ref.
  std::optional<uint32_t> i31_get_u(Store::Context cx) const {
    uint32_t dst;
    if (wasmtime_eqref_i31_get_u(cx.capi(), &raw, &dst))
      return dst;
    return std::nullopt;
  }

  /// Get the i31 value as a signed 32-bit integer.
  /// Returns `std::nullopt` if this eqref is not an i31ref.
  std::optional<int32_t> i31_get_s(Store::Context cx) const {
    int32_t dst;
    if (wasmtime_eqref_i31_get_s(cx.capi(), &raw, &dst))
      return dst;
    return std::nullopt;
  }

  /// Returns `true` if this eqref is a structref.
  bool is_struct(Store::Context cx) const {
    return wasmtime_eqref_is_struct(cx.capi(), &raw);
  }

  /// Returns `true` if this eqref is an arrayref.
  bool is_array(Store::Context cx) const {
    return wasmtime_eqref_is_array(cx.capi(), &raw);
  }

  /// Upcast this `eqref` to an `anyref`.
  AnyRef to_anyref() const;

  /// Downcast this `eqref` into a `structref`.
  //
  // as_struct() defined after StructRef below.
  std::optional<StructRef> as_struct(Store::Context cx) const;

  /// Downcast this `eqref` into an `arrayref`.
  //
  // as_array() defined after ArrayRef below.
  std::optional<ArrayRef> as_array(Store::Context cx) const;
};

} // namespace wasmtime

#endif // WASMTIME_FEATURE_GC

#endif // WASMTIME_EQREF_CLASS_HH
