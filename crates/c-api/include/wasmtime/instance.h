/**
 * \file wasmtime/instance.h
 *
 * Wasmtime APIs for interacting with wasm instances.
 */

#ifndef WASMTIME_INSTANCE_H
#define WASMTIME_INSTANCE_H

#include <wasm.h>
#include <wasmtime/extern.h>
#include <wasmtime/module.h>
#include <wasmtime/store.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * \brief An opaque object representing the type of an instance.
 */
typedef struct wasmtime_instancetype wasmtime_instancetype_t;

/// \brief Deletes an instance type
WASM_API_EXTERN void wasmtime_instancetype_delete(wasmtime_instancetype_t *ty);

/**
 * \brief Returns the list of exports that this instance type provides.
 *
 * This function does not take ownership of the provided instance type but
 * ownership of `out` is passed to the caller. Note that `out` is treated as
 * uninitialized when passed to this function.
 */
WASM_API_EXTERN void wasmtime_instancetype_exports(const wasmtime_instancetype_t*, wasm_exporttype_vec_t* out);

/**
 * \brief Converts a #wasmtime_instancetype_t to a #wasm_externtype_t
 *
 * The returned value is owned by the #wasmtime_instancetype_t argument and should not
 * be deleted.
 */
WASM_API_EXTERN wasm_externtype_t* wasmtime_instancetype_as_externtype(wasmtime_instancetype_t*);

/**
 * \brief Attempts to convert a #wasm_externtype_t to a #wasmtime_instancetype_t
 *
 * The returned value is owned by the #wasmtime_instancetype_t argument and should not
 * be deleted. Returns `NULL` if the provided argument is not a
 * #wasmtime_instancetype_t.
 */
WASM_API_EXTERN wasmtime_instancetype_t* wasmtime_externtype_as_instancetype(wasm_externtype_t*);

/**
 * \brief Instantiate a wasm module.
 *
 * This function will instantiate a WebAssembly module with the provided
 * imports, creating a WebAssembly instance. The returned instance can then
 * afterwards be inspected for exports.
 *
 * \param store the store in which to create the instance
 * \param module the module that's being instantiated
 * \param imports the imports provided to the module
 * \param nimports the size of `imports`
 * \param instance where to store the returned instance
 * \param trap where to store the returned trap
 *
 * This function requires that `imports` is the same size as the imports that
 * `module` has. Additionally the `imports` array must be 1:1 lined up with the
 * imports of the `module` specified. This is intended to be relatively low
 * level, and #wasmtime_linker_instantiate is provided for a more ergonomic
 * name-based resolution API.
 *
 * The states of return values from this function are similar to
 * #wasmtime_func_call where an error can be returned meaning something like a
 * link error in this context. A trap can be returned (meaning no error or
 * instance is returned), or an instance can be returned (meaning no error or
 * trap is returned).
 *
 * Note that this function requires that all `imports` specified must be owned
 * by the `store` provided as well.
 *
 * This function does not take ownership of any of its arguments, but all return
 * values are owned by the caller.
 */
WASM_API_EXTERN wasmtime_error_t *wasmtime_instance_new(
    wasmtime_context_t *store,
    const wasmtime_module_t *module,
    const wasmtime_extern_t* imports,
    size_t nimports,
    wasmtime_instance_t *instance,
    wasm_trap_t **trap
);

/**
 * \brief Returns the type of the specified instance.
 *
 * The returned type is owned by the caller.
 */
WASM_API_EXTERN wasmtime_instancetype_t *wasmtime_instance_type(
    const wasmtime_context_t *store,
    const wasmtime_instance_t *instance
);

/**
 * \brief Get an export by name from an instance.
 *
 * \param store the store that owns `instance`
 * \param instance the instance to lookup within
 * \param name the export name to lookup
 * \param name_len the byte length of `name`
 * \param item where to store the returned value
 *
 * Returns nonzero if the export was found, and `item` is filled in. Otherwise
 * returns 0.
 *
 * Doesn't take ownership of any arguments but does return ownership of the
 * #wasmtime_extern_t.
 */
WASM_API_EXTERN bool wasmtime_instance_export_get(
    wasmtime_context_t *store,
    const wasmtime_instance_t *instance,
    const char *name,
    size_t name_len,
    wasmtime_extern_t *item
);

/**
 * \brief Get an export by index from an instance.
 *
 * \param store the store that owns `instance`
 * \param instance the instance to lookup within
 * \param index the index to lookup
 * \param name where to store the name of the export
 * \param name_len where to store the byte length of the name
 * \param item where to store the export itself
 *
 * Returns nonzero if the export was found, and `name`, `name_len`, and `item`
 * are filled in. Otherwise returns 0.
 *
 * Doesn't take ownership of any arguments but does return ownership of the
 * #wasmtime_extern_t. The `name` pointer return value is owned by the `store`
 * and must be immediately used before calling any other APIs on
 * #wasmtime_context_t.
 */
WASM_API_EXTERN bool wasmtime_instance_export_nth(
    wasmtime_context_t *store,
    const wasmtime_instance_t *instance,
    size_t index,
    char **name,
    size_t *name_len,
    wasmtime_extern_t *item
);

#ifdef __cplusplus
}  // extern "C"
#endif

#endif // WASMTIME_INSTANCE_H
