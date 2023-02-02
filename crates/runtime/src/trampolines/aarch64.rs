use wasmtime_asm_macros::asm_func;

#[rustfmt::skip]
asm_func!(
    "host_to_wasm_trampoline",
    r#"
        .cfi_startproc
        bti c

        // Load the pointer to `VMRuntimeLimits` in `x9`.
        ldur x9, [x1, #8]

        // Check to see if callee is a core `VMContext` (MAGIC == "core"). NB:
        // we do not support big-endian aarch64 so the magic value is always
        // little-endian encoded.
        ldur w10, [x0]
        mov  w11, #0x6f63
        movk w11, #0x6572, lsl #16
        cmp  w10, w11

        // Store the last Wasm SP into the `last_wasm_entry_sp` in the limits, if
        // this was core Wasm, otherwise store an invalid sentinal value.
        mov  x12, #-1
        mov  x13, sp
        csel x12, x13, x12, eq
        stur x12, [x9, #40]

        // Tail call to the callee function pointer in the vmctx.
        ldur x16, [x1, #16]
        br   x16

        .cfi_endproc
    "#
);

#[cfg(test)]
mod host_to_wasm_trampoline_offsets_tests {
    use wasmtime_environ::{Module, PtrSize, VMOffsets};

    #[test]
    fn test() {
        let module = Module::new();
        let offsets = VMOffsets::new(std::mem::size_of::<*mut u8>() as u8, &module);

        assert_eq!(8, offsets.vmctx_runtime_limits());
        assert_eq!(40, offsets.ptr.vmruntime_limits_last_wasm_entry_sp());
        assert_eq!(16, offsets.vmctx_callee());
        assert_eq!(0x65726f63, u32::from_le_bytes(*b"core"));
    }
}

asm_func!(
    "wasm_to_host_trampoline",
    "
        .cfi_startproc
        bti c

        // Load the pointer to `VMRuntimeLimits` in `x9`.
        ldur x9, [x1, #8]

        // Store the last Wasm FP into the `last_wasm_exit_fp` in the limits.
        stur fp, [x9, #24]

        // Store the last Wasm PC into the `last_wasm_exit_pc` in the limits.
        stur lr, [x9, #32]

        // Tail call to the actual host function.
        //
        // This *must* be a tail call so that we do not push to the stack and mess
        // up the offsets of stack arguments (if any).
        ldur x16, [x0, #8]
        br   x16

        .cfi_endproc
    ",
);

#[cfg(test)]
mod wasm_to_host_trampoline_offsets_tests {
    use crate::VMHostFuncContext;
    use memoffset::offset_of;
    use wasmtime_environ::{Module, PtrSize, VMOffsets};

    #[test]
    fn test() {
        let module = Module::new();
        let offsets = VMOffsets::new(std::mem::size_of::<*mut u8>() as u8, &module);

        assert_eq!(8, offsets.vmctx_runtime_limits());
        assert_eq!(24, offsets.ptr.vmruntime_limits_last_wasm_exit_fp());
        assert_eq!(32, offsets.ptr.vmruntime_limits_last_wasm_exit_pc());
        assert_eq!(8, offset_of!(VMHostFuncContext, host_func));
    }
}

#[rustfmt::skip]
macro_rules! wasm_to_libcall_trampoline {
    ($libcall:ident ; $libcall_impl:ident) => {
        wasmtime_asm_macros::asm_func!(
            stringify!($libcall),
            "
                .cfi_startproc
                bti c

                // Load the pointer to `VMRuntimeLimits` in `x9`.
                ldur x9, [x0, #8]

                // Store the last Wasm FP into the `last_wasm_exit_fp` in the limits.
                stur fp, [x9, #24]

                // Store the last Wasm PC into the `last_wasm_exit_pc` in the limits.
                stur lr, [x9, #32]

                // Tail call to the actual implementation of this libcall.
                b {}

                .cfi_endproc
            ",
            sym $libcall_impl
        );
    };
}
