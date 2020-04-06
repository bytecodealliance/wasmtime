// WebAssembly C API extension for Wasmtime

#ifndef WASMTIME_API_H
#define WASMTIME_API_H

#include <wasm.h>
#include <wasi.h>

#ifdef __cplusplus
extern "C" {
#endif

#define own

#define WASMTIME_DECLARE_OWN(name) \
  typedef struct wasmtime_##name##_t wasmtime_##name##_t; \
  \
  WASM_API_EXTERN void wasmtime_##name##_delete(own wasmtime_##name##_t*);

WASMTIME_DECLARE_OWN(error)

WASM_API_EXTERN void wasmtime_error_message(
    const wasmtime_error_t *error,
    own wasm_name_t *message
);

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
  WASMTIME_PROFILING_STRATEGY_VTUNE,
};

#define WASMTIME_CONFIG_PROP(ret, name, ty) \
    WASM_API_EXTERN ret wasmtime_config_##name##_set(wasm_config_t*, ty);

WASMTIME_CONFIG_PROP(void, debug_info, bool)
WASMTIME_CONFIG_PROP(void, wasm_threads, bool)
WASMTIME_CONFIG_PROP(void, wasm_reference_types, bool)
WASMTIME_CONFIG_PROP(void, wasm_simd, bool)
WASMTIME_CONFIG_PROP(void, wasm_bulk_memory, bool)
WASMTIME_CONFIG_PROP(void, wasm_multi_value, bool)
WASMTIME_CONFIG_PROP(wasmtime_error_t*, strategy, wasmtime_strategy_t)
WASMTIME_CONFIG_PROP(void, cranelift_debug_verifier, bool)
WASMTIME_CONFIG_PROP(void, cranelift_opt_level, wasmtime_opt_level_t)
WASMTIME_CONFIG_PROP(wasmtime_error_t*, profiler, wasmtime_profiling_strategy_t)

///////////////////////////////////////////////////////////////////////////////

// Converts from the text format of WebAssembly to to the binary format.
//
// * `wat` - this it the input buffer with the WebAssembly Text Format inside of
//   it. This will be parsed and converted to the binary format.
// * `ret` - if the conversion is successful, this byte vector is filled in with
//   the WebAssembly binary format.
//
// Returns a non-null error if parsing fails, or returns `NULL`. If parsing
// fails then `ret` isn't touched.
WASM_API_EXTERN own wasmtime_error_t* wasmtime_wat2wasm(
    const wasm_byte_vec_t *wat,
    own wasm_byte_vec_t *ret
);

///////////////////////////////////////////////////////////////////////////////
//
// wasmtime_linker_t extension type, binding the `Linker` type in the Rust API

WASMTIME_DECLARE_OWN(linker)

WASM_API_EXTERN own wasmtime_linker_t* wasmtime_linker_new(wasm_store_t* store);

WASM_API_EXTERN void wasmtime_linker_allow_shadowing(wasmtime_linker_t* linker, bool allow_shadowing);

WASM_API_EXTERN own wasmtime_error_t* wasmtime_linker_define(
    wasmtime_linker_t *linker,
    const wasm_name_t *module,
    const wasm_name_t *name,
    const wasm_extern_t *item
);

WASM_API_EXTERN own wasmtime_error_t* wasmtime_linker_define_wasi(
    wasmtime_linker_t *linker,
    const wasi_instance_t *instance
);

WASM_API_EXTERN own wasmtime_error_t* wasmtime_linker_define_instance(
    wasmtime_linker_t *linker,
    const wasm_name_t *name,
    const wasm_instance_t *instance
);

WASM_API_EXTERN own wasmtime_error_t* wasmtime_linker_instantiate(
    const wasmtime_linker_t *linker,
    const wasm_module_t *module,
    own wasm_instance_t **instance,
    own wasm_trap_t **trap
);

///////////////////////////////////////////////////////////////////////////////
//
// wasmtime_caller_t extension, binding the `Caller` type in the Rust API

typedef struct wasmtime_caller_t wasmtime_caller_t;

typedef own wasm_trap_t* (*wasmtime_func_callback_t)(const wasmtime_caller_t* caller, const wasm_val_t args[], wasm_val_t results[]);
typedef own wasm_trap_t* (*wasmtime_func_callback_with_env_t)(const wasmtime_caller_t* caller, void* env, const wasm_val_t args[], wasm_val_t results[]);

