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

typedef struct wasmtime_component_linker_instance_t
    wasmtime_component_linker_instance_t;

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
 * \brief Creates a builder, #wasmtime_component_linker_instance_t, for the
 * specified named instance
 *
 * \param linker the linker in which to build out the instance in
 * \param name the instance name
 * \param linker_instance_out on success, the
 *        #wasmtime_component_linker_instance_t
 *
 * \return wasmtime_error_t* on success `NULL` is returned, otherwise an error
 *         is returned which describes why the build failed.
 *
 * \note This mutably borrows the provided linker, meaning nothing else should
 * access the linker until the returned #wasmtime_component_linker_instance_t is
 * deleted. The linker also needs to stay alive as long as the returned
 * #wasmtime_component_linker_instance_t is alive.
 */
WASM_API_EXTERN wasmtime_error_t *wasmtime_component_linker_instance(
    wasmtime_component_linker_t *linker, const char *name,
    wasmtime_component_linker_instance_t **linker_instance_out);

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
    wasmtime_component_instance_t **instance_out);

/**
 * \brief Deletes a #wasmtime_component_linker_t created by
 * #wasmtime_component_linker_new
 *
 * \param linker the #wasmtime_component_linker_t to delete
 */
WASM_API_EXTERN void
wasmtime_component_linker_delete(wasmtime_component_linker_t *linker);

/**
 * \brief Deletes a #wasmtime_component_linker_instance_t created by
 * #wasmtime_component_linker_instance
 *
 * \param linker_instance the #wasmtime_component_linker_instance_t to delete
 */
WASM_API_EXTERN void wasmtime_component_linker_instance_delete(
    wasmtime_component_linker_instance_t *linker_instance);

#ifdef __cplusplus
} // extern "C"
#endif

#endif // WASMTIME_FEATURE_COMPONENT_MODEL

#endif // WASMTIME_COMPONENT_LINKER_H
