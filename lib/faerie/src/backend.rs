//! Defines `FaerieBackend`.

use container;
use cretonne_codegen::binemit::{Addend, CodeOffset, NullTrapSink, Reloc, RelocSink};
use cretonne_codegen::isa::TargetIsa;
use cretonne_codegen::{self, binemit, ir};
use cretonne_module::{Backend, DataContext, DataDescription, Init, Linkage, ModuleError,
                      ModuleNamespace, ModuleResult};
use faerie;
use failure::Error;
use std::fs::File;
use target_lexicon::BinaryFormat;
use traps::{FaerieTrapManifest, FaerieTrapSink};

#[derive(Debug)]
/// Setting to enable collection of traps. Setting this to `Enabled` in
/// `FaerieBuilder` means that a `FaerieTrapManifest` will be present
/// in the `FaerieProduct`.
pub enum FaerieTrapCollection {
    /// `FaerieProduct::trap_manifest` will be `None`
    Disabled,
    /// `FaerieProduct::trap_manifest` will be `Some`
    Enabled,
}

/// A builder for `FaerieBackend`.
pub struct FaerieBuilder {
    isa: Box<TargetIsa>,
    name: String,
    format: BinaryFormat,
    collect_traps: FaerieTrapCollection,
    libcall_names: Box<Fn(ir::LibCall) -> String>,
}

impl FaerieBuilder {
    /// Create a new `FaerieBuilder` using the given Cretonne target, that
    /// can be passed to
    /// [`Module::new`](cretonne_module/struct.Module.html#method.new].
    ///
    /// Faerie output requires that TargetIsa have PIC (Position Independent Code) enabled.
    ///
    /// `collect_traps` setting determines whether trap information is collected in a
    /// `FaerieTrapManifest` available in the `FaerieProduct`.
    ///
    /// The `libcall_names` function provides a way to translate `cretonne_codegen`'s `ir::LibCall`
    /// enum to symbols. LibCalls are inserted in the IR as part of the legalization for certain
    /// floating point instructions, and for stack probes. If you don't know what to use for this
    /// argument, use `FaerieBuilder::default_libcall_names()`.
    pub fn new(
        isa: Box<TargetIsa>,
        name: String,
        format: BinaryFormat,
        collect_traps: FaerieTrapCollection,
        libcall_names: Box<Fn(ir::LibCall) -> String>,
    ) -> ModuleResult<Self> {
        if !isa.flags().is_pic() {
            return Err(ModuleError::Backend(
                "faerie requires TargetIsa be PIC".to_owned(),
            ));
        }
        Ok(Self {
            isa,
            name,
            format,
            collect_traps,
            libcall_names,
        })
    }

    /// Default names for `ir::LibCall`s. A function by this name is imported into the object as
    /// part of the translation of a `ir::ExternalName::LibCall` variant. Calls to a LibCall should
    /// only be inserted into the IR by the `cretonne_codegen` legalizer pass.
    pub fn default_libcall_names() -> Box<Fn(ir::LibCall) -> String> {
        Box::new(move |libcall| match libcall {
            ir::LibCall::Probestack => "__cretonne_probestack".to_owned(),
            ir::LibCall::CeilF32 => "ceilf".to_owned(),
            ir::LibCall::CeilF64 => "ceil".to_owned(),
            ir::LibCall::FloorF32 => "floorf".to_owned(),
            ir::LibCall::FloorF64 => "floor".to_owned(),
            ir::LibCall::TruncF32 => "truncf".to_owned(),
            ir::LibCall::TruncF64 => "trunc".to_owned(),
            ir::LibCall::NearestF32 => "nearbyintf".to_owned(),
            ir::LibCall::NearestF64 => "nearbyint".to_owned(),
        })
    }
}

/// A `FaerieBackend` implements `Backend` and emits ".o" files using the `faerie` library.
pub struct FaerieBackend {
    isa: Box<TargetIsa>,
    artifact: faerie::Artifact,
    format: BinaryFormat,
    trap_manifest: Option<FaerieTrapManifest>,
    libcall_names: Box<Fn(ir::LibCall) -> String>,
}

pub struct FaerieCompiledFunction {}

pub struct FaerieCompiledData {}

