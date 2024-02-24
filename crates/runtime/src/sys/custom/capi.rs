#![allow(non_camel_case_types)]

// Flags to either `wasmtime_mmap_{new,remap}` or `wasmtime_mprotect`.

/// Indicates that the memory region should be readable.
pub const WASMTIME_PROT_READ: u32 = 1 << 0;
/// Indicates that the memory region should be writable.
pub const WASMTIME_PROT_WRITE: u32 = 1 << 1;
/// Indicates that the memory region should be executable.
pub const WASMTIME_PROT_EXEC: u32 = 1 << 2;

pub use WASMTIME_PROT_EXEC as PROT_EXEC;
pub use WASMTIME_PROT_READ as PROT_READ;
pub use WASMTIME_PROT_WRITE as PROT_WRITE;

/// Handler function for traps in Wasmtime passed to `wasmtime_init_traps`.
///
/// This function is invoked whenever a trap is caught by the system. For
/// example this would be invoked during a signal handler on Linux. This
/// function is passed a number of parameters indicating information about the
/// trap:
///
/// * `ip` - the instruction pointer at the time of the trap.
/// * `fp` - the frame pointer register's value at the time of the trap.
/// * `has_faulting_addr` - whether this trap is associated with an access
///   violation (e.g. a segfault) meaning memory was accessed when it shouldn't
///   be. If this is `true` then the next parameter is filled in.
/// * `faulting_addr` - if `has_faulting_addr` is true then this is the address
///   that was attempted to be accessed. Otherwise this value is not used.
///
/// If this function returns then the trap was not handled. This probably means
/// that a fatal exception happened and the process should be aborted.
///
/// This function may not return as it may invoke `wasmtime_longjmp` if a wasm
/// trap is detected.
pub type wasmtime_trap_handler_t =
    extern "C" fn(ip: usize, fp: usize, has_faulting_addr: bool, faulting_addr: usize);

/// Abstract pointer type used in the `wasmtime_memory_image_*` APIs which
/// is defined by the embedder.
pub enum wasmtime_memory_image {}

extern "C" {
    /// Creates a new virtual memory mapping of the `size` specified with
    /// protection bits specified in `prot_flags`.
    ///
    /// Memory can be lazily committed.
    ///
    /// Returns the base pointer of the new mapping. Aborts the process on
    /// failure.
    ///
    /// Similar to `mmap(0, size, prot_flags, MAP_PRIVATE, 0, -1)` on Linux.
    pub fn wasmtime_mmap_new(size: usize, prot_flags: u32) -> *mut u8;

    /// Remaps the virtual memory starting at `addr` going for `size` bytes to
    /// the protections specified with a new blank mapping.
    ///
    /// This will unmap any prior mappings and decommit them. New mappings for
    /// anonymous memory are used to replace these mappings and the new area
    /// should have the protection specified by `prot_flags`.
    ///
    /// Aborts the process on failure.
    ///
    /// Similar to `mmap(addr, size, prot_flags, MAP_PRIVATE | MAP_FIXED, 0, -1)` on Linux.
    pub fn wasmtime_mmap_remap(addr: *mut u8, size: usize, prot_flags: u32);

    /// Unmaps memory at the specified `ptr` for `size` bytes.
    ///
    /// The memory should be discarded and decommitted and should generate a
    /// segfault if accessed after this function call.
    ///
    /// Aborts the process on failure.
    ///
    /// Similar to `munmap` on Linux.
    pub fn wasmtime_munmap(ptr: *mut u8, size: usize);

    /// Configures the protections associated with a region of virtual memory
    /// starting at `ptr` and going to `size`.
    ///
    /// Aborts the process on failure.
    ///
    /// Similar to `mprotect` on Linux.
    pub fn wasmtime_mprotect(ptr: *mut u8, size: usize, prot_flags: u32);

    /// Returns the page size, in bytes, of the current system.
    pub fn wasmtime_page_size() -> usize;

    /// Used to setup a frame on the stack to longjmp back to in the future.
    ///
    /// This function is used for handling traps in WebAssembly and is paried
    /// with `wasmtime_longjmp`.
    ///
    /// * `jmp_buf` - this argument is filled in with a pointer which if used
    ///   will be passed to `wasmtime_longjmp` later on by the runtime.
    /// * `callback` - this callback should be invoked after `jmp_buf` is
    ///   configured.
    /// * `payload` and `callee` - the two arguments to pass to `callback`.
    ///
    /// Returns 0 if `wasmtime_longjmp` was used to return to this function.
    /// Returns 1 if `wasmtime_longjmp` was not called an `callback` returned.
    pub fn wasmtime_setjmp(
        jmp_buf: *mut *const u8,
        callback: extern "C" fn(*mut u8, *mut u8),
        payload: *mut u8,
        callee: *mut u8,
    ) -> i32;

    /// Paired with `wasmtime_setjmp` this is used to jump back to the `setjmp`
    /// point.
    ///
    /// The argument here was originally passed to `wasmtime_setjmp` through its
    /// out-param.
    ///
    /// This function cannot return.
    ///
    /// This function may be invoked from the `wasmtime_trap_handler_t`
    /// configured by `wasmtime_init_traps`.
    pub fn wasmtime_longjmp(jmp_buf: *const u8) -> !;

    /// Initializes trap-handling logic for this platform.
    ///
    /// Wasmtime's implementation of WebAssembly relies on the ability to catch
    /// signals/traps/etc. For example divide-by-zero may raise a machine
    /// exception. Out-of-bounds memory accesses may also raise a machine
    /// exception. This function is used to initialize trap handling.
    ///
    /// The `handler` provided is a function pointer to invoke whenever a trap
    /// is encountered. The `handler` is invoked whenever a trap is caught by
    /// the system.
    pub fn wasmtime_init_traps(handler: wasmtime_trap_handler_t);

    /// Attempts to create a new in-memory image of the `ptr`/`len` combo which
    /// can be mapped to virtual addresses in the future. The returned
    /// `wasmtime_memory_image` pointer can be `NULL` to indicate that an image
    /// cannot be created. The structure otherwise will later be deallocated
    /// with `wasmtime_memory_image_free` and `wasmtime_memory_image_map_at`
    /// will be used to map the image into new regions of the address space.
    pub fn wasmtime_memory_image_new(ptr: *const u8, len: usize) -> *mut wasmtime_memory_image;

    /// Maps the `image` provided to the virtual address at `addr` and `len`.
    ///
    /// This semantically should make it such that `addr` and `len` looks the
    /// same as the contents of what the memory image was first created with.
    /// The mappings of `addr` should be private and changes do not reflect back
    /// to `wasmtime_memory_image`.
    ///
    /// In effect this is to create a copy-on-write mapping at `addr`/`len`
    /// pointing back to the memory used by the image originally.
    ///
    /// Aborts the process on failure.
    pub fn wasmtime_memory_image_map_at(
        image: *mut wasmtime_memory_image,
        addr: *mut u8,
        len: usize,
    );

    /// Replaces the VM mappings at `addr` and `len` with zeros.
    ///
    /// Aborts the process on failure.
    pub fn wasmtime_memory_image_remap_zeros(
        image: *mut wasmtime_memory_image,
        addr: *mut u8,
        len: usize,
    );

    /// Deallocates the provided `wasmtime_memory_image`.
    pub fn wasmtime_memory_image_free(image: *mut wasmtime_memory_image);
}