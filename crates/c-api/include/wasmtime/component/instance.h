#ifndef WASMTIME_COMPONENT_INSTANCE_H
#define WASMTIME_COMPONENT_INSTANCE_H

#include <wasmtime/component/func.h>
#include <wasmtime/conf.h>
#include <wasmtime/store.h>

#ifdef WASMTIME_FEATURE_COMPONENT_MODEL

#ifdef __cplusplus
extern "C" {
#endif

/// \brief Representation of a instance in Wasmtime.
///
/// Instances are represented with a 64-bit identifying integer in Wasmtime.
/// They do not have any destructor associated with them. Instances cannot
/// interoperate between #wasmtime_store_t instances and if the wrong instance
/// is passed to the wrong store then it may trigger an assertion to abort the
/// process.
typedef struct wasmtime_component_instance {
  /// Internal identifier of what store this belongs to, never zero.
  uint64_t store_id;
  /// Internal index within the store.
  size_t index;
} wasmtime_component_instance_t;

/**
 * \brief Looks up an exported function by name within this
 * #wasmtime_component_instance_t.
 *
 * \param instance the instance to look up this name in
 * \param context the store that \p instance lives in
 * \param name the name of the function to look up
 * \param func_out if found, the function corresponding to \p name
 * \return boolean marking if a function for \p name was found
 */
WASM_API_EXTERN bool wasmtime_component_instance_get_func(
    const wasmtime_component_instance_t *instance, wasmtime_context_t *context,
    const char *name, wasmtime_component_func_t *func_out);

#ifdef __cplusplus
} // extern "C"
#endif

#endif // WASMTIME_FEATURE_COMPONENT_MODEL

#endif // WASMTIME_COMPONENT_INSTANCE_H
