/**
 * \file wasmtime.h
 */

#ifndef WASMTIME_API_H
#define WASMTIME_API_H

#include <wasmtime/config.h>
#include <wasmtime/error.h>
#include <wasmtime/extern.h>
#include <wasmtime/func.h>
#include <wasmtime/global.h>
#include <wasmtime/instance.h>
#include <wasmtime/linker.h>
#include <wasmtime/memory.h>
#include <wasmtime/module.h>
#include <wasmtime/store.h>
#include <wasmtime/table.h>
#include <wasmtime/trap.h>
#include <wasmtime/val.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * \brief Converts from the text format of WebAssembly to to the binary format.
 *
 * \param wat this it the input pointer with the WebAssembly Text Format inside of
 *   it. This will be parsed and converted to the binary format.
 * \param wat_len this it the length of `wat`, in bytes.
 * \param ret if the conversion is successful, this byte vector is filled in with
 *   the WebAssembly binary format.
 *
 * \return a non-null error if parsing fails, or returns `NULL`. If parsing
 * fails then `ret` isn't touched.
 *
 * This function does not take ownership of `wat`, and the caller is expected to
 * deallocate the returned #wasmtime_error_t and #wasm_byte_vec_t.
 */
WASM_API_EXTERN wasmtime_error_t* wasmtime_wat2wasm(
    const char *wat,
    size_t wat_len,
    wasm_byte_vec_t *ret
);

#ifdef __cplusplus
}  // extern "C"
#endif

#endif // WASMTIME_API_H
