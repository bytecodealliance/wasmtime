/**
 * \file wasmtime/async.h
 *
 * \brief Wasmtime async functionality
 */

#ifndef WASMTIME_ASYNC_H
#define WASMTIME_ASYNC_H

#include <wasm.h>
#include <wasmtime/error.h>
#include <wasmtime/config.h>
#include <wasmtime/store.h>
#include <wasmtime/func.h>
#include <wasmtime/linker.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * \brief Whether or not to enable support for asynchronous functions in Wasmtime.
 *
 * When enabled, the config can optionally define host functions with async. 
 * Instances created and functions called with this Config must be called through their asynchronous APIs, however.
 * For example using wasmtime_func_call will panic when used with this config.
 *
 * For more information see the Rust documentation at
 * https://docs.wasmtime.dev/api/wasmtime/struct.Config.html#method.async_support
 */
WASMTIME_CONFIG_PROP(void, async_support, bool)

/**
 * \brief Configures the size of the stacks used for asynchronous execution.
 *
 * This setting configures the size of the stacks that are allocated for asynchronous execution. 
 *
 * The value cannot be less than max_wasm_stack.
 *
 * The amount of stack space guaranteed for host functions is async_stack_size - max_wasm_stack, so take care
 * not to set these two values close to one another; doing so may cause host functions to overflow the stack 
 * and abort the process.
 *
 * By default this option is 2 MiB.
 *
 * For more information see the Rust documentation at
 * https://docs.wasmtime.dev/api/wasmtime/struct.Config.html#method.async_stack_size
 */
WASMTIME_CONFIG_PROP(void, async_stack_size, uint64_t)

/**
 * \brief Configures a Store to yield execution of async WebAssembly code periodically.
 *
 * When a Store is configured to consume fuel with #wasmtime_config_consume_fuel 
 * this method will configure what happens when fuel runs out. Specifically executing
 * WebAssembly will be suspended and control will be yielded back to the caller.
 *
 * This is only suitable with use of a store associated with an async config because
 * only then are futures used and yields are possible.
 */
WASM_API_EXTERN void wasmtime_context_out_of_fuel_async_yield(
    wasmtime_context_t *context,
    uint64_t injection_count,
    uint64_t fuel_to_inject);

/**
 * \brief Configures epoch-deadline expiration to yield to the async caller and the update the deadline.
 *
 * This is only suitable with use of a store associated with an async config because
 * only then are futures used and yields are possible.
 *
 * See the Rust documentation for more:
 * https://docs.wasmtime.dev/api/wasmtime/struct.Store.html#method.epoch_deadline_async_yield_and_update
 */
WASM_API_EXTERN void wasmtime_context_epoch_deadline_async_yield_and_update(wasmtime_context_t *context, uint64_t delta);

/**
 * The callback to determine a continuation's current state.
 *
 * Return true if the host call has completed, otherwise false will 
 * continue to yield WebAssembly execution.
 *
 * \param if error is assigned a non `NULL` value then the called function will 
 *        trap with the returned error. Note that ownership of error is transferred 
 *        to wasmtime.
 */
typedef bool (*wasmtime_func_async_continuation_callback_t)(
    void *env,
    wasmtime_caller_t *caller,
    wasm_trap_t **error);

/**
 * A continuation for the current state of the host function's execution.
 * 
 * This continutation can be polled via the callback and returns the current state.
 */
typedef struct wasmtime_async_continuation_t {
  wasmtime_func_async_continuation_callback_t callback;
  void *env;
  void (*finalizer)(void *);
} wasmtime_async_continuation_t;

/**
 * \brief Callback signature for #wasmtime_linker_define_async_func.
 *
 * This is a host function that returns a continuation to be called later. 
 * The continuation returned is owned by wasmtime and will be deleted when it completes.
 *
 * All the arguments to this function will be kept alive until the continuation
 * returns that it has errored or has completed.
 *
 * Only supported for async stores.
 *
 * See #wasmtime_func_callback_t for more information.
 */
typedef wasmtime_async_continuation_t *(*wasmtime_func_async_callback_t)(
    void *env,
    wasmtime_caller_t *caller, 
    const wasmtime_val_t *args,
    size_t nargs, 
    wasmtime_val_t *results,
    size_t nresults);

