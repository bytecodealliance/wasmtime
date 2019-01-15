use backend::TranslatedCodeSection;
use error::Error;
use std::borrow::Cow;
use std::mem;
use translate_sections;
use wasmparser::{FuncType, ModuleReader, SectionCode, Type};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature {
    params: Cow<'static, [Type]>,
    returns: Cow<'static, [Type]>,
}

impl PartialEq<FuncType> for Signature {
    fn eq(&self, other: &FuncType) -> bool {
        &self.params[..] == &other.params[..] && &self.returns[..] == &other.returns[..]
    }
}

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

pub trait FunctionArgs {
    unsafe fn call<T>(self, start: *const u8, vm_ctx: *const u8) -> T;
}

type VmCtx = u64;

macro_rules! impl_function_args {
    ($first:ident $(, $rest:ident)*) => {
        impl<$first, $($rest),*> FunctionArgs for ($first, $($rest),*) {
            #[allow(non_snake_case)]
            unsafe fn call<T>(self, start: *const u8, vm_ctx: *const u8) -> T {
                let func = mem::transmute::<_, extern "sysv64" fn($first $(, $rest)*, VmCtx) -> T>(start);
                {
                    let ($first, $($rest),*) = self;
                    func($first $(, $rest)*, vm_ctx as VmCtx)
                }
            }
        }

        impl<$first: AsValueType, $($rest: AsValueType),*> TypeList for ($first, $($rest),*) {
            const TYPE_LIST: &'static [Type] = &[$first::TYPE, $($rest::TYPE),*];
        }

        impl_function_args!($($rest),*);
    };
    () => {
        impl FunctionArgs for () {
            unsafe fn call<T>(self, start: *const u8, vm_ctx: *const u8) -> T {
                let func = mem::transmute::<_, extern "sysv64" fn(VmCtx) -> T>(start);
                func(vm_ctx as VmCtx)
            }
        }

        impl TypeList for () {
            const TYPE_LIST: &'static [Type] = &[];
        }
    };
}

impl_function_args!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S);

#[derive(Default, Debug)]
pub struct TranslatedModule {
    translated_code_section: Option<TranslatedCodeSection>,
    types: FuncTyStore,
    // Note: This vector should never be deallocated or reallocated or the pointer
    //       to its contents otherwise invalidated while the JIT'd code is still
    //       callable.
    // TODO: Should we wrap this in a `Mutex` so that calling functions from multiple
    //       threads doesn't cause data races?
    memory: Option<Vec<u8>>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ExecutionError {
    FuncIndexOutOfBounds,
    TypeMismatch,
}

impl TranslatedModule {
    // For testing only.
    // TODO: Handle generic signatures.
    pub fn execute_func<Args: FunctionArgs + TypeList, T: TypeList>(
        &self,
        func_idx: u32,
        args: Args,
    ) -> Result<T, ExecutionError> {
        let code_section = self
            .translated_code_section
            .as_ref()
            .expect("no code section");

        if func_idx as usize >= self.types.func_ty_indicies.len() {
            return Err(ExecutionError::FuncIndexOutOfBounds);
        }

        let type_ = self.types.func_type(func_idx);

        if (&type_.params[..], &type_.returns[..]) != (Args::TYPE_LIST, T::TYPE_LIST) {
            return Err(ExecutionError::TypeMismatch);
        }

        let start_buf = code_section.func_start(func_idx as usize);

        Ok(unsafe {
            args.call(
                start_buf,
                self.memory
                    .as_ref()
                    .map(|b| b.as_ptr())
                    .unwrap_or(std::ptr::null()),
            )
        })
    }

    pub fn disassemble(&self) {
        self.translated_code_section
            .as_ref()
            .expect("no code section")
            .disassemble();
    }
}

#[derive(Default, Debug)]
pub struct FuncTyStore {
    types: Vec<FuncType>,
    func_ty_indicies: Vec<u32>,
}

impl FuncTyStore {
    pub fn func_type(&self, func_idx: u32) -> &FuncType {
        // TODO: This assumes that there is no imported functions.
        let func_ty_idx = self.func_ty_indicies[func_idx as usize];
        &self.types[func_ty_idx as usize]
    }

    // TODO: type of a global
}

/// Translate from a slice of bytes holding a wasm module.
pub fn translate(data: &[u8]) -> Result<TranslatedModule, Error> {
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
        translate_sections::table(tables)?;

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
            output.memory = Some(vec![0; mem.limits.initial as usize * 65_536]);
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
