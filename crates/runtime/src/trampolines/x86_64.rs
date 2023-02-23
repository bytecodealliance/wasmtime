use wasmtime_asm_macros::asm_func;

// Helper macros for getting the first and second arguments according to the
// system calling convention, as well as some callee-saved scratch registers we
// can safely use in the trampolines.
cfg_if::cfg_if! {
    if #[cfg(windows)] {
        macro_rules! arg0 { () => ("rcx") }
        macro_rules! arg1 { () => ("rdx") }
        macro_rules! scratch0 { () => ("r10") }
        macro_rules! scratch1 { () => ("r11") }
    } else if #[cfg(unix)] {
        macro_rules! arg0 { () => ("rdi") }
        macro_rules! arg1 { () => ("rsi") }
        macro_rules! scratch0 { () => ("r10") }
        macro_rules! scratch1 { () => ("r11") }
    } else {
        compile_error!("platform not supported");
    }
}

#[rustfmt::skip]
asm_func!(
    "host_to_wasm_trampoline",
    concat!(
        "
            .cfi_startproc simple
            .cfi_def_cfa_offset 0

            // Load the pointer to `VMRuntimeLimits` in `scratch0`.
            mov ", scratch0!(), ", 8[", arg1!(), "]

            // Check to see if this is a core `VMContext` (MAGIC == 'core').
            cmp DWORD PTR [", arg0!(), "], 0x65726f63

            // Store the last Wasm SP into the `last_wasm_entry_sp` in the limits, if this
            // was core Wasm, otherwise store an invalid sentinal value.
            mov ", scratch1!(), ", -1
            cmove ", scratch1!(), ", rsp
            mov 40[", scratch0!(), "], ", scratch1!(), "

            // Tail call to the callee function pointer in the vmctx.
            jmp 16[", arg1!(), "]

            .cfi_endproc
        ",
    ),
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

#[rustfmt::skip]
asm_func!(
    "wasm_to_host_trampoline",
    concat!(
        "
            .cfi_startproc simple
            .cfi_def_cfa_offset 0

            // Load the pointer to `VMRuntimeLimits` in `scratch0`.
            mov ", scratch0!(), ", 8[", arg1!(), "]

            // Store the last Wasm FP into the `last_wasm_exit_fp` in the limits.
            mov 24[", scratch0!(), "], rbp

            // Store the last Wasm PC into the `last_wasm_exit_pc` in the limits.
            mov ", scratch1!(), ", [rsp]
            mov 32[", scratch0!(), "], ", scratch1!(), "

            // Tail call to the actual host function.
            //
            // This *must* be a tail call so that we do not push to the stack and mess
            // up the offsets of stack arguments (if any).
            jmp 8[", arg0!(), "]

            .cfi_endproc
        ",
    ),
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
            concat!(
                "
                   .cfi_startproc simple
                   .cfi_def_cfa_offset 0

                    // Load the pointer to `VMRuntimeLimits` in `", scratch0!(), "`.
                    mov ", scratch0!(), ", 8[", arg0!(), "]

                    // Store the last Wasm FP into the `last_wasm_exit_fp` in the limits.
                    mov 24[", scratch0!(), "], rbp

                    // Store the last Wasm PC into the `last_wasm_exit_pc` in the limits.
                    mov ", scratch1!(), ", [rsp]
                    mov 32[", scratch0!(), "], ", scratch1!(), "

                    // Tail call to the actual implementation of this libcall.
                    jmp {}

                    .cfi_endproc
                ",
            ),
            sym $libcall_impl
        );
    };
}
