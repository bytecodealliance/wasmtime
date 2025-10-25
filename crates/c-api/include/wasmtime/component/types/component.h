/// \file wasmtime/component/types/component.h

#ifndef WASMTIME_COMPONENT_TYPES_COMPONENT_H
#define WASMTIME_COMPONENT_TYPES_COMPONENT_H

#include <wasmtime/conf.h>

#ifdef WASMTIME_FEATURE_COMPONENT_MODEL

#include <wasm.h>
#include <wasmtime/component/types/func.h>
#include <wasmtime/component/types/instance.h>
#include <wasmtime/component/types/module.h>
#include <wasmtime/component/types/resource.h>
#include <wasmtime/component/types/val.h>

#ifdef __cplusplus
extern "C" {
#endif

/// \brief Represents the type of a WebAssembly component.
typedef struct wasmtime_component_type_t wasmtime_component_type_t;

/// \brief Clones a component type.
///
/// The returned pointer must be deallocated wit
/// h`wasmtime_component_type_delete`.
WASM_API_EXTERN
wasmtime_component_type_t *
wasmtime_component_type_clone(const wasmtime_component_type_t *ty);

/// \brief Deallocates a component type.
WASM_API_EXTERN
void wasmtime_component_type_delete(wasmtime_component_type_t *ty);

/// \brief Returns the number of imports of a component type.
WASM_API_EXTERN
size_t wasmtime_component_type_import_count(const wasmtime_component_type_t *ty,
                                            const wasm_engine_t *engine);

/// \brief Retrieves the import with the specified name.
///
/// The returned `wasmtime_component_item_t` must be deallocated with
/// `wasmtime_component_item_delete`.
WASM_API_EXTERN
bool wasmtime_component_type_import_get(const wasmtime_component_type_t *ty,
                                        const wasm_engine_t *engine,
                                        const char *name, size_t name_len,
                                        struct wasmtime_component_item_t *ret);

/// \brief Retrieves the nth import.
///
/// The returned `wasmtime_component_item_t` must be deallocated with
/// `wasmtime_component_item_delete`.
WASM_API_EXTERN
bool wasmtime_component_type_import_nth(
    const wasmtime_component_type_t *ty, const wasm_engine_t *engine,
    size_t nth, const char **name_ret, size_t *name_len_ret,
    struct wasmtime_component_item_t *type_ret);

/// \brief Returns the number of exports of a component type.
WASM_API_EXTERN
size_t wasmtime_component_type_export_count(const wasmtime_component_type_t *ty,
                                            const wasm_engine_t *engine);

/// \brief Retrieves the export with the specified name.
///
/// The returned `wasmtime_component_item_t` must be deallocated with
/// `wasmtime_component_item_delete`.
WASM_API_EXTERN
bool wasmtime_component_type_export_get(const wasmtime_component_type_t *ty,
                                        const wasm_engine_t *engine,
                                        const char *name, size_t name_len,
                                        struct wasmtime_component_item_t *ret);

/// \brief Retrieves the nth export.
///
/// The returned `wasmtime_component_item_t` must be deallocated with
/// `wasmtime_component_item_delete`.
WASM_API_EXTERN
bool wasmtime_component_type_export_nth(
    const wasmtime_component_type_t *ty, const wasm_engine_t *engine,
    size_t nth, const char **name_ret, size_t *name_len_ret,
    struct wasmtime_component_item_t *type_ret);

/// \brief Value of #wasmtime_component_item_kind_t meaning that
/// #wasmtime_component_item_t is a component.
#define WASMTIME_COMPONENT_ITEM_COMPONENT 0
/// \brief Value of #wasmtime_component_item_kind_t meaning that
/// #wasmtime_component_item_t is a component instance.
#define WASMTIME_COMPONENT_ITEM_COMPONENT_INSTANCE 1
/// \brief Value of #wasmtime_component_item_kind_t meaning that
/// #wasmtime_component_item_t is a module.
#define WASMTIME_COMPONENT_ITEM_MODULE 2
/// \brief Value of #wasmtime_component_item_kind_t meaning that
/// #wasmtime_component_item_t is a component function.
#define WASMTIME_COMPONENT_ITEM_COMPONENT_FUNC 3
/// \brief Value of #wasmtime_component_item_kind_t meaning that
/// #wasmtime_component_item_t is a resource.
#define WASMTIME_COMPONENT_ITEM_RESOURCE 4
/// \brief Value of #wasmtime_component_item_kind_t meaning that
/// #wasmtime_component_item_t is a core function.
#define WASMTIME_COMPONENT_ITEM_CORE_FUNC 5
/// \brief Value of #wasmtime_component_item_kind_t meaning that
/// #wasmtime_component_item_t is a type.
#define WASMTIME_COMPONENT_ITEM_TYPE 6

/// \brief Discriminant used in #wasmtime_component_item_t::kind
typedef uint8_t wasmtime_component_item_kind_t;

/// \brief Represents a single item in a component's import or export list.
typedef union wasmtime_component_item_union {
  /// Field used if #wasmtime_component_item_t::kind is
  /// #WASMTIME_COMPONENT_ITEM_COMPONENT
  wasmtime_component_type_t *component;
  /// Field used if #wasmtime_component_item_t::kind is
  /// #WASMTIME_COMPONENT_ITEM_COMPONENT_INSTANCE
  wasmtime_component_instance_type_t *component_instance;
  /// Field used if #wasmtime_component_item_t::kind is
  /// #WASMTIME_COMPONENT_ITEM_MODULE
  wasmtime_module_type_t *module;
  /// Field used if #wasmtime_component_item_t::kind is
  /// #WASMTIME_COMPONENT_ITEM_COMPONENT_FUNC
  wasmtime_component_func_type_t *component_func;
  /// Field used if #wasmtime_component_item_t::kind is
  /// #WASMTIME_COMPONENT_ITEM_RESOURCE
  wasmtime_component_resource_type_t *resource;
  /// Field used if #wasmtime_component_item_t::kind is
  /// #WASMTIME_COMPONENT_ITEM_CORE_FUNC
  wasm_functype_t *core_func;
  /// Field used if #wasmtime_component_item_t::kind is
  /// #WASMTIME_COMPONENT_ITEM_TYPE
  wasmtime_component_valtype_t type;
} wasmtime_component_item_union_t;

/// \brief Represents a single item in a component's import or export list.
typedef struct wasmtime_component_item_t {
  /// The type discriminant for the `of` union.
  wasmtime_component_item_kind_t kind;
  /// The actual item.
  wasmtime_component_item_union_t of;
} wasmtime_component_item_t;

/// \brief Clones a component item.
///
/// The returned pointer must be deallocated with
/// `wasmtime_component_item_delete`.
WASM_API_EXTERN
void wasmtime_component_item_clone(const wasmtime_component_item_t *item,
                                   wasmtime_component_item_t *out);

/// \brief Deallocates a component item.
WASM_API_EXTERN
void wasmtime_component_item_delete(wasmtime_component_item_t *ptr);

#ifdef __cplusplus
} // extern "C"
#endif

#endif // WASMTIME_FEATURE_COMPONENT_MODEL

#endif // WASMTIME_COMPONENT_TYPES_COMPONENT_H
