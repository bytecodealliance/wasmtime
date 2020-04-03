//! Defines `FaerieBackend`.

use crate::container;
use anyhow::anyhow;
use cranelift_codegen::binemit::{
    Addend, CodeOffset, NullStackmapSink, Reloc, RelocSink, Stackmap, StackmapSink, TrapSink,
};
use cranelift_codegen::isa::TargetIsa;
use cranelift_codegen::{self, binemit, ir};
use cranelift_module::{
    Backend, DataContext, DataDescription, DataId, FuncId, Init, Linkage, ModuleError,
    ModuleNamespace, ModuleResult,
};
use faerie;
use std::convert::TryInto;
use std::fs::File;
use target_lexicon::Triple;

/// A builder for `FaerieBackend`.
pub struct FaerieBuilder {
    isa: Box<dyn TargetIsa>,
    name: String,
    libcall_names: Box<dyn Fn(ir::LibCall) -> String>,
}

impl FaerieBuilder {
    /// Create a new `FaerieBuilder` using the given Cranelift target, that
    /// can be passed to
    /// [`Module::new`](cranelift_module::Module::new)
    ///
    /// Faerie output requires that TargetIsa have PIC (Position Independent Code) enabled.
    ///
    /// The `libcall_names` function provides a way to translate `cranelift_codegen`'s `ir::LibCall`
    /// enum to symbols. LibCalls are inserted in the IR as part of the legalization for certain
    /// floating point instructions, and for stack probes. If you don't know what to use for this
    /// argument, use `cranelift_module::default_libcall_names()`.
    pub fn new(
        isa: Box<dyn TargetIsa>,
        name: String,
        libcall_names: Box<dyn Fn(ir::LibCall) -> String>,
    ) -> ModuleResult<Self> {
        if !isa.flags().is_pic() {
            return Err(ModuleError::Backend(anyhow!(
                "faerie requires TargetIsa be PIC"
            )));
        }
        Ok(Self {
            isa,
            name,
            libcall_names,
        })
    }
}

/// A `FaerieBackend` implements `Backend` and emits ".o" files using the `faerie` library.
///
/// See the `FaerieBuilder` for a convenient way to construct `FaerieBackend` instances.
pub struct FaerieBackend {
    isa: Box<dyn TargetIsa>,
    artifact: faerie::Artifact,
    libcall_names: Box<dyn Fn(ir::LibCall) -> String>,
}

pub struct FaerieCompiledFunction {
    code_length: u32,
}

impl FaerieCompiledFunction {
    pub fn code_length(&self) -> u32 {
        self.code_length
    }
}

pub struct FaerieCompiledData {}

impl Backend for FaerieBackend {
    type Builder = FaerieBuilder;

    type CompiledFunction = FaerieCompiledFunction;
    type CompiledData = FaerieCompiledData;

    // There's no need to return individual artifacts; we're writing them into
    // the output file instead.
    type FinalizedFunction = ();
    type FinalizedData = ();

    /// The returned value here provides functions for emitting object files
    /// to memory and files.
    type Product = FaerieProduct;

    /// Create a new `FaerieBackend` using the given Cranelift target.
    fn new(builder: FaerieBuilder) -> Self {
        Self {
            artifact: faerie::Artifact::new(builder.isa.triple().clone(), builder.name),
            isa: builder.isa,
            libcall_names: builder.libcall_names,
        }
    }

    fn isa(&self) -> &dyn TargetIsa {
        &*self.isa
    }

    fn declare_function(&mut self, _id: FuncId, name: &str, linkage: Linkage) {
        self.artifact
            .declare(name, translate_function_linkage(linkage))
            .expect("inconsistent declarations");
    }

    fn declare_data(
        &mut self,
        _id: DataId,
        name: &str,
        linkage: Linkage,
        writable: bool,
        tls: bool,
        align: Option<u8>,
    ) {
        assert!(!tls, "Faerie doesn't yet support TLS");
        self.artifact
            .declare(name, translate_data_linkage(linkage, writable, align))
            .expect("inconsistent declarations");
    }

    fn define_function<TS>(
        &mut self,
        _id: FuncId,
        name: &str,
        ctx: &cranelift_codegen::Context,
        namespace: &ModuleNamespace<Self>,
        total_size: u32,
        trap_sink: &mut TS,
    ) -> ModuleResult<FaerieCompiledFunction>
    where
        TS: TrapSink,
    {
        let mut code: Vec<u8> = vec![0; total_size as usize];
        // TODO: Replace this with FaerieStackmapSink once it is implemented.
        let mut stackmap_sink = NullStackmapSink {};

        // Non-lexical lifetimes would obviate the braces here.
        {
            let mut reloc_sink = FaerieRelocSink {
                triple: self.isa.triple().clone(),
                artifact: &mut self.artifact,
                name,
                namespace,
                libcall_names: &*self.libcall_names,
            };

            unsafe {
                ctx.emit_to_memory(
                    &*self.isa,
                    code.as_mut_ptr(),
                    &mut reloc_sink,
                    trap_sink,
                    &mut stackmap_sink,
                )
            };
        }

        // because `define` will take ownership of code, this is our last chance
        let code_length = code.len() as u32;

        self.artifact
            .define(name, code)
            .expect("inconsistent declaration");

        Ok(FaerieCompiledFunction { code_length })
    }

