/**
 * \file wasmtime/externref.hh
 */

#ifndef WASMTIME_EXTERNREF_HH
#define WASMTIME_EXTERNREF_HH

#include <wasmtime/_externref_class.hh>

#ifdef WASMTIME_FEATURE_GC

#include <wasmtime/_func_class.hh>

namespace wasmtime {

/// Type information for `externref`, represented on the host as an optional
/// `ExternRef`.
template <> struct detail::WasmType<std::optional<ExternRef>> {
  static const bool valid = true;
  static const ValKind kind = ValKind::ExternRef;
  static void store(Store::Context cx, wasmtime_val_raw_t *p,
                    std::optional<ExternRef> &&ref) {
    if (ref) {
      p->externref = ref->take_raw(cx);
    } else {
      p->externref = 0;
    }
  }
  static void store(Store::Context cx, wasmtime_val_raw_t *p,
                    const std::optional<ExternRef> &ref) {
    if (ref) {
      p->externref = ref->borrow_raw(cx);
    } else {
      p->externref = 0;
    }
  }

  static std::optional<ExternRef> load(Store::Context cx,
                                       wasmtime_val_raw_t *p) {
    if (p->externref == 0) {
      return std::nullopt;
    }
    wasmtime_externref_t val;
    wasmtime_externref_from_raw(cx.capi(), p->externref, &val);
    return ExternRef(val);
  }
};

} // namespace wasmtime

#endif // WASMTIME_FEATURE_GC

#endif // WASMTIME_EXTERNREF_HH
