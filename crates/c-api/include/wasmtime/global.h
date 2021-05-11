/**
 * \file wasmtime/global.h
 *
 * TODO
 */

#ifndef WASMTIME_GLOBAL_H
#define WASMTIME_GLOBAL_H

#include <wasm.h>
#include <wasmtime/extern.h>
#include <wasmtime/store.h>
#include <wasmtime/val.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * \brief Creates a new global value.
 *
 * Similar to #wasm_global_new, but with a few tweaks:
 *
 * * An error is returned instead of #wasm_global_t, which is taken as an
 *   out-parameter
 * * An error happens when the `type` specified does not match the type of the
 *   value `val`, or if it comes from a different store than `store`.
 *
 * This function does not take ownership of any of its arguments but returned
 * values are owned by the caller.
 */
WASM_API_EXTERN wasmtime_error_t *wasmtime_global_new(
    wasmtime_context_t *store,
    const wasm_globaltype_t *type,
    const wasmtime_val_t *val,
    wasmtime_global_t *ret
);

/// TODO
WASM_API_EXTERN wasm_globaltype_t* wasmtime_global_type(
    const wasmtime_context_t *store,
    wasmtime_global_t global
);

/// TODO
WASM_API_EXTERN void wasmtime_global_get(
    wasmtime_context_t *store,
    wasmtime_global_t global,
    wasmtime_val_t *out
);

/**
 * \brief Sets a global to a new value.
 *
 * This function is the same as #wasm_global_set, except in the case of an error
 * a #wasmtime_error_t is returned.
 */
WASM_API_EXTERN wasmtime_error_t *wasmtime_global_set(
    wasmtime_context_t *store,
    wasmtime_global_t global,
    const wasmtime_val_t *val
);

#ifdef __cplusplus
}  // extern "C"
#endif

#endif // WASMTIME_GLOBAL_H
