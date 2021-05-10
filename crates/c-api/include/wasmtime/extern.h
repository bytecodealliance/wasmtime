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

typedef uint8_t wasmtime_extern_kind_t;
#define WASMTIME_EXTERN_FUNC 0
#define WASMTIME_EXTERN_GLOBAL 1
#define WASMTIME_EXTERN_TABLE 2
#define WASMTIME_EXTERN_MEMORY 3
#define WASMTIME_EXTERN_INSTANCE 4
#define WASMTIME_EXTERN_MODULE 5

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

