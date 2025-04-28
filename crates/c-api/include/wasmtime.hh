/**
 * This project is a C++ API for
 * [Wasmtime](https://github.com/bytecodealliance/wasmtime). Support for the
 * C++ API is exclusively built on the [C API of
 * Wasmtime](https://docs.wasmtime.dev/c-api/), so the C++ support for this is
 * simply a single header file. To use this header file, though, it must be
 * combined with the header and binary of Wasmtime's C API. Note, though, that
 * while this header is built on top of the `wasmtime.h` header file you should
 * only need to use the contents of this header file to interact with Wasmtime.
 *
 * Examples can be [found
 * online](https://github.com/bytecodealliance/wasmtime/tree/main/examples)
 * and otherwise be sure to check out the
 * [README](https://github.com/bytecodealliance/wasmtime/blob/main/crates/c-api/README.md)
 * for simple usage instructions. Otherwise you can dive right in to the
 * reference documentation of \ref wasmtime.hh
 *
 * \example hello.cc
 * \example gcd.cc
 * \example linking.cc
 * \example memory.cc
 * \example interrupt.cc
 * \example externref.cc
 */

/**
 * \file wasmtime.hh
 */

#ifndef WASMTIME_HH
#define WASMTIME_HH

#include <any>
#include <array>
#include <cstdio>
#include <initializer_list>
#include <iosfwd>
#include <limits>
#include <memory>
#include <optional>
#include <ostream>
#include <variant>
#include <vector>

#include <wasmtime.h>
#include <wasmtime/config.hh>
#include <wasmtime/engine.hh>
#include <wasmtime/error.hh>
#include <wasmtime/extern.hh>
#include <wasmtime/func.hh>
#include <wasmtime/global.hh>
#include <wasmtime/memory.hh>
#include <wasmtime/module.hh>
#include <wasmtime/store.hh>
#include <wasmtime/table.hh>
#include <wasmtime/trap.hh>
#include <wasmtime/types.hh>
#include <wasmtime/val.hh>
#include <wasmtime/wasi.hh>

namespace wasmtime {

/**
 * \brief A WebAssembly instance.
 *
 * This class represents a WebAssembly instance, created by instantiating a
 * module. An instance is the collection of items exported by the module, which
 * can be accessed through the `Store` that owns the instance.
 *
 * Note that this type does not itself own any resources. It points to resources
 * owned within a `Store` and the `Store` must be passed in as the first
 * argument to the functions defined on `Instance`. Note that if the wrong
 * `Store` is passed in then the process will be aborted.
 */
class Instance {
  friend class Linker;
  friend class Caller;

  wasmtime_instance_t instance;

  static Extern cvt(wasmtime_extern_t &e) {
    switch (e.kind) {
    case WASMTIME_EXTERN_FUNC:
      return Func(e.of.func);
    case WASMTIME_EXTERN_GLOBAL:
      return Global(e.of.global);
    case WASMTIME_EXTERN_MEMORY:
      return Memory(e.of.memory);
    case WASMTIME_EXTERN_TABLE:
      return Table(e.of.table);
    }
    std::abort();
  }

  static void cvt(const Extern &e, wasmtime_extern_t &raw) {
    if (const auto *func = std::get_if<Func>(&e)) {
      raw.kind = WASMTIME_EXTERN_FUNC;
      raw.of.func = func->func;
    } else if (const auto *global = std::get_if<Global>(&e)) {
      raw.kind = WASMTIME_EXTERN_GLOBAL;
      raw.of.global = global->global;
    } else if (const auto *table = std::get_if<Table>(&e)) {
      raw.kind = WASMTIME_EXTERN_TABLE;
      raw.of.table = table->table;
    } else if (const auto *memory = std::get_if<Memory>(&e)) {
      raw.kind = WASMTIME_EXTERN_MEMORY;
      raw.of.memory = memory->memory;
    } else {
      std::abort();
    }
  }

public:
  /// Creates a new instance from the raw underlying C API representation.
  Instance(wasmtime_instance_t instance) : instance(instance) {}

