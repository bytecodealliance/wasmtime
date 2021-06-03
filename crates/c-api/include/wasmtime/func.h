/**
 * \file wasmtime/func.h
 *
 * Wasmtime definitions of how to interact with host and wasm functions.
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
 *
 * This structure is an argument to #wasmtime_func_callback_t. The purpose
 * of this structure is acquire a #wasmtime_context_t pointer to interact with
 * objects, but it can also be used for inspect the state of the caller (such as
 * getting memories and functions) with #wasmtime_caller_export_get.
 *
 * This object is never owned and does not need to be deleted.
 */
typedef struct wasmtime_caller wasmtime_caller_t;

/**
 * \brief Callback signature for #wasmtime_func_new.
 *
 * This is the function signature for host functions that can be made accessible
 * to WebAssembly. The arguments to this function are:
 *
 * \param env user-provided argument passed to #wasmtime_func_new
 * \param caller a temporary object that can only be used during this function
 * call. Used to acquire #wasmtime_context_t or caller's state
 * \param args the arguments provided to this function invocation
 * \param nargs how many arguments are provided
 * \param results where to write the results of this function
 * \param nresults how many results must be produced
 *
 * Callbacks are guaranteed to get called with the right types of arguments, but
 * they must produce the correct number and types of results. Failure to do so
 * will cause traps to get raised on the wasm side.
 *
 * This callback can optionally return a #wasm_trap_t indicating that a trap
 * should be raised in WebAssembly. It's expected that in this case the caller
 * relinquishes ownership of the trap and it is passed back to the engine.
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
 * Inserts a host-defined function into the `store` provided which can be used
 * to then instantiate a module with or define within a #wasmtime_linker_t.
 *
 * \param store the store in which to create the function
 * \param type the wasm type of the function that's being created
 * \param callback the host-defined callback to invoke
 * \param env host-specific data passed to the callback invocation, can be
 * `NULL`
 * \param finalizer optional finalizer for `env`, can be `NULL`
 * \param ret the #wasmtime_func_t return value to be filled in.
 *
 * The returned function can only be used with the specified `store`.
 */
WASM_API_EXTERN void wasmtime_func_new(
  wasmtime_context_t *store,
  const wasm_functype_t* type,
  wasmtime_func_callback_t callback,
  void *env,
  void (*finalizer)(void*),
  wasmtime_func_t *ret
);

/**
 * \brief Returns the type of the function specified
 *
 * The returned #wasm_functype_t is owned by the caller.
 */
WASM_API_EXTERN wasm_functype_t* wasmtime_func_type(
    const wasmtime_context_t *store,
    const wasmtime_func_t *func
);

/**
 * \brief Call a WebAssembly function.
 *
 * This function is used to invoke a function defined within a store. For
 * example this might be used after extracting a function from a
 * #wasmtime_instance_t.
 *
 * \param store the store which owns `func`
 * \param func the function to call
 * \param args the arguments to the function call
 * \param nargs the number of arguments provided
 * \param results where to write the results of the function call
 * \param nresults the number of results expected
 * \param trap where to store a trap, if one happens.
 *
 * There are three possible return states from this function:
 *
 * 1. The returned error is non-null. This means `results`
 *    wasn't written to and `trap` will have `NULL` written to it. This state
 *    means that programmer error happened when calling the function, for
 *    example when the size of the arguments/results was wrong, the types of the
 *    arguments were wrong, or arguments may come from the wrong store.
 * 2. The trap pointer is filled in. This means the returned error is `NULL` and
 *    `results` was not written to. This state means that the function was
 *    executing but hit a wasm trap while executing.
 * 3. The error and trap returned are both `NULL` and `results` are written to.
 *    This means that the function call succeeded and the specified results were
 *    produced.
 *
 * The `trap` pointer cannot be `NULL`. The `args` and `results` pointers may be
 * `NULL` if the corresponding length is zero.
 *
 * Does not take ownership of #wasmtime_val_t arguments. Gives ownership of
 * #wasmtime_val_t results.
 */
WASM_API_EXTERN wasmtime_error_t *wasmtime_func_call(
    wasmtime_context_t *store,
    const wasmtime_func_t *func,
    const wasmtime_val_t *args,
    size_t nargs,
    wasmtime_val_t *results,
    size_t nresults,
    wasm_trap_t **trap
);

/**
 * \brief Loads a #wasmtime_extern_t from the caller's context
 *
 * This function will attempt to look up the export named `name` on the caller
 * instance provided. If it is found then the #wasmtime_extern_t for that is
 * returned, otherwise `NULL` is returned.
 *
 * Note that this only works for exported memories right now for WASI
 * compatibility.
 *
 * \param caller the caller object to look up the export from
 * \param name the name that's being looked up
 * \param name_len the byte length of `name`
 * \param item where to store the return value
 *
 * Returns a nonzero value if the export was found, or 0 if the export wasn't
 * found. If the export wasn't found then `item` isn't written to.
 */
WASM_API_EXTERN bool wasmtime_caller_export_get(
    wasmtime_caller_t *caller,
    const char *name,
    size_t name_len,
    wasmtime_extern_t *item
);

/**
 * \brief Returns the store context of the caller object.
 */
WASM_API_EXTERN wasmtime_context_t* wasmtime_caller_context(wasmtime_caller_t* caller);

#ifdef __cplusplus
}  // extern "C"
#endif

#endif // WASMTIME_FUNC_H
