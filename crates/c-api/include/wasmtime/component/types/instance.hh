/**
 * \file wasmtime/component/types/instance.hh
 */

#ifndef WASMTIME_COMPONENT_TYPES_INSTANCE_HH
#define WASMTIME_COMPONENT_TYPES_INSTANCE_HH

#include <wasmtime/conf.h>

#ifdef WASMTIME_FEATURE_COMPONENT_MODEL

#include <memory>
#include <optional>
#include <string>
#include <wasmtime/component/types/instance.h>
#include <wasmtime/engine.hh>
#include <wasmtime/helpers.hh>

namespace wasmtime {
namespace component {

class ComponentExtern;

/**
 * \brief Represents the type of a component instance.
 */
class ComponentInstanceType {
  WASMTIME_CLONE_WRAPPER(ComponentInstanceType,
                         wasmtime_component_instance_type);

  /// Returns the number of exports of this component instance type.
  size_t export_count(const Engine &engine) const {
    return wasmtime_component_instance_type_export_count(capi(), engine.capi());
  }

  /// Retrieves the export with the specified name.
  ///
  /// The returned `ComponentExtern` borrows from this type and the engine and
  /// must not outlive either of them.
  std::optional<ComponentExtern> export_get(const Engine &engine,
                                            std::string_view name) const;

  /// Retrieves the nth export.
  ///
  /// The returned `ComponentExtern` borrows from this type and the engine and
  /// must not outlive either of them.
  std::optional<std::pair<std::string_view, ComponentExtern>>
  export_nth(const Engine &engine, size_t nth) const;
};

} // namespace component
} // namespace wasmtime

#include <wasmtime/component/types/component.hh>

inline std::optional<wasmtime::component::ComponentExtern>
wasmtime::component::ComponentInstanceType::export_get(
    const wasmtime::Engine &engine, std::string_view name) const {
  wasmtime_component_extern_t *e;
  bool found = wasmtime_component_instance_type_export_get(
      capi(), engine.capi(), name.data(), name.size(), &e);
  if (!found) {
    return std::nullopt;
  }
  return wasmtime::component::ComponentExtern(e);
}

inline std::optional<
    std::pair<std::string_view, wasmtime::component::ComponentExtern>>
wasmtime::component::ComponentInstanceType::export_nth(
    const wasmtime::Engine &engine, size_t nth) const {
  wasmtime_component_extern_t *e;
  const char *name_data;
  size_t name_size;
  bool found = wasmtime_component_instance_type_export_nth(
      capi(), engine.capi(), nth, &name_data, &name_size, &e);
  if (!found) {
    return std::nullopt;
  }
  return std::make_pair(std::string_view(name_data, name_size),
                        wasmtime::component::ComponentExtern(e));
}

#endif // WASMTIME_FEATURE_COMPONENT_MODEL

#endif // WASMTIME_COMPONENT_TYPES_INSTANCE_HH
