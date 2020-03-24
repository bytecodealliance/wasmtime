// WebAssembly C API extension for Wasmtime

#ifndef WASMTIME_API_H
#define WASMTIME_API_H

#include <wasm.h>
#include <wasi.h>

#ifdef __cplusplus
extern "C" {
#endif

#define own

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

typedef uint8_t wasmtime_profiling_strategy_t;
enum wasmtime_profiling_strategy_t { // ProfilingStrategy
  WASMTIME_PROFILING_STRATEGY_NONE,
  WASMTIME_PROFILING_STRATEGY_JITDUMP,
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
WASMTIME_CONFIG_PROP(profiler, wasmtime_profiling_strategy_t)

///////////////////////////////////////////////////////////////////////////////

// Converts from the text format of WebAssembly to to the binary format.
//
// * `engine` - a previously created engine which will drive allocations and
//   such
// * `wat` - this it the input buffer with the WebAssembly Text Format inside of
//   it. This will be parsed and converted to the binary format.
// * `ret` - if the conversion is successful, this byte vector is filled in with
//   the WebAssembly binary format.
// * `error_message` - if the conversion fails, this is filled in with a
//   descriptive error message of why parsing failed. This parameter is
//   optional.
//
// Returns `true` if conversion succeeded, or `false` if it failed.
WASM_API_EXTERN bool wasmtime_wat2wasm(
    wasm_engine_t *engine,
    const wasm_byte_vec_t *wat,
    own wasm_byte_vec_t *ret,
    own wasm_byte_vec_t *error_message
);

#define WASMTIME_DECLARE_OWN(name) \
  typedef struct wasmtime_##name##_t wasmtime_##name##_t; \
  \
  WASM_API_EXTERN void wasmtime_##name##_delete(own wasmtime_##name##_t*);

WASMTIME_DECLARE_OWN(linker)

WASM_API_EXTERN own wasmtime_linker_t* wasmtime_linker_new(wasm_store_t* store);

WASM_API_EXTERN bool wasmtime_linker_define(
    wasmtime_linker_t *linker,
    const wasm_name_t *module,
    const wasm_name_t *name,
    const wasm_extern_t *item
);

WASM_API_EXTERN bool wasmtime_linker_define_wasi(
    wasmtime_linker_t *linker,
    const wasi_instance_t *instance
);

WASM_API_EXTERN bool wasmtime_linker_define_instance(
    wasmtime_linker_t *linker,
    const wasm_name_t *name,
    const wasm_instance_t *instance
);

WASM_API_EXTERN wasm_instance_t* wasmtime_linker_instantiate(
    const wasmtime_linker_t *linker,
    const wasm_module_t *module,
    own wasm_trap_t **trap
);

#undef own

#ifdef __cplusplus
}  // extern "C"
#endif

#endif // WASMTIME_API_H