impl Backend for FaerieBackend {
    type Builder = FaerieBuilder;

    type CompiledFunction = FaerieCompiledFunction;
    type CompiledData = FaerieCompiledData;

    // There's no need to return invidual artifacts; we're writing them into
    // the output file instead.
    type FinalizedFunction = ();
    type FinalizedData = ();

    /// The returned value here provides functions for emitting object files
    /// to memory and files.
    type Product = FaerieProduct;

    /// Create a new `FaerieBackend` using the given Cretonne target.
    fn new(builder: FaerieBuilder) -> Self {
        Self {
            artifact: faerie::Artifact::new(builder.isa.triple().clone(), builder.name),
            isa: builder.isa,
            format: builder.format,
            trap_manifest: match builder.collect_traps {
                FaerieTrapCollection::Enabled => Some(FaerieTrapManifest::new()),
                FaerieTrapCollection::Disabled => None,
            },
            libcall_names: builder.libcall_names,
        }
    }

    fn isa(&self) -> &TargetIsa {
        &*self.isa
    }

    fn declare_function(&mut self, name: &str, linkage: Linkage) {
        self.artifact
            .declare(name, translate_function_linkage(linkage))
            .expect("inconsistent declarations");
    }

    fn declare_data(&mut self, name: &str, linkage: Linkage, writable: bool) {
        self.artifact
            .declare(name, translate_data_linkage(linkage, writable))
            .expect("inconsistent declarations");
    }

    fn define_function(
        &mut self,
        name: &str,
        ctx: &cretonne_codegen::Context,
        namespace: &ModuleNamespace<Self>,
        code_size: u32,
    ) -> ModuleResult<FaerieCompiledFunction> {
        let mut code: Vec<u8> = Vec::with_capacity(code_size as usize);
        code.resize(code_size as usize, 0);

        // Non-lexical lifetimes would obviate the braces here.
        {
            let mut reloc_sink = FaerieRelocSink {
                format: self.format,
                artifact: &mut self.artifact,
                name,
                namespace,
                libcall_names: &self.libcall_names,
            };

            if let Some(ref mut trap_manifest) = self.trap_manifest {
                let mut trap_sink = FaerieTrapSink::new(name, code_size);
                unsafe {
                    ctx.emit_to_memory(
                        &*self.isa,
                        code.as_mut_ptr(),
                        &mut reloc_sink,
                        &mut trap_sink,
                    )
                };
                trap_manifest.add_sink(trap_sink);
            } else {
                let mut trap_sink = NullTrapSink {};
                unsafe {
                    ctx.emit_to_memory(
                        &*self.isa,
                        code.as_mut_ptr(),
                        &mut reloc_sink,
                        &mut trap_sink,
                    )
                };
            }
        }

        self.artifact
            .define(name, code)
            .expect("inconsistent declaration");
        Ok(FaerieCompiledFunction {})
    }

    fn define_data(
        &mut self,
        name: &str,
        data_ctx: &DataContext,
        namespace: &ModuleNamespace<Self>,
    ) -> ModuleResult<FaerieCompiledData> {
        let &DataDescription {
            writable: _writable,
            ref init,
            ref function_decls,
            ref data_decls,
            ref function_relocs,
            ref data_relocs,
        } = data_ctx.description();

        let size = init.size();
        let mut bytes = Vec::with_capacity(size);
        match *init {
            Init::Uninitialized => {
                panic!("data is not initialized yet");
            }
            Init::Zeros { .. } => {
                bytes.resize(size, 0);
            }
            Init::Bytes { ref contents } => {
                bytes.extend_from_slice(contents);
            }
        }

        for &(offset, id) in function_relocs {
            let to = &namespace.get_function_decl(&function_decls[id]).name;
            self.artifact
                .link(faerie::Link {
                    from: name,
                    to,
                    at: offset as usize,
                })
                .map_err(|e| ModuleError::Backend(e.to_string()))?;
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
                    at: offset as usize,
                })
                .map_err(|e| ModuleError::Backend(e.to_string()))?;
        }

        self.artifact
            .define(name, bytes)
            .expect("inconsistent declaration");
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
        _func: &FaerieCompiledFunction,
        _namespace: &ModuleNamespace<Self>,
    ) {
        // Nothing to do.
    }

    fn finalize_data(&mut self, _data: &FaerieCompiledData, _namespace: &ModuleNamespace<Self>) {
        // Nothing to do.
    }

    fn finish(self) -> FaerieProduct {
        FaerieProduct {
            artifact: self.artifact,
            trap_manifest: self.trap_manifest,
        }
    }
}

