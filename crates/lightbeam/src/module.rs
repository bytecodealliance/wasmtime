use crate::backend::TranslatedCodeSection;
use crate::error::Error;
use crate::microwasm;
use crate::translate_sections;
use cranelift_codegen::{
    ir::{self, AbiParam, Signature as CraneliftSignature},
    isa,
};
use memoffset::offset_of;

use std::{convert::TryInto, mem};
use thiserror::Error;
use wasmparser::{FuncType, MemoryType, Parser, Payload, ResizableLimits, Type};

pub trait AsValueType {
    const TYPE: Type;
}

pub trait TypeList {
    const TYPE_LIST: &'static [Type];
}

impl<T> TypeList for T
where
    T: AsValueType,
{
    const TYPE_LIST: &'static [Type] = &[T::TYPE];
}

impl AsValueType for i32 {
    const TYPE: Type = Type::I32;
}
impl AsValueType for i64 {
    const TYPE: Type = Type::I64;
}
impl AsValueType for u32 {
    const TYPE: Type = Type::I32;
}
impl AsValueType for u64 {
    const TYPE: Type = Type::I64;
}
impl AsValueType for f32 {
    const TYPE: Type = Type::F32;
}
impl AsValueType for f64 {
    const TYPE: Type = Type::F64;
}

pub trait FunctionArgs<O> {
    type FuncType;

    unsafe fn call(self, func: Self::FuncType, vm_ctx: *const u8) -> O;
    fn into_func(start: *const u8) -> Self::FuncType;
}

type VmCtxPtr = u64;

macro_rules! impl_function_args {
    ($first:ident $(, $rest:ident)*) => {
        impl<Output, $first, $($rest),*> FunctionArgs<Output> for ($first, $($rest),*) {
            type FuncType = unsafe extern "sysv64" fn(VmCtxPtr, $first $(, $rest)*) -> Output;

            #[allow(non_snake_case)]
            unsafe fn call(self, func: Self::FuncType, vm_ctx: *const u8) -> Output {
                let ($first, $($rest),*) = self;
                func(vm_ctx as VmCtxPtr, $first $(, $rest)*)
            }

            fn into_func(start: *const u8) -> Self::FuncType {
                unsafe { mem::transmute(start) }
            }
        }

        impl<$first: AsValueType, $($rest: AsValueType),*> TypeList for ($first, $($rest),*) {
            const TYPE_LIST: &'static [Type] = &[$first::TYPE, $($rest::TYPE),*];
        }

        impl_function_args!($($rest),*);
    };
    () => {
        impl<Output> FunctionArgs<Output> for () {
            type FuncType = unsafe extern "sysv64" fn(VmCtxPtr) -> Output;

            unsafe fn call(self, func: Self::FuncType, vm_ctx: *const u8) -> Output {
                func(vm_ctx as VmCtxPtr)
            }

            fn into_func(start: *const u8) -> Self::FuncType {
                unsafe { mem::transmute(start) }
            }
        }

        impl TypeList for () {
            const TYPE_LIST: &'static [Type] = &[];
        }
    };
}

impl_function_args!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S);

#[derive(Default)]
pub struct TranslatedModule {
    translated_code_section: Option<TranslatedCodeSection>,
    ctx: SimpleContext,
    // TODO: Should we wrap this in a `Mutex` so that calling functions from multiple
    //       threads doesn't cause data races?
    memory: Option<ResizableLimits>,
}

impl TranslatedModule {
    pub fn instantiate(self) -> ExecutableModule {
        let mem_size = self.memory.map(|limits| limits.initial).unwrap_or(0) as usize;
        let mem: BoxSlice<_> = vec![0u8; mem_size * WASM_PAGE_SIZE]
            .into_boxed_slice()
            .into();

        let ctx = if mem.len > 0 {
            Some(Box::new(VmCtx { mem }) as Box<VmCtx>)
        } else {
            None
        };

        ExecutableModule {
            module: self,
            context: ctx,
        }
    }

    pub fn disassemble(&self) {
        self.translated_code_section
            .as_ref()
            .expect("no code section")
            .disassemble();
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Error)]
pub enum ExecutionError {
    #[error("function index out of bounds")]
    FuncIndexOutOfBounds,
    #[error("type mismatch")]
    TypeMismatch,
}

pub struct ExecutableModule {
    module: TranslatedModule,
    context: Option<Box<VmCtx>>,
}

