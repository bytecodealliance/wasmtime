// Helper macros for getting the first and second arguments according to the
// system calling convention, as well as some callee-saved scratch registers we
// can safely use in the trampolines.
cfg_if::cfg_if! {
    if #[cfg(windows)] {
        macro_rules! callee_vmctx { () => ("rcx") }
        #[allow(unused)] macro_rules! caller_vmctx { () => ("rdx") }
        macro_rules! scratch0 { () => ("r10") }
        macro_rules! scratch1 { () => ("r11") }
    } else if #[cfg(unix)] {
        macro_rules! callee_vmctx { () => ("rdi") }
        #[allow(unused)] macro_rules! caller_vmctx { () => ("rsi") }
        macro_rules! scratch0 { () => ("r10") }
        macro_rules! scratch1 { () => ("r11") }
    } else {
        compile_error!("platform not supported");
    }
}

#[rustfmt::skip]
macro_rules! wasm_to_libcall_trampoline {
    ($libcall:ident ; $libcall_impl:ident) => {
        wasmtime_asm_macros::asm_func!(
            wasmtime_versioned_export_macros::versioned_stringify_ident!($libcall),
            concat!(
                "
                   .cfi_startproc simple
                   .cfi_def_cfa_offset 0

                    // Load the pointer to `VMRuntimeLimits` in `", scratch0!(), "`.
                    mov ", scratch0!(), ", 8[", callee_vmctx!(), "]

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

#[cfg(test)]
mod wasm_to_libcall_trampoline_offsets_tests {
    use wasmtime_environ::{Module, PtrSize, VMOffsets};

    #[test]
    fn test() {
        let module = Module::new();
        let offsets = VMOffsets::new(std::mem::size_of::<*mut u8>() as u8, &module);

        assert_eq!(8, offsets.vmctx_runtime_limits());
        assert_eq!(24, offsets.ptr.vmruntime_limits_last_wasm_exit_fp());
        assert_eq!(32, offsets.ptr.vmruntime_limits_last_wasm_exit_pc());
    }
}
