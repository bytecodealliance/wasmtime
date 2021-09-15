/**
 * \file wasmtime/store.h
 *
 * Wasmtime definition of a "store".
 */

#ifndef WASMTIME_STORE_H
#define WASMTIME_STORE_H

#include <wasm.h>
#include <wasi.h>
#include <wasmtime/error.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * \typedef wasmtime_store_t
 * \brief Convenience alias for #wasmtime_store_t
 *
 * \struct wasmtime_store
 * \brief Storage of WebAssembly objects
 *
 * A store is the unit of isolation between WebAssembly instances in an
 * embedding of Wasmtime. Values in one #wasmtime_store_t cannot flow into
 * another #wasmtime_store_t. Stores are cheap to create and cheap to dispose.
 * It's expected that one-off stores are common in embeddings.
 *
 * Objects stored within a #wasmtime_store_t are referenced with integer handles
 * rather than interior pointers. This means that most APIs require that the
 * store be explicitly passed in, which is done via #wasmtime_context_t. It is
 * safe to move a #wasmtime_store_t to any thread at any time. A store generally
 * cannot be concurrently used, however.
 */
typedef struct wasmtime_store wasmtime_store_t;

/**
 * \typedef wasmtime_context_t
 * \brief Convenience alias for #wasmtime_context
 *
 * \struct wasmtime_context
 * \brief An interior pointer into a #wasmtime_store_t which is used as
 * "context" for many functions.
 *
 * This context pointer is used pervasively throught Wasmtime's API. This can be
 * acquired from #wasmtime_store_context or #wasmtime_caller_context. The
 * context pointer for a store is the same for the entire lifetime of a store,
 * so it can safely be stored adjacent to a #wasmtime_store_t itself.
 *
 * Usage of a #wasmtime_context_t must not outlive the original
 * #wasmtime_store_t. Additionally #wasmtime_context_t can only be used in
 * situations where it has explicitly been granted access to doing so. For
 * example finalizers cannot use #wasmtime_context_t because they are not given
 * access to it.
 */
typedef struct wasmtime_context wasmtime_context_t;

/**
 * \brief Creates a new store within the specified engine.
 *
 * \param engine the compilation environment with configuration this store is
 * connected to
 * \param data user-provided data to store, can later be acquired with
 * #wasmtime_context_get_data.
 * \param finalizer an optional finalizer for `data`
 *
 * This function creates a fresh store with the provided configuration settings.
 * The returned store must be deleted with #wasmtime_store_delete.
 */
WASM_API_EXTERN wasmtime_store_t *wasmtime_store_new(
    wasm_engine_t *engine,
    void *data,
    void (*finalizer)(void*)
);

/**
 * \brief Returns the interior #wasmtime_context_t pointer to this store
 */
WASM_API_EXTERN wasmtime_context_t *wasmtime_store_context(wasmtime_store_t *store);

/**
 * \brief Deletes a store.
 */
WASM_API_EXTERN void wasmtime_store_delete(wasmtime_store_t *store);

/**
 * \brief Returns the user-specified data associated with the specified store
 */
WASM_API_EXTERN void *wasmtime_context_get_data(const wasmtime_context_t* context);

/**
 * \brief Overwrites the user-specified data associated with this store.
 *
 * Note that this does not execute the original finalizer for the provided data,
 * and the original finalizer will be executed for the provided data when the
 * store is deleted.
 */
WASM_API_EXTERN void wasmtime_context_set_data(wasmtime_context_t* context, void *data);

/**
 * \brief Perform garbage collection within the given context.
 *
 * Garbage collects `externref`s that are used within this store. Any
 * `externref`s that are discovered to be unreachable by other code or objects
 * will have their finalizers run.
 *
 * The `context` argument must not be NULL.
 */
WASM_API_EXTERN void wasmtime_context_gc(wasmtime_context_t* context);

