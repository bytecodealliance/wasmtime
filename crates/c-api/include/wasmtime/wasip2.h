/// \file wasmtime/wasip2.h

#ifndef WASMTIME_WASIP2_H
#define WASMTIME_WASIP2_H

#include <wasmtime/conf.h>

#ifdef WASMTIME_FEATURE_WASI
#ifdef WASMTIME_FEATURE_COMPONENT_MODEL

#ifdef __cplusplus
extern "C" {
#endif

typedef struct wasmtime_wasip2_config_t wasmtime_wasip2_config_t;

WASM_API_EXTERN wasmtime_wasip2_config_t *wasmtime_wasip2_config_new();

WASM_API_EXTERN void
wasmtime_wasip2_config_inherit_stdout(wasmtime_wasip2_config_t *config);

WASM_API_EXTERN void
wasmtime_wasip2_config_delete(wasmtime_wasip2_config_t *config);

#ifdef __cplusplus
} // extern "C"
#endif

#endif // WASMTIME_FEATURE_COMPONENT_MODEL
#endif // WASMTIME_FEATURE_WASI

#endif // WASMTIME_WASIP2_H
