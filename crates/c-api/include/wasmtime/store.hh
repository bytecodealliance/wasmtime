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

inline Store::Context::Context(Caller &caller)
    : Context(wasmtime_caller_context(caller.ptr)) {}
inline Store::Context::Context(Caller *caller) : Context(*caller) {}

#ifdef WASMTIME_FEATURE_GC
inline Trap Store::Context::throw_exception(ExnRef exn) {
  return Trap(wasmtime_context_set_exception(capi(), exn.capi()));
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
