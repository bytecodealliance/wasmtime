/**
 * \file wasmtime/profiling.h
 *
 * \brief API for Wasmtime guest profiler
 */

#ifndef WASMTIME_PROFILING_H
#define WASMTIME_PROFILING_H

#include <wasm.h>
#include <wasmtime/error.h>

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
 * \brief Begin profiling a new guest.
 *
 * \param module_name    name recorded in the profile
 * \param interval_nanos intended sampling interval in nanoseconds recorded in
 *                       the profile
 * \param modules_len    count of tuples passed in `modules_name` and
 *                       `modules_module`
 * \param modules_name   names recorded in the profile associated with provided
 *                       modules, pointer to the first element
 * \param modules_module modules that will appear in captured stack traces,
 *                       pointer to the first element
 *
 * \return Created profiler that is owned by the caller.
 *
 * List of (#wasm_name_t*, #wasmtime_module_t*) tuples of `modules_len` length
 * is passed column-major in `modules_name` and `modules_module`. This function
 * does not take ownership of the arguments.
 *
 * For more information see the Rust documentation at:
 * https://docs.wasmtime.dev/api/wasmtime/struct.GuestProfiler.html#method.new
 */
WASM_API_EXTERN /* own */ wasmtime_guestprofiler_t *wasmtime_guestprofiler_new(
    const wasm_name_t *module_name, uint64_t interval_nanos, size_t modules_len,
    const wasm_name_t **modules_name, const wasmtime_module_t **modules_module);

/**
 * \brief Add a sample to the profile.
 *
 * \param guestprofiler the profiler the sample is being added to
 * \param store         that is being used to collect the backtraces
 *
 * This function does not take ownership of the arguments.
 *
 * For more information see the Rust documentation at:
 * https://docs.wasmtime.dev/api/wasmtime/struct.GuestProfiler.html#method.sample
 */
WASM_API_EXTERN void
wasmtime_guestprofiler_sample(wasmtime_guestprofiler_t *guestprofiler,
                              const wasmtime_store_t *store);

/**
 * \brief Writes out the captured profile.
 *
 * \param guestprofiler the profiler which is being finished and deleted
 * \param out           byte vector that receives the generated file
 *
 * \return Returns #wasmtime_error_t owned by the caller in case of error,
 * `NULL` otherwise.
 *
 * This function takes ownership of `guestprofiler`, even when error is
 * returned.
 *
 * For more information see the Rust documentation at:
 * https://docs.wasmtime.dev/api/wasmtime/struct.GuestProfiler.html#method.finish
 */
WASM_API_EXTERN /* own */ wasmtime_error_t *
wasmtime_guestprofiler_finish(/* own */ wasmtime_guestprofiler_t *guestprofiler,
                              wasm_byte_vec_t *out);

#ifdef __cplusplus
} // extern "C"
#endif

#endif // WASMTIME_PROFILING_H
