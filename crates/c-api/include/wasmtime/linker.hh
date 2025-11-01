/**
 * \file wasmtime/linker.hh
 */

#ifndef WASMTIME_LINKER_HH
#define WASMTIME_LINKER_HH

#include <wasmtime/engine.hh>
#include <wasmtime/error.hh>
#include <wasmtime/extern.hh>
#include <wasmtime/helpers.hh>
#include <wasmtime/instance.hh>
#include <wasmtime/linker.h>
#include <wasmtime/store.hh>
#include <wasmtime/trap.hh>

namespace wasmtime {

/**
 * \brief Helper class for linking modules together with name-based resolution.
 *
 * This class is used for easily instantiating `Module`s by defining names into
 * the linker and performing name-based resolution during instantiation. A
 * `Linker` can also be used to link in WASI functions to instantiate a module.
 */
class Linker {
  WASMTIME_OWN_WRAPPER(Linker, wasmtime_linker);

  /// Creates a new linker which will instantiate in the given engine.
  explicit Linker(Engine &engine) : ptr(wasmtime_linker_new(engine.capi())) {}

  /// Configures whether shadowing previous names is allowed or not.
  ///
  /// By default shadowing is not allowed.
  void allow_shadowing(bool allow) {
    wasmtime_linker_allow_shadowing(ptr.get(), allow);
  }

  /// Defines the provided item into this linker with the given name.
  Result<std::monostate> define(Store::Context cx, std::string_view module,
                                std::string_view name, const Extern &item) {
    wasmtime_extern_t raw;
    detail::cvt_extern(item, raw);
    auto *error =
        wasmtime_linker_define(ptr.get(), cx.ptr, module.data(), module.size(),
                               name.data(), name.size(), &raw);
    if (error != nullptr) {
      return Error(error);
    }
    return std::monostate();
  }

#ifdef WASMTIME_FEATURE_WASI
  /// Defines WASI functions within this linker.
  ///
  /// Note that `Store::Context::set_wasi` must also be used for instantiated
  /// modules to have access to configured WASI state.
  Result<std::monostate> define_wasi() {
    auto *error = wasmtime_linker_define_wasi(ptr.get());
    if (error != nullptr) {
      return Error(error);
    }
    return std::monostate();
  }
#endif // WASMTIME_FEATURE_WASI

  /// Defines all exports of the `instance` provided in this linker with the
  /// given module name of `name`.
  Result<std::monostate>
  define_instance(Store::Context cx, std::string_view name, Instance instance) {
    auto *error = wasmtime_linker_define_instance(
        ptr.get(), cx.ptr, name.data(), name.size(), &instance.instance);
    if (error != nullptr) {
      return Error(error);
    }
    return std::monostate();
  }

  /// Instantiates the module `m` provided within the store `cx` using the items
  /// defined within this linker.
  TrapResult<Instance> instantiate(Store::Context cx, const Module &m) {
    wasmtime_instance_t instance;
    wasm_trap_t *trap = nullptr;
    auto *error = wasmtime_linker_instantiate(ptr.get(), cx.ptr, m.capi(),
                                              &instance, &trap);
    if (error != nullptr) {
      return TrapError(Error(error));
    }
    if (trap != nullptr) {
      return TrapError(Trap(trap));
    }
    return Instance(instance);
  }

  /// Defines instantiations of the module `m` within this linker under the
  /// given `name`.
  Result<std::monostate> module(Store::Context cx, std::string_view name,
                                const Module &m) {
    auto *error = wasmtime_linker_module(ptr.get(), cx.ptr, name.data(),
                                         name.size(), m.capi());
    if (error != nullptr) {
      return Error(error);
    }
    return std::monostate();
  }

  /// Attempts to load the specified named item from this linker, returning
  /// `std::nullopt` if it was not defined.
  [[nodiscard]] std::optional<Extern>
  get(Store::Context cx, std::string_view module, std::string_view name) {
    wasmtime_extern_t item;
    if (wasmtime_linker_get(ptr.get(), cx.ptr, module.data(), module.size(),
                            name.data(), name.size(), &item)) {
      return detail::cvt_extern(item);
    }
    return std::nullopt;
  }

  /// Defines a new function in this linker in the style of the `Func`
  /// constructor.
  template <typename F,
            std::enable_if_t<
                std::is_invocable_r_v<Result<std::monostate, Trap>, F, Caller,
                                      Span<const Val>, Span<Val>>,
                bool> = true>
  Result<std::monostate> func_new(std::string_view module,
                                  std::string_view name, const FuncType &ty,
                                  F &&f) {

    auto *error = wasmtime_linker_define_func(
        ptr.get(), module.data(), module.length(), name.data(), name.length(),
        ty.ptr.get(), Func::raw_callback<std::remove_reference_t<F>>,
        std::make_unique<std::remove_reference_t<F>>(std::forward<F>(f))
            .release(),
        Func::raw_finalize<std::remove_reference_t<F>>);

    if (error != nullptr) {
      return Error(error);
    }

    return std::monostate();
  }

  /// Defines a new function in this linker in the style of the `Func::wrap`
  /// constructor.
  template <typename F,
            std::enable_if_t<WasmHostFunc<F>::Params::valid, bool> = true,
            std::enable_if_t<WasmHostFunc<F>::Results::valid, bool> = true>
  Result<std::monostate> func_wrap(std::string_view module,
                                   std::string_view name, F &&f) {
    using HostFunc = WasmHostFunc<F>;
    auto params = HostFunc::Params::types();
    auto results = HostFunc::Results::types();
    auto ty = FuncType::from_iters(params, results);
    auto *error = wasmtime_linker_define_func_unchecked(
        ptr.get(), module.data(), module.length(), name.data(), name.length(),
        ty.ptr.get(), Func::raw_callback_unchecked<std::remove_reference_t<F>>,
        std::make_unique<std::remove_reference_t<F>>(std::forward<F>(f))
            .release(),
        Func::raw_finalize<std::remove_reference_t<F>>);

    if (error != nullptr) {
      return Error(error);
    }

    return std::monostate();
  }

  /// Loads the "default" function, according to WASI commands and reactors, of
  /// the module named `name` in this linker.
  Result<Func> get_default(Store::Context cx, std::string_view name) {
    wasmtime_func_t item;
    auto *error = wasmtime_linker_get_default(ptr.get(), cx.ptr, name.data(),
                                              name.size(), &item);
    if (error != nullptr) {
      return Error(error);
    }
    return Func(item);
  }

  /// \brief Defines any import of `module` previously unknown to this linker
  /// as a trap.
  Result<std::monostate> define_unknown_imports_as_traps(Module &module) {
    auto *error = wasmtime_linker_define_unknown_imports_as_traps(
        ptr.get(), module.capi());
    if (error != nullptr) {
      return Error(error);
    }
    return std::monostate();
  }

  /// \brief Defines any import of `module` previously unknown to this linker
  /// as the "default" value for that import, for example a function that
  /// returns zeros.
  Result<std::monostate>
  define_unknown_imports_as_default_values(Store::Context cx, Module &module) {
    auto *error = wasmtime_linker_define_unknown_imports_as_default_values(
        ptr.get(), cx.ptr, module.capi());
    if (error != nullptr) {
      return Error(error);
    }
    return std::monostate();
  }
};

} // namespace wasmtime

#endif // WASMTIME_LINKER_HH
