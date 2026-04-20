/**
 * \file wasmtime/val.hh
 */

#ifndef WASMTIME_VAL_HH
#define WASMTIME_VAL_HH

#include <wasmtime/_anyref_class.hh>
#include <wasmtime/_externref_class.hh>
#include <wasmtime/_func_class.hh>
#include <wasmtime/_val_class.hh>

namespace wasmtime {

inline Val::Val(std::optional<Func> func) : val{} {
  val.kind = WASMTIME_FUNCREF;
  if (func) {
    val.of.funcref = (*func).func;
  } else {
    wasmtime_funcref_set_null(&val.of.funcref);
  }
}

inline Val::Val(Func func) : Val(std::optional(func)) {}

inline std::optional<Func> Val::funcref() const {
  if (val.kind != WASMTIME_FUNCREF) {
    std::abort();
  }
  if (val.of.funcref.store_id == 0) {
    return std::nullopt;
  }
  return Func(val.of.funcref);
}

#ifdef WASMTIME_FEATURE_GC

inline Val::Val(std::optional<AnyRef> ptr) : val{} {
  val.kind = WASMTIME_ANYREF;
  if (ptr) {
    val.of.anyref = *ptr->capi();
    wasmtime_anyref_set_null(ptr->capi());
  } else {
    wasmtime_anyref_set_null(&val.of.anyref);
  }
}

inline Val::Val(AnyRef ptr) : Val(std::optional(ptr)) {}

inline std::optional<AnyRef> Val::anyref() const {
  if (val.kind != WASMTIME_ANYREF) {
    std::abort();
  }
  if (wasmtime_anyref_is_null(&val.of.anyref)) {
    return std::nullopt;
  }
  wasmtime_anyref_t other;
  wasmtime_anyref_clone(&val.of.anyref, &other);
  return AnyRef(other);
}

inline Val::Val(std::optional<ExternRef> ptr) : val{} {
  val.kind = WASMTIME_EXTERNREF;
  if (ptr) {
    val.of.externref = *ptr->capi();
    wasmtime_externref_set_null(ptr->capi());
  } else {
    wasmtime_externref_set_null(&val.of.externref);
  }
}

inline Val::Val(ExternRef ptr) : Val(std::optional(ptr)) {}

inline std::optional<ExternRef> Val::externref() const {
  if (val.kind != WASMTIME_EXTERNREF) {
    std::abort();
  }
  if (wasmtime_externref_is_null(&val.of.externref)) {
    return std::nullopt;
  }
  wasmtime_externref_t other;
  wasmtime_externref_clone(&val.of.externref, &other);
  return ExternRef(other);
}

#endif // WASMTIME_FEATURE_GC

/// Type information for the `V128` host value used as a wasm value.
template <> struct detail::WasmType<V128> {
  static const bool valid = true;
  static const ValKind kind = ValKind::V128;
  static void store(Store::Context cx, wasmtime_val_raw_t *p, const V128 &t) {
    (void)cx;
    memcpy(&p->v128[0], &t.v128[0], sizeof(wasmtime_v128));
  }
  static V128 load(Store::Context cx, wasmtime_val_raw_t *p) {
    (void)cx;
    return p->v128;
  }
};

} // namespace wasmtime

#endif // WASMTIME_VAL_HH