  /**
   * \brief Instantiates the module `m` with the provided `imports`
   *
   * \param cx the store in which to instantiate the provided module
   * \param m the module to instantiate
   * \param imports the list of imports to use to instantiate the module
   *
   * This `imports` parameter is expected to line up 1:1 with the imports
   * required by the `m`. The type of `m` can be inspected to determine in which
   * order to provide the imports. Note that this is a relatively low-level API
   * and it's generally recommended to use `Linker` instead for name-based
   * instantiation.
   *
   * This function can return an error if any of the `imports` have the wrong
   * type, or if the wrong number of `imports` is provided.
   */
  static TrapResult<Instance> create(Store::Context cx, const Module &m,
                                     const std::vector<Extern> &imports) {
    std::vector<wasmtime_extern_t> raw_imports;
    for (const auto &item : imports) {
      raw_imports.push_back(wasmtime_extern_t{});
      auto &last = raw_imports.back();
      Instance::cvt(item, last);
    }
    wasmtime_instance_t instance;
    wasm_trap_t *trap = nullptr;
    auto *error = wasmtime_instance_new(cx.ptr, m.ptr.get(), raw_imports.data(),
                                        raw_imports.size(), &instance, &trap);
    if (error != nullptr) {
      return TrapError(Error(error));
    }
    if (trap != nullptr) {
      return TrapError(Trap(trap));
    }
    return Instance(instance);
  }

  /**
   * \brief Load an instance's export by name.
   *
   * This function will look for an export named `name` on this instance and, if
   * found, return it as an `Extern`.
   */
  std::optional<Extern> get(Store::Context cx, std::string_view name) {
    wasmtime_extern_t e;
    if (!wasmtime_instance_export_get(cx.ptr, &instance, name.data(),
                                      name.size(), &e)) {
      return std::nullopt;
    }
    return Instance::cvt(e);
  }

  /**
   * \brief Load an instance's export by index.
   *
   * This function will look for the `idx`th export of this instance. This will
   * return both the name of the export as well as the exported item itself.
   */
  std::optional<std::pair<std::string_view, Extern>> get(Store::Context cx,
                                                         size_t idx) {
    wasmtime_extern_t e;
    // I'm not sure why clang-tidy thinks this is using va_list or anything
    // related to that...
    // NOLINTNEXTLINE(cppcoreguidelines-pro-type-vararg)
    char *name = nullptr;
    size_t len = 0;
    if (!wasmtime_instance_export_nth(cx.ptr, &instance, idx, &name, &len,
                                      &e)) {
      return std::nullopt;
    }
    std::string_view n(name, len);
    return std::pair(n, Instance::cvt(e));
  }
};

inline std::optional<Extern> Caller::get_export(std::string_view name) {
  wasmtime_extern_t item;
  if (wasmtime_caller_export_get(ptr, name.data(), name.size(), &item)) {
    return Instance::cvt(item);
  }
  return std::nullopt;
}

/**
 * \brief Helper class for linking modules together with name-based resolution.
 *
 * This class is used for easily instantiating `Module`s by defining names into
 * the linker and performing name-based resolution during instantiation. A
 * `Linker` can also be used to link in WASI functions to instantiate a module.
 */
class Linker {
  struct deleter {
    void operator()(wasmtime_linker_t *p) const { wasmtime_linker_delete(p); }
  };

  std::unique_ptr<wasmtime_linker_t, deleter> ptr;

public:
  /// Creates a new linker which will instantiate in the given engine.
  explicit Linker(Engine &engine)
      : ptr(wasmtime_linker_new(engine.ptr.get())) {}

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
    Instance::cvt(item, raw);
    auto *error =
        wasmtime_linker_define(ptr.get(), cx.ptr, module.data(), module.size(),
                               name.data(), name.size(), &raw);
    if (error != nullptr) {
      return Error(error);
    }
    return std::monostate();
  }

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
    auto *error = wasmtime_linker_instantiate(ptr.get(), cx.ptr, m.ptr.get(),
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
                                         name.size(), m.ptr.get());
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
      return Instance::cvt(item);
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
                                  F&& f) {

    auto *error = wasmtime_linker_define_func(
        ptr.get(), module.data(), module.length(), name.data(), name.length(),
        ty.ptr.get(), Func::raw_callback<std::remove_reference_t<F>>, std::make_unique<std::remove_reference_t<F>>(std::forward<F>(f)).release(),
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
                                   std::string_view name, F&& f) {
    using HostFunc = WasmHostFunc<F>;
    auto params = HostFunc::Params::types();
    auto results = HostFunc::Results::types();
    auto ty = FuncType::from_iters(params, results);
    auto *error = wasmtime_linker_define_func_unchecked(
        ptr.get(), module.data(), module.length(), name.data(), name.length(),
        ty.ptr.get(), Func::raw_callback_unchecked<std::remove_reference_t<F>>,
        std::make_unique<std::remove_reference_t<F>>(std::forward<F>(f)).release(), Func::raw_finalize<std::remove_reference_t<F>>);

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
};

} // namespace wasmtime

#endif // WASMTIME_HH
