/**
 * \file wasmtime/types/exnref.h
 */

#ifndef WASMTIME_TYPES_EXNREF_H
#define WASMTIME_TYPES_EXNREF_H

#include <wasm.h>
#include <wasmtime/error.h>

#ifdef __cplusplus
extern "C" {
#endif

/// \brief A type of a WebAssembly exception.
typedef struct wasmtime_exn_type wasmtime_exn_type_t;

/// \brief Creates a new exception type with the given parameter types.
///
/// Fills in `out` on success and returns `NULL`. Otherwise returns an
/// error and does not modify `out`.
WASM_API_EXTERN wasmtime_error_t *
wasmtime_exn_type_new(const wasm_engine_t *engine,
                      const wasm_valtype_vec_t *params,
                      wasmtime_exn_type_t **out);

/// \brief Deletes an exception type.
WASM_API_EXTERN void wasmtime_exn_type_delete(wasmtime_exn_type_t *ty);

/// \brief Clones `ty`, returning a pointer that must be deleted with
/// `wasmtime_exn_type_delete`.
WASM_API_EXTERN wasmtime_exn_type_t *
wasmtime_exn_type_copy(const wasmtime_exn_type_t *ty);

/// \brief Returns tag type associated with this exception type.
WASM_API_EXTERN wasm_tagtype_t *
wasmtime_exn_type_tag_type(const wasmtime_exn_type_t *ty);

#ifdef __cplusplus
} // extern "C"
#endif

#endif // WASMTIME_TYPES_EXNREF_H
