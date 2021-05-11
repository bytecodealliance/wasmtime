/**
 * \file wasmtime/func.h
 *
 * TODO
 */

#ifndef WASMTIME_FUNC_H
#define WASMTIME_FUNC_H

#include <wasm.h>
#include <wasmtime/val.h>
#include <wasmtime/store.h>
#include <wasmtime/extern.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * \typedef wasmtime_caller_t
 * \brief Alias to #wasmtime_caller
 *
 * \brief Structure used to learn about the caller of a host-defined function.
 * \struct wasmtime_caller
 * \headerfile wasmtime/func.h
 *
 * This structure is the first argument of #wasmtime_func_callback_t and
 * wasmtime_func_callback_with_env_t. The main purpose of this structure is for
 * building a WASI-like API which can inspect the memory of the caller,
 * regardless of the caller.
 *
 * This is intended to be a temporary API extension until interface types have
 * become more prevalent. This is not intended to be supported until the end of
 * time, but it will be supported so long as WASI requires it.
 */
typedef struct wasmtime_caller wasmtime_caller_t;

/**
 * \brief Callback signature for #wasmtime_func_new.
 *
 * This function is the same as #wasm_func_callback_t except that its first
 * argument is a #wasmtime_caller_t which allows learning information about the
 * caller.
 */
typedef wasm_trap_t* (*wasmtime_func_callback_t)(
    void *env,
    wasmtime_caller_t* caller,
    const wasmtime_val_t *args,
    size_t nargs,
    wasmtime_val_t *results,
    size_t nresults);

/**
 * \brief Creates a new host-defined function.
 *
 * TODO
 *
 * This function is the same as #wasm_func_new, except the callback has the type
 * signature #wasmtime_func_callback_t which gives a #wasmtime_caller_t as its
 * first argument.
 */
WASM_API_EXTERN wasmtime_func_t wasmtime_func_new(
  wasmtime_context_t *store,
  const wasm_functype_t* type,
  wasmtime_func_callback_t callback,
  void *env,
  void (*finalizer)(void*)
);

/// TODO
WASM_API_EXTERN wasm_functype_t* wasmtime_func_type(
    const wasmtime_context_t *store,
    wasmtime_func_t func
);

/**
 * \brief Call a WebAssembly function.
 *
 * This function is similar to #wasm_func_call, but with a few tweaks:
 *
 * * An error *and* a trap can be returned
 * * Errors are returned if `args` have the wrong types, if the args/results
 *   arrays have the wrong lengths, or if values come from the wrong store.
 *
 * There are three possible return states from this function:
 *
 * 1. The returned error is non-null. This means `results`
 *    wasn't written to and `trap` will have `NULL` written to it. This state
 *    means that programmer error happened when calling the function (e.g. the
 *    size of the args/results were wrong)
 * 2. The trap pointer is filled in. This means the returned error is `NULL` and
 *    `results` was not written to. This state means that the function was
 *    executing but hit a wasm trap while executing.
 * 3. The error and trap returned are both `NULL` and `results` are written to.
 *    This means that the function call worked and the specified results were
 *    produced.
 *
 * The `trap` pointer cannot be `NULL`. The `args` and `results` pointers may be
 * `NULL` if the corresponding length is zero.
 *
 * Does not take ownership of `wasm_val_t` arguments. Gives ownership of
 * `wasm_val_t` results.
 */
WASM_API_EXTERN wasmtime_error_t *wasmtime_func_call(
    wasmtime_context_t *store,
    wasmtime_func_t func,
    const wasmtime_val_t *args,
    size_t nargs,
     wasmtime_val_t *results,
    size_t nresults,
    wasm_trap_t **trap
);

/**
 * \brief Loads a #wasm_extern_t from the caller's context
 *
 * This function will attempt to look up the export named `name` on the caller
 * instance provided. If it is found then the #wasm_extern_t for that is
 * returned, otherwise `NULL` is returned.
 *
 * Note that this only works for exported memories right now for WASI
 * compatibility.
 *
 * TODO
 */
WASM_API_EXTERN bool wasmtime_caller_export_get(
    wasmtime_caller_t *caller,
    const char *name,
    size_t name_len,
    wasmtime_extern_t *item
);

/// TODO
WASM_API_EXTERN wasmtime_context_t* wasmtime_caller_context(wasmtime_caller_t* caller);

#ifdef __cplusplus
}  // extern "C"
#endif

#endif // WASMTIME_FUNC_H
