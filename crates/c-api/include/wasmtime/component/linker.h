#ifndef WASMTIME_COMPONENT_LINKER_H
#define WASMTIME_COMPONENT_LINKER_H

#include <wasm.h>
#include <wasmtime/component/component.h>
#include <wasmtime/component/instance.h>
#include <wasmtime/conf.h>
#include <wasmtime/error.h>
#include <wasmtime/store.h>

#ifdef WASMTIME_FEATURE_COMPONENT_MODEL

#ifdef __cplusplus
extern "C" {
#endif

typedef struct wasmtime_component_linker_t wasmtime_component_linker_t;

/**
 * \brief Creates a new #wasmtime_component_linker_t for the specified engine.
 *
 * \param engine the compilation environment and configuration
 *
 * \return a pointer to the newly created #wasmtime_component_linker_t
 */
WASM_API_EXTERN wasmtime_component_linker_t *
wasmtime_component_linker_new(const wasm_engine_t *engine);

/**
 * \brief Instantiates a component instance in a given #wasmtime_context_t
 *
 * \param linker a #wasmtime_component_linker_t that will help provide host
 *        functions
 * \param context the #wasmtime_context_t in which the instance should be
 *        created
 * \param component the #wasmtime_component_t to instantiate
 * \param instance_out on success, the instantiated
 *        #wasmtime_component_instance_t
 *
 * \return wasmtime_error_t* on success `NULL` is returned, otherwise an error
 *         is returned which describes why the build failed.
 */
WASM_API_EXTERN wasmtime_error_t *wasmtime_component_linker_instantiate(
    const wasmtime_component_linker_t *linker, wasmtime_context_t *context,
    const wasmtime_component_t *component,
    wasmtime_component_instance_t *instance_out);

/**
 * \brief Deletes a #wasmtime_component_linker_t created by
 * #wasmtime_component_linker_new
 *
 * \param linker the #wasmtime_component_linker_t to delete
 */
WASM_API_EXTERN void
wasmtime_component_linker_delete(wasmtime_component_linker_t *linker);

#ifdef __cplusplus
} // extern "C"
#endif

#endif // WASMTIME_FEATURE_COMPONENT_MODEL

#endif // WASMTIME_COMPONENT_LINKER_H