/// This is the output of `Module`'s
/// [`finish`](../cretonne_module/struct.Module.html#method.finish) function.
/// It provides functions for writing out the object file to memory or a file.
pub struct FaerieProduct {
    /// Faerie artifact with all functions, data, and links from the module defined
    pub artifact: faerie::Artifact,
    /// Optional trap manifest. Contains `FaerieTrapManifest` when `FaerieBuilder.collect_traps` is
    /// set to `FaerieTrapCollection::Enabled`.
    pub trap_manifest: Option<FaerieTrapManifest>,
}

impl FaerieProduct {
    /// Return the name of the output file. This is the name passed into `new`.
    pub fn name(&self) -> &str {
        &self.artifact.name
    }

    /// Call `emit` on the faerie `Artifact`, producing bytes in memory.
    pub fn emit(&self) -> Result<Vec<u8>, Error> {
        self.artifact.emit()
    }

    /// Call `write` on the faerie `Artifact`, writing to a file.
    pub fn write(&self, sink: File) -> Result<(), Error> {
        self.artifact.write(sink)
    }
}

fn translate_function_linkage(linkage: Linkage) -> faerie::Decl {
    match linkage {
        Linkage::Import => faerie::Decl::FunctionImport,
        Linkage::Local => faerie::Decl::Function { global: false },
        Linkage::Preemptible | Linkage::Export => faerie::Decl::Function { global: true },
    }
}

fn translate_data_linkage(linkage: Linkage, writable: bool) -> faerie::Decl {
    match linkage {
        Linkage::Import => faerie::Decl::DataImport,
        Linkage::Local => faerie::Decl::Data {
            global: false,
            writeable: writable,
        },
        Linkage::Export => faerie::Decl::Data {
            global: true,
            writeable: writable,
        },
        Linkage::Preemptible => {
            unimplemented!("faerie doesn't support preemptible globals yet");
        }
    }
}

struct FaerieRelocSink<'a> {
    format: BinaryFormat,
    artifact: &'a mut faerie::Artifact,
    name: &'a str,
    namespace: &'a ModuleNamespace<'a, FaerieBackend>,
    libcall_names: &'a Box<Fn(ir::LibCall) -> String>,
}

impl<'a> RelocSink for FaerieRelocSink<'a> {
    fn reloc_ebb(&mut self, _offset: CodeOffset, _reloc: Reloc, _ebb_offset: CodeOffset) {
        unimplemented!();
    }

    fn reloc_external(
        &mut self,
        offset: CodeOffset,
        reloc: Reloc,
        name: &ir::ExternalName,
        addend: Addend,
    ) {
        let ref_name: String = match name {
            &ir::ExternalName::User { .. } => {
                if self.namespace.is_function(name) {
                    self.namespace.get_function_decl(name).name.clone()
                } else {
                    self.namespace.get_data_decl(name).name.clone()
                }
            }
            &ir::ExternalName::LibCall(ref libcall) => {
                let sym = (self.libcall_names)(*libcall);
                self.artifact
                    .declare(sym.clone(), faerie::Decl::FunctionImport)
                    .expect("faerie declaration of libcall");
                sym
            }
            _ => panic!("invalid ExternalName {}", name),
        };
        let addend_i32 = addend as i32;
        debug_assert!(i64::from(addend_i32) == addend);
        let raw_reloc = container::raw_relocation(reloc, self.format);
        self.artifact
            .link_with(
                faerie::Link {
                    from: self.name,
                    to: &ref_name,
                    at: offset as usize,
                },
                faerie::RelocOverride {
                    reloc: raw_reloc,
                    addend: addend_i32,
                },
            )
            .expect("faerie relocation error");
    }

    fn reloc_jt(&mut self, _offset: CodeOffset, _reloc: Reloc, _jt: ir::JumpTable) {
        unimplemented!();
    }
}
