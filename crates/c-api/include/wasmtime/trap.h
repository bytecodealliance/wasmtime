/**
 * \file wasmtime/trap.h
 *
 * Wasmtime APIs for interacting with traps and extensions to #wasm_trap_t.
 */

#ifndef WASMTIME_TRAP_H
#define WASMTIME_TRAP_H

#include <wasm.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * \brief Creates a new trap.
 *
 * \param msg the message to associate with this trap
 * \param msg_len the byte length of `msg`
 *
 * The #wasm_trap_t returned is owned by the caller.
 */
WASM_API_EXTERN wasm_trap_t *wasmtime_trap_new(const char *msg, size_t msg_len);

/**
 * \brief Attempts to extract a WASI-specific exit status from this trap.
 *
 * Returns `true` if the trap is a WASI "exit" trap and has a return status. If
 * `true` is returned then the exit status is returned through the `status`
 * pointer. If `false` is returned then this is not a wasi exit trap.
 */
WASM_API_EXTERN bool wasmtime_trap_exit_status(const wasm_trap_t*, int *status);

/**
 * \brief Returns a human-readable name for this frame's function.
 *
 * This function will attempt to load a human-readable name for function this
 * frame points to. This function may return `NULL`.
 *
 * The lifetime of the returned name is the same as the #wasm_frame_t itself.
 */
WASM_API_EXTERN const wasm_name_t *wasmtime_frame_func_name(const wasm_frame_t*);

/**
 * \brief Returns a human-readable name for this frame's module.
 *
 * This function will attempt to load a human-readable name for module this
 * frame points to. This function may return `NULL`.
 *
 * The lifetime of the returned name is the same as the #wasm_frame_t itself.
 */
WASM_API_EXTERN const wasm_name_t *wasmtime_frame_module_name(const wasm_frame_t*);


#ifdef __cplusplus
}  // extern "C"
#endif

#endif // WASMTIME_TRAP_H