    fn define_function_bytes(
        &mut self,
        _id: FuncId,
        name: &str,
        bytes: &[u8],
        _namespace: &ModuleNamespace<Self>,
    ) -> ModuleResult<FaerieCompiledFunction> {
        let code_length: u32 = match bytes.len().try_into() {
            Ok(code_length) => code_length,
            _ => Err(ModuleError::FunctionTooLarge(name.to_string()))?,
        };

        self.artifact
            .define(name, bytes.to_vec())
            .expect("inconsistent declaration");

        Ok(FaerieCompiledFunction { code_length })
    }

    fn define_data(
        &mut self,
        _id: DataId,
        name: &str,
        _writable: bool,
        tls: bool,
        _align: Option<u8>,
        data_ctx: &DataContext,
        namespace: &ModuleNamespace<Self>,
    ) -> ModuleResult<FaerieCompiledData> {
        assert!(!tls, "Faerie doesn't yet support TLS");
        let &DataDescription {
            ref init,
            ref function_decls,
            ref data_decls,
            ref function_relocs,
            ref data_relocs,
        } = data_ctx.description();

        for &(offset, id) in function_relocs {
            let to = &namespace.get_function_decl(&function_decls[id]).name;
            self.artifact
                .link(faerie::Link {
                    from: name,
                    to,
                    at: u64::from(offset),
                })
                .map_err(|e| ModuleError::Backend(e.into()))?;
        }
        for &(offset, id, addend) in data_relocs {
            debug_assert_eq!(
                addend, 0,
                "faerie doesn't support addends in data section relocations yet"
            );
            let to = &namespace.get_data_decl(&data_decls[id]).name;
            self.artifact
                .link(faerie::Link {
                    from: name,
                    to,
                    at: u64::from(offset),
                })
                .map_err(|e| ModuleError::Backend(e.into()))?;
        }

        match *init {
            Init::Uninitialized => {
                panic!("data is not initialized yet");
            }
            Init::Zeros { size } => {
                self.artifact
                    .define_zero_init(name, size)
                    .expect("inconsistent declaration");
            }
            Init::Bytes { ref contents } => {
                self.artifact
                    .define(name, contents.to_vec())
                    .expect("inconsistent declaration");
            }
        }

        Ok(FaerieCompiledData {})
    }

    fn write_data_funcaddr(
        &mut self,
        _data: &mut FaerieCompiledData,
        _offset: usize,
        _what: ir::FuncRef,
    ) {
        unimplemented!()
    }

    fn write_data_dataaddr(
        &mut self,
        _data: &mut FaerieCompiledData,
        _offset: usize,
        _what: ir::GlobalValue,
        _usize: binemit::Addend,
    ) {
        unimplemented!()
    }

    fn finalize_function(
        &mut self,
        _id: FuncId,
        _func: &FaerieCompiledFunction,
        _namespace: &ModuleNamespace<Self>,
    ) {
        // Nothing to do.
    }

    fn get_finalized_function(&self, _func: &FaerieCompiledFunction) {
        // Nothing to do.
    }

    fn finalize_data(
        &mut self,
        _id: DataId,
        _data: &FaerieCompiledData,
        _namespace: &ModuleNamespace<Self>,
    ) {
        // Nothing to do.
    }

    fn get_finalized_data(&self, _data: &FaerieCompiledData) {
        // Nothing to do.
    }

    fn publish(&mut self) {
        // Nothing to do.
    }

    fn finish(self, _namespace: &ModuleNamespace<Self>) -> FaerieProduct {
        FaerieProduct {
            artifact: self.artifact,
        }
    }
}

/// This is the output of `Module`'s
/// [`finish`](../cranelift_module/struct.Module.html#method.finish) function.
/// It provides functions for writing out the object file to memory or a file.
#[derive(Debug)]
pub struct FaerieProduct {
    /// Faerie artifact with all functions, data, and links from the module defined
    pub artifact: faerie::Artifact,
}

impl FaerieProduct {
    /// Return the name of the output file. This is the name passed into `new`.
    pub fn name(&self) -> &str {
        &self.artifact.name
    }