/**
 * \brief Adds fuel to this context's store for wasm to consume while executing.
 *
 * For this method to work fuel consumption must be enabled via
 * #wasmtime_config_consume_fuel_set. By default a store starts with 0 fuel
 * for wasm to execute with (meaning it will immediately trap).
 * This function must be called for the store to have
 * some fuel to allow WebAssembly to execute.
 *
 * Note that at this time when fuel is entirely consumed it will cause
 * wasm to trap. More usages of fuel are planned for the future.
 *
 * If fuel is not enabled within this store then an error is returned. If fuel
 * is successfully added then NULL is returned.
 */
WASM_API_EXTERN wasmtime_error_t *wasmtime_context_add_fuel(wasmtime_context_t *store, uint64_t fuel);

/**
 * \brief Returns the amount of fuel consumed by this context's store execution
 * so far.
 *
 * If fuel consumption is not enabled via #wasmtime_config_consume_fuel_set
 * then this function will return false. Otherwise true is returned and the
 * fuel parameter is filled in with fuel consuemd so far.
 *
 * Also note that fuel, if enabled, must be originally configured via
 * #wasmtime_context_add_fuel.
 */
WASM_API_EXTERN bool wasmtime_context_fuel_consumed(const wasmtime_context_t *context, uint64_t *fuel);

/**
 * \brief Attempt to manually consume fuel from the store.
 *
 * If fuel consumption is not enabled via #wasmtime_config_consume_fuel_set then
 * this function will return an error. Otherwise this will attempt to consume
 * the specified amount of `fuel` from the store. If successful the remaining
 * amount of fuel is stored into `remaining`. If `fuel` couldn't be consumed
 * then an error is returned.
 *
 * Also note that fuel, if enabled, must be originally configured via
 * #wasmtime_context_add_fuel.
 */
WASM_API_EXTERN wasmtime_error_t *wasmtime_context_consume_fuel(wasmtime_context_t *context, uint64_t fuel, uint64_t *remaining);

/**
 * \brief Configres WASI state within the specified store.
 *
 * This function is required if #wasmtime_linker_define_wasi is called. This
 * will configure the WASI state for instances defined within this store to the
 * configuration specified.
 *
 * This function does not take ownership of `context` but it does take ownership
 * of `wasi`. The caller should no longer use `wasi` after calling this function
 * (even if an error is returned).
 */
WASM_API_EXTERN wasmtime_error_t *wasmtime_context_set_wasi(wasmtime_context_t *context, wasi_config_t *wasi);

/**
 * \typedef wasmtime_interrupt_handle_t
 * \brief Convenience alias for #wasmtime_interrupt_handle_t
 *
 * \struct wasmtime_interrupt_handle_t
 * \brief A handle used to interrupt executing WebAssembly code.
 *
 * This structure is an opaque handle that represents a handle to a store. This
 * handle can be used to remotely (from another thread) interrupt currently
 * executing WebAssembly code.
 *
 * This structure is safe to share from multiple threads.
 */
typedef struct wasmtime_interrupt_handle wasmtime_interrupt_handle_t;

/**
 * \brief Creates a new interrupt handle to interrupt executing WebAssembly from
 * the provided store.
 *
 * There are a number of caveats about how interrupt is handled in Wasmtime. For
 * more information see the [Rust
 * documentation](https://bytecodealliance.github.io/wasmtime/api/wasmtime/struct.Store.html#method.interrupt_handle).
 *
 * This function returns `NULL` if the store's configuration does not have
 * interrupts enabled. See #wasmtime_config_interruptable_set.
 */
WASM_API_EXTERN wasmtime_interrupt_handle_t *wasmtime_interrupt_handle_new(wasmtime_context_t *context);

/**
 * \brief Requests that WebAssembly code running in the store attached to this
 * interrupt handle is interrupted.
 *
 * For more information about interrupts see #wasmtime_interrupt_handle_new.
 *
 * Note that this is safe to call from any thread.
 */
WASM_API_EXTERN void wasmtime_interrupt_handle_interrupt(wasmtime_interrupt_handle_t *handle);

/**
 * \brief Deletes an interrupt handle.
 */
WASM_API_EXTERN void wasmtime_interrupt_handle_delete(wasmtime_interrupt_handle_t *handle);

#ifdef __cplusplus
}  // extern "C"
#endif

#endif // WASMTIME_STORE_H

