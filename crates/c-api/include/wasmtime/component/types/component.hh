/**
 * \file wasmtime/component/types/component.hh
 */

#ifndef WASMTIME_COMPONENT_TYPES_COMPONENT_HH
#define WASMTIME_COMPONENT_TYPES_COMPONENT_HH

#include <wasmtime/conf.h>

#ifdef WASMTIME_FEATURE_COMPONENT_MODEL

#include <memory>
#include <optional>
#include <string>
#include <wasmtime/component/types/component.h>
#include <wasmtime/engine.hh>
#include <wasmtime/helpers.hh>
#include <wasmtime/types/func.hh>

namespace wasmtime {

namespace component {

class ComponentItem;
class ComponentExtern;

/**
 * \brief Represents the type of a WebAssembly component.
 */
class ComponentType {
  WASMTIME_CLONE_WRAPPER(ComponentType, wasmtime_component_type);

  /// Returns the number of imports of this component type.
  size_t import_count(const Engine &engine) const {
    return wasmtime_component_type_import_count(capi(), engine.capi());
  }

  /// Retrieves the import with the specified name.
  ///
  /// The returned `ComponentExtern` borrows from this type and the engine and
  /// must not outlive either of them.
  std::optional<ComponentExtern> import_get(const Engine &engine,
                                            std::string_view name) const;

  /// Retrieves the nth import.
  ///
  /// The returned `ComponentExtern` borrows from this type and the engine and
  /// must not outlive either of them.
  std::optional<std::pair<std::string_view, ComponentExtern>>
  import_nth(const Engine &engine, size_t nth) const;

