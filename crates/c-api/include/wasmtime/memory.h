/**
 * \file wasmtime/memory.h
 *
 * Wasmtime API for interacting with wasm memories.
 */

#ifndef WASMTIME_MEMORY_H
#define WASMTIME_MEMORY_H

#include <wasm.h>
#include <wasmtime/extern.h>
#include <wasmtime/store.h>
#include <wasmtime/error.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * \brief Creates a new WebAssembly linear memory
 *
 * \param store the store to create the memory within
 * \param ty the type of the memory to create
 * \param ret where to store the returned memory
 *
 * If an error happens when creating the memory it's returned and owned by the
 * caller. If an error happens then `ret` is not filled in.
 */
WASM_API_EXTERN wasmtime_error_t *wasmtime_memory_new(
    wasmtime_context_t *store,
    const wasm_memorytype_t* ty,
    wasmtime_memory_t *ret
);

/**
 * \brief Returns the tyep of the memory specified
 */
WASM_API_EXTERN wasm_memorytype_t* wasmtime_memory_type(
    const wasmtime_context_t *store,
    const wasmtime_memory_t *memory
);

/**
 * \brief Returns the base pointer in memory where the linear memory starts.
 */
WASM_API_EXTERN uint8_t *wasmtime_memory_data(
    const wasmtime_context_t *store,
    const wasmtime_memory_t *memory
);

/**
 * \brief Returns the byte length of this linear memory.
 */
WASM_API_EXTERN size_t wasmtime_memory_data_size(
    const wasmtime_context_t *store,
    const wasmtime_memory_t *memory
);

/**
 * \brief Returns the length, in WebAssembly pages, of this linear memory
 */
WASM_API_EXTERN uint32_t wasmtime_memory_size(
    const wasmtime_context_t *store,
    const wasmtime_memory_t *memory
);

/**
 * \brief Attempts to grow the specified memory by `delta` pages.
 *
 * \param store the store that owns `memory`
 * \param memory the memory to grow
 * \param delta the number of pages to grow by
 * \param prev_size where to store the previous size of memory
 *
 * If memory cannot be grown then `prev_size` is left unchanged and an error is
 * returned. Otherwise `prev_size` is set to the previous size of the memory, in
 * WebAssembly pages, and `NULL` is returned.
 */
WASM_API_EXTERN wasmtime_error_t *wasmtime_memory_grow(
    wasmtime_context_t *store,
    const wasmtime_memory_t *memory,
    uint32_t delta,
    uint32_t *prev_size
);

#ifdef __cplusplus
}  // extern "C"
#endif

#endif // WASMTIME_MEMORY_H
