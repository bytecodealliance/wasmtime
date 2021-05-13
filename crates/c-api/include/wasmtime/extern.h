/**
 * \file wasmtime/extern.h
 *
 * TODO
 */

#ifndef WASMTIME_EXTERN_H
#define WASMTIME_EXTERN_H

#include <wasmtime/module.h>
#include <wasmtime/store.h>

#ifdef __cplusplus
extern "C" {
#endif

/// TODO
typedef uint64_t wasmtime_func_t;
/// TODO
typedef uint64_t wasmtime_table_t;
/// TODO
typedef uint64_t wasmtime_memory_t;
/// TODO
typedef uint64_t wasmtime_instance_t;
/// TODO
typedef uint64_t wasmtime_global_t;

/// TODO
typedef uint8_t wasmtime_extern_kind_t;
/// TODO
#define WASMTIME_EXTERN_FUNC 0
/// TODO
#define WASMTIME_EXTERN_GLOBAL 1
/// TODO
#define WASMTIME_EXTERN_TABLE 2
/// TODO
#define WASMTIME_EXTERN_MEMORY 3
/// TODO
#define WASMTIME_EXTERN_INSTANCE 4
/// TODO
#define WASMTIME_EXTERN_MODULE 5

/**
 * TODO
 */
typedef union wasmtime_extern_union {
    /**
     * TODO
     */
    wasmtime_func_t func;
    /**
     * TODO
     */
    wasmtime_global_t global;
    /**
     * TODO
     */
    wasmtime_table_t table;
    /**
     * TODO
     */
    wasmtime_memory_t memory;
    /**
     * TODO
     */
    wasmtime_instance_t instance;
    /**
     * TODO
     */
    wasmtime_module_t *module;
} wasmtime_extern_union_t;

/**
 * TODO
 */
typedef struct wasmtime_extern {
    /**
     * TODO
     */
    wasmtime_extern_kind_t kind;
    /**
     * TODO
     */
    wasmtime_extern_union_t of;
} wasmtime_extern_t;

/// TODO
void wasmtime_extern_delete(wasmtime_extern_t *val);

/// TODO
wasm_externtype_t *wasmtime_extern_type(wasmtime_context_t *context, wasmtime_extern_t *val);

#ifdef __cplusplus
}  // extern "C"
#endif

#endif // WASMTIME_EXTERN_H