impl ExecutableModule {
    /// Executes the function identified by `func_idx`.
    ///
    /// # Safety
    ///
    /// Executes the function _without_ checking the argument types.
    /// This can cause undefined memory to be accessed.
    pub unsafe fn execute_func_unchecked<Args: FunctionArgs<T>, T>(
        &self,
        func_idx: u32,
        args: Args,
    ) -> T {
        let code_section = self
            .module
            .translated_code_section
            .as_ref()
            .expect("no code section");
        let start_buf = code_section.func_start(func_idx as usize);

        args.call(
            Args::into_func(start_buf),
            self.context
                .as_ref()
                .map(|ctx| (&**ctx) as *const VmCtx as *const u8)
                .unwrap_or(std::ptr::null()),
        )
    }

    pub fn execute_func<Args: FunctionArgs<T> + TypeList, T: TypeList>(
        &self,
        func_idx: u32,
        args: Args,
    ) -> Result<T, ExecutionError> {
        let module = &self.module;

        if func_idx as usize >= module.ctx.func_ty_indices.len() {
            return Err(ExecutionError::FuncIndexOutOfBounds);
        }

        let type_ = module.ctx.func_type(func_idx);

        // TODO: Handle "compatible" types (i.e. f32 and i32)
        if (&type_.params[..], &type_.returns[..]) != (Args::TYPE_LIST, T::TYPE_LIST) {
            return Err(ExecutionError::TypeMismatch);
        }

        Ok(unsafe { self.execute_func_unchecked(func_idx, args) })
    }

    pub fn disassemble(&self) {
        self.module.disassemble();
    }
}

struct BoxSlice<T> {
    len: usize,
    ptr: *mut T,
}

impl<T> From<Box<[T]>> for BoxSlice<T> {
    fn from(mut other: Box<[T]>) -> Self {
        let out = BoxSlice {
            len: other.len(),
            ptr: other.as_mut_ptr(),
        };
        mem::forget(other);
        out
    }
}

unsafe impl<T: Send> Send for BoxSlice<T> {}
unsafe impl<T: Sync> Sync for BoxSlice<T> {}

impl<T> Drop for BoxSlice<T> {
    fn drop(&mut self) {
        unsafe { Vec::from_raw_parts(self.ptr, self.len, self.len) };
    }
}

type BoxByteSlice = BoxSlice<u8>;

pub struct VmCtx {
    mem: BoxByteSlice,
}

impl VmCtx {
    pub fn offset_of_memory_ptr() -> u32 {
        (offset_of!(VmCtx, mem) + offset_of!(BoxByteSlice, ptr))
            .try_into()
            .expect("Offset exceeded size of u32")
    }

    pub fn offset_of_memory_len() -> u32 {
        (offset_of!(VmCtx, mem) + offset_of!(BoxByteSlice, len))
            .try_into()
            .expect("Offset exceeded size of u32")
    }
}

#[derive(Default, Debug)]
pub struct SimpleContext {
    types: Vec<FuncType>,
    func_ty_indices: Vec<u32>,
}

pub const WASM_PAGE_SIZE: usize = 65_536;

pub trait Signature {
    type Type: SigType;

    fn params(&self) -> &[Self::Type];
    fn returns(&self) -> &[Self::Type];
}

pub trait SigType {
    fn to_microwasm_type(&self) -> microwasm::SignlessType;
}

impl SigType for ir::Type {
    fn to_microwasm_type(&self) -> microwasm::SignlessType {
        use crate::microwasm::{Size::*, Type::*};

        if self.is_int() {
            match self.bits() {
                32 => Int(_32),
                64 => Int(_64),
                _ => unimplemented!(),
            }
        } else if self.is_float() {
            match self.bits() {
                32 => Float(_32),
                64 => Float(_64),
                _ => unimplemented!(),
            }
        } else {
            unimplemented!()
        }
    }
}

impl SigType for AbiParam {
    fn to_microwasm_type(&self) -> microwasm::SignlessType {
        self.value_type.to_microwasm_type()
    }
}

impl Signature for CraneliftSignature {
    type Type = AbiParam;

    fn params(&self) -> &[Self::Type] {
        // TODO: We want to instead add the `VMContext` to the signature used by
        //       cranelift, removing the special-casing from the internals.
        assert_eq!(self.params[0].purpose, ir::ArgumentPurpose::VMContext);
        // `self.params[1]` should be caller vmctx
        assert_eq!(self.call_conv, isa::CallConv::SystemV);
        &self.params[2..]
    }

    fn returns(&self) -> &[Self::Type] {
        assert_eq!(self.call_conv, isa::CallConv::SystemV);
        &self.returns
    }
}

