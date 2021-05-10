#ifndef WASMTIME_TABLE_H
#define WASMTIME_TABLE_H

#include <wasm.h>
#include <wasmtime/extern.h>
#include <wasmtime/store.h>
#include <wasmtime/error.h>
#include <wasmtime/val.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * \brief Creates a new host-defined wasm table.
 *
 * This function is the same as #wasm_table_new except that it's specialized for
 * funcref tables by taking a `wasm_func_t` initialization value. Additionally
 * it returns errors via #wasmtime_error_t.
 *
 * This function does not take ownership of any of its parameters, but yields
 * ownership of returned values (the table and error).
 */
WASM_API_EXTERN wasmtime_error_t *wasmtime_table_new(
    wasmtime_context_t *store,
    const wasm_tabletype_t *element_ty,
    wasmtime_val_t *init,
    wasmtime_table_t *table
);

WASM_API_EXTERN wasm_tabletype_t* wasmtime_table_type(
    const wasmtime_context_t *store,
    wasmtime_table_t table
);

/**
 * \brief Gets a value in a table.
 *
 * This function is the same as #wasm_table_get except that it's specialized for
 * funcref tables by returning a `wasm_func_t` value. Additionally a `bool`
 * return value indicates whether the `index` provided was in bounds.
 *
 * This function does not take ownership of any of its parameters, but yields
 * ownership of returned #wasm_func_t.
 */
WASM_API_EXTERN bool wasmtime_table_get(
    wasmtime_context_t *store,
    wasmtime_table_t table,
    uint32_t index,
    wasmtime_val_t *val
);

/**
 * \brief Sets a value in a table.
 *
 * This function is similar to #wasm_table_set, but has a few differences:
 *
 * * An error is returned through #wasmtime_error_t describing erroneous
 *   situations.
 * * The value being set is specialized to #wasm_func_t.
 *
 * This function does not take ownership of any of its parameters, but yields
 * ownership of returned #wasmtime_error_t.
 */
WASM_API_EXTERN wasmtime_error_t *wasmtime_table_set(
    wasmtime_context_t *store,
    wasmtime_table_t table,
    uint32_t index,
    const wasmtime_val_t *value
);

WASM_API_EXTERN uint32_t wasmtime_table_size(
    const wasmtime_context_t *store,
    wasmtime_table_t table
);

/**
 * \brief Grows a table.
 *
 * This function is similar to #wasm_table_grow, but has a few differences:
 *
 * * An error is returned through #wasmtime_error_t describing erroneous
 *   situations.
 * * The initialization value is specialized to #wasm_func_t.
 * * The previous size of the table is returned through `prev_size`.
 *
 * This function does not take ownership of any of its parameters, but yields
 * ownership of returned #wasmtime_error_t.
 */
WASM_API_EXTERN wasmtime_error_t *wasmtime_table_grow(
    wasmtime_context_t *store,
    wasmtime_table_t table,
    uint32_t delta,
    const wasmtime_val_t *init,
    wasm_table_size_t *prev_size
);

#ifdef __cplusplus
}  // extern "C"
#endif

#endif // WASMTIME_TABLE_H

