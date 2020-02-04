// WebAssembly C API extension for Wasmtime

#ifndef WASMTIME_API_H
#define WASMTIME_API_H

#include <wasm.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef uint8_t wasmtime_strategy_t;
enum wasmtime_strategy_enum { // Strategy
  WASMTIME_STRATEGY_AUTO,
  WASMTIME_STRATEGY_CRANELIFT,
  WASMTIME_STRATEGY_LIGHTBEAM,
};

typedef uint8_t wasmtime_opt_level_t;
enum wasmtime_opt_level_enum { // OptLevel
  WASMTIME_OPT_LEVEL_NONE,
  WASMTIME_OPT_LEVEL_SPEED,
  WASMTIME_OPT_LEVEL_SPEED_AND_SIZE,
};

#define WASMTIME_CONFIG_PROP(name, ty) \
    WASM_API_EXTERN void wasmtime_config_##name##_set(wasm_config_t*, ty);

WASMTIME_CONFIG_PROP(debug_info, bool)
WASMTIME_CONFIG_PROP(wasm_threads, bool)
WASMTIME_CONFIG_PROP(wasm_reference_types, bool)
WASMTIME_CONFIG_PROP(wasm_simd, bool)
WASMTIME_CONFIG_PROP(wasm_bulk_memory, bool)
WASMTIME_CONFIG_PROP(wasm_multi_value, bool)
WASMTIME_CONFIG_PROP(strategy, wasmtime_strategy_t)
WASMTIME_CONFIG_PROP(cranelift_debug_verifier, bool)
WASMTIME_CONFIG_PROP(cranelift_opt_level, wasmtime_opt_level_t)

///////////////////////////////////////////////////////////////////////////////

#ifdef __cplusplus
}  // extern "C"
#endif

#endif // WASMTIME_API_H
