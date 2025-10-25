/**
 * \file wasmtime/component/types/module.hh
 */

#ifndef WASMTIME_COMPONENT_TYPES_MODULE_HH
#define WASMTIME_COMPONENT_TYPES_MODULE_HH

#include <wasmtime/conf.h>

#ifdef WASMTIME_FEATURE_COMPONENT_MODEL

#include <memory>
#include <optional>
#include <string>
#include <wasm.h>
#include <wasmtime/component/types/module.h>
#include <wasmtime/engine.hh>
#include <wasmtime/helpers.hh>
#include <wasmtime/types/export.hh>
#include <wasmtime/types/extern.hh>
#include <wasmtime/types/import.hh>

namespace wasmtime {
namespace component {

/**
 * \brief Represents the type of a module.
 */
class ModuleType {
  WASMTIME_CLONE_WRAPPER(ModuleType, wasmtime_module_type)

  /// Returns the number of imports of this module type.
  size_t import_count(const Engine &engine) const {
    return wasmtime_module_type_import_count(ptr.get(), engine.capi());
  }

  /// Retrieves the nth import.
  std::optional<ImportType> import_nth(const Engine &engine, size_t nth) const {
    auto ret = wasmtime_module_type_import_nth(ptr.get(), engine.capi(), nth);
    if (ret)
      return ImportType(ret);
    return std::nullopt;
  }

  /// Returns the number of exports of this module type.
  size_t export_count(const Engine &engine) const {
    return wasmtime_module_type_export_count(ptr.get(), engine.capi());
  }

  /// Retrieves the nth export.
  std::optional<ExportType> export_nth(const Engine &engine, size_t nth) const {
    auto ret = wasmtime_module_type_export_nth(ptr.get(), engine.capi(), nth);
    if (ret)
      return ExportType(ret);
    return std::nullopt;
  }
};

} // namespace component
} // namespace wasmtime

#endif // WASMTIME_FEATURE_COMPONENT_MODEL

#endif // WASMTIME_COMPONENT_TYPES_MODULE_HH
