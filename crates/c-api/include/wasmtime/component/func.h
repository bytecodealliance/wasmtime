#ifndef WASMTIME_COMPONENT_FUNC_H
#define WASMTIME_COMPONENT_FUNC_H

#include <wasmtime/component/val.h>
#include <wasmtime/conf.h>
#include <wasmtime/error.h>
#include <wasmtime/store.h>

#ifdef WASMTIME_FEATURE_COMPONENT_MODEL

#ifdef __cplusplus
extern "C" {
#endif

/// \brief Representation of a function in Wasmtime.
///
/// Functions in Wasmtime are represented as an index into a store and don't
/// have any data or destructor associated with the value. Functions cannot
/// interoperate between #wasmtime_store_t instances and if the wrong function
/// is passed to the wrong store then it may trigger an assertion to abort the
/// process.
typedef struct wasmtime_component_func {
  /// Internal identifier of what store this belongs to, never zero.
  uint64_t store_id;
  /// Internal index within the store.
  size_t index;
} wasmtime_component_func_t;

WASM_API_EXTERN wasmtime_error_t *wasmtime_component_func_call(
    const wasmtime_component_func_t *func, wasmtime_context_t *context,
    wasmtime_component_val_t *args, size_t args_size,
    wasmtime_component_val_t *results, size_t results_size);

#ifdef __cplusplus
} // extern "C"
#endif

#endif // WASMTIME_FEATURE_COMPONENT_MODEL

#endif // WASMTIME_COMPONENT_FUNC_H
