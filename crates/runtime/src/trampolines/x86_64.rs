use wasmtime_asm_macros::asm_func;

asm_func!(
    "host_to_wasm_trampoline",
    "
        .cfi_startproc simple
        .cfi_def_cfa_offset 0

        // Load the pointer to `VMRuntimeLimits` in `r10`.
        mov r10, 8[rsi]

        // Check to see if this is a core `VMContext` (MAGIC == 'core').
        cmp DWORD PTR [rdi], 0x65726f63

        // Store the last Wasm SP into the `last_wasm_entry_sp` in the limits, if this
        // was core Wasm, otherwise store an invalid sentinal value.
        mov r11, -1
        cmove r11, rsp
        mov 40[r10], r11

        // Tail call to the callee function pointer in the vmctx.
        jmp 16[rsi]

        .cfi_endproc
    ",
);

#[cfg(test)]
mod host_to_wasm_trampoline_offsets_tests {
    use wasmtime_environ::{Module, VMOffsets};

    #[test]
    fn test() {
        let module = Module::new();
        let offsets = VMOffsets::new(std::mem::size_of::<*mut u8>() as u8, &module);

        assert_eq!(8, offsets.vmctx_runtime_limits());
        assert_eq!(40, offsets.vmruntime_limits_last_wasm_entry_sp());
        assert_eq!(16, offsets.vmctx_callee());
        assert_eq!(0x65726f63, u32::from_le_bytes(*b"core"));
    }
}

asm_func!(
    "wasm_to_host_trampoline",
    "
        .cfi_startproc simple
        .cfi_def_cfa_offset 0

        // Load the pointer to `VMRuntimeLimits` in `r10`.
        mov r10, 8[rsi]

        // Store the last Wasm FP into the `last_wasm_exit_fp` in the limits.
        mov 24[r10], rbp

        // Store the last Wasm PC into the `last_wasm_exit_pc` in the limits.
        mov r11, [rsp]
        mov 32[r10], r11

        // Tail call to the actual host function.
        //
        // This *must* be a tail call so that we do not push to the stack and mess
        // up the offsets of stack arguments (if any).
        jmp 8[rdi]

        .cfi_endproc
    ",
);

#[cfg(test)]
mod wasm_to_host_trampoline_offsets_tests {
    use crate::VMHostFuncContext;
    use memoffset::offset_of;
    use wasmtime_environ::{Module, VMOffsets};

    #[test]
    fn test() {
        let module = Module::new();
        let offsets = VMOffsets::new(std::mem::size_of::<*mut u8>() as u8, &module);

        assert_eq!(8, offsets.vmctx_runtime_limits());
        assert_eq!(24, offsets.vmruntime_limits_last_wasm_exit_fp());
        assert_eq!(32, offsets.vmruntime_limits_last_wasm_exit_pc());
        assert_eq!(8, offset_of!(VMHostFuncContext, host_func));
    }
}

#[rustfmt::skip]
macro_rules! wasm_to_libcall_trampoline {
    ($libcall:ident ; $libcall_impl:ident) => {
        wasmtime_asm_macros::asm_func!(
            stringify!($libcall),
            "
               .cfi_startproc simple
               .cfi_def_cfa_offset 0

                // Load the pointer to `VMRuntimeLimits` in `r10`.
                mov r10, 8[rdi]

                // Store the last Wasm FP into the `last_wasm_exit_fp` in the limits.
                mov 24[r10], rbp

                // Store the last Wasm PC into the `last_wasm_exit_pc` in the limits.
                mov r11, [rsp]
                mov 32[r10], r11

                // Tail call to the actual implementation of this libcall.
                jmp ", stringify!($libcall_impl), "

                .cfi_endproc
            ",
        );
    };
}
