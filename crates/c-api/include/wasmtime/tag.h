/**
 * \file wasmtime/tag.h
 *
 * \brief Wasmtime APIs for WebAssembly exception tag types.
 *
 * This header defines the C API for `wasmtime_tagtype_t`, the type descriptor
 * for WebAssembly exception tags (wasm exception-handling proposal).  Because
 * `wasm.h` is vendored from the upstream wasm-c-api repository and does not
 * yet include tag-type support, the declarations live here instead.
 */

#ifndef WASMTIME_TAG_H
#define WASMTIME_TAG_H

#include <stddef.h>
#include <wasm.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * \brief Opaque type representing a WebAssembly exception tag type.
 *
 * A tag type is described by a function type whose parameters are the exception
 * payload types and whose results are the tag's result types (currently always
 * empty, but reserved for the stack-switching proposal).
 */
typedef struct wasmtime_tagtype_t wasmtime_tagtype_t;

/**
 * \brief Value returned by #wasm_externtype_kind for exception tags.
 *
 * This constant extends the `WASM_EXTERN_*` range from `wasm.h` (0–3) with
 * tag support.  It is distinct from #WASMTIME_EXTERN_SHAREDMEMORY (which is
 * a discriminant for the runtime #wasmtime_extern_t union in
 * `wasmtime/extern.h`).
 */
#define WASMTIME_EXTERNTYPE_TAG 4

/**
 * \brief Creates a new tag type from the given function type.
 *
 * The function type describes the exception payload: its parameters are the
 * tag's exception payload types and its results are the tag's result types.
 * `engine` is used to resolve `functype` if it has not yet been interned.
 *
 * Returns an owned #wasmtime_tagtype_t that must be freed with
 * #wasmtime_tagtype_delete.
 */
WASM_API_EXTERN wasmtime_tagtype_t *
wasmtime_tagtype_new(wasm_engine_t *engine, const wasm_functype_t *functype);

/// \brief Deletes a #wasmtime_tagtype_t.
WASM_API_EXTERN void wasmtime_tagtype_delete(wasmtime_tagtype_t *);

/// \brief Returns a copy of the given #wasmtime_tagtype_t (caller owns the
/// result).
WASM_API_EXTERN wasmtime_tagtype_t *
wasmtime_tagtype_copy(const wasmtime_tagtype_t *);

/**
 * \brief Returns the function type describing this tag's exception payload.
 *
 * The caller owns the returned #wasm_functype_t and must free it with
 * #wasm_functype_delete.
 */
WASM_API_EXTERN wasm_functype_t *
wasmtime_tagtype_functype(const wasmtime_tagtype_t *);

/// \brief Converts a #wasmtime_tagtype_t to a #wasm_externtype_t (borrowed).
WASM_API_EXTERN wasm_externtype_t *
wasmtime_tagtype_as_externtype(wasmtime_tagtype_t *);

/// \brief Converts a const #wasmtime_tagtype_t to a const #wasm_externtype_t
/// (borrowed).
WASM_API_EXTERN const wasm_externtype_t *
wasmtime_tagtype_as_externtype_const(const wasmtime_tagtype_t *);

/// \brief Converts a #wasm_externtype_t to a #wasmtime_tagtype_t, or NULL if
/// not a tag.
WASM_API_EXTERN wasmtime_tagtype_t *
wasmtime_externtype_as_tagtype(wasm_externtype_t *);

/// \brief Converts a const #wasm_externtype_t to a const #wasmtime_tagtype_t,
/// or NULL if not a tag.
WASM_API_EXTERN const wasmtime_tagtype_t *
wasmtime_externtype_as_tagtype_const(const wasm_externtype_t *);

#ifdef __cplusplus
} // extern "C"
#endif

#endif // WASMTIME_TAG_H
