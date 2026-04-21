/**
 * \file wasmtime/func.hh
 */

#ifndef WASMTIME_FUNC_HH
#define WASMTIME_FUNC_HH

#include <wasmtime/_func_class.hh>
#include <wasmtime/_val_class.hh>

namespace wasmtime {

namespace detail {

/// Helper macro to define `WasmType` definitions for primitive types like
/// int32_t and such.
// NOLINTNEXTLINE
#define NATIVE_WASM_TYPE(native, valkind, field)                               \
  template <> struct WasmType<native> {                                        \
    static const bool valid = true;                                            \
    static const ValKind kind = ValKind::valkind;                              \
    static void store(Store::Context cx, wasmtime_val_raw_t *p,                \
                      const native &t) {                                       \
      (void)cx;                                                                \
      p->field = t;                                                            \
    }                                                                          \
    static native load(Store::Context cx, wasmtime_val_raw_t *p) {             \
      (void)cx;                                                                \
      return p->field;                                                         \
    }                                                                          \
  };

NATIVE_WASM_TYPE(int32_t, I32, i32)
NATIVE_WASM_TYPE(uint32_t, I32, i32)
NATIVE_WASM_TYPE(int64_t, I64, i64)
NATIVE_WASM_TYPE(uint64_t, I64, i64)
NATIVE_WASM_TYPE(float, F32, f32)
NATIVE_WASM_TYPE(double, F64, f64)

#undef NATIVE_WASM_TYPE

/// std::monostate translates to an empty list of types.
template <> struct WasmTypeList<std::monostate> {
  static const bool valid = true;
  static const size_t size = 0;
  static bool matches(ValType::ListRef types) { return types.size() == 0; }
  static void store(Store::Context cx, wasmtime_val_raw_t *storage,
                    const std::monostate &t) {
    (void)cx;
    (void)storage;
    (void)t;
  }
  static std::monostate load(Store::Context cx, wasmtime_val_raw_t *storage) {
    (void)cx;
    (void)storage;
    return std::monostate();
  }
  static std::vector<ValType> types() { return {}; }
};

/// std::tuple<> translates to the corresponding list of types
template <typename... T> struct WasmTypeList<std::tuple<T...>> {
  static const bool valid = (WasmType<T>::valid && ...);
  static const size_t size = sizeof...(T);
  static bool matches(ValType::ListRef types) {
    if (types.size() != size) {
      return false;
    }
    size_t n = 0;
    return ((WasmType<T>::kind == types.begin()[n++].kind()) && ...);
  }
  static void store(Store::Context cx, wasmtime_val_raw_t *storage,
                    std::tuple<T...> &&t) {
    size_t n = 0;
    std::apply(
        [&](auto &...val) {
          (WasmType<T>::store(cx, &storage[n++], val), ...); // NOLINT
        },
        t);
  }
  static void store(Store::Context cx, wasmtime_val_raw_t *storage,
                    const std::tuple<T...> &t) {
    size_t n = 0;
    std::apply(
        [&](const auto &...val) {
          (WasmType<T>::store(cx, &storage[n++], val), ...); // NOLINT
        },
        t);
  }
  static std::tuple<T...> load(Store::Context cx, wasmtime_val_raw_t *storage) {
    (void)cx;
    return std::tuple<T...>{WasmType<T>::load(cx, storage++)...}; // NOLINT
  }
  static std::vector<ValType> types() { return {WasmType<T>::kind...}; }
};

/// Host functions can return nothing
template <> struct WasmHostRet<void> {
  using Results = WasmTypeList<std::tuple<>>;

  template <typename F, typename... A>
  static std::optional<Trap> invoke(F f, Caller cx, wasmtime_val_raw_t *raw,
                                    A... args) {
    (void)cx;
    (void)raw;
    f(args...);
    return std::nullopt;
  }
};

// Alternative method of returning "nothing" (also enables `std::monostate` in
// the `R` type of `Result` below)
template <> struct WasmHostRet<std::monostate> : public WasmHostRet<void> {};

/// Host functions can return a result which allows them to also possibly return
/// a trap.
template <typename R> struct WasmHostRet<Result<R, Trap>> {
  using Results = WasmTypeList<R>;

  template <typename F, typename... A>
  static std::optional<Trap> invoke(F f, Caller cx, wasmtime_val_raw_t *raw,
                                    A... args) {
    Result<R, Trap> ret = f(args...);
    if (!ret) {
      return ret.err();
    }
    Results::store(cx, raw, ret.ok());
    return std::nullopt;
  }
};

/// Base type information for host free-function pointers being used as wasm
/// functions
template <typename R, typename... A> struct WasmHostFunc<R (*)(A...)> {
  using Params = WasmTypeList<std::tuple<A...>>;
  using Results = typename WasmHostRet<R>::Results;

  template <typename F>
  static std::optional<Trap> invoke(F &f, Caller cx, wasmtime_val_raw_t *raw) {
    auto params = Params::load(cx, raw);
    return std::apply(
        [&](const auto &...val) {
          return WasmHostRet<R>::invoke(f, cx, raw, val...);
        },
        params);
  }
};

/// Function type information, but with a `Caller` first parameter
template <typename R, typename... A>
struct WasmHostFunc<R (*)(Caller, A...)> : public WasmHostFunc<R (*)(A...)> {
  // Override `invoke` here to pass the `cx` as the first parameter
  template <typename F>
  static std::optional<Trap> invoke(F &f, Caller cx, wasmtime_val_raw_t *raw) {
    auto params = WasmTypeList<std::tuple<A...>>::load(cx, raw);
    return std::apply(
        [&](const auto &...val) {
          return WasmHostRet<R>::invoke(f, cx, raw, cx, val...);
        },
        params);
  }
};

/// Function type information, but with a class method.
template <typename R, typename C, typename... A>
struct WasmHostFunc<R (C::*)(A...)> : public WasmHostFunc<R (*)(A...)> {};

/// Function type information, but with a const class method.
template <typename R, typename C, typename... A>
struct WasmHostFunc<R (C::*)(A...) const> : public WasmHostFunc<R (*)(A...)> {};

/// Function type information, but as a host method with a `Caller` first
/// parameter.
template <typename R, typename C, typename... A>
struct WasmHostFunc<R (C::*)(Caller, A...)>
    : public WasmHostFunc<R (*)(Caller, A...)> {};

/// Function type information, but as a host const method with a `Caller`
/// first parameter.
template <typename R, typename C, typename... A>
struct WasmHostFunc<R (C::*)(Caller, A...) const>
    : public WasmHostFunc<R (*)(Caller, A...)> {};

/// Base type information for host callables being used as wasm
/// functions
template <typename T>
struct WasmHostFunc<T, std::void_t<decltype(&T::operator())>>
    : public WasmHostFunc<decltype(&T::operator())> {};

} // namespace detail

using namespace detail;

template <typename F>
inline wasm_trap_t *Func::raw_callback(void *env, wasmtime_caller_t *caller,
                                       const wasmtime_val_t *args, size_t nargs,
                                       wasmtime_val_t *results,
                                       size_t nresults) {
  static_assert(alignof(Val) == alignof(wasmtime_val_t));
  static_assert(sizeof(Val) == sizeof(wasmtime_val_t));
  F *func = reinterpret_cast<F *>(env);                          // NOLINT
  Span<const Val> args_span(reinterpret_cast<const Val *>(args), // NOLINT
                            nargs);
  Span<Val> results_span(reinterpret_cast<Val *>(results), // NOLINT
                         nresults);
  Result<std::monostate, Trap> result =
      (*func)(Caller(caller), args_span, results_span);
  if (!result) {
    return result.err().capi_release();
  }
  return nullptr;
}

template <typename I>
inline TrapResult<std::vector<Val>>
Func::call(Store::Context cx, const I &begin, const I &end) const {
  std::vector<wasmtime_val_t> raw_params;
  raw_params.reserve(end - begin);
  for (auto i = begin; i != end; i++) {
    raw_params.push_back(i->val);
  }
  size_t nresults = this->type(cx)->results().size();
  std::vector<wasmtime_val_t> raw_results(nresults);

  wasm_trap_t *trap = nullptr;
  auto *error =
      wasmtime_func_call(cx.ptr, &func, raw_params.data(), raw_params.size(),
                         raw_results.data(), raw_results.capacity(), &trap);
  if (error != nullptr) {
    return TrapError(Error(error));
  }
  if (trap != nullptr) {
    return TrapError(Trap(trap));
  }

  std::vector<Val> results;
  results.reserve(nresults);
  for (size_t i = 0; i < nresults; i++) {
    results.push_back(raw_results[i]);
  }
  return results;
}

inline TrapResult<std::vector<Val>>
Func::call(Store::Context cx, const std::initializer_list<Val> &params) const {
  return this->call(cx, params.begin(), params.end());
}

inline TrapResult<std::vector<Val>>
Func::call(Store::Context cx, const std::vector<Val> &params) const {
  return this->call(cx, params.begin(), params.end());
}

} // namespace wasmtime

#endif // WASMTIME_FUNC_HH
