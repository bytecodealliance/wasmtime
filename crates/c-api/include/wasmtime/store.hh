/**
 * \file wasmtime/store.hh
 */

#ifndef WASMTIME_STORE_HH
#define WASMTIME_STORE_HH

#include <wasmtime/_exnref_class.hh>
#include <wasmtime/_func_class.hh>
#include <wasmtime/_store_class.hh>
#include <wasmtime/trap.hh>

namespace wasmtime {

/// Definition for the `funcref` native wasm type
template <> struct detail::WasmType<std::optional<Func>> {
  /// @private
  static const bool valid = true;
  /// @private
  static const ValKind kind = ValKind::FuncRef;
  /// @private
  static void store(Store::Context cx, wasmtime_val_raw_t *p,
                    const std::optional<Func> func) {
    if (func) {
      p->funcref = wasmtime_func_to_raw(cx.capi(), &func->capi());
    } else {
      p->funcref = 0;
    }
  }
  /// @private
  static std::optional<Func> load(Store::Context cx, wasmtime_val_raw_t *p) {
    if (p->funcref == 0) {
      return std::nullopt;
    }
    wasmtime_func_t ret;
    wasmtime_func_from_raw(cx.capi(), p->funcref, &ret);
    return ret;
  }
};

inline Store::Context::Context(Caller &caller)
    : Context(wasmtime_caller_context(caller.ptr)) {}
inline Store::Context::Context(Caller *caller) : Context(*caller) {}

#ifdef WASMTIME_FEATURE_GC
inline Trap Store::Context::throw_exception(ExnRef exn) {
  auto *ret = wasmtime_context_set_exception(capi(), exn.capi());
  wasmtime_exnref_set_null(exn.capi());
  return Trap(ret);
}

inline std::optional<ExnRef> Store::Context::take_exception() {
  wasmtime_exnref_t exn;
  if (wasmtime_context_take_exception(capi(), &exn)) {
    return ExnRef(exn);
  }
  return std::nullopt;
}

inline bool Store::Context::has_exception() {
  return wasmtime_context_has_exception(capi());
}
#endif

} // namespace wasmtime

#endif // WASMTIME_STORE_HH
