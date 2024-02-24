#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

/**
 * Indicates that the memory region should be readable.
 */
#define WASMTIME_PROT_READ (1 << 0)

/**
 * Indicates that the memory region should be writable.
 */
#define WASMTIME_PROT_WRITE (1 << 1)

/**
 * Indicates that the memory region should be executable.
 */
#define WASMTIME_PROT_EXEC (1 << 2)

/**
 * Abstract pointer type used in the `wasmtime_memory_image_*` APIs which
 * is defined by the embedder.
 */
typedef struct wasmtime_memory_image wasmtime_memory_image;

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
 * If this function returns then the trap was not handled. This probably means
 * that a fatal exception happened and the process should be aborted.
 *
 * This function may not return as it may invoke `wasmtime_longjmp` if a wasm
 * trap is detected.
 */
typedef void (*wasmtime_trap_handler_t)(uintptr_t ip,
                                        uintptr_t fp,
                                        bool has_faulting_addr,
                                        uintptr_t faulting_addr);

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

/**
 * Creates a new virtual memory mapping of the `size` specified with
 * protection bits specified in `prot_flags`.
 *
 * Memory can be lazily committed.
 *
 * Returns the base pointer of the new mapping. Aborts the process on
 * failure.
 *
 * Similar to `mmap(0, size, prot_flags, MAP_PRIVATE, 0, -1)` on Linux.
 */
extern uint8_t *wasmtime_mmap_new(uintptr_t size, uint32_t prot_flags);

/**
 * Remaps the virtual memory starting at `addr` going for `size` bytes to
 * the protections specified with a new blank mapping.
 *
 * This will unmap any prior mappings and decommit them. New mappings for
 * anonymous memory are used to replace these mappings and the new area
 * should have the protection specified by `prot_flags`.
 *
 * Aborts the process on failure.
 *
 * Similar to `mmap(addr, size, prot_flags, MAP_PRIVATE | MAP_FIXED, 0, -1)` on Linux.
 */
extern void wasmtime_mmap_remap(uint8_t *addr, uintptr_t size, uint32_t prot_flags);

/**
 * Unmaps memory at the specified `ptr` for `size` bytes.
 *
 * The memory should be discarded and decommitted and should generate a
 * segfault if accessed after this function call.
 *
 * Aborts the process on failure.
 *
 * Similar to `munmap` on Linux.
 */
extern void wasmtime_munmap(uint8_t *ptr, uintptr_t size);

/**
 * Configures the protections associated with a region of virtual memory
 * starting at `ptr` and going to `size`.
 *
 * Aborts the process on failure.
 *
 * Similar to `mprotect` on Linux.
 */
extern void wasmtime_mprotect(uint8_t *ptr, uintptr_t size, uint32_t prot_flags);

/**
 * Returns the page size, in bytes, of the current system.
 */
extern uintptr_t wasmtime_page_size(void);

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
 * Returns 0 if `wasmtime_longjmp` was used to return to this function.
 * Returns 1 if `wasmtime_longjmp` was not called an `callback` returned.
 */
extern int32_t wasmtime_setjmp(const uint8_t **jmp_buf,
                               void (*callback)(uint8_t*, uint8_t*),
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
 */
extern void wasmtime_init_traps(wasmtime_trap_handler_t handler);

/**
 * Attempts to create a new in-memory image of the `ptr`/`len` combo which
 * can be mapped to virtual addresses in the future. The returned
 * `wasmtime_memory_image` pointer can be `NULL` to indicate that an image
 * cannot be created. The structure otherwise will later be deallocated
 * with `wasmtime_memory_image_free` and `wasmtime_memory_image_map_at`
 * will be used to map the image into new regions of the address space.
 */
extern struct wasmtime_memory_image *wasmtime_memory_image_new(const uint8_t *ptr, uintptr_t len);

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
 * Aborts the process on failure.
 */
extern void wasmtime_memory_image_map_at(struct wasmtime_memory_image *image,
                                         uint8_t *addr,
                                         uintptr_t len);

/**
 * Replaces the VM mappings at `addr` and `len` with zeros.
 *
 * Aborts the process on failure.
 */
extern void wasmtime_memory_image_remap_zeros(struct wasmtime_memory_image *image,
                                              uint8_t *addr,
                                              uintptr_t len);

/**
 * Deallocates the provided `wasmtime_memory_image`.
 */
extern void wasmtime_memory_image_free(struct wasmtime_memory_image *image);

#ifdef __cplusplus
} // extern "C"
#endif // __cplusplus