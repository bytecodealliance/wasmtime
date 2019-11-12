pub use cranelift_codegen as codegen;
pub use wasmtime_runtime::{
    Export as InstanceHandleExport, InstanceHandle, VMContext, VMFunctionBody,
};

mod r#macro;

pub trait IntoIRType {
    fn into_ir_type() -> codegen::ir::Type;
}

pub trait AbiPrimitive {
    type Abi: IntoIRType;
    fn convert_to_abi(self) -> Self::Abi;
    fn create_from_abi(ret: Self::Abi) -> Self;
}

macro_rules! cast32 {
    ($($i:ident)*) => ($(
        impl AbiPrimitive for $i {
            type Abi = i32;

            fn convert_to_abi(self) -> Self::Abi {
                self as i32
            }

            fn create_from_abi(p: Self::Abi) -> Self {
                p as $i
            }
        }
    )*)
}

macro_rules! cast64 {
    ($($i:ident)*) => ($(
        impl AbiPrimitive for $i {
            type Abi = i64;

            fn convert_to_abi(self) -> Self::Abi {
                self as i64
            }

            fn create_from_abi(p: Self::Abi) -> Self {
                p as $i
            }
        }
    )*)
}

cast32!(i8 i16 i32 u8 u16 u32);
cast64!(i64 u64);

pub trait WasmMem {
    type Abi;
    fn as_ptr<T>(&self, off: Self::Abi) -> *mut T;
    fn as_off<T>(&self, ptr: *const T) -> Self::Abi;
}

pub struct VMContextWrapper(pub *mut VMContext);

impl WasmMem for VMContextWrapper {
    type Abi = i32;
    fn as_ptr<T>(&self, _off: Self::Abi) -> *mut T {
        unimplemented!();
    }
    fn as_off<T>(&self, _ptr: *const T) -> Self::Abi {
        unimplemented!();
    }
}

impl IntoIRType for i32 {
    fn into_ir_type() -> codegen::ir::Type {
        codegen::ir::types::I32
    }
}

impl IntoIRType for u32 {
    fn into_ir_type() -> codegen::ir::Type {
        codegen::ir::types::I32
    }
}

impl IntoIRType for i64 {
    fn into_ir_type() -> codegen::ir::Type {
        codegen::ir::types::I64
    }
}

pub fn get_ir_type<T: IntoIRType>() -> codegen::ir::Type {
    T::into_ir_type()
}

pub fn get_body(export: &InstanceHandleExport) -> (*const VMFunctionBody, *mut VMContext) {
    // TODO check signature?
    if let InstanceHandleExport::Function { address, vmctx, .. } = export {
        (*address, *vmctx)
    } else {
        panic!("not a function export")
    }
}

pub struct FnMetadata {
    pub name: &'static str,
    pub signature: codegen::ir::Signature,
    pub address: *const u8,
}

pub fn get_host_call_conv() -> codegen::isa::CallConv {
    codegen::isa::CallConv::triple_default(&target_lexicon::HOST)
}
