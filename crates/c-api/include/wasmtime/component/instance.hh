/// \file wasmtime/component/instance.hh

#ifndef WASMTIME_COMPONENT_INSTANCE_HH
#define WASMTIME_COMPONENT_INSTANCE_HH

#include <wasmtime/conf.h>

#ifdef WASMTIME_FEATURE_COMPONENT_MODEL

#include <string_view>
#include <wasmtime/component/component.hh>
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
                                              ExportIndex *instance,
                                              std::string_view name) {
    wasmtime_component_export_index_t *ret =
        wasmtime_component_instance_get_export_index(
            &this->instance, cx.capi(), instance ? instance->capi() : nullptr,
            name.data(), name.size());
    if (ret == nullptr) {
      return std::nullopt;
    }
    return ExportIndex(ret);
  }

  // TODO: get_func via `wasmtime_component_instance_get_func`

  /// \brief Returns the underlying C API pointer.
  const wasmtime_component_instance_t *capi() const { return &instance; }

  /// \brief Returns the underlying C API pointer.
  wasmtime_component_instance_t *capi() { return &instance; }
};

} // namespace component
} // namespace wasmtime

#endif // WASMTIME_FEATURE_COMPONENT_MODEL

#endif // WASMTIME_COMPONENT_INSTANCE_H