    /// Call `emit` on the faerie `Artifact`, producing bytes in memory.
    pub fn emit(&self) -> Result<Vec<u8>, faerie::ArtifactError> {
        self.artifact.emit()
    }

    /// Call `write` on the faerie `Artifact`, writing to a file.
    pub fn write(&self, sink: File) -> Result<(), faerie::ArtifactError> {
        self.artifact.write(sink)
    }
}

fn translate_function_linkage(linkage: Linkage) -> faerie::Decl {
    match linkage {
        Linkage::Import => faerie::Decl::function_import().into(),
        Linkage::Local => faerie::Decl::function().into(),
        Linkage::Preemptible => faerie::Decl::function().weak().into(),
        Linkage::Hidden => faerie::Decl::function().global().hidden().into(),
        Linkage::Export => faerie::Decl::function().global().into(),
    }
}

fn translate_data_linkage(linkage: Linkage, writable: bool, align: Option<u8>) -> faerie::Decl {
    let align = align.map(u64::from);
    match linkage {
        Linkage::Import => faerie::Decl::data_import().into(),
        Linkage::Local => faerie::Decl::data()
            .with_writable(writable)
            .with_align(align)
            .into(),
        Linkage::Preemptible => faerie::Decl::data()
            .weak()
            .with_writable(writable)
            .with_align(align)
            .into(),
        Linkage::Hidden => faerie::Decl::data()
            .global()
            .hidden()
            .with_writable(writable)
            .with_align(align)
            .into(),
        Linkage::Export => faerie::Decl::data()
            .global()
            .with_writable(writable)
            .with_align(align)
            .into(),
    }
}

struct FaerieRelocSink<'a> {
    triple: Triple,
    artifact: &'a mut faerie::Artifact,
    name: &'a str,
    namespace: &'a ModuleNamespace<'a, FaerieBackend>,
    libcall_names: &'a dyn Fn(ir::LibCall) -> String,
}

impl<'a> RelocSink for FaerieRelocSink<'a> {
    fn reloc_block(&mut self, _offset: CodeOffset, _reloc: Reloc, _block_offset: CodeOffset) {
        unimplemented!();
    }

    fn reloc_external(
        &mut self,
        offset: CodeOffset,
        reloc: Reloc,
        name: &ir::ExternalName,
        addend: Addend,
    ) {
        let ref_name: String = match *name {
            ir::ExternalName::User { .. } => {
                if self.namespace.is_function(name) {
                    self.namespace.get_function_decl(name).name.clone()
                } else {
                    self.namespace.get_data_decl(name).name.clone()
                }
            }
            ir::ExternalName::LibCall(ref libcall) => {
                let sym = (self.libcall_names)(*libcall);
                self.artifact
                    .declare(sym.clone(), faerie::Decl::function_import())
                    .expect("faerie declaration of libcall");
                sym
            }
            _ => panic!("invalid ExternalName {}", name),
        };
        let (raw_reloc, raw_addend) = container::raw_relocation(reloc, &self.triple);
        // TODO: Handle overflow.
        let final_addend = addend + raw_addend;
        let addend_i32 = final_addend as i32;
        debug_assert!(i64::from(addend_i32) == final_addend);
        self.artifact
            .link_with(
                faerie::Link {
                    from: self.name,
                    to: &ref_name,
                    at: u64::from(offset),
                },
                faerie::Reloc::Raw {
                    reloc: raw_reloc,
                    addend: addend_i32,
                },
            )
            .expect("faerie relocation error");
    }

    fn reloc_jt(&mut self, _offset: CodeOffset, reloc: Reloc, _jt: ir::JumpTable) {
        match reloc {
            Reloc::X86PCRelRodata4 => {
                // Not necessary to record this unless we are going to split apart code and its
                // jumptbl/rodata.
            }
            _ => {
                panic!("Unhandled reloc");
            }
        }
    }

    fn reloc_constant(&mut self, _offset: CodeOffset, reloc: Reloc, _jt: ir::ConstantOffset) {
        match reloc {
            Reloc::X86PCRelRodata4 => {
                // Not necessary to record this unless we are going to split apart code and its
                // jumptbl/rodata.
            }
            _ => {
                panic!("Unhandled reloc");
            }
        }
    }
}

#[allow(dead_code)]
struct FaerieStackmapSink<'a> {
    artifact: &'a mut faerie::Artifact,
    namespace: &'a ModuleNamespace<'a, FaerieBackend>,
}

/// Faerie is currently not used in SpiderMonkey. Methods are unimplemented.
impl<'a> StackmapSink for FaerieStackmapSink<'a> {
    fn add_stackmap(&mut self, _: CodeOffset, _: Stackmap) {
        unimplemented!("faerie support for stackmaps");
    }
}
