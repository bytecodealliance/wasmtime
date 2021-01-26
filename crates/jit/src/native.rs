#![allow(missing_docs)]

use cranelift_codegen;

pub fn builder() -> cranelift_codegen::isa::Builder {
    cranelift_native::builder().expect("host machine is not a supported target")
}

pub fn builder_without_flags() -> cranelift_codegen::isa::Builder {
    cranelift_native::builder_with_options(cranelift_codegen::isa::BackendVariant::Any, false)
        .expect("host machine is not a supported target")
}

pub fn call_conv() -> cranelift_codegen::isa::CallConv {
    use target_lexicon::HOST;
    cranelift_codegen::isa::CallConv::triple_default(&HOST)
}

pub use cranelift_codegen::isa::lookup;