impl SigType for wasmparser::Type {
    fn to_microwasm_type(&self) -> microwasm::SignlessType {
        microwasm::Type::from_wasm(*self).unwrap()
    }
}

impl Signature for FuncType {
    type Type = wasmparser::Type;

    fn params(&self) -> &[Self::Type] {
        &*self.params
    }

    fn returns(&self) -> &[Self::Type] {
        &*self.returns
    }
}

pub trait ModuleContext {
    type Signature: Signature;
    type GlobalType: SigType;

    fn vmctx_builtin_function(&self, index: u32) -> u32;
    fn vmctx_vmglobal_definition(&self, index: u32) -> u32;
    fn vmctx_vmglobal_import_from(&self, index: u32) -> u32;
    fn vmctx_vmmemory_import_from(&self, memory_index: u32) -> u32;
    fn vmctx_vmmemory_definition(&self, defined_memory_index: u32) -> u32;
    fn vmctx_vmmemory_definition_base(&self, defined_memory_index: u32) -> u32;
    fn vmctx_vmmemory_definition_current_length(&self, defined_memory_index: u32) -> u32;
    fn vmmemory_definition_base(&self) -> u8;
    fn vmmemory_definition_current_length(&self) -> u8;
    fn vmctx_vmtable_import_from(&self, table_index: u32) -> u32;
    fn vmctx_vmtable_definition(&self, defined_table_index: u32) -> u32;
    fn vmctx_vmtable_definition_base(&self, defined_table_index: u32) -> u32;
    fn vmctx_vmtable_definition_current_elements(&self, defined_table_index: u32) -> u32;
    fn vmctx_vmfunction_import_body(&self, func_index: u32) -> u32;
    fn vmctx_vmfunction_import_vmctx(&self, func_index: u32) -> u32;
    fn vmtable_definition_base(&self) -> u8;
    fn vmtable_definition_current_elements(&self) -> u8;
    fn vmctx_vmshared_signature_id(&self, signature_idx: u32) -> u32;
    fn vmcaller_checked_anyfunc_type_index(&self) -> u8;
    fn vmcaller_checked_anyfunc_func_ptr(&self) -> u8;
    fn vmcaller_checked_anyfunc_vmctx(&self) -> u8;
    fn size_of_vmcaller_checked_anyfunc(&self) -> u8;

    fn defined_table_index(&self, table_index: u32) -> Option<u32>;
    fn defined_memory_index(&self, index: u32) -> Option<u32>;

    fn defined_global_index(&self, global_index: u32) -> Option<u32>;
    fn global_type(&self, global_index: u32) -> &Self::GlobalType;

    fn func_type_index(&self, func_idx: u32) -> u32;
    fn signature(&self, index: u32) -> &Self::Signature;

    fn func_index(&self, defined_func_index: u32) -> u32;
    fn defined_func_index(&self, func_index: u32) -> Option<u32>;

    fn defined_func_type(&self, defined_func_idx: u32) -> &Self::Signature {
        self.func_type(self.func_index(defined_func_idx))
    }

    fn func_type(&self, func_idx: u32) -> &Self::Signature {
        self.signature(self.func_type_index(func_idx))
    }

    fn emit_memory_bounds_check(&self) -> bool {
        true
    }
}

impl ModuleContext for SimpleContext {
    type Signature = FuncType;
    type GlobalType = wasmparser::Type;

    // TODO: We don't support external functions yet
    fn func_index(&self, func_idx: u32) -> u32 {
        func_idx
    }

    fn defined_func_index(&self, func_idx: u32) -> Option<u32> {
        Some(func_idx)
    }

    fn func_type_index(&self, func_idx: u32) -> u32 {
        self.func_ty_indices[func_idx as usize]
    }

    fn defined_global_index(&self, _index: u32) -> Option<u32> {
        unimplemented!()
    }

    fn global_type(&self, _global_index: u32) -> &Self::GlobalType {
        unimplemented!()
    }

    fn signature(&self, index: u32) -> &Self::Signature {
        &self.types[index as usize]
    }

    fn vmctx_vmglobal_definition(&self, _index: u32) -> u32 {
        unimplemented!()
    }

    fn vmctx_vmglobal_import_from(&self, _index: u32) -> u32 {
        unimplemented!()
    }

    fn defined_memory_index(&self, _index: u32) -> Option<u32> {
        unimplemented!()
    }

    fn defined_table_index(&self, index: u32) -> Option<u32> {
        Some(index)
    }

    fn vmctx_builtin_function(&self, _index: u32) -> u32 {
        unimplemented!()
    }

