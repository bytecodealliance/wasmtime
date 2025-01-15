// Platform support for Wasmtime's `no_std` build.
//
// This header file is what Wasmtime will rely on when it does not otherwise
// have support for the native platform. This can happen with `no_std` binaries
// for example where the traditional Unix-or-Windows implementation is not
// suitable.
//
// Embedders are expected to implement the symbols defined in this header file.
// These symbols can be defined either in C/C++ or in Rust (using
// `#[unsafe(no_mangle)]`).
//
// Note that there are some `#define`s here which can be added before this
// header file is included to indicate how Wasmtime was built. This corresponds
// to the `wasmtime` crate's Cargo features where if the feature is disabled
// then the symbols will not be required.
//
// * `WASMTIME_SIGNALS_BASED_TRAPS` - corresponds to `signals-based-traps`
//
// Some more information about this header can additionally be found at
// <https://docs.wasmtime.dev/stability-platform-support.html>.


#ifndef _WASMTIME_PLATFORM_H
#define _WASMTIME_PLATFORM_H

/* Generated with cbindgen:0.27.0 */

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

#if defined(WASMTIME_VIRTUAL_MEMORY)
/**
 * Indicates that the memory region should be readable.
 */
#define WASMTIME_PROT_READ (1 << 0)
#endif

#if defined(WASMTIME_VIRTUAL_MEMORY)
/**
 * Indicates that the memory region should be writable.
 */
#define WASMTIME_PROT_WRITE (1 << 1)
#endif

#if defined(WASMTIME_VIRTUAL_MEMORY)
/**
 * Indicates that the memory region should be executable.
 */
#define WASMTIME_PROT_EXEC (1 << 2)
#endif

#if defined(WASMTIME_VIRTUAL_MEMORY)
/**
 * Abstract pointer type used in the `wasmtime_memory_image_*` APIs which
 * is defined by the embedder.
 */
typedef struct wasmtime_memory_image wasmtime_memory_image;
#endif

#if defined(WASMTIME_NATIVE_SIGNALS)
/**
 * Handler function for traps in Wasmtime passed to `wasmtime_init_traps`.
 *
 * This function is invoked whenever a trap is caught by the system. For
 * example this would be invoked during a signal handler on Linux. This
 * function is passed a number of parameters indicating information about the
 * trap:
 *
 * * `ip` - the instruction pointer at the time of the trap.
 * * `fp` - the frame pointer register's value at the time of the trap.
 * * `has_faulting_addr` - whether this trap is associated with an access
 *   violation (e.g. a segfault) meaning memory was accessed when it shouldn't
 *   be. If this is `true` then the next parameter is filled in.
 * * `faulting_addr` - if `has_faulting_addr` is true then this is the address
 *   that was attempted to be accessed. Otherwise this value is not used.
 *
 * If this function returns then the trap was not handled by Wasmtime. This
 * means that it's left up to the embedder how to deal with the trap/signal
 * depending on its default behavior. This could mean forwarding to a
 * non-Wasmtime handler, aborting the process, logging then crashing, etc. The
 * meaning of a trap that's not handled by Wasmtime depends on the context in
 * which the trap was generated.
 *
 * When this function does not return it's because `wasmtime_longjmp` is
 * used to handle a Wasm-based trap.
 */
typedef void (*wasmtime_trap_handler_t)(uintptr_t ip,
                                        uintptr_t fp,
                                        bool has_faulting_addr,
                                        uintptr_t faulting_addr);
#endif

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

#if defined(WASMTIME_VIRTUAL_MEMORY)
/**
 * Creates a new virtual memory mapping of the `size` specified with
 * protection bits specified in `prot_flags`.
 *
 * Memory can be lazily committed.
 *
 * Stores the base pointer of the new mapping in `ret` on success.
 *
 * Returns 0 on success and an error code on failure.
 *
 * Similar to `mmap(0, size, prot_flags, MAP_PRIVATE, 0, -1)` on Linux.
 */
extern int32_t wasmtime_mmap_new(uintptr_t size, uint32_t prot_flags, uint8_t **ret);
#endif

#if defined(WASMTIME_VIRTUAL_MEMORY)
/**
 * Remaps the virtual memory starting at `addr` going for `size` bytes to
 * the protections specified with a new blank mapping.
 *
 * This will unmap any prior mappings and decommit them. New mappings for
 * anonymous memory are used to replace these mappings and the new area
 * should have the protection specified by `prot_flags`.
 *
 * Returns 0 on success and an error code on failure.
 *
 * Similar to `mmap(addr, size, prot_flags, MAP_PRIVATE | MAP_FIXED, 0, -1)` on Linux.
 */
extern int32_t wasmtime_mmap_remap(uint8_t *addr, uintptr_t size, uint32_t prot_flags);
#endif

#if defined(WASMTIME_VIRTUAL_MEMORY)
/**
 * Unmaps memory at the specified `ptr` for `size` bytes.
 *
 * The memory should be discarded and decommitted and should generate a
 * segfault if accessed after this function call.
 *
 * Returns 0 on success and an error code on failure.
 *
 * Similar to `munmap` on Linux.
 */
extern int32_t wasmtime_munmap(uint8_t *ptr, uintptr_t size);
#endif