WASM_API_EXTERN own wasm_func_t* wasmtime_func_new(wasm_store_t*, const wasm_functype_t*, wasmtime_func_callback_t callback);

WASM_API_EXTERN own wasm_func_t* wasmtime_func_new_with_env(
  wasm_store_t* store,
  const wasm_functype_t* type,
  wasmtime_func_callback_with_env_t callback,
  void* env,
  void (*finalizer)(void*)
);

WASM_API_EXTERN own wasm_extern_t* wasmtime_caller_export_get(const wasmtime_caller_t* caller, const wasm_name_t* name);

///////////////////////////////////////////////////////////////////////////////
//
// Extensions to `wasm_frame_t`

WASM_API_EXTERN const wasm_name_t *wasmtime_frame_func_name(const wasm_frame_t*);
WASM_API_EXTERN const wasm_name_t *wasmtime_frame_module_name(const wasm_frame_t*);

///////////////////////////////////////////////////////////////////////////////
//
// Extensions to the C API which augment existing functionality with extra
// error reporting, safety, etc.

// Similar to `wasm_func_call`, but with a few tweaks:
//
// * `args` and `results` have a size parameter saying how big the arrays are
// * An error *and* a trap can be returned
// * Errors are returned if `args` have the wrong types, if the args/results
//   arrays have the wrong lengths, or if values come from the wrong store.
//
// The are three possible return states from this function:
//
// 1. The returned error is non-null. This means `results`
//    wasn't written to and `trap` will have `NULL` written to it. This state
//    means that programmer error happened when calling the function (e.g. the
//    size of the args/results were wrong)
// 2. The trap pointer is filled in. This means the returned error is `NULL` and
//    `results` was not written to. This state means that the function was
//    executing but hit a wasm trap while executing.
// 3. The error and trap returned are both `NULL` and `results` are written to.
//    This means that the function call worked and the specified results were
//    produced.
//
// The `trap` pointer cannot be `NULL`. The `args` and `results` pointers may be
// `NULL` if the corresponding length is zero.
WASM_API_EXTERN own wasmtime_error_t *wasmtime_func_call(
    wasm_func_t *func,
    const wasm_val_t *args,
    size_t num_args,
    wasm_val_t *results,
    size_t num_results,
    own wasm_trap_t **trap
);

// Similar to `wasm_global_new`, but with a few tweaks:
//
// * An error is returned instead of `wasm_global_t`, which is taken as an
//   out-parameter
// * An error happens when the `type` specified does not match the type of the
//   value `val`, or if it comes from a different store than `store`.
WASM_API_EXTERN own wasmtime_error_t *wasmtime_global_new(
    wasm_store_t *store,
    const wasm_globaltype_t *type,
    const wasm_val_t *val,
    own wasm_global_t **ret
);

// Similar to `wasm_global_set`, but with an error that can be returned if the
// specified value does not come from the same store as this global, if the
// global is immutable, or if the specified value has the wrong type.
WASM_API_EXTERN own wasmtime_error_t *wasmtime_global_set(
    wasm_global_t *global,
    const wasm_val_t *val
);

// Similar to `wasm_instance_new`, but with tweaks:
//
// * An error message can be returned from this function.
// * The number of imports specified is passed as an argument
// * The `trap` pointer is required to not be NULL.
// * No `wasm_store_t` argument is required.
//
// The states of return values from this function are similar to
// `wasmtime_func_call` where an error can be returned meaning something like a
// link error in this context. A trap can be returned (meaning no error or
// instance is returned), or an instance can be returned (meaning no error or
// trap is returned).
WASM_API_EXTERN own wasmtime_error_t *wasmtime_instance_new(
    const wasm_module_t *module,
    const wasm_extern_t* const imports[],
    size_t num_imports,
    own wasm_instance_t **instance,
    own wasm_trap_t **trap
);

// Similar to `wasm_module_new`, but an error is returned to return a
// descriptive error message in case compilation fails.
WASM_API_EXTERN own wasmtime_error_t *wasmtime_module_new(
    wasm_store_t *store,
    const wasm_byte_vec_t *binary,
    own wasm_module_t **ret
);

// Similar to `wasm_module_validate`, but an error is returned to return a
// descriptive error message in case compilation fails.
WASM_API_EXTERN own wasmtime_error_t *wasmtime_module_validate(
    wasm_store_t *store,
    const wasm_byte_vec_t *binary
);

#undef own

#ifdef __cplusplus
}  // extern "C"
#endif

#endif // WASMTIME_API_H
