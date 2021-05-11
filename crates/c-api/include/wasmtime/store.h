#ifndef WASMTIME_STORE_H
#define WASMTIME_STORE_H

#include <wasm.h>
#include <wasmtime/error.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * \typedef wasmtime_store_t
 * \brief Convenience alias for #wasmtime_store_t
 *
 * \struct wasmtime_store
 * \brief TODO
 *
 * TODO
 */
typedef struct wasmtime_store wasmtime_store_t;

typedef struct wasmtime_context wasmtime_context_t;

WASM_API_EXTERN wasmtime_store_t *wasmtime_store_new(
    wasm_engine_t *engine,
    void *data,
    void(*finalizer)(void*)
);

WASM_API_EXTERN wasmtime_context_t *wasmtime_store_context(wasmtime_store_t *store);

/*
 * \brief Deletes a store.
 */
WASM_API_EXTERN void wasmtime_store_delete(wasmtime_store_t *store);

WASM_API_EXTERN void *wasmtime_context_get_data(const wasmtime_context_t* context);
WASM_API_EXTERN void wasmtime_context_set_data(wasmtime_context_t* context, void *data);

/**
 * \brief Perform garbage collection within the given context.
 *
 * Garbage collects `externref`s that are used within this store. Any
 * `externref`s that are discovered to be unreachable by other code or objects
 * will have their finalizers run.
 *
 * The `store` argument must not be NULL.
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
 * #wasmtime_store_add_fuel.
 */
WASM_API_EXTERN bool wasmtime_context_fuel_consumed(const wasmtime_context_t *context, uint64_t *fuel);

typedef struct wasi_config_t wasi_config_t;

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

/*
 * \brief Deletes an interrupt handle.
 */
WASM_API_EXTERN void wasmtime_interrupt_handle_delete(wasmtime_interrupt_handle_t *handle);

#ifdef __cplusplus
}  // extern "C"
#endif

#endif // WASMTIME_STORE_H

