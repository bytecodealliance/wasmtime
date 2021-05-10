#ifndef WASMTIME_MEMORY_H
#define WASMTIME_MEMORY_H

#include <wasm.h>
#include <wasmtime/extern.h>
#include <wasmtime/store.h>
#include <wasmtime/error.h>

#ifdef __cplusplus
extern "C" {
#endif

WASM_API_EXTERN wasmtime_error_t *wasmtime_memory_new(
    wasmtime_context_t *store,
    const wasm_memorytype_t* ty,
    wasmtime_memory_t *ret
);

WASM_API_EXTERN wasm_memorytype_t* wasmtime_memory_type(
    const wasmtime_context_t *store,
    wasmtime_memory_t memory
);

WASM_API_EXTERN uint8_t *wasmtime_memory_data(
    const wasmtime_context_t *store,
    wasmtime_memory_t memory
);
WASM_API_EXTERN size_t *wasmtime_memory_data_size(
    const wasmtime_context_t *store,
    wasmtime_memory_t memory
);
WASM_API_EXTERN uint32_t *wasmtime_memory_size(
    const wasmtime_context_t *store,
    wasmtime_memory_t memory
);
WASM_API_EXTERN wasmtime_error_t *wasmtime_memory_grow(
    wasmtime_context_t *store,
    wasmtime_memory_t memory,
    uint32_t delta,
    uint32_t *prev_size
);

#ifdef __cplusplus
}  // extern "C"
#endif

#endif // WASMTIME_MEMORY_H
