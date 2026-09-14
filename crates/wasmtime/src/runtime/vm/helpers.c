#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

#define CONCAT2(a, b) a##b
#define CONCAT(a, b) CONCAT2(a, b)
#define VERSIONED_SYMBOL(a) CONCAT(a, VERSIONED_SUFFIX)

#ifdef FEATURE_DEBUG_BUILTINS
#ifdef CFG_TARGET_OS_windows
#define DEBUG_BUILTIN_EXPORT __declspec(dllexport)
#else
#define DEBUG_BUILTIN_EXPORT
#endif

// This set of symbols is defined here in C because Rust's #[export_name]
// functions are not dllexported on Windows when building an executable. These
// symbols are directly referenced by name from the native DWARF info.
void *VERSIONED_SYMBOL(resolve_vmctx_memory_ptr)(void *);
DEBUG_BUILTIN_EXPORT void *
VERSIONED_SYMBOL(wasmtime_resolve_vmctx_memory_ptr)(void *p) {
  return VERSIONED_SYMBOL(resolve_vmctx_memory_ptr)(p);
}
void VERSIONED_SYMBOL(set_vmctx_memory)(void *);
DEBUG_BUILTIN_EXPORT void VERSIONED_SYMBOL(wasmtime_set_vmctx_memory)(void *p) {
  VERSIONED_SYMBOL(set_vmctx_memory)(p);
}

// Helper symbol called from Rust to force the above two functions to not get
// stripped by the linker.
void VERSIONED_SYMBOL(wasmtime_debug_builtins_init)() {
#ifndef CFG_TARGET_OS_windows
  void *volatile p;
  p = (void *)&VERSIONED_SYMBOL(wasmtime_resolve_vmctx_memory_ptr);
  p = (void *)&VERSIONED_SYMBOL(wasmtime_set_vmctx_memory);
  (void)p;
#endif
}
#endif // FEATURE_DEBUG_BUILTINS

// For more information about this see `unix/unwind.rs` and the
// `Libunwind` struct. The basic idea is that weak symbols aren't stable
// in Rust so we use a bit of C to work around that.
#ifndef CFG_TARGET_OS_windows
struct Libunwind {
  void (*add_dynamic_fde)(uintptr_t);
  void (*add_dynamic_eh_frame_section)(uintptr_t);
  void (*remove_dynamic_eh_frame_section)(uintptr_t);
};

__attribute__((weak)) extern void __unw_add_dynamic_fde(uintptr_t);
#ifndef CFG_TARGET_OS_macos
__attribute__((weak)) extern void __unw_add_dynamic_eh_frame_section(uintptr_t);
__attribute__((weak)) extern void
    __unw_remove_dynamic_eh_frame_section(uintptr_t);
#endif

struct Libunwind VERSIONED_SYMBOL(wasmtime_libunwind)(void) {
  return (struct Libunwind){
      .add_dynamic_fde = __unw_add_dynamic_fde,
#ifndef CFG_TARGET_OS_macos
      .add_dynamic_eh_frame_section = __unw_add_dynamic_eh_frame_section,
      .remove_dynamic_eh_frame_section = __unw_remove_dynamic_eh_frame_section,
#endif
  };
}
#endif
