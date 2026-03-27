/**
 * \file wasmtime/exn.h
 *
 * \brief Wasmtime APIs for WebAssembly exception objects.
 *
 * Exception objects carry a tag and a set of field values. They are
 * allocated on the GC heap within a store.
 *
 * ## Throwing from host functions
 *
 * To throw an exception from a host function implemented via the C API:
 *
 * 1. Create an exception object with #wasmtime_exn_new.
 * 2. Set it as the pending exception with #wasmtime_context_set_exception.
 * 3. Return a trap from the host callback (e.g. via `wasmtime_trap_new`).
 *
 * The runtime will propagate the exception through WebAssembly
 * `try_table`/`catch` blocks.
 *
 * ## Catching exceptions from Wasm
 *
 * When a call to a WebAssembly function (e.g. via #wasmtime_func_call)
 * returns a trap, check for a pending exception:
 *
 * 1. Call #wasmtime_context_has_exception or #wasmtime_context_take_exception.
 * 2. If present, examine the exception's tag and fields.
 */

#ifndef WASMTIME_EXN_H
#define WASMTIME_EXN_H

#include <wasm.h>
#include <wasmtime/error.h>
#include <wasmtime/store.h>
#include <wasmtime/tag.h>
#include <wasmtime/val.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * \typedef wasmtime_exn_t
 * \brief An opaque type representing a WebAssembly exception object.
 *
 * Exception objects are allocated on the GC heap and referenced through
 * this handle. The handle is owned by the caller and must be freed
 * with #wasmtime_exn_delete.
 */
typedef struct wasmtime_exn wasmtime_exn_t;

/**
 * \brief Deletes a #wasmtime_exn_t.
 */
WASM_API_EXTERN void wasmtime_exn_delete(wasmtime_exn_t *exn);

/**
 * \brief Creates a new exception object.
 *
 * \param store the store context
 * \param tag the tag to associate with this exception
 * \param tag_type the tag type (must match `tag`)
 * \param fields pointer to an array of field values matching the tag's
 *        payload signature
 * \param nfields the number of elements in `fields`
 * \param exn_ret on success, set to the newly allocated exception.
 *        The caller owns the returned pointer and must free it with
 *        #wasmtime_exn_delete.
 *
 * \return NULL on success, or an error on failure.
 */
WASM_API_EXTERN wasmtime_error_t *
wasmtime_exn_new(wasmtime_context_t *store, const wasmtime_tag_t *tag,
                 const wasm_tagtype_t *tag_type, const wasmtime_val_t *fields,
                 size_t nfields, wasmtime_exn_t **exn_ret);

/**
 * \brief Returns the tag associated with this exception.
 *
 * \param store the store context
 * \param exn the exception to query
 * \param tag_ret on success, filled with the exception's tag
 *
 * \return NULL on success, or an error on failure.
 */
WASM_API_EXTERN wasmtime_error_t *wasmtime_exn_tag(wasmtime_context_t *store,
                                                   const wasmtime_exn_t *exn,
                                                   wasmtime_tag_t *tag_ret);

/**
 * \brief Returns the number of fields in this exception.
 *
 * \param store the store context
 * \param exn the exception to query
 */
WASM_API_EXTERN size_t wasmtime_exn_field_count(wasmtime_context_t *store,
                                                const wasmtime_exn_t *exn);

/**
 * \brief Reads a field value from this exception by index.
 *
 * \param store the store context
 * \param exn the exception to query
 * \param index the field index (0-based)
 * \param val_ret on success, filled with the field value
 *
 * \return NULL on success, or an error if the index is out of bounds.
 */
WASM_API_EXTERN wasmtime_error_t *wasmtime_exn_field(wasmtime_context_t *store,
                                                     const wasmtime_exn_t *exn,
                                                     size_t index,
                                                     wasmtime_val_t *val_ret);

/**
 * \brief Sets the pending exception on the store and returns a trap.
 *
 * This transfers ownership of `exn` to the store. After this call,
 * the caller must not use or free `exn`.
 *
 * Returns a `wasm_trap_t` that the host callback MUST return to signal
 * to the Wasm runtime that an exception was thrown. The caller owns
 * the returned trap.
 *
 * \param store the store context
 * \param exn the exception to throw (ownership transferred)
 * \return a trap to return from the host callback (caller-owned)
 */
WASM_API_EXTERN wasm_trap_t *
wasmtime_context_set_exception(wasmtime_context_t *store, wasmtime_exn_t *exn);

/**
 * \brief Takes the pending exception from the store, if any.
 *
 * If there is a pending exception, removes it from the store and
 * returns it. The caller owns the returned pointer and must free it
 * with #wasmtime_exn_delete.
 *
 * \param store the store context
 * \param exn_ret on success, set to the exception (caller-owned)
 *
 * \return true if there was a pending exception, false otherwise.
 */
WASM_API_EXTERN bool wasmtime_context_take_exception(wasmtime_context_t *store,
                                                     wasmtime_exn_t **exn_ret);

/**
 * \brief Tests whether there is a pending exception on the store.
 *
 * \param store the store context
 *
 * \return true if a pending exception is set, false otherwise.
 */
WASM_API_EXTERN bool wasmtime_context_has_exception(wasmtime_context_t *store);

#ifdef __cplusplus
} // extern "C"
#endif

#endif // WASMTIME_EXN_H
