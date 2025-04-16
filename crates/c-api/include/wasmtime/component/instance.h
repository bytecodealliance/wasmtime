#ifndef WASMTIME_COMPONENT_INSTANCE_H
#define WASMTIME_COMPONENT_INSTANCE_H

#ifdef WASMTIME_FEATURE_COMPONENT_MODEL

#ifdef __cplusplus
extern "C" {
#endif

typedef struct wasmtime_component_instance_t wasmtime_component_instance_t;

/**
 * \brief Deletes a #wasmtime_component_instance_t created by
 * #wasmtime_component_linker_instantiate
 *
 * \param instance the #wasmtime_component_instance_t to delete
 */
WASM_API_EXTERN void
wasmtime_component_instance_delete(wasmtime_component_instance_t *instance);

#ifdef __cplusplus
} // extern "C"
#endif

#endif // WASMTIME_FEATURE_COMPONENT_MODEL

#endif // WASMTIME_COMPONENT_INSTANCE_H
