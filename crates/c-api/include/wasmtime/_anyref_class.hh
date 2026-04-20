#ifndef WASMTIME_ANYREF_CLASS_HH
#define WASMTIME_ANYREF_CLASS_HH

#include <wasmtime/conf.h>

#ifdef WASMTIME_FEATURE_GC

#include <wasmtime/_store_class.hh>
#include <wasmtime/anyref.h>
#include <wasmtime/helpers.hh>

namespace wasmtime {

class ArrayRef;
class EqRef;
class StructRef;

/**
 * \brief Representation of a WebAssembly `anyref` value.
 */
class AnyRef {
  WASMTIME_TOP_REF_WRAPPER(AnyRef, wasmtime_anyref);

  /// Creates a new `AnyRef` which is an `i31` with the given `value`,
  /// truncated if the upper bit is set.
  static AnyRef i31(Store::Context cx, uint32_t value) {
    wasmtime_anyref_t other;
    wasmtime_anyref_from_i31(cx.capi(), value, &other);
    return AnyRef(other);
  }

  /// \brief If this is an `i31`, get the value zero-extended.
  std::optional<uint32_t> u31(Store::Context cx) const {
    uint32_t ret = 0;
    if (wasmtime_anyref_i31_get_u(cx.capi(), &raw, &ret))
      return ret;
    return std::nullopt;
  }

  /// \brief If this is an `i31`, get the value sign-extended.
  std::optional<int32_t> i31(Store::Context cx) const {
    int32_t ret = 0;
    if (wasmtime_anyref_i31_get_s(cx.capi(), &raw, &ret))
      return ret;
    return std::nullopt;
  }

  /// \brief Returns `true` if this anyref is an i31ref.
  bool is_i31(Store::Context cx) const {
    return wasmtime_anyref_is_i31(cx.capi(), &raw);
  }

  /// \brief Returns `true` if this anyref is an eqref.
  inline bool is_eqref(Store::Context cx) const;

  /// \brief Returns `true` if this anyref is a structref.
  inline bool is_struct(Store::Context cx) const;

  /// \brief Returns `true` if this anyref is an arrayref.
  inline bool is_array(Store::Context cx) const;

  /// \brief Downcast to eqref. Returns null eqref if not an eqref.
  inline std::optional<EqRef> as_eqref(Store::Context cx) const;

  /// \brief Downcast to structref. Returns null structref if not a structref.
  inline std::optional<StructRef> as_struct(Store::Context cx) const;

  /// \brief Downcast to arrayref. Returns null arrayref if not an arrayref.
  inline std::optional<ArrayRef> as_array(Store::Context cx) const;
};

} // namespace wasmtime

#endif // WASMTIME_FEATURE_GC

#endif // WASMTIME_ANYREF_CLASS_HH
