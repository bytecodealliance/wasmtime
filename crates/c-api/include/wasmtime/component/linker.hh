/// \file wasmtime/component/linker.hh

#ifndef WASMTIME_COMPONENT_LINKER_HH
#define WASMTIME_COMPONENT_LINKER_HH

#include <wasmtime/conf.h>

#ifdef WASMTIME_FEATURE_COMPONENT_MODEL

#include <memory>
#include <string_view>
#include <wasmtime/component/instance.hh>
#include <wasmtime/component/linker.h>
#include <wasmtime/component/val.hh>
#include <wasmtime/engine.hh>
#include <wasmtime/module.hh>

namespace wasmtime {
namespace component {

/**
 * \brief Helper class for linking modules together with name-based resolution.
 *
 * This class is used for easily instantiating `Module`s by defining names into
 * the linker and performing name-based resolution during instantiation. A
 * `Linker` can also be used to link in WASI functions to instantiate a module.
 */
class LinkerInstance {
  struct deleter {
    void operator()(wasmtime_component_linker_instance_t *p) const {
      wasmtime_component_linker_instance_delete(p);
    }
  };

  std::unique_ptr<wasmtime_component_linker_instance_t, deleter> ptr;

public:
  /// Creates a new linker instance from the given C API pointer.
  explicit LinkerInstance(wasmtime_component_linker_instance_t *ptr)
      : ptr(ptr) {}

  /// \brief Returns the underlying C API pointer.
  const wasmtime_component_linker_instance_t *capi() const { return ptr.get(); }

  /// \brief Returns the underlying C API pointer.
  wasmtime_component_linker_instance_t *capi() { return ptr.get(); }

  /**
   * \brief Adds a module to this linker instance under the specified name.
   */
  Result<std::monostate> add_module(std::string_view name, Module &module) {
    wasmtime_error_t *error = wasmtime_component_linker_instance_add_module(
        ptr.get(), name.data(), name.size(), module.capi());
    if (error != nullptr) {
      return Error(error);
    }
    return std::monostate();
  }

  /**
   * \brief Adds an new instance to this linker instance under the specified
   * name.
   *
   * Note that this `LinkerInstance` can no longer be used until the returned
   * `LinkerInstance` is dropped.
   */
  Result<LinkerInstance> add_instance(std::string_view name) {
    wasmtime_component_linker_instance_t *ret = nullptr;
    wasmtime_error_t *error = wasmtime_component_linker_instance_add_instance(
        ptr.get(), name.data(), name.size(), &ret);
    if (error != nullptr) {
      return Error(error);
    }
    return LinkerInstance(ret);
  }

private:
  template <typename F>
  static wasmtime_error_t *
  raw_callback(void *env, wasmtime_context_t *store,
               const wasmtime_component_val_t *args, size_t nargs,
               wasmtime_component_val_t *results, size_t nresults) {
    static_assert(alignof(Val) == alignof(wasmtime_component_val_t));
    static_assert(sizeof(Val) == sizeof(wasmtime_component_val_t));
    F *func = reinterpret_cast<F *>(env);
    Span<const Val> args_span(Val::from_capi(args), nargs);
    Span<Val> results_span(Val::from_capi(results), nresults);
    Result<std::monostate> result =
        (*func)(Store::Context(store), args_span, results_span);
    if (!result) {
      return result.err().release();
    }
    return nullptr;
  }

  template <typename F> static void raw_finalize(void *env) {
    std::unique_ptr<F> ptr(reinterpret_cast<F *>(env));
  }

public:
  /// \brief Defines a function within this linker instance.
  template <typename F,
            std::enable_if_t<
                std::is_invocable_r_v<Result<std::monostate>, F, Store::Context,
                                      Span<const Val>, Span<Val>>,
                bool> = true>
  Result<std::monostate> add_func(std::string_view name, F &&f) {
    auto *error = wasmtime_component_linker_instance_add_func(
        ptr.get(), name.data(), name.length(),
        raw_callback<std::remove_reference_t<F>>,
        std::make_unique<std::remove_reference_t<F>>(std::forward<F>(f))
            .release(),
        raw_finalize<std::remove_reference_t<F>>);

    if (error != nullptr) {
      return Error(error);
    }

    return std::monostate();
  }
};

/**
 * \brief Class used to instantiate a `Component` into an instance.
 */
class Linker {
  struct deleter {
    void operator()(wasmtime_component_linker_t *p) const {
      wasmtime_component_linker_delete(p);
    }
  };

  std::unique_ptr<wasmtime_component_linker_t, deleter> ptr;

public:
  /// Creates a new linker which will instantiate in the given engine.
  explicit Linker(Engine &engine)
      : ptr(wasmtime_component_linker_new(engine.capi())) {}

  /**
   * \brief Gets the "root" instance of this linker which can be used to define
   * items into the linker under the top-level namespace.
   *
   * This `Linker` cannot be used while the returned `LinkerInstance` is in
   * scope. To use more methods on this `Linker` it's required that the instance
   * returned here is dropped first.
   */
  LinkerInstance root() {
    wasmtime_component_linker_instance_t *instance_capi =
        wasmtime_component_linker_root(ptr.get());
    return LinkerInstance(instance_capi);
  }

  /// Configures whether shadowing previous names is allowed or not.
  ///
  /// By default shadowing is not allowed.
  void allow_shadowing(bool allow) {
    wasmtime_component_linker_allow_shadowing(ptr.get(), allow);
  }

  /// \brief Instantiates the given component within this linker.
  Result<Instance> instantiate(Store::Context cx, Component &component) {
    wasmtime_component_instance_t ret;
    wasmtime_error_t *error = wasmtime_component_linker_instantiate(
        ptr.get(), cx.capi(), component.capi(), &ret);
    if (error != nullptr) {
      return Error(error);
    }
    return Instance(ret);
  }

#ifdef WASMTIME_FEATURE_WASI
  /**
   * \brief Adds WASIp2 API definitions to this linker.
   *
   * This will use the WASIp2 API definitions in Wasmtime to this linker. Note
   * that this adds *synchronous* versions of WASIp2 definitions which will
   * block the caller when invoked. Internally this will use Wasmtime's
   * default async runtime implemented with Tokio to handle async I/O.
   */
  Result<std::monostate> add_wasip2() {
    wasmtime_error_t *error = wasmtime_component_linker_add_wasip2(ptr.get());
    if (error != nullptr) {
      return Error(error);
    }
    return std::monostate();
  }
#endif // WASMTIME_FEATURE_WASI

  /// \brief Returns the underlying C API pointer.
  const wasmtime_component_linker_t *capi() const { return ptr.get(); }

  /// \brief Returns the underlying C API pointer.
  wasmtime_component_linker_t *capi() { return ptr.get(); }
};

} // namespace component
} // namespace wasmtime

#endif // WASMTIME_FEATURE_COMPONENT_MODEL

#endif // WASMTIME_COMPONENT_LINKER_H
