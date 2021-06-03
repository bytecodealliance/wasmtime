/**
 * \file wasmtime/module.h
 *
 * APIs for interacting with modules in Wasmtime
 */

#ifndef WASMTIME_MODULE_H
#define WASMTIME_MODULE_H

#include <wasm.h>
#include <wasmtime/error.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * \brief An opaque object representing the type of a module.
 */
typedef struct wasmtime_moduletype wasmtime_moduletype_t;

/**
 * \brief Deletes a module type.
 */
WASM_API_EXTERN void wasmtime_moduletype_delete(wasmtime_moduletype_t *ty);

/**
 * \brief Returns the list of imports that this module type requires.
 *
 * This function does not take ownership of the provided module type but
 * ownership of `out` is passed to the caller. Note that `out` is treated as
 * uninitialized when passed to this function.
 */
WASM_API_EXTERN void wasmtime_moduletype_imports(const wasmtime_moduletype_t*, wasm_importtype_vec_t* out);

/**
 * \brief Returns the list of exports that this module type provides.
 *
 * This function does not take ownership of the provided module type but
 * ownership of `out` is passed to the caller. Note that `out` is treated as
 * uninitialized when passed to this function.
 */
WASM_API_EXTERN void wasmtime_moduletype_exports(const wasmtime_moduletype_t*, wasm_exporttype_vec_t* out);

/**
 * \brief Converts a #wasmtime_moduletype_t to a #wasm_externtype_t
 *
 * The returned value is owned by the #wasmtime_moduletype_t argument and should not
 * be deleted.
 */
WASM_API_EXTERN wasm_externtype_t* wasmtime_moduletype_as_externtype(wasmtime_moduletype_t*);

/**
 * \brief Attempts to convert a #wasm_externtype_t to a #wasmtime_moduletype_t
 *
 * The returned value is owned by the #wasmtime_moduletype_t argument and
 * should not be deleted. Returns `NULL` if the provided argument is not a
 * #wasmtime_moduletype_t.
 */
WASM_API_EXTERN wasmtime_moduletype_t* wasmtime_externtype_as_moduletype(wasm_externtype_t*);

/**
 * \typedef wasmtime_module_t
 * \brief Convenience alias for #wasmtime_module
 *
 * \struct wasmtime_module
 * \brief A compiled Wasmtime module.
 *
 * This type represents a compiled WebAssembly module. The compiled module is
 * ready to be instantiated and can be inspected for imports/exports. It is safe
 * to use a module across multiple threads simultaneously.
 */
typedef struct wasmtime_module wasmtime_module_t;

/**
 * \brief Compiles a WebAssembly binary into a #wasmtime_module_t
 *
 * This function will compile a WebAssembly binary into an owned #wasm_module_t.
 * This performs the same as #wasm_module_new except that it returns a
 * #wasmtime_error_t type to get richer error information.
 *
 * On success the returned #wasmtime_error_t is `NULL` and the `ret` pointer is
 * filled in with a #wasm_module_t. On failure the #wasmtime_error_t is
 * non-`NULL` and the `ret` pointer is unmodified.
 *
 * This function does not take ownership of any of its arguments, but the
 * returned error and module are owned by the caller.
 */
WASM_API_EXTERN wasmtime_error_t *wasmtime_module_new(
    wasm_engine_t *engine,
    const uint8_t *wasm,
    size_t wasm_len,
    wasmtime_module_t **ret
);

/**
 * \brief Deletes a module.
 */
WASM_API_EXTERN void wasmtime_module_delete(wasmtime_module_t *m);

/**
 * \brief Creates a shallow clone of the specified module, increasing the
 * internal reference count.
 */
WASM_API_EXTERN wasmtime_module_t *wasmtime_module_clone(wasmtime_module_t *m);

/**
 * \brief Validate a WebAssembly binary.
 *
 * This function will validate the provided byte sequence to determine if it is
 * a valid WebAssembly binary within the context of the engine provided.
 *
 * This function does not take ownership of its arguments but the caller is
 * expected to deallocate the returned error if it is non-`NULL`.
 *
 * If the binary validates then `NULL` is returned, otherwise the error returned
 * describes why the binary did not validate.
 */
WASM_API_EXTERN wasmtime_error_t *wasmtime_module_validate(
    wasm_engine_t *engine,
    const uint8_t *wasm,
    size_t wasm_len
);

/**
 * \brief Returns the type of this module.
 *
 * The returned #wasmtime_moduletype_t is expected to be deallocated by the
 * caller.
 */
WASM_API_EXTERN wasmtime_moduletype_t* wasmtime_module_type(const wasmtime_module_t*);

/**
 * \brief This function serializes compiled module artifacts as blob data.
 *
 * \param module the module
 * \param ret if the conversion is successful, this byte vector is filled in with
 *   the serialized compiled module.
 *
 * \return a non-null error if parsing fails, or returns `NULL`. If parsing
 * fails then `ret` isn't touched.
 *
 * This function does not take ownership of `module`, and the caller is
 * expected to deallocate the returned #wasmtime_error_t and #wasm_byte_vec_t.
 */
WASM_API_EXTERN wasmtime_error_t* wasmtime_module_serialize(
    wasmtime_module_t* module,
    wasm_byte_vec_t *ret
);

/**
 * \brief Build a module from serialized data.
 *
 * This function does not take ownership of any of its arguments, but the
 * returned error and module are owned by the caller.
 *
 * This function is not safe to receive arbitrary user input. See the Rust
 * documentation for more information on what inputs are safe to pass in here
 * (e.g. only that of #wasmtime_module_serialize)
 */
WASM_API_EXTERN wasmtime_error_t *wasmtime_module_deserialize(
    wasm_engine_t *engine,
    const uint8_t *bytes,
    size_t bytes_len,
    wasmtime_module_t **ret
);

#ifdef __cplusplus
}  // extern "C"
#endif

#endif // WASMTIME_MODULE_H
