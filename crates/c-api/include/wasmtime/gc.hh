/**
 * \file wasmtime/gc.hh
 *
 * C++ API for WebAssembly GC types: eqref, structref, and arrayref.
 */

#ifndef WASMTIME_GC_HH
#define WASMTIME_GC_HH

#include <wasmtime/gc.h>
#include <wasmtime/val.hh>

namespace wasmtime {

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
  friend class Val;
  friend class AnyRef;

  wasmtime_eqref_t val;

public:
  /// Creates a new `EqRef` from its C-API representation.
  explicit EqRef(wasmtime_eqref_t val) : val(val) {}

  /// Copy constructor.
  EqRef(const EqRef &other) { wasmtime_eqref_clone(&other.val, &val); }

  /// Copy assignment.
  EqRef &operator=(const EqRef &other) {
    wasmtime_eqref_unroot(&val);
    wasmtime_eqref_clone(&other.val, &val);
    return *this;
  }

  /// Move constructor.
  EqRef(EqRef &&other) {
    val = other.val;
    wasmtime_eqref_set_null(&other.val);
  }

  /// Move assignment.
  EqRef &operator=(EqRef &&other) {
    wasmtime_eqref_unroot(&val);
    val = other.val;
    wasmtime_eqref_set_null(&other.val);
    return *this;
  }

  ~EqRef() { wasmtime_eqref_unroot(&val); }

  /// Create an `eqref` from an i31 value.
  static EqRef from_i31(Store::Context cx, uint32_t val) {
    wasmtime_eqref_t out;
    wasmtime_eqref_from_i31(cx.capi(), val, &out);
    return EqRef(out);
  }

  /// Returns `true` if this eqref is an i31ref.
  bool is_i31(Store::Context cx) const {
    return wasmtime_eqref_is_i31(cx.capi(), &val);
  }

  /// Get the i31 value as an unsigned 32-bit integer.
  /// Returns `std::nullopt` if this eqref is not an i31ref.
  std::optional<uint32_t> i31_get_u(Store::Context cx) const {
    uint32_t dst;
    if (wasmtime_eqref_i31_get_u(cx.capi(), &val, &dst))
      return dst;
    return std::nullopt;
  }

  /// Get the i31 value as a signed 32-bit integer.
  /// Returns `std::nullopt` if this eqref is not an i31ref.
  std::optional<int32_t> i31_get_s(Store::Context cx) const {
    int32_t dst;
    if (wasmtime_eqref_i31_get_s(cx.capi(), &val, &dst))
      return dst;
    return std::nullopt;
  }

  /// Upcast this `eqref` to an `anyref`.
  AnyRef to_anyref() const {
    wasmtime_anyref_t out;
    wasmtime_eqref_to_anyref(&val, &out);
    return AnyRef(out);
  }
};

} // namespace wasmtime

#endif // WASMTIME_GC_HH