/**
 * \brief The structure representing a asynchronously running function.
 *
 * This structure is always owned by the caller and must be deleted using wasmtime_call_future_delete.
 *
 *
 *
 */
typedef struct wasmtime_call_future wasmtime_call_future_t;

/**
 * \brief Executes WebAssembly in the function.
 *
 * Returns true if the function call has completed, which then wasmtime_call_future_get_results should be called.
 * After this function returns true, it should *not* be called again for a given future.
 *
 * This function returns false if execution has yielded either due to being out of fuel 
 * (see wasmtime_store_out_of_fuel_async_yield), or the epoch has been incremented enough 
 * (see wasmtime_store_epoch_deadline_async_yield_and_update).
 *
 * The function may also return false if asynchronous host functions have been called, which then calling this 
 * function will call the continuation from the async host function.
 *
 * For more see the information at
 * https://docs.wasmtime.dev/api/wasmtime/struct.Config.html#asynchronous-wasm
 *
 */
WASM_API_EXTERN bool wasmtime_call_future_poll(wasmtime_call_future_t *future);

/**
 * /brief Frees the underlying memory for a future.
 *
 * All wasmtime_call_future_t are owned by the caller and should be deleted using this function no 
 * matter the result.
 */
WASM_API_EXTERN void wasmtime_call_future_delete(wasmtime_call_future_t *future);

/**
 * \brief Invokes this function with the params given, returning the results asynchronously.
 *
 * This function is the same as wasmtime_func_call except that it is asynchronous.
 * This is only compatible with stores associated with an asynchronous config.
 *
 * The result is a future that is owned by the caller and must be deleted via #wasmtime_call_future_delete. 
 *
 * The `args` and `results` pointers may be `NULL` if the corresponding length is zero.
 *
 * Does not take ownership of #wasmtime_val_t arguments or #wasmtime_val_t results,
 * the arguments and results must be kept alive until the returned #wasmtime_call_future_t is deleted.
 *
 * See #wasmtime_call_future_t for for more information.
 *
 * For more information see the Rust documentation at
 * https://docs.wasmtime.dev/api/wasmtime/struct.Func.html#method.call_async
 */
WASM_API_EXTERN wasmtime_call_future_t* wasmtime_func_call_async(
    wasmtime_context_t *context,
    const wasmtime_func_t *func,
    const wasmtime_val_t *args,
    size_t nargs,
    wasmtime_val_t *results,
    size_t nresults,
    wasm_trap_t** trap_ret,
    wasmtime_error_t** wasmtime_error_t);

/**
 * \brief Defines a new async function in this linker.
 *
 * This function behaves similar to #wasmtime_linker_define_func, except it supports async
 * callbacks
 */
WASM_API_EXTERN wasmtime_error_t *wasmtime_linker_define_async_func(
    wasmtime_linker_t *linker,
    const char *module,
    size_t module_len,
    const char *name,
    size_t name_len,
    const wasm_functype_t *ty,
    wasmtime_func_async_callback_t cb,
    void *data,
    void (*finalizer)(void *));

/**
 * \brief Instantiates a #wasm_module_t with the items defined in this linker for an async store.
 *
 * This is the same as #wasmtime_linker_instantiate but used for async stores 
 * (which requires functions are called asynchronously). The returning #wasmtime_call_future_t 
 * must be polled using #wasmtime_call_future_poll, and is owned and must be deleted using #wasmtime_call_future_delete.
 * The future's results are retrieved using `wasmtime_call_future_get_results after polling has returned true marking 
 * the future as completed.
 *
 * All arguments to this function must outlive the returned future.
 */
WASM_API_EXTERN wasmtime_call_future_t *wasmtime_linker_instantiate_async(
    const wasmtime_linker_t *linker,
    wasmtime_context_t *store,
    const wasmtime_module_t *module,
    wasmtime_instance_t *instance,
    wasm_trap_t** trap_ret,
    wasmtime_error_t** wasmtime_error_t);

#ifdef __cplusplus
}  // extern "C"
#endif

#endif // WASMTIME_ASYNC_H

