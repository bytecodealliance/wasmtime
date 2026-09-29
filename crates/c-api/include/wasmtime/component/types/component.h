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
/// The returned pointer must be deallocated with
/// `wasmtime_component_type_delete`.
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
/// The returned `wasmtime_component_extern_t` must be deallocated with
/// `wasmtime_component_extern_delete`.
WASM_API_EXTERN
bool wasmtime_component_type_import_get(
    const wasmtime_component_type_t *ty, const wasm_engine_t *engine,
    const char *name, size_t name_len,
    struct wasmtime_component_extern_t **ret);

/// \brief Retrieves the nth import.
///
/// The returned `wasmtime_component_extern_t` must be deallocated with
/// `wasmtime_component_extern_delete`.
WASM_API_EXTERN
bool wasmtime_component_type_import_nth(
    const wasmtime_component_type_t *ty, const wasm_engine_t *engine,
    size_t nth, const char **name_ret, size_t *name_len_ret,
    struct wasmtime_component_extern_t **ret);

/// \brief Returns the number of exports of a component type.
WASM_API_EXTERN
size_t wasmtime_component_type_export_count(const wasmtime_component_type_t *ty,
                                            const wasm_engine_t *engine);

/// \brief Retrieves the export with the specified name.
///
/// The returned `wasmtime_component_extern_t` must be deallocated with
/// `wasmtime_component_extern_delete`.
WASM_API_EXTERN
bool wasmtime_component_type_export_get(
    const wasmtime_component_type_t *ty, const wasm_engine_t *engine,
    const char *name, size_t name_len,
    struct wasmtime_component_extern_t **ret);

/// \brief Retrieves the nth export.
///
/// The returned `wasmtime_component_extern_t` must be deallocated with
/// `wasmtime_component_extern_delete`.
WASM_API_EXTERN
bool wasmtime_component_type_export_nth(
    const wasmtime_component_type_t *ty, const wasm_engine_t *engine,
    size_t nth, const char **name_ret, size_t *name_len_ret,
    struct wasmtime_component_extern_t **ret);

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

/// \brief Full description of a single component or component instance export
/// or import.
///
/// This carries the type of the item being imported or exported along with
/// any metadata attached to it such as `(implements "...")` or
/// `(external-id "...")` annotations.
///
/// Note that this structure contains pointers to the original component or
/// instance type that it's acquired from. This must always be
/// destroyed/accessed before those original types are destroyed.
typedef struct wasmtime_component_extern_t wasmtime_component_extern_t;

/// \brief Clones a component extern.
///
/// The returned pointer must be deallocated with
/// `wasmtime_component_extern_delete`.
///
/// Note that this does not clone the internal pointers that the
/// `wasmtime_component_extern_t` struct refers to, so the returned structure
/// still cannot outlive the original source that this comes from.
WASM_API_EXTERN
wasmtime_component_extern_t *
wasmtime_component_extern_clone(const wasmtime_component_extern_t *e);

/// \brief Returns the type of the item that this import/export refers to.
///
/// The returned `wasmtime_component_item_t` must be deallocated with
/// `wasmtime_component_item_delete`.
WASM_API_EXTERN
void wasmtime_component_extern_type(const wasmtime_component_extern_t *e,
                                    wasmtime_component_item_t *ret);

/// \brief Returns the `implements` attribute of this import/export.
///
/// Returns `NULL` if this isn't present. Writes the length of the return value
/// to `len`. The returned pointer cannot be used outside the lifetime of `e`.
WASM_API_EXTERN
const char *
wasmtime_component_extern_implements(const wasmtime_component_extern_t *e,
                                     size_t *len);

/// \brief Returns whether this import/export `e` has an `implements` attribute
/// which matches the `name` provided (which is `len` bytes long).
///
/// This returns `false` if `e` has no `implements` attribute. Otherwise this
/// returns `true` if the attribute is exactly equal to `name` or if it's a
/// semver-compatible version of `name`. For example `(implements
/// "a:b/c@1.1.0")` matches both `a:b/c@1.0.0` and `a:b/c@1.2.0`.
WASM_API_EXTERN
bool wasmtime_component_extern_is_implements(
    const wasmtime_component_extern_t *e, const char *name, size_t len);

/// \brief Returns the `external-id` attribute of this import/export.
///
/// Returns `NULL` if this isn't present. Writes the length of the return value
/// to `len`. The returned pointer cannot be used outside the lifetime of `e`.
WASM_API_EXTERN
const char *
wasmtime_component_extern_external_id(const wasmtime_component_extern_t *e,
                                      size_t *len);

/// \brief Deallocates a component extern.
WASM_API_EXTERN
void wasmtime_component_extern_delete(wasmtime_component_extern_t *ptr);

#ifdef __cplusplus
} // extern "C"
#endif

#endif // WASMTIME_FEATURE_COMPONENT_MODEL

#endif // WASMTIME_COMPONENT_TYPES_COMPONENT_H
