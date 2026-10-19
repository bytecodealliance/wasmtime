/// \file wasmtime/component/instance.hh

#ifndef WASMTIME_COMPONENT_INSTANCE_HH
#define WASMTIME_COMPONENT_INSTANCE_HH

#include <wasmtime/conf.h>

#ifdef WASMTIME_FEATURE_COMPONENT_MODEL

#include <string_view>
#include <utility>
#include <wasmtime/component/component.hh>
#include <wasmtime/component/func.hh>
#include <wasmtime/component/instance.h>
#include <wasmtime/store.hh>

namespace wasmtime {
namespace component {

/**
 * \brief Class representing an instantiated WebAssembly component.
 */
class Instance {
  wasmtime_component_instance_t instance;

public:
  /// \brief Constructs an Instance from the underlying C API struct.
  explicit Instance(const wasmtime_component_instance_t &inst)
      : instance(inst) {}

  /// \brief Looks up an exported item from this instance by name, returning the
  /// index at which it can be found.
  ///
  /// The returned `ExportIndex` references the underlying item within this
  /// instance which can then be accessed via that index specifically. The
  /// `instance` provided as an argument to this function is the containing
  /// export instance, if any, that `name` is looked up under.
  std::optional<ExportIndex> get_export_index(Store::Context cx,
                                              const ExportIndex *instance,
                                              std::string_view name) const {
    wasmtime_component_export_index_t *ret =
        wasmtime_component_instance_get_export_index(
            &this->instance, cx.capi(), instance ? instance->capi() : nullptr,
            name.data(), name.size());
    if (ret == nullptr) {
      return std::nullopt;
    }
    return ExportIndex(ret);
  }

  /// \brief Looks up an exported item from this instance by name, returning a
  /// description of it along with the index at which it can be found.
  ///
  /// This is similar to `get_export_index` except that the returned
  /// `ComponentExtern` additionally describes the export's type and any
  /// metadata attached to its name, such as `(implements "...")` or
  /// `(external-id "...")` annotations. The `ComponentExtern` borrows from the
  /// store and must not outlive it.
  std::optional<std::pair<ComponentExtern, ExportIndex>>
  get_export_extern(Store::Context cx, const ExportIndex *instance,
                    std::string_view name) const {
    wasmtime_component_extern_t *e = nullptr;
    wasmtime_component_export_index_t *ret =
        wasmtime_component_instance_get_export_extern(
            &this->instance, cx.capi(), instance ? instance->capi() : nullptr,
            name.data(), name.size(), &e);
    if (ret == nullptr) {
      return std::nullopt;
    }
    return std::make_pair(ComponentExtern(e), ExportIndex(ret));
  }

  /// \brief Looks up an exported function by its export index.
  std::optional<Func> get_func(Store::Context cx,
                               const ExportIndex &index) const {
    wasmtime_component_func_t ret;
    bool found = wasmtime_component_instance_get_func(&instance, cx.capi(),
                                                      index.capi(), &ret);
    if (!found)
      return std::nullopt;
    return Func(ret);
  }

  /// \brief Returns the underlying C API pointer.
  const wasmtime_component_instance_t *capi() const { return &instance; }
};

} // namespace component
} // namespace wasmtime

#endif // WASMTIME_FEATURE_COMPONENT_MODEL

#endif // WASMTIME_COMPONENT_INSTANCE_HH