  /// Returns the number of exports of this component type.
  size_t export_count(const Engine &engine) const {
    return wasmtime_component_type_export_count(capi(), engine.capi());
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

class ComponentInstanceType;
class ModuleType;
class FuncType;
class ResourceType;
class ValType;

/**
 * \brief Represents a single item in a component's import or export list.
 */
class ComponentItem {
  wasmtime_component_item_t item;

public:
  /// Creates a component item from the raw C API representation.
  explicit ComponentItem(wasmtime_component_item_t &&item) : item(item) {
    item.kind = WASMTIME_COMPONENT_ITEM_TYPE;
    item.of.type.kind = WASMTIME_COMPONENT_VALTYPE_BOOL;
  }

  /// Copies another item into this one.
  ComponentItem(const ComponentItem &other) {
    wasmtime_component_item_clone(&other.item, &item);
  }

  /// Copies another item into this one.
  ComponentItem &operator=(const ComponentItem &other) {
    wasmtime_component_item_delete(&item);
    wasmtime_component_item_clone(&other.item, &item);
    return *this;
  }

  /// Moves another item into this one.
  ComponentItem(ComponentItem &&other) : item(other.item) {
    other.item.kind = WASMTIME_COMPONENT_ITEM_TYPE;
    other.item.of.type.kind = WASMTIME_COMPONENT_VALTYPE_BOOL;
  }

  /// Moves another item into this one.
  ComponentItem &operator=(ComponentItem &&other) {
    wasmtime_component_item_delete(&item);
    item = other.item;
    other.item.kind = WASMTIME_COMPONENT_ITEM_TYPE;
    other.item.of.type.kind = WASMTIME_COMPONENT_VALTYPE_BOOL;
    return *this;
  }

  ~ComponentItem() { wasmtime_component_item_delete(&item); }

  /// Returns true if this is a component.
  bool is_component() const {
    return item.kind == WASMTIME_COMPONENT_ITEM_COMPONENT;
  }

  /// Returns true if this is a component instance.
  bool is_component_instance() const {
    return item.kind == WASMTIME_COMPONENT_ITEM_COMPONENT_INSTANCE;
  }

  /// Returns true if this is a module.
  bool is_module() const { return item.kind == WASMTIME_COMPONENT_ITEM_MODULE; }

  /// Returns true if this is a component function.
  bool is_component_func() const {
    return item.kind == WASMTIME_COMPONENT_ITEM_COMPONENT_FUNC;
  }

  /// Returns true if this is a resource.
  bool is_resource() const {
    return item.kind == WASMTIME_COMPONENT_ITEM_RESOURCE;
  }

  /// Returns true if this is a core function.
  bool is_core_func() const {
    return item.kind == WASMTIME_COMPONENT_ITEM_CORE_FUNC;
  }

  /// Returns true if this is a type.
  bool is_type() const { return item.kind == WASMTIME_COMPONENT_ITEM_TYPE; }

  /// Returns the component type this item represents, asserting that this is
  /// indeed a component.
  const ComponentType &component() const {
    assert(is_component());
    return *ComponentType::from_capi(&item.of.component);
  }

  /// Returns the component instance type this item represents, asserting that
  /// this is indeed a component instance.
  const ComponentInstanceType &component_instance() const;

  /// Returns the module type this item represents, asserting that this is
  /// indeed a module.
  const ModuleType &module() const;

  /// Returns the component function type this item represents, asserting that
  /// this is indeed a component function.
  const FuncType &component_func() const;

  /// Returns the resource type this item represents, asserting that this is
  /// indeed a resource.
  const ResourceType &resource() const;

  /// Returns the core function type this item represents, asserting that this
  /// is indeed a core function.
  wasmtime::FuncType::Ref core_func() const {
    assert(is_core_func());
    return item.of.core_func;
  }

  /// Returns the type this item represents, asserting that this is
  /// indeed a type.
  const ValType &type() const;
};

/**
 * \brief Full description of a single import or export of a component or a
 * component instance.
 *
 * This carries the type of the item being imported or exported along with any
 * `(implements "...")` or `(external-id "...")` annotations attached to it.
 *
 * Note that this borrows string data from the `ComponentType` or
 * `ComponentInstanceType` it was acquired from (and the `Engine` used to
 * acquire it), so it must not outlive either of those. Copies of this class
 * share the same borrow and are subject to the same restriction.
 */
class ComponentExtern {
  WASMTIME_CLONE_WRAPPER(ComponentExtern, wasmtime_component_extern);

  /// Returns the type of the item that this import/export refers to.
  ComponentItem type() const;

  /// Returns the `(implements "...")` annotation of this import/export, if
  /// present.
  std::optional<std::string_view> implements() const {
    size_t len = 0;
    const char *ptr = wasmtime_component_extern_implements(capi(), &len);
    if (ptr == nullptr) {
      return std::nullopt;
    }
    return std::string_view(ptr, len);
  }

  /// Returns whether this import/export has an `(implements "...")`
  /// annotation which is compatible with `name`.
  ///
  /// This returns `false` if there is no `implements` annotation. Otherwise
  /// this returns `true` if the annotation is exactly `name` or a
  /// semver-compatible version of it, for example `a:b/c@1.1.0` matches
  /// `a:b/c@1.0.0` and `a:b/c@1.2.0`.
  bool is_implements(std::string_view name) const {
    return wasmtime_component_extern_is_implements(capi(), name.data(),
                                                   name.size());
  }

  /// Returns the `(external-id "...")` annotation of this import/export, if
  /// present.
  std::optional<std::string_view> external_id() const {
    size_t len = 0;
    const char *ptr = wasmtime_component_extern_external_id(capi(), &len);
    if (ptr == nullptr) {
      return std::nullopt;
    }
    return std::string_view(ptr, len);
  }
};

} // namespace component
} // namespace wasmtime

#include <wasmtime/component/types/func.hh>
#include <wasmtime/component/types/instance.hh>
#include <wasmtime/component/types/module.hh>
#include <wasmtime/component/types/val.hh>

inline wasmtime::component::ComponentItem
wasmtime::component::ComponentExtern::type() const {
  wasmtime_component_item_t item;
  wasmtime_component_extern_type(capi(), &item);
  return wasmtime::component::ComponentItem(std::move(item));
}

inline std::optional<wasmtime::component::ComponentExtern>
wasmtime::component::ComponentType::import_get(const wasmtime::Engine &engine,
                                               std::string_view name) const {
  wasmtime_component_extern_t *e;
  bool found = wasmtime_component_type_import_get(capi(), engine.capi(),
                                                  name.data(), name.size(), &e);
  if (!found) {
    return std::nullopt;
  }
  return wasmtime::component::ComponentExtern(e);
}

inline std::optional<
    std::pair<std::string_view, wasmtime::component::ComponentExtern>>
wasmtime::component::ComponentType::import_nth(const wasmtime::Engine &engine,
                                               size_t nth) const {
  wasmtime_component_extern_t *e;
  const char *name_data;
  size_t name_size;
  bool found = wasmtime_component_type_import_nth(capi(), engine.capi(), nth,
                                                  &name_data, &name_size, &e);
  if (!found) {
    return std::nullopt;
  }
  return std::make_pair(std::string_view(name_data, name_size),
                        wasmtime::component::ComponentExtern(e));
}

inline std::optional<wasmtime::component::ComponentExtern>
wasmtime::component::ComponentType::export_get(const wasmtime::Engine &engine,
                                               std::string_view name) const {
  wasmtime_component_extern_t *e;
  bool found = wasmtime_component_type_export_get(capi(), engine.capi(),
                                                  name.data(), name.size(), &e);
  if (!found) {
    return std::nullopt;
  }
  return wasmtime::component::ComponentExtern(e);
}

inline std::optional<
    std::pair<std::string_view, wasmtime::component::ComponentExtern>>
wasmtime::component::ComponentType::export_nth(const wasmtime::Engine &engine,
                                               size_t nth) const {
  wasmtime_component_extern_t *e;
  const char *name_data;
  size_t name_size;
  bool found = wasmtime_component_type_export_nth(capi(), engine.capi(), nth,
                                                  &name_data, &name_size, &e);
  if (!found) {
    return std::nullopt;
  }
  return std::make_pair(std::string_view(name_data, name_size),
                        wasmtime::component::ComponentExtern(e));
}

inline const wasmtime::component::ComponentInstanceType &
wasmtime::component::ComponentItem::component_instance() const {
  assert(is_component_instance());
  return *ComponentInstanceType::from_capi(&item.of.component_instance);
}

inline const wasmtime::component::ModuleType &
wasmtime::component::ComponentItem::module() const {
  assert(is_module());
  return *ModuleType::from_capi(&item.of.module);
}

inline const wasmtime::component::FuncType &
wasmtime::component::ComponentItem::component_func() const {
  assert(is_component_func());
  return *FuncType::from_capi(&item.of.component_func);
}

inline const wasmtime::component::ResourceType &
wasmtime::component::ComponentItem::resource() const {
  assert(is_resource());
  return *ResourceType::from_capi(&item.of.resource);
}

inline const wasmtime::component::ValType &
wasmtime::component::ComponentItem::type() const {
  assert(is_type());
  return *ValType::from_capi(&item.of.type);
}

#endif // WASMTIME_FEATURE_COMPONENT_MODEL

#endif // WASMTIME_COMPONENT_TYPES_COMPONENT_HH