    fn vmctx_vmfunction_import_body(&self, _func_index: u32) -> u32 {
        unimplemented!()
    }
    fn vmctx_vmfunction_import_vmctx(&self, _func_index: u32) -> u32 {
        unimplemented!()
    }

    fn vmctx_vmtable_import_from(&self, _table_index: u32) -> u32 {
        unimplemented!()
    }

    fn vmctx_vmmemory_definition(&self, _defined_memory_index: u32) -> u32 {
        unimplemented!()
    }
    fn vmctx_vmmemory_import_from(&self, _memory_index: u32) -> u32 {
        unimplemented!()
    }
    fn vmmemory_definition_base(&self) -> u8 {
        unimplemented!()
    }
    fn vmmemory_definition_current_length(&self) -> u8 {
        unimplemented!()
    }
    fn vmctx_vmmemory_definition_base(&self, defined_memory_index: u32) -> u32 {
        assert_eq!(defined_memory_index, 0);
        VmCtx::offset_of_memory_ptr()
    }

    fn vmctx_vmmemory_definition_current_length(&self, defined_memory_index: u32) -> u32 {
        assert_eq!(defined_memory_index, 0);
        VmCtx::offset_of_memory_len()
    }

    fn vmctx_vmtable_definition(&self, _defined_table_index: u32) -> u32 {
        unimplemented!()
    }

    fn vmctx_vmtable_definition_base(&self, _defined_table_index: u32) -> u32 {
        unimplemented!()
    }

    fn vmctx_vmtable_definition_current_elements(&self, _defined_table_index: u32) -> u32 {
        unimplemented!()
    }

    fn vmtable_definition_base(&self) -> u8 {
        unimplemented!()
    }

    fn vmtable_definition_current_elements(&self) -> u8 {
        unimplemented!()
    }

    fn vmcaller_checked_anyfunc_vmctx(&self) -> u8 {
        unimplemented!()
    }

    fn vmcaller_checked_anyfunc_type_index(&self) -> u8 {
        unimplemented!()
    }

    fn vmcaller_checked_anyfunc_func_ptr(&self) -> u8 {
        unimplemented!()
    }

    fn size_of_vmcaller_checked_anyfunc(&self) -> u8 {
        unimplemented!()
    }

    fn vmctx_vmshared_signature_id(&self, _signature_idx: u32) -> u32 {
        unimplemented!()
    }

    // TODO: type of a global
}

pub fn translate(data: &[u8]) -> Result<ExecutableModule, Error> {
    translate_only(data).map(|m| m.instantiate())
}

/// Translate from a slice of bytes holding a wasm module.
pub fn translate_only(data: &[u8]) -> Result<TranslatedModule, Error> {
    let mut output = TranslatedModule::default();

    for payload in Parser::new(0).parse_all(data) {
        match payload? {
            Payload::TypeSection(s) => output.ctx.types = translate_sections::type_(s)?,
            Payload::ImportSection(s) => translate_sections::import(s)?,
            Payload::FunctionSection(s) => {
                output.ctx.func_ty_indices = translate_sections::function(s)?;
            }
            Payload::TableSection(s) => {
                translate_sections::table(s)?;
            }
            Payload::MemorySection(s) => {
                let mem = translate_sections::memory(s)?;

                if mem.len() > 1 {
                    return Err(Error::Input(
                        "Multiple memory sections not yet implemented".to_string(),
                    ));
                }

                if !mem.is_empty() {
                    let mem = mem[0];
                    let limits = match mem {
                        MemoryType::M32 {
                            limits,
                            shared: false,
                        } => limits,
                        _ => return Err(Error::Input("unsupported memory".to_string())),
                    };
                    if Some(limits.initial) != limits.maximum {
                        return Err(Error::Input(
                            "Custom memory limits not supported in lightbeam".to_string(),
                        ));
                    }
                    output.memory = Some(limits);
                }
            }
            Payload::GlobalSection(s) => {
                translate_sections::global(s)?;
            }
            Payload::ExportSection(s) => {
                translate_sections::export(s)?;
            }
            Payload::StartSection { func, .. } => {
                translate_sections::start(func)?;
            }
            Payload::ElementSection(s) => {
                translate_sections::element(s)?;
            }
            Payload::DataSection(s) => {
                translate_sections::data(s)?;
            }
            Payload::CodeSectionStart { .. }
            | Payload::CustomSection { .. }
            | Payload::Version { .. } => {}

            other => unimplemented!("can't translate {:?}", other),
        }
    }

    Ok(output)
}
