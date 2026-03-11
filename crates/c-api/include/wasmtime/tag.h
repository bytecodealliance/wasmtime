/**
 * \file wasmtime/tag.h
 *
 * \brief Wasmtime APIs for WebAssembly exception tag types.
 *
 * This header defines the C API for `wasm_tagtype_t`, the type descriptor for
 * WebAssembly exception tags (wasm exception-handling proposal).  Because
 * `wasm.h` is vendored from the upstream wasm-c-api repository and does not
 * yet include tag-type support, the declarations live here instead.
 */

#ifndef WASMTIME_TAG_H
#define WASMTIME_TAG_H

#include <wasm.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * \brief Opaque type representing a WebAssembly exception tag type.
 *
 * A tag type describes the payload types of an exception tag — equivalent to
 * the parameter types of an associated function type (tags have no results).
 */
typedef struct wasm_tagtype_t wasm_tagtype_t;

/// \brief Value returned by #wasm_externtype_kind for exception tags.
///
/// This constant serves the same role as `WASM_EXTERN_FUNC` etc. in `wasm.h`
/// but is defined here because the vendored `wasm.h` does not yet include it.
#define WASM_EXTERN_TAG 4

/**
 * \brief Creates a new tag type with the given exception payload types.
 *
 * Takes ownership of \p params.  Returns an owned #wasm_tagtype_t that must be
 * freed with #wasm_tagtype_delete.
 */
WASM_API_EXTERN wasm_tagtype_t *wasm_tagtype_new(wasm_valtype_vec_t *params);

/// \brief Deletes a #wasm_tagtype_t.
WASM_API_EXTERN void wasm_tagtype_delete(wasm_tagtype_t *);

/// \brief Returns a copy of the given #wasm_tagtype_t (caller owns the result).
WASM_API_EXTERN wasm_tagtype_t *wasm_tagtype_copy(const wasm_tagtype_t *);

/**
 * \brief Returns the exception payload (parameter) types of the tag.
 *
 * Does not take ownership; the returned vector is valid for the lifetime of
 * the tag type.
 */
WASM_API_EXTERN const wasm_valtype_vec_t *wasm_tagtype_params(const wasm_tagtype_t *);

/// \brief Converts a #wasm_tagtype_t to a #wasm_externtype_t (borrowed).
WASM_API_EXTERN wasm_externtype_t *wasm_tagtype_as_externtype(wasm_tagtype_t *);
/// \brief Converts a const #wasm_tagtype_t to a const #wasm_externtype_t (borrowed).
WASM_API_EXTERN const wasm_externtype_t *wasm_tagtype_as_externtype_const(const wasm_tagtype_t *);

/// \brief Converts a #wasm_externtype_t to a #wasm_tagtype_t, or NULL if not a tag.
WASM_API_EXTERN wasm_tagtype_t *wasm_externtype_as_tagtype(wasm_externtype_t *);
/// \brief Converts a const #wasm_externtype_t to a const #wasm_tagtype_t, or NULL if not a tag.
WASM_API_EXTERN const wasm_tagtype_t *wasm_externtype_as_tagtype_const(const wasm_externtype_t *);

#ifdef __cplusplus
} // extern "C"
#endif

#endif // WASMTIME_TAG_H
