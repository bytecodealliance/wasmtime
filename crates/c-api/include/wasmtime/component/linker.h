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
 * \brief Returns the "root instance" of this linker, used to define names into
 * the root namespace.
 *
 * \warning This acquires exclusive access to the \p linker. The \p linker
 * *MUST* not be accessed by anything until the returned
 * #wasmtime_component_linker_instance_t in \p linker_instance_out is destroyed
 * by #wasmtime_component_linker_instance_delete.
 */
WASM_API_EXTERN wasmtime_component_linker_instance_t *
wasmtime_component_linker_root(wasmtime_component_linker_t *linker);

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

/**
 * \brief Defines a nested instance within this instance.
 *
 * This can be used to describe arbitrarily nested levels of instances within a
 * linker to satisfy nested instance exports of components.
 *
 * \warning This acquires exclusive access to the \p linker_instance. The \p
 * linker_instance *MUST* not be accessed by anything until the returned
 * #wasmtime_component_linker_instance_t in \p linker_instance_out is destroyed
 * by #wasmtime_component_linker_instance_delete.
 *
 * \param linker_instance the linker instance from which the new one is created
 * \param name new instance name
 * \param linker_instance_out on success, the new #component_linker_instance_t
 * \return on success `NULL`, otherwise an error
 */
WASM_API_EXTERN wasmtime_error_t *
wasmtime_component_linker_instance_add_instance(
    wasmtime_component_linker_instance_t *linker_instance, const char *name,
    wasmtime_component_linker_instance_t **linker_instance_out);

/**
 * \brief Defines a #wasmtime_module_t within this instance.
 *
 * This can be used to provide a core wasm #wasmtime_module_t as an import
 * to a component. The #wasmtime_module_t provided is saved within the
 * linker for the specified \p name in this instance.
 *
 * \param linker_instance the instance to define the module in
 * \param name the module name
 * \param module the module
 * \return on success `NULL`, otherwise an error
 */
WASM_API_EXTERN wasmtime_error_t *wasmtime_component_linker_instance_add_module(
    wasmtime_component_linker_instance_t *linker_instance, const char *name,
    const wasmtime_module_t *module);

/**
 * \brief Deletes a #wasmtime_component_linker_instance_t
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
