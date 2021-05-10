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
WASM_API_EXTERN wasmtime_instancetype_t* wasm_externtype_as_instancetype(wasm_externtype_t*);

/**
 * \brief Wasmtime-specific function to instantiate a module.
 *
 * This function is similar to #wasm_instance_new, but with a few tweaks:
 *
 * * An error message can be returned from this function.
 * * The `trap` pointer is required to not be NULL.
 *
 * The states of return values from this function are similar to
 * #wasmtime_func_call where an error can be returned meaning something like a
 * link error in this context. A trap can be returned (meaning no error or
 * instance is returned), or an instance can be returned (meaning no error or
 * trap is returned).
 *
 * This function does not take ownership of any of its arguments, but all return
 * values are owned by the caller.
 *
 * See #wasm_instance_new for information about how to fill in the `imports`
 * array.
 */
WASM_API_EXTERN wasmtime_error_t *wasmtime_instance_new(
    wasmtime_context_t *store,
    const wasmtime_module_t *module,
    const wasmtime_extern_t* imports,
    size_t nimports,
    wasmtime_instance_t *instance,
    wasm_trap_t **trap
);

WASM_API_EXTERN wasmtime_instancetype_t *wasmtime_instance_type(
    const wasmtime_context_t *store,
    wasmtime_instance_t instance
);

WASM_API_EXTERN bool wasmtime_instance_export_get(
    wasmtime_context_t *store,
    wasmtime_instance_t instance,
    char *name,
    size_t name_len,
    wasmtime_extern_t *item
);

WASM_API_EXTERN bool wasmtime_instance_export_nth(
    wasmtime_context_t *store,
    wasmtime_instance_t instance,
    size_t index,
    char **name,
    size_t *name_len,
    wasmtime_extern_t *item
);

#ifdef __cplusplus
}  // extern "C"
#endif

#endif // WASMTIME_INSTANCE_H
