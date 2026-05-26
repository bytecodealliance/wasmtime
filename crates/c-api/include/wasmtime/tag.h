/**
 * \file wasmtime/tag.h
 *
 * \brief Wasmtime APIs for interacting with WebAssembly tags.
 *
 * Tags are used to create and identify exception objects. A tag describes
 * the signature (payload types) of exceptions created with it.
 */

#ifndef WASMTIME_TAG_H
#define WASMTIME_TAG_H

#include <wasm.h>
#include <wasmtime/error.h>
#include <wasmtime/store.h>

#ifdef __cplusplus
extern "C" {
#endif

/// \brief Representation of a tag in Wasmtime.
///
/// Tags in Wasmtime are represented as an index into a store and don't
/// have any data or destructor associated with the #wasmtime_tag_t value.
/// Tags cannot interoperate between #wasmtime_store_t instances and if the
/// wrong tag is passed to the wrong store then it may trigger an assertion
/// to abort the process.
typedef struct wasmtime_tag {
  struct {
    /// Internal identifier of what store this belongs to, never zero.
    uint64_t store_id;
    /// Private field for Wasmtime.
    uint32_t __private1;
  };
  /// Private field for Wasmtime.
  uint32_t __private2;
} wasmtime_tag_t;

/// \brief Value of #wasmtime_extern_kind_t meaning that #wasmtime_extern_t is a
/// tag
#define WASMTIME_EXTERN_TAG 5

/**
 * \brief Creates a new host-defined tag.
 *
 * \param store the store in which to create the tag
 * \param tt the tag type that describes the tag's exception payload
 * \param ret on success, filled with the new tag
 *
 * \return NULL on success, otherwise an error describing the failure.
 */
WASM_API_EXTERN wasmtime_error_t *wasmtime_tag_new(wasmtime_context_t *store,
                                                   const wasm_tagtype_t *tt,
                                                   wasmtime_tag_t *ret);

/**
 * \brief Returns the type of the given tag.
 *
 * The returned #wasm_tagtype_t is owned by the caller.
 */
WASM_API_EXTERN wasm_tagtype_t *
wasmtime_tag_type(const wasmtime_context_t *store, const wasmtime_tag_t *tag);

/**
 * \brief Tests whether two tags are identical (same definition).
 */
WASM_API_EXTERN bool wasmtime_tag_eq(const wasmtime_context_t *store,
                                     const wasmtime_tag_t *a,
                                     const wasmtime_tag_t *b);

#ifdef __cplusplus
} // extern "C"
#endif

#endif // WASMTIME_TAG_H
