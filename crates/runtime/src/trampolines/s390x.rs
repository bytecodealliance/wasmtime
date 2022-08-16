// The host_to_wasm_trampoline implementation is in the s390x.S
// file, but we still want to have this unit test here.

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

// The wasm_to_host_trampoline implementation is in the s390x.S
// file, but we still want to have this unit test here.

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

// The implementation for libcall trampolines is in the s390x.S
// file.  We provide this dummy definition of wasm_to_libcall_trampoline
// here to make libcalls.rs compile on s390x.  Note that this means we
// have to duplicate the list of libcalls used in the assembler file.

macro_rules! wasm_to_libcall_trampoline {
    ($libcall:ident ; $libcall_impl:ident) => {};
}
