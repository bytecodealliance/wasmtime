/**
 * \file wasmtime/profiling.h
 *
 * \brief API for Wasmtime guest profiler
 */

#ifndef WASMTIME_PROFILING_H
#define WASMTIME_PROFILING_H

#include <wasm.h>
#include <wasmtime/conf.h>
#include <wasmtime/error.h>
#include <wasmtime/module.h>
#include <wasmtime/store.h>

#ifdef WASMTIME_FEATURE_PROFILING

#ifdef __cplusplus
extern "C" {
#endif

/**
 * \brief Collects basic profiling data for a single WebAssembly guest.
 *
 * To use this, you’ll need to arrange to call #wasmtime_guestprofiler_sample at
 * regular intervals while the guest is on the stack. The most straightforward
 * way to do that is to call it from a callback registered with
 * #wasmtime_store_epoch_deadline_callback.
 *
 * For more information see the Rust documentation at:
 * https://docs.wasmtime.dev/api/wasmtime/struct.GuestProfiler.html
 */
typedef struct wasmtime_guestprofiler wasmtime_guestprofiler_t;

/**
 * \brief Deletes profiler without finishing it.
 *
 * \param guestprofiler profiler that is being deleted
 */
WASM_API_EXTERN void wasmtime_guestprofiler_delete(
    /* own */ wasmtime_guestprofiler_t *guestprofiler);

/**
 * \typedef wasmtime_guestprofiler_modules_t
 * \brief Alias to #wasmtime_guestprofiler_modules
 *
 * \struct #wasmtime_guestprofiler_modules
 * \brief Tuple of name and module for passing into #wasmtime_guestprofiler_new
 * and #wasmtime_guestprofiler_module_set_new.
 */
typedef struct wasmtime_guestprofiler_modules {
  const wasm_name_t *name; //!< Name recorded in the profile.
  const wasmtime_module_t
      *mod; //!< Module that is being allowed to appear in captured stack trace.
} wasmtime_guestprofiler_modules_t;

/**
 * \brief Begin profiling a new guest.
 *
 * \param engine         engine in which to perform the profiling
 * \param module_name    name recorded in the profile
 * \param interval_nanos intended sampling interval in nanoseconds recorded in
 *                       the profile
 * \param modules        modules and associated names that will appear in
 *                       captured stack traces, pointer to the first element
 * \param modules_len    count of elements in `modules`
 *
 * \return Created profiler that is owned by the caller.
 *
 * This function does not take ownership of the arguments.
 *
 * For more information see the Rust documentation at:
 * https://docs.wasmtime.dev/api/wasmtime/struct.GuestProfiler.html#method.new
 */
WASM_API_EXTERN /* own */ wasmtime_guestprofiler_t *wasmtime_guestprofiler_new(
    const wasm_engine_t *engine, const wasm_name_t *module_name,
    uint64_t interval_nanos, const wasmtime_guestprofiler_modules_t *modules,
    size_t modules_len);

/**
 * \brief A set of modules prepared once for use by any number of profilers.
 *
 * Preparing a module builds its symbol table, which demangles the name of
 * every function it defines; for modules with many functions this is most of
 * the cost of #wasmtime_guestprofiler_new, which repeats it for every profile.
 * Embedders that profile the same modules repeatedly can prepare them once
 * with #wasmtime_guestprofiler_module_set_new and start each profile with
 * #wasmtime_guestprofiler_new_with_module_set.
 *
 * A module set may be shared between threads, and profiles started from the
 * same set are independent of one another.
 *
 * For more information see the Rust documentation at:
 * https://docs.wasmtime.dev/api/wasmtime/struct.GuestProfilerModules.html
 */
typedef struct wasmtime_guestprofiler_module_set
    wasmtime_guestprofiler_module_set_t;

/**
 * \brief Prepares a set of modules for profiling.
 *
 * \param engine      engine the modules were created with
 * \param modules     modules and associated names that will appear in
 *                    captured stack traces, pointer to the first element
 * \param modules_len count of elements in `modules`
 * \param out         where to write the created module set, which is owned by
 *                    the caller, on success
 *
 * \return Returns #wasmtime_error_t owned by the caller in case of error,
 * `NULL` otherwise. An error is returned if `engine` has guest debugging
 * enabled.
 *
 * This function does not take ownership of `engine` or of the elements of
 * `modules`. See the Rust documentation for guidance on which modules should
 * not be included in `modules`.
 *
 * For more information see the Rust documentation at:
 * https://docs.wasmtime.dev/api/wasmtime/struct.GuestProfilerModules.html#method.new
 */
WASM_API_EXTERN /* own */ wasmtime_error_t *
wasmtime_guestprofiler_module_set_new(
    const wasm_engine_t *engine, const wasmtime_guestprofiler_modules_t *modules,
    size_t modules_len,
    /* own */ wasmtime_guestprofiler_module_set_t **out);

/**
 * \brief Creates a shallow clone of the specified module set, increasing the
 * internal reference count.
 */
WASM_API_EXTERN /* own */ wasmtime_guestprofiler_module_set_t *
wasmtime_guestprofiler_module_set_clone(
    const wasmtime_guestprofiler_module_set_t *set);

/**
 * \brief Deletes a module set.
 *
 * Profilers already started from the set are unaffected.
 *
 * \param set module set that is being deleted
 */
WASM_API_EXTERN void wasmtime_guestprofiler_module_set_delete(
    /* own */ wasmtime_guestprofiler_module_set_t *set);

/**
 * \brief Begin profiling a new guest using a prepared module set.
 *
 * \param module_name    name recorded in the profile
 * \param interval_nanos intended sampling interval in nanoseconds recorded in
 *                       the profile
 * \param modules        module set whose modules will appear in captured stack
 *                       traces
 *
 * \return Created profiler that is owned by the caller.
 *
 * This behaves like #wasmtime_guestprofiler_new, except that the per-module
 * preparation has already been done. This function does not take ownership of
 * the arguments; the profiler keeps its own reference to `modules`, which may
 * be deleted while the profiler is still in use.
 *
 * For more information see the Rust documentation at:
 * https://docs.wasmtime.dev/api/wasmtime/struct.GuestProfiler.html#method.new_with_modules
 */
WASM_API_EXTERN /* own */ wasmtime_guestprofiler_t *
wasmtime_guestprofiler_new_with_module_set(
    const wasm_name_t *module_name, uint64_t interval_nanos,
    const wasmtime_guestprofiler_module_set_t *modules);

/**
 * \brief Add a sample to the profile.
 *
 * \param guestprofiler the profiler the sample is being added to
 * \param store         store that is being used to collect the backtraces
 * \param delta_nanos   CPU time in nanoseconds that was used by this guest
 *                      since the previous sample
 *
 * Zero can be passed as `delta_nanos` if recording CPU usage information
 * is not needed.
 * This function does not take ownership of the arguments.
 *
 * For more information see the Rust documentation at:
 * https://docs.wasmtime.dev/api/wasmtime/struct.GuestProfiler.html#method.sample
 */
WASM_API_EXTERN void
wasmtime_guestprofiler_sample(wasmtime_guestprofiler_t *guestprofiler,
                              const wasmtime_store_t *store,
                              uint64_t delta_nanos);

/**
 * \brief Writes out the captured profile.
 *
 * \param guestprofiler the profiler which is being finished and deleted
 * \param out           pointer to where #wasm_byte_vec_t containing generated
 *                      file will be written
 *
 * \return Returns #wasmtime_error_t owned by the caller in case of error,
 * `NULL` otherwise.
 *
 * This function takes ownership of `guestprofiler`, even when error is
 * returned.
 * Only when returning without error `out` is filled with #wasm_byte_vec_t owned
 * by the caller.
 *
 * For more information see the Rust documentation at:
 * https://docs.wasmtime.dev/api/wasmtime/struct.GuestProfiler.html#method.finish
 */
WASM_API_EXTERN /* own */ wasmtime_error_t *
wasmtime_guestprofiler_finish(/* own */ wasmtime_guestprofiler_t *guestprofiler,
                              /* own */ wasm_byte_vec_t *out);

#ifdef __cplusplus
} // extern "C"
#endif

#endif // WASMTIME_FEATURE_PROFILING

#endif // WASMTIME_PROFILING_H
