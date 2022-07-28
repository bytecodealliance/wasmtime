// Currently `global_asm!` isn't stable on s390x, so this is an external
// assembler file built with the `build.rs`.

        .machine z13
        .text

        .hidden host_to_wasm_trampoline
        .globl host_to_wasm_trampoline
        .type host_to_wasm_trampoline,@function
        .p2align 2

host_to_wasm_trampoline:
        .cfi_startproc

        // Load the pointer to `VMRuntimeLimits` in `%r1`.
        lg %r1, 8(%r3)

        // Check to see if callee is a core `VMContext` (MAGIC == "core").
        l %r0, 0(%r2)
        cfi %r0, 0x65726f63

        // Store the last Wasm SP into the `last_wasm_entry_sp` in the limits, if
        // this was core Wasm, otherwise store an invalid sentinal value.
        lgr %r0, %r15
        locghine %r0, -1
        stg %r0, 40(%r1)

        // Tail call to the callee function pointer in the vmctx.
        lg %r1, 16(%r3)
        br %r1

        .cfi_endproc
        .size host_to_wasm_trampoline,.-host_to_wasm_trampoline

        .hidden wasm_to_host_trampoline
        .globl wasm_to_host_trampoline
        .type wasm_to_host_trampoline,@function
        .p2align 2

wasm_to_host_trampoline:
        .cfi_startproc

        // Load the pointer to `VMRuntimeLimits` in `%r1`.
        lg %r1, 8(%r3)

        // Store the last Wasm FP into the `last_wasm_exit_fp` in the limits.
        lg %r0, 0(%r15)
        stg %r0, 24(%r1)

        // Store the last Wasm PC into the `last_wasm_exit_pc` in the limits.
        stg %r14, 32(%r1)

        // Tail call to the actual host function.
        //
        // This *must* be a tail call so that we do not push to the stack and mess
        // up the offsets of stack arguments (if any).
        lg %r1, 8(%r2)
        br %r1

        .cfi_endproc
        .size wasm_to_host_trampoline,.-wasm_to_host_trampoline

#define LIBCALL_TRAMPOLINE(libcall, libcall_impl)                                 \
        .hidden libcall ;                                                         \
        .globl libcall ;                                                          \
        .type libcall,@function ;                                                 \
        .p2align 2 ;                                                              \
libcall: ;                                                                        \
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
        jg libcall_impl ;                                                         \
                                                                                  \
        .cfi_endproc ;                                                            \
        .size libcall,.-libcall

LIBCALL_TRAMPOLINE(memory32_grow, impl_memory32_grow)
LIBCALL_TRAMPOLINE(table_grow_funcref, impl_table_grow_funcref)
LIBCALL_TRAMPOLINE(table_grow_externref, impl_table_grow_externref)
LIBCALL_TRAMPOLINE(table_fill_funcref, impl_table_fill_funcref)
LIBCALL_TRAMPOLINE(table_fill_externref, impl_table_fill_externref)
LIBCALL_TRAMPOLINE(table_copy, impl_table_copy)
LIBCALL_TRAMPOLINE(table_init, impl_table_init)
LIBCALL_TRAMPOLINE(elem_drop, impl_elem_drop)
LIBCALL_TRAMPOLINE(memory_copy, impl_memory_copy)
LIBCALL_TRAMPOLINE(memory_fill, impl_memory_fill)
LIBCALL_TRAMPOLINE(memory_init, impl_memory_init)
LIBCALL_TRAMPOLINE(ref_func, impl_ref_func)
LIBCALL_TRAMPOLINE(data_drop, impl_data_drop)
LIBCALL_TRAMPOLINE(table_get_lazy_init_funcref, impl_table_get_lazy_init_funcref)
LIBCALL_TRAMPOLINE(drop_externref, impl_drop_externref)
LIBCALL_TRAMPOLINE(activations_table_insert_with_gc, impl_activations_table_insert_with_gc)
LIBCALL_TRAMPOLINE(externref_global_get, impl_externref_global_get)
LIBCALL_TRAMPOLINE(externref_global_set, impl_externref_global_set)
LIBCALL_TRAMPOLINE(memory_atomic_notify, impl_memory_atomic_notify)
LIBCALL_TRAMPOLINE(memory_atomic_wait32, impl_memory_atomic_wait32)
LIBCALL_TRAMPOLINE(memory_atomic_wait64, impl_memory_atomic_wait64)
LIBCALL_TRAMPOLINE(out_of_gas, impl_out_of_gas)
LIBCALL_TRAMPOLINE(new_epoch, impl_new_epoch)
