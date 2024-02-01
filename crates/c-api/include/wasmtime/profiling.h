/**
 * \file wasmtime/profiling.h
 *
 * \brief API for Wasmtime guest profiler
 */

#ifndef WASMTIME_PROFILING_H
#define WASMTIME_PROFILING_H

#include <wasm.h>
#include <wasmtime/error.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct wasmtime_guestprofiler_t wasmtime_guestprofiler_t;

WASM_API_EXTERN void wasmtime_guestprofiler_delete(
    /* own */ wasmtime_guestprofiler_t *guestprofiler);
WASM_API_EXTERN /* own */ wasmtime_guestprofiler_t *
wasmtime_guestprofiler_new(wasm_name_t *module_name, uint64_t interval_nanos,
                           size_t modules_size, wasm_name_t **modules_name,
                           wasmtime_module_t **modules_module);
WASM_API_EXTERN void
wasmtime_guestprofiler_sample(wasmtime_guestprofiler_t *guestprofiler,
                              wasmtime_store_t *store);
WASM_API_EXTERN /* own */ wasmtime_error_t *
wasmtime_guestprofiler_finish(/* own */ wasmtime_guestprofiler_t *guestprofiler,
                              wasm_byte_vec_t *out);

#ifdef __cplusplus
} // extern "C"
#endif

#endif // WASMTIME_PROFILING_H
