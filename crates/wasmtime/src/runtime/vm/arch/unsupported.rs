compile_error!("Wasmtime's runtime is being compiled for an architecture that it does not support");

cfg_if::cfg_if! {
    if #[cfg(target_arch = "riscv32")] {
    compile_error!("\
the tracking issue for riscv32 support is https://github.com/bytecodealliance/wasmtime/issues/8768 \
");
    } else {
    compile_error!("\
if you'd like feel free to file an issue for platform support at
https://github.com/bytecodealliance/wasmtime/issues/new
");
    }
}

pub fn get_stack_pointer() -> usize {
    panic!()
}

pub unsafe fn get_next_older_pc_from_fp(_fp: usize) -> usize {
    panic!()
}

pub const NEXT_OLDER_FP_FROM_FP_OFFSET: usize = 0;

pub fn assert_fp_is_aligned(_fp: usize) {
    panic!()
}
