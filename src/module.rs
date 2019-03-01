use crate::microwasm;
use backend::TranslatedCodeSection;
use cranelift_codegen::{
    ir::{self, AbiParam, Signature as CraneliftSignature},
    isa,
};
use error::Error;
use std::{
    convert::TryInto,
    hash::{Hash, Hasher},
    mem,
};
use translate_sections;
use wasmparser::{FuncType, MemoryType, ModuleReader, SectionCode, TableType, Type};

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
    types: SimpleContext,
    // TODO: Should we wrap this in a `Mutex` so that calling functions from multiple
    //       threads doesn't cause data races?
    table: Option<(TableType, Vec<u32>)>,
    memory: Option<MemoryType>,
}

pub fn quickhash<H: Hash>(h: H) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    h.hash(&mut hasher);
    hasher.finish()
}

impl TranslatedModule {
    pub fn instantiate(mut self) -> ExecutableModule {
        let table = {
            let code_section = self
                .translated_code_section
                .as_ref()
                .expect("We don't currently support a table section without a code section");
            let types = &self.types;

            self.table
                .as_mut()
                .map(|&mut (_, ref mut idxs)| {
                    let initial = idxs
                        .iter()
                        .map(|i| {
                            let start = code_section.func_start(*i as _);
                            let ty = types.func_type(*i);

                            RuntimeFunc {
                                func_start: start,
                                sig_hash: quickhash(ty) as u32,
                            }
                        })
                        .collect::<Vec<_>>();
                    let out = BoxSlice::from(initial.into_boxed_slice());
                    out
                })
                .unwrap_or(BoxSlice {
                    ptr: std::ptr::NonNull::dangling().as_ptr(),
                    len: 0,
                })
        };

        let mem_size = self.memory.map(|m| m.limits.initial).unwrap_or(0) as usize;
        let mem: BoxSlice<_> = vec![0u8; mem_size * WASM_PAGE_SIZE]
            .into_boxed_slice()
            .into();

        let ctx = if mem.len > 0 || table.len > 0 {
            Some(Box::new(VmCtx { table, mem }))
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ExecutionError {
    FuncIndexOutOfBounds,
    TypeMismatch,
}

pub struct ExecutableModule {
    module: TranslatedModule,
    context: Option<Box<VmCtx>>,
}

impl ExecutableModule {
    /// Executes the function _without checking types_. This can cause undefined
    /// memory to be accessed.
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

        if func_idx as usize >= module.types.func_ty_indicies.len() {
            return Err(ExecutionError::FuncIndexOutOfBounds);
        }

        let type_ = module.types.func_type(func_idx);

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

type FuncRef = *const u8;

pub struct RuntimeFunc {
    sig_hash: u32,
    func_start: FuncRef,
}

unsafe impl Send for RuntimeFunc {}
unsafe impl Sync for RuntimeFunc {}

impl RuntimeFunc {
    pub fn offset_of_sig_hash() -> usize {
        offset_of!(Self, sig_hash)
    }

    pub fn offset_of_func_start() -> usize {
        offset_of!(Self, func_start)
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

pub struct VmCtx {
    table: BoxSlice<RuntimeFunc>,
    mem: BoxSlice<u8>,
}

impl VmCtx {
    pub fn offset_of_memory_ptr() -> u8 {
        offset_of!(Self, mem.ptr)
            .try_into()
            .expect("Offset exceeded size of u8")
    }

    pub fn offset_of_memory_len() -> u8 {
        offset_of!(Self, mem.len)
            .try_into()
            .expect("Offset exceeded size of u8")
    }

    pub fn offset_of_funcs_ptr() -> u8 {
        offset_of!(Self, table.ptr)
            .try_into()
            .expect("Offset exceeded size of u8")
    }

    pub fn offset_of_funcs_len() -> u8 {
        offset_of!(Self, table.len)
            .try_into()
            .expect("Offset exceeded size of u8")
    }
}

#[derive(Default, Debug)]
pub struct SimpleContext {
    types: Vec<FuncType>,
    func_ty_indicies: Vec<u32>,
}

const WASM_PAGE_SIZE: usize = 65_536;

pub trait Signature {
    type Type: SigType;

    fn params(&self) -> &[Self::Type];
    fn returns(&self) -> &[Self::Type];
}

pub trait SigType {
    fn to_microwasm_type(&self) -> microwasm::SignlessType;
    fn is_float(&self) -> bool;
}

impl SigType for AbiParam {
    fn to_microwasm_type(&self) -> microwasm::SignlessType {
        use microwasm::{Size::*, Type::*};

        if self.value_type.is_int() {
            match self.value_type.bits() {
                32 => Int(_32),
                64 => Int(_64),
                _ => unimplemented!(),
            }
        } else if self.value_type.is_float() {
            match self.value_type.bits() {
                32 => Float(_32),
                64 => Float(_64),
                _ => unimplemented!(),
            }
        } else {
            unimplemented!()
        }
    }

    fn is_float(&self) -> bool {
        self.value_type.is_float()
    }
}

impl Signature for CraneliftSignature {
    type Type = AbiParam;

    fn params(&self) -> &[Self::Type] {
        // TODO: We want to instead add the `VMContext` to the signature used by
        //       cranelift, removing the special-casing from the internals.
        assert_eq!(self.params[0].purpose, ir::ArgumentPurpose::VMContext);
        assert_eq!(self.call_conv, isa::CallConv::SystemV);
        &self.params[1..]
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

    fn is_float(&self) -> bool {
        match self {
            wasmparser::Type::F32 | wasmparser::Type::F64 => true,
            _ => false,
        }
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
    type Signature: Signature + Hash;

    fn func_type_index(&self, func_idx: u32) -> u32;
    fn signature(&self, index: u32) -> &Self::Signature;
    fn offset_of_memory_ptr(&self) -> u8;
    fn offset_of_memory_len(&self) -> u8;
    fn offset_of_funcs_ptr(&self) -> u8;
    fn offset_of_funcs_len(&self) -> u8;

    fn func_index(&self, defined_func_index: u32) -> u32;
    fn defined_func_index(&self, func_index: u32) -> Option<u32>;

    fn defined_func_type(&self, func_idx: u32) -> &Self::Signature {
        // TODO: This assumes that there are no imported functions.
        self.func_type(self.func_index(func_idx))
    }

    fn func_type(&self, func_idx: u32) -> &Self::Signature {
        // TODO: This assumes that there are no imported functions.
        self.signature(self.func_type_index(func_idx))
    }
}

impl ModuleContext for SimpleContext {
    type Signature = FuncType;

    // TODO: We don't support external functions yet
    fn func_index(&self, func_idx: u32) -> u32 {
        func_idx
    }

    fn defined_func_index(&self, func_idx: u32) -> Option<u32> {
        Some(func_idx)
    }

    fn func_type_index(&self, func_idx: u32) -> u32 {
        self.func_ty_indicies[func_idx as usize]
    }

    fn signature(&self, index: u32) -> &Self::Signature {
        &self.types[index as usize]
    }

    fn offset_of_memory_ptr(&self) -> u8 {
        VmCtx::offset_of_memory_ptr()
    }

    fn offset_of_memory_len(&self) -> u8 {
        VmCtx::offset_of_memory_len()
    }

    fn offset_of_funcs_ptr(&self) -> u8 {
        VmCtx::offset_of_funcs_ptr()
    }

    fn offset_of_funcs_len(&self) -> u8 {
        VmCtx::offset_of_funcs_len()
    }

    // TODO: type of a global
}

pub fn translate(data: &[u8]) -> Result<ExecutableModule, Error> {
    translate_only(data).map(|m| m.instantiate())
}

/// Translate from a slice of bytes holding a wasm module.
pub fn translate_only(data: &[u8]) -> Result<TranslatedModule, Error> {
    let mut reader = ModuleReader::new(data)?;
    let mut output = TranslatedModule::default();
    let mut table = None;

    reader.skip_custom_sections()?;
    if reader.eof() {
        return Ok(output);
    }
    let mut section = reader.read()?;

    if let SectionCode::Type = section.code {
        let types_reader = section.get_type_section_reader()?;
        output.types.types = translate_sections::type_(types_reader)?;

        reader.skip_custom_sections()?;
        if reader.eof() {
            return Ok(output);
        }
        section = reader.read()?;
    }

    if let SectionCode::Import = section.code {
        let imports = section.get_import_section_reader()?;
        translate_sections::import(imports)?;

        reader.skip_custom_sections()?;
        if reader.eof() {
            return Ok(output);
        }
        section = reader.read()?;
    }

    if let SectionCode::Function = section.code {
        let functions = section.get_function_section_reader()?;
        output.types.func_ty_indicies = translate_sections::function(functions)?;

        reader.skip_custom_sections()?;
        if reader.eof() {
            return Ok(output);
        }
        section = reader.read()?;
    }

    if let SectionCode::Table = section.code {
        let tables = section.get_table_section_reader()?;
        let mut tables = translate_sections::table(tables)?;

        assert!(tables.len() <= 1);

        table = tables.drain(..).next();

        reader.skip_custom_sections()?;
        if reader.eof() {
            return Ok(output);
        }
        section = reader.read()?;
    }

    if let SectionCode::Memory = section.code {
        let memories = section.get_memory_section_reader()?;
        let mem = translate_sections::memory(memories)?;

        assert!(
            mem.len() <= 1,
            "Multiple memory sections not yet unimplemented"
        );

        if !mem.is_empty() {
            let mem = mem[0];
            assert_eq!(Some(mem.limits.initial), mem.limits.maximum);
            output.memory = Some(mem);
        }

        reader.skip_custom_sections()?;
        if reader.eof() {
            return Ok(output);
        }
        section = reader.read()?;
    }

    if let SectionCode::Global = section.code {
        let globals = section.get_global_section_reader()?;
        translate_sections::global(globals)?;

        reader.skip_custom_sections()?;
        if reader.eof() {
            return Ok(output);
        }
        section = reader.read()?;
    }

    if let SectionCode::Export = section.code {
        let exports = section.get_export_section_reader()?;
        translate_sections::export(exports)?;

        reader.skip_custom_sections()?;
        if reader.eof() {
            return Ok(output);
        }
        section = reader.read()?;
    }

    if let SectionCode::Start = section.code {
        let start = section.get_start_section_content()?;
        translate_sections::start(start)?;

        reader.skip_custom_sections()?;
        if reader.eof() {
            return Ok(output);
        }
        section = reader.read()?;
    }

    if let SectionCode::Element = section.code {
        let elements = section.get_element_section_reader()?;
        let elements = translate_sections::element(elements)?;

        output.table = Some((
            table.expect("Element section with no table section"),
            elements,
        ));

        reader.skip_custom_sections()?;
        if reader.eof() {
            return Ok(output);
        }
        section = reader.read()?;
    }

    if let SectionCode::Code = section.code {
        let code = section.get_code_section_reader()?;
        output.translated_code_section = Some(translate_sections::code(code, &output.types)?);

        reader.skip_custom_sections()?;
        if reader.eof() {
            return Ok(output);
        }
        section = reader.read()?;
    }

    if let SectionCode::Data = section.code {
        let data = section.get_data_section_reader()?;
        translate_sections::data(data)?;
    }

    assert!(reader.eof());

    Ok(output)
}