#if defined(WASMTIME_VIRTUAL_MEMORY)
/**
 * Configures the protections associated with a region of virtual memory
 * starting at `ptr` and going to `size`.
 *
 * Returns 0 on success and an error code on failure.
 *
 * Similar to `mprotect` on Linux.
 */
extern int32_t wasmtime_mprotect(uint8_t *ptr, uintptr_t size, uint32_t prot_flags);
#endif

#if defined(WASMTIME_VIRTUAL_MEMORY)
/**
 * Returns the page size, in bytes, of the current system.
 */
extern uintptr_t wasmtime_page_size(void);
#endif

/**
 * Used to setup a frame on the stack to longjmp back to in the future.
 *
 * This function is used for handling traps in WebAssembly and is paried
 * with `wasmtime_longjmp`.
 *
 * * `jmp_buf` - this argument is filled in with a pointer which if used
 *   will be passed to `wasmtime_longjmp` later on by the runtime.
 * * `callback` - this callback should be invoked after `jmp_buf` is
 *   configured.
 * * `payload` and `callee` - the two arguments to pass to `callback`.
 *
 * Returns false if `wasmtime_longjmp` was used to return to this function.
 * Returns true if `wasmtime_longjmp` was not called and `callback` returned.
 */
extern bool wasmtime_setjmp(const uint8_t **jmp_buf,
                            bool (*callback)(uint8_t*, uint8_t*),
                            uint8_t *payload,
                            uint8_t *callee);

/**
 * Paired with `wasmtime_setjmp` this is used to jump back to the `setjmp`
 * point.
 *
 * The argument here was originally passed to `wasmtime_setjmp` through its
 * out-param.
 *
 * This function cannot return.
 *
 * This function may be invoked from the `wasmtime_trap_handler_t`
 * configured by `wasmtime_init_traps`.
 */
extern void wasmtime_longjmp(const uint8_t *jmp_buf);

#if defined(WASMTIME_NATIVE_SIGNALS)
/**
 * Initializes trap-handling logic for this platform.
 *
 * Wasmtime's implementation of WebAssembly relies on the ability to catch
 * signals/traps/etc. For example divide-by-zero may raise a machine
 * exception. Out-of-bounds memory accesses may also raise a machine
 * exception. This function is used to initialize trap handling.
 *
 * The `handler` provided is a function pointer to invoke whenever a trap
 * is encountered. The `handler` is invoked whenever a trap is caught by
 * the system.
 *
 * Returns 0 on success and an error code on failure.
 */
extern int32_t wasmtime_init_traps(wasmtime_trap_handler_t handler);
#endif

#if defined(WASMTIME_VIRTUAL_MEMORY)
/**
 * Attempts to create a new in-memory image of the `ptr`/`len` combo which
 * can be mapped to virtual addresses in the future.
 *
 * On success the returned `wasmtime_memory_image` pointer is stored into `ret`.
 * This value stored can be `NULL` to indicate that an image cannot be
 * created but no failure occurred. The structure otherwise will later be
 * deallocated with `wasmtime_memory_image_free` and
 * `wasmtime_memory_image_map_at` will be used to map the image into new
 * regions of the address space.
 *
 * The `ptr` and `len` arguments are only valid for this function call, if
 * the image needs to refer to them in the future then it must make a copy.
 *
 * Both `ptr` and `len` are guaranteed to be page-aligned.
 *
 * Returns 0 on success and an error code on failure. Note that storing
 * `NULL` into `ret` is not considered a failure, and failure is used to
 * indicate that something fatal has happened and Wasmtime will propagate
 * the error upwards.
 */
extern int32_t wasmtime_memory_image_new(const uint8_t *ptr,
                                         uintptr_t len,
                                         struct wasmtime_memory_image **ret);
#endif

#if defined(WASMTIME_VIRTUAL_MEMORY)
/**
 * Maps the `image` provided to the virtual address at `addr` and `len`.
 *
 * This semantically should make it such that `addr` and `len` looks the
 * same as the contents of what the memory image was first created with.
 * The mappings of `addr` should be private and changes do not reflect back
 * to `wasmtime_memory_image`.
 *
 * In effect this is to create a copy-on-write mapping at `addr`/`len`
 * pointing back to the memory used by the image originally.
 *
 * Note that the memory region will be unmapped with `wasmtime_munmap` in
 * the future.
 *
 * Aborts the process on failure.
 */
extern int32_t wasmtime_memory_image_map_at(struct wasmtime_memory_image *image,
                                            uint8_t *addr,
                                            uintptr_t len);
#endif

#if defined(WASMTIME_VIRTUAL_MEMORY)
/**
 * Deallocates the provided `wasmtime_memory_image`.
 *
 * Note that mappings created from this image are not guaranteed to be
 * deallocated and/or unmapped before this is called.
 */
extern void wasmtime_memory_image_free(struct wasmtime_memory_image *image);
#endif

/**
 * Wasmtime requires a single pointer's space of TLS to be used at runtime,
 * and this function returns the current value of the TLS variable.
 *
 * This value should default to `NULL`.
 */
extern uint8_t *wasmtime_tls_get(void);

/**
 * Sets the current TLS value for Wasmtime to the provided value.
 *
 * This value should be returned when later calling `wasmtime_tls_get`.
 */
extern void wasmtime_tls_set(uint8_t *ptr);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#endif  /* _WASMTIME_PLATFORM_H */
