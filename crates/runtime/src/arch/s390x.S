// Currently `global_asm!` isn't stable on s390x, so this is an external
// assembler file built with the `build.rs`.

        .machine z13
        .text

        .hidden host_to_wasm_trampoline
        .globl host_to_wasm_trampoline
        .type host_to_wasm_trampoline,@function
        .p2align 2

#define CONCAT2(a, b) a ## b
#define CONCAT(a, b) CONCAT2(a , b)
#define VERSIONED_SYMBOL(a) CONCAT(a, VERSIONED_SUFFIX)

#define LIBCALL_TRAMPOLINE(libcall, libcall_impl)                                 \
        .hidden VERSIONED_SYMBOL(libcall) ;                                       \
        .globl VERSIONED_SYMBOL(libcall) ;                                        \
        .type VERSIONED_SYMBOL(libcall),@function ;                               \
        .p2align 2 ;                                                              \
VERSIONED_SYMBOL(libcall): ;                                                      \
        .cfi_startproc ;                                                          \
                                                                                  \
        /* Load the pointer to `VMRuntimeLimits` in `%r1`.  */                    \
        lg %r1, 8(%r2) ;                                                          \
                                                                                  \
        /* Store the last Wasm FP into the `last_wasm_exit_fp` in the limits.  */ \
        lg %r0, 0(%r15) ;                                                         \
        stg %r0, 24(%r1) ;                                                        \
                                                                                  \
        /* Store the last Wasm PC into the `last_wasm_exit_pc` in the limits.  */ \
        stg %r14, 32(%r1) ;                                                       \
                                                                                  \
        /* Tail call to the actual implementation of this libcall.  */            \
        jg VERSIONED_SYMBOL(libcall_impl) ;                                       \
                                                                                  \
        .cfi_endproc ;                                                            \
        .size VERSIONED_SYMBOL(libcall),.-VERSIONED_SYMBOL(libcall)

LIBCALL_TRAMPOLINE(memory32_grow, impl_memory32_grow)
LIBCALL_TRAMPOLINE(table_grow_func_ref, impl_table_grow_func_ref)
LIBCALL_TRAMPOLINE(table_grow_externref, impl_table_grow_externref)
LIBCALL_TRAMPOLINE(table_fill_func_ref, impl_table_fill_func_ref)
LIBCALL_TRAMPOLINE(table_fill_externref, impl_table_fill_externref)
LIBCALL_TRAMPOLINE(table_copy, impl_table_copy)
LIBCALL_TRAMPOLINE(table_init, impl_table_init)
LIBCALL_TRAMPOLINE(elem_drop, impl_elem_drop)
LIBCALL_TRAMPOLINE(memory_copy, impl_memory_copy)
LIBCALL_TRAMPOLINE(memory_fill, impl_memory_fill)
LIBCALL_TRAMPOLINE(memory_init, impl_memory_init)
LIBCALL_TRAMPOLINE(ref_func, impl_ref_func)
LIBCALL_TRAMPOLINE(data_drop, impl_data_drop)
LIBCALL_TRAMPOLINE(table_get_lazy_init_func_ref, impl_table_get_lazy_init_func_ref)
LIBCALL_TRAMPOLINE(drop_gc_ref, impl_drop_gc_ref)
LIBCALL_TRAMPOLINE(gc, gc)
LIBCALL_TRAMPOLINE(gc_ref_global_get, impl_gc_ref_global_get)
LIBCALL_TRAMPOLINE(gc_ref_global_set, impl_gc_ref_global_set)
LIBCALL_TRAMPOLINE(memory_atomic_notify, impl_memory_atomic_notify)
LIBCALL_TRAMPOLINE(memory_atomic_wait32, impl_memory_atomic_wait32)
LIBCALL_TRAMPOLINE(memory_atomic_wait64, impl_memory_atomic_wait64)
LIBCALL_TRAMPOLINE(out_of_gas, impl_out_of_gas)
LIBCALL_TRAMPOLINE(new_epoch, impl_new_epoch)
LIBCALL_TRAMPOLINE(check_malloc, impl_check_malloc)
LIBCALL_TRAMPOLINE(check_free, impl_check_free)
LIBCALL_TRAMPOLINE(check_load, impl_check_load)
LIBCALL_TRAMPOLINE(check_store, impl_check_store)
LIBCALL_TRAMPOLINE(malloc_start, impl_malloc_start)
LIBCALL_TRAMPOLINE(free_start, impl_free_start)
LIBCALL_TRAMPOLINE(update_stack_pointer, impl_update_stack_pointer)
LIBCALL_TRAMPOLINE(update_mem_size, impl_update_mem_size)
