use backend::TranslatedCodeSection;
use error::Error;
use std::{
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
    types: FuncTyStore,
    // TODO: Should we wrap this in a `Mutex` so that calling functions from multiple
    //       threads doesn't cause data races?
    table: Option<(TableType, Vec<RuntimeFunc>)>,
    memory: Option<MemoryType>,
}

fn quickhash<H: Hash>(h: H) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    h.hash(&mut hasher);
    hasher.finish()
}

impl TranslatedModule {
    pub fn instantiate(mut self) -> ExecutableModule {
        use std::alloc::{self, Layout};

        let slice = self
            .table
            .as_mut()
            .map(|&mut (_, ref mut initial)| {
                initial.shrink_to_fit();
                let out = BoxSlice {
                    ptr: initial.as_mut_ptr(),
                    len: initial.len(),
                };
                mem::forget(mem::replace(initial, Default::default()));
                out
            })
            .unwrap_or(BoxSlice {
                ptr: std::ptr::NonNull::dangling().as_ptr(),
                len: 0,
            });

        let mem_size = self.memory.map(|m| m.limits.initial).unwrap_or(0) as usize;
        let layout = Layout::new::<VmCtx>()
            .extend(Layout::array::<u8>(mem_size * WASM_PAGE_SIZE).unwrap())
            .unwrap()
            .0;

        let ptr = unsafe { alloc::alloc_zeroed(layout) } as *mut VmCtx;

        unsafe {
            *ptr = VmCtx {
                table: slice,
                mem_size,
            }
        }

        ExecutableModule {
            module: self,
            context: Allocation { ptr, layout },
        }
    }

    pub fn disassemble(&self) {
        self.translated_code_section
            .as_ref()
            .expect("no code section")
            .disassemble();
    }
}

struct Allocation<T> {
    ptr: *mut T,
    layout: std::alloc::Layout,
}

unsafe impl<T> Send for Allocation<T> where T: Send {}
unsafe impl<T> Sync for Allocation<T> where T: Sync {}

impl<T> Drop for Allocation<T> {
    fn drop(&mut self) {
        if mem::needs_drop::<T>() {
            unsafe { std::ptr::drop_in_place::<T>(self.ptr) };
        }

        unsafe { std::alloc::dealloc(self.ptr as _, self.layout) };
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ExecutionError {
    FuncIndexOutOfBounds,
    TypeMismatch,
}

pub struct ExecutableModule {
    module: TranslatedModule,
    context: Allocation<VmCtx>,
}

impl ExecutableModule {
    // For testing only.
    // TODO: Handle generic signatures.
    pub fn execute_func<Args: FunctionArgs<T> + TypeList, T: TypeList>(
        &self,
        func_idx: u32,
        args: Args,
    ) -> Result<T, ExecutionError> {
        let module = &self.module;
        let code_section = module
            .translated_code_section
            .as_ref()
            .expect("no code section");

        if func_idx as usize >= module.types.func_ty_indicies.len() {
            return Err(ExecutionError::FuncIndexOutOfBounds);
        }

        let type_ = module.types.func_type(func_idx);

        if (&type_.params[..], &type_.returns[..]) != (Args::TYPE_LIST, T::TYPE_LIST) {
            return Err(ExecutionError::TypeMismatch);
        }

        let start_buf = code_section.func_start(func_idx as usize);

        Ok(unsafe {
            args.call(
                Args::into_func(start_buf),
                self.context.ptr as *const VmCtx as *const u8,
            )
        })
    }

    pub fn disassemble(&self) {
        self.module.disassemble();
    }
}

type FuncRef = unsafe extern "sysv64" fn();

#[repr(C)]
pub struct RuntimeFunc {
    sig_hash: u32,
    func_start: FuncRef,
}

#[repr(C)]
struct BoxSlice<T> {
    len: usize,
    ptr: *mut T,
}

#[repr(C)]
pub struct VmCtx {
    table: BoxSlice<RuntimeFunc>,
    mem_size: usize,
}

unsafe impl Send for VmCtx {}
unsafe impl Sync for VmCtx {}

impl VmCtx {
    pub fn offset_of_memory() -> usize {
        mem::size_of::<Self>()
    }
}

impl<T> Drop for BoxSlice<T> {
    fn drop(&mut self) {
        unsafe { Vec::from_raw_parts(self.ptr, self.len, self.len) };
    }
}

#[derive(Default, Debug)]
pub struct FuncTyStore {
    types: Vec<FuncType>,
    func_ty_indicies: Vec<u32>,
}

const WASM_PAGE_SIZE: usize = 65_536;

impl FuncTyStore {
    pub fn func_count(&self) -> usize {
        self.func_ty_indicies.len()
    }

    pub fn func_type_index(&self, func_idx: u32) -> u32 {
        self.func_ty_indicies[func_idx as usize]
    }

    pub fn signature(&self, index: u32) -> &FuncType {
        &self.types[index as usize]
    }

    pub fn func_type(&self, func_idx: u32) -> &FuncType {
        // TODO: This assumes that there are no imported functions.
        self.signature(self.func_type_index(func_idx))
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
        let tables = translate_sections::table(tables)?;

        assert!(tables.len() <= 1);

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
        translate_sections::element(elements)?;

        reader.skip_custom_sections()?;
        if reader.eof() {
            return Ok(output);
        }
        section = reader.read()?;
    }

    if let SectionCode::Code = section.code {
        let code = section.get_code_section_reader()?;
        output.translated_code_section = Some(translate_sections::code(
            code,
            &output.types,
            output.memory.is_some(),
        )?);

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
