#ifndef WASMTIME_EXTERN_H
#define WASMTIME_EXTERN_H

#include <wasmtime/module.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef uint64_t wasmtime_func_t;
typedef uint64_t wasmtime_table_t;
typedef uint64_t wasmtime_memory_t;
typedef uint64_t wasmtime_instance_t;
typedef uint64_t wasmtime_global_t;

typedef enum wasmtime_extern_kind {
    WASMTIME_EXTERN_FUNC,
    WASMTIME_EXTERN_GLOBAL,
    WASMTIME_EXTERN_TABLE,
    WASMTIME_EXTERN_MEMORY,
    WASMTIME_EXTERN_INSTANCE,
    WASMTIME_EXTERN_MODULE,
} wasmtime_extern_kind_t;

typedef union wasmtime_extern_union {
    wasmtime_func_t func;
    wasmtime_global_t global;
    wasmtime_table_t table;
    wasmtime_memory_t memory;
    wasmtime_instance_t instance;
    wasmtime_module_t *module;
} wasmtime_extern_union_t;

typedef struct wasmtime_extern {
    wasmtime_extern_kind_t kind;
    wasmtime_extern_union_t of;
} wasmtime_extern_t;

void wasmtime_extern_delete(wasmtime_extern_t *val);

#ifdef __cplusplus
}  // extern "C"
#endif

#endif // WASMTIME_EXTERN_H

