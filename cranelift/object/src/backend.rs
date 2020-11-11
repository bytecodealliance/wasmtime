//! Defines `ObjectModule`.

use anyhow::anyhow;
use cranelift_codegen::entity::SecondaryMap;
use cranelift_codegen::isa::TargetIsa;
use cranelift_codegen::{self, ir};
use cranelift_codegen::{
    binemit::{Addend, CodeInfo, CodeOffset, NullStackMapSink, Reloc, RelocSink, TrapSink},
    CodegenError,
};
use cranelift_module::{
    DataContext, DataDescription, DataId, FuncId, Init, Linkage, Module, ModuleCompiledFunction,
    ModuleDeclarations, ModuleError, ModuleResult, RelocRecord,
};
use log::info;
use object::write::{
    Object, Relocation, SectionId, StandardSection, Symbol, SymbolId, SymbolSection,
};
use object::{
    RelocationEncoding, RelocationKind, SectionKind, SymbolFlags, SymbolKind, SymbolScope,
};
use std::collections::HashMap;
use std::convert::TryInto;
use std::mem;
use target_lexicon::PointerWidth;

/// A builder for `ObjectModule`.
pub struct ObjectBuilder {
    isa: Box<dyn TargetIsa>,
    binary_format: object::BinaryFormat,
    architecture: object::Architecture,
    endian: object::Endianness,
    name: Vec<u8>,
    libcall_names: Box<dyn Fn(ir::LibCall) -> String + Send + Sync>,
    function_alignment: u64,
    per_function_section: bool,
}

impl ObjectBuilder {
    /// Create a new `ObjectBuilder` using the given Cranelift target, that
    /// can be passed to [`ObjectModule::new`].
    ///
    /// The `libcall_names` function provides a way to translate `cranelift_codegen`'s `ir::LibCall`
    /// enum to symbols. LibCalls are inserted in the IR as part of the legalization for certain
    /// floating point instructions, and for stack probes. If you don't know what to use for this
    /// argument, use `cranelift_module::default_libcall_names()`.
    pub fn new<V: Into<Vec<u8>>>(
        isa: Box<dyn TargetIsa>,
        name: V,
        libcall_names: Box<dyn Fn(ir::LibCall) -> String + Send + Sync>,
    ) -> ModuleResult<Self> {
        let binary_format = match isa.triple().binary_format {
            target_lexicon::BinaryFormat::Elf => object::BinaryFormat::Elf,
            target_lexicon::BinaryFormat::Coff => object::BinaryFormat::Coff,
            target_lexicon::BinaryFormat::Macho => object::BinaryFormat::MachO,
            target_lexicon::BinaryFormat::Wasm => {
                return Err(ModuleError::Backend(anyhow!(
                    "binary format wasm is unsupported",
                )))
            }
            target_lexicon::BinaryFormat::Unknown => {
                return Err(ModuleError::Backend(anyhow!("binary format is unknown")))
            }
            other => {
                return Err(ModuleError::Backend(anyhow!(
                    "binary format {} not recognized",
                    other
                )))
            }
        };
        let architecture = match isa.triple().architecture {
            target_lexicon::Architecture::X86_32(_) => object::Architecture::I386,
            target_lexicon::Architecture::X86_64 => object::Architecture::X86_64,
            target_lexicon::Architecture::Arm(_) => object::Architecture::Arm,
            target_lexicon::Architecture::Aarch64(_) => object::Architecture::Aarch64,
            architecture => {
                return Err(ModuleError::Backend(anyhow!(
                    "target architecture {:?} is unsupported",
                    architecture,
                )))
            }
        };
        let endian = match isa.triple().endianness().unwrap() {
            target_lexicon::Endianness::Little => object::Endianness::Little,
            target_lexicon::Endianness::Big => object::Endianness::Big,
        };
        Ok(Self {
            isa,
            binary_format,
            architecture,
            endian,
            name: name.into(),
            libcall_names,
            function_alignment: 1,
            per_function_section: false,
        })
    }

    /// Set the alignment used for functions.
    pub fn function_alignment(&mut self, alignment: u64) -> &mut Self {
        self.function_alignment = alignment;
        self
    }

    /// Set if every function should end up in their own section.
    pub fn per_function_section(&mut self, per_function_section: bool) -> &mut Self {
        self.per_function_section = per_function_section;
        self
    }
}

/// An `ObjectModule` implements `Module` and emits ".o" files using the `object` library.
///
/// See the `ObjectBuilder` for a convenient way to construct `ObjectModule` instances.
pub struct ObjectModule {
    isa: Box<dyn TargetIsa>,
    object: Object,
    declarations: ModuleDeclarations,
    functions: SecondaryMap<FuncId, Option<(SymbolId, bool)>>,
    data_objects: SecondaryMap<DataId, Option<(SymbolId, bool)>>,
    relocs: Vec<SymbolRelocs>,
    libcalls: HashMap<ir::LibCall, SymbolId>,
    libcall_names: Box<dyn Fn(ir::LibCall) -> String + Send + Sync>,
    function_alignment: u64,
    per_function_section: bool,
}

impl ObjectModule {
    /// Create a new `ObjectModule` using the given Cranelift target.
    pub fn new(builder: ObjectBuilder) -> Self {
        let mut object = Object::new(builder.binary_format, builder.architecture, builder.endian);
        object.add_file_symbol(builder.name);
        Self {
            isa: builder.isa,
            object,
            declarations: ModuleDeclarations::default(),
            functions: SecondaryMap::new(),
            data_objects: SecondaryMap::new(),
            relocs: Vec::new(),
            libcalls: HashMap::new(),
            libcall_names: builder.libcall_names,
            function_alignment: builder.function_alignment,
            per_function_section: builder.per_function_section,
        }
    }
}

impl Module for ObjectModule {
    fn isa(&self) -> &dyn TargetIsa {
        &*self.isa
    }

    fn declarations(&self) -> &ModuleDeclarations {
        &self.declarations
    }

    fn declare_function(
        &mut self,
        name: &str,
        linkage: Linkage,
        signature: &ir::Signature,
    ) -> ModuleResult<FuncId> {
        let (id, decl) = self
            .declarations
            .declare_function(name, linkage, signature)?;

        let (scope, weak) = translate_linkage(decl.linkage);

        if let Some((function, _defined)) = self.functions[id] {
            let symbol = self.object.symbol_mut(function);
            symbol.scope = scope;
            symbol.weak = weak;
        } else {
            let symbol_id = self.object.add_symbol(Symbol {
                name: name.as_bytes().to_vec(),
                value: 0,
                size: 0,
                kind: SymbolKind::Text,
                scope,
                weak,
                section: SymbolSection::Undefined,
                flags: SymbolFlags::None,
            });
            self.functions[id] = Some((symbol_id, false));
        }

        Ok(id)
    }

    fn declare_data(
        &mut self,
        name: &str,
        linkage: Linkage,
        writable: bool,
        tls: bool,
    ) -> ModuleResult<DataId> {
        let (id, decl) = self
            .declarations
            .declare_data(name, linkage, writable, tls)?;

        let kind = if decl.tls {
            SymbolKind::Tls
        } else {
            SymbolKind::Data
        };
        let (scope, weak) = translate_linkage(decl.linkage);

        if let Some((data, _defined)) = self.data_objects[id] {
            let symbol = self.object.symbol_mut(data);
            symbol.kind = kind;
            symbol.scope = scope;
            symbol.weak = weak;
        } else {
            let symbol_id = self.object.add_symbol(Symbol {
                name: name.as_bytes().to_vec(),
                value: 0,
                size: 0,
                kind,
                scope,
                weak,
                section: SymbolSection::Undefined,
                flags: SymbolFlags::None,
            });
            self.data_objects[id] = Some((symbol_id, false));
        }

        Ok(id)
    }

    fn define_function<TS>(
        &mut self,
        func_id: FuncId,
        ctx: &mut cranelift_codegen::Context,
        trap_sink: &mut TS,
    ) -> ModuleResult<ModuleCompiledFunction>
    where
        TS: TrapSink,
    {
        info!(
            "defining function {}: {}",
            func_id,
            ctx.func.display(self.isa())
        );
        let CodeInfo {
            total_size: code_size,
            ..
        } = ctx.compile(self.isa())?;
        let mut code: Vec<u8> = vec![0; code_size as usize];
        let mut reloc_sink = ObjectRelocSink::default();
        let mut stack_map_sink = NullStackMapSink {};

        unsafe {
            ctx.emit_to_memory(
                &*self.isa,
                code.as_mut_ptr(),
                &mut reloc_sink,
                trap_sink,
                &mut stack_map_sink,
            )
        };

        self.define_function_bytes(func_id, &code, &reloc_sink.relocs)
    }

    fn define_function_bytes(
        &mut self,
        func_id: FuncId,
        bytes: &[u8],
        relocs: &[RelocRecord],
    ) -> ModuleResult<ModuleCompiledFunction> {
        info!("defining function {} with bytes", func_id);
        let total_size: u32 = match bytes.len().try_into() {
            Ok(total_size) => total_size,
            _ => Err(CodegenError::CodeTooLarge)?,
        };

        let decl = self.declarations.get_function_decl(func_id);
        if !decl.linkage.is_definable() {
            return Err(ModuleError::InvalidImportDefinition(decl.name.clone()));
        }

        let &mut (symbol, ref mut defined) = self.functions[func_id].as_mut().unwrap();
        if *defined {
            return Err(ModuleError::DuplicateDefinition(decl.name.clone()));
        }
        *defined = true;

        let (section, offset) = if self.per_function_section {
            let symbol_name = self.object.symbol(symbol).name.clone();
            let (section, offset) = self.object.add_subsection(
                StandardSection::Text,
                &symbol_name,
                bytes,
                self.function_alignment,
            );
            self.object.symbol_mut(symbol).section = SymbolSection::Section(section);
            self.object.symbol_mut(symbol).value = offset;
            (section, offset)
        } else {
            let section = self.object.section_id(StandardSection::Text);
            let offset =
                self.object
                    .add_symbol_data(symbol, section, bytes, self.function_alignment);
            (section, offset)
        };

        if !relocs.is_empty() {
            let relocs = relocs
                .iter()
                .map(|record| self.process_reloc(record))
                .collect();
            self.relocs.push(SymbolRelocs {
                section,
                offset,
                relocs,
            });
        }

        Ok(ModuleCompiledFunction { size: total_size })
    }

    fn define_data(&mut self, data_id: DataId, data_ctx: &DataContext) -> ModuleResult<()> {
        let decl = self.declarations.get_data_decl(data_id);
        if !decl.linkage.is_definable() {
            return Err(ModuleError::InvalidImportDefinition(decl.name.clone()));
        }

        let &mut (symbol, ref mut defined) = self.data_objects[data_id].as_mut().unwrap();
        if *defined {
            return Err(ModuleError::DuplicateDefinition(decl.name.clone()));
        }
        *defined = true;

        let &DataDescription {
            ref init,
            function_decls: _,
            data_decls: _,
            function_relocs: _,
            data_relocs: _,
            ref custom_segment_section,
            align,
        } = data_ctx.description();

        let pointer_reloc = match self.isa.triple().pointer_width().unwrap() {
            PointerWidth::U16 => unimplemented!("16bit pointers"),
            PointerWidth::U32 => Reloc::Abs4,
            PointerWidth::U64 => Reloc::Abs8,
        };
        let relocs = data_ctx
            .description()
            .all_relocs(pointer_reloc)
            .map(|record| self.process_reloc(&record))
            .collect::<Vec<_>>();

        let section = if custom_segment_section.is_none() {
            let section_kind = if let Init::Zeros { .. } = *init {
                if decl.tls {
                    StandardSection::UninitializedTls
                } else {
                    StandardSection::UninitializedData
                }
            } else if decl.tls {
                StandardSection::Tls
            } else if decl.writable {
                StandardSection::Data
            } else if relocs.is_empty() {
                StandardSection::ReadOnlyData
            } else {
                StandardSection::ReadOnlyDataWithRel
            };
            self.object.section_id(section_kind)
        } else {
            if decl.tls {
                return Err(cranelift_module::ModuleError::Backend(anyhow::anyhow!(
                    "Custom section not supported for TLS"
                )));
            }
            let (seg, sec) = &custom_segment_section.as_ref().unwrap();
            self.object.add_section(
                seg.clone().into_bytes(),
                sec.clone().into_bytes(),
                if decl.writable {
                    SectionKind::Data
                } else if relocs.is_empty() {
                    SectionKind::ReadOnlyData
                } else {
                    SectionKind::Data
                },
            )
        };

        let align = align.unwrap_or(1);
        let offset = match *init {
            Init::Uninitialized => {
                panic!("data is not initialized yet");
            }
            Init::Zeros { size } => self
                .object
                .add_symbol_bss(symbol, section, size as u64, align),
            Init::Bytes { ref contents } => self
                .object
                .add_symbol_data(symbol, section, &contents, align),
        };
        if !relocs.is_empty() {
            self.relocs.push(SymbolRelocs {
                section,
                offset,
                relocs,
            });
        }
        Ok(())
    }
}

impl ObjectModule {
    /// Finalize all relocations and output an object.
    pub fn finish(mut self) -> ObjectProduct {
        let symbol_relocs = mem::take(&mut self.relocs);
        for symbol in symbol_relocs {
            for &ObjectRelocRecord {
                offset,
                ref name,
                kind,
                encoding,
                size,
                addend,
            } in &symbol.relocs
            {
                let target_symbol = self.get_symbol(name);
                self.object
                    .add_relocation(
                        symbol.section,
                        Relocation {
                            offset: symbol.offset + u64::from(offset),
                            size,
                            kind,
                            encoding,
                            symbol: target_symbol,
                            addend,
                        },
                    )
                    .unwrap();
            }
        }

        // Indicate that this object has a non-executable stack.
        if self.object.format() == object::BinaryFormat::Elf {
            self.object.add_section(
                vec![],
                ".note.GNU-stack".as_bytes().to_vec(),
                SectionKind::Linker,
            );
        }

        ObjectProduct {
            object: self.object,
            functions: self.functions,
            data_objects: self.data_objects,
        }
    }

    /// This should only be called during finish because it creates
    /// symbols for missing libcalls.
    fn get_symbol(&mut self, name: &ir::ExternalName) -> SymbolId {
        match *name {
            ir::ExternalName::User { .. } => {
                if ModuleDeclarations::is_function(name) {
                    let id = FuncId::from_name(name);
                    self.functions[id].unwrap().0
                } else {
                    let id = DataId::from_name(name);
                    self.data_objects[id].unwrap().0
                }
            }
            ir::ExternalName::LibCall(ref libcall) => {
                let name = (self.libcall_names)(*libcall);
                if let Some(symbol) = self.object.symbol_id(name.as_bytes()) {
                    symbol
                } else if let Some(symbol) = self.libcalls.get(libcall) {
                    *symbol
                } else {
                    let symbol = self.object.add_symbol(Symbol {
                        name: name.as_bytes().to_vec(),
                        value: 0,
                        size: 0,
                        kind: SymbolKind::Text,
                        scope: SymbolScope::Unknown,
                        weak: false,
                        section: SymbolSection::Undefined,
                        flags: SymbolFlags::None,
                    });
                    self.libcalls.insert(*libcall, symbol);
                    symbol
                }
            }
            _ => panic!("invalid ExternalName {}", name),
        }
    }

    fn process_reloc(&self, record: &RelocRecord) -> ObjectRelocRecord {
        let mut addend = record.addend;
        let (kind, encoding, size) = match record.reloc {
            Reloc::Abs4 => (RelocationKind::Absolute, RelocationEncoding::Generic, 32),
            Reloc::Abs8 => (RelocationKind::Absolute, RelocationEncoding::Generic, 64),
            Reloc::X86PCRel4 => (RelocationKind::Relative, RelocationEncoding::Generic, 32),
            Reloc::X86CallPCRel4 => (RelocationKind::Relative, RelocationEncoding::X86Branch, 32),
            // TODO: Get Cranelift to tell us when we can use
            // R_X86_64_GOTPCRELX/R_X86_64_REX_GOTPCRELX.
            Reloc::X86CallPLTRel4 => (
                RelocationKind::PltRelative,
                RelocationEncoding::X86Branch,
                32,
            ),
            Reloc::X86GOTPCRel4 => (RelocationKind::GotRelative, RelocationEncoding::Generic, 32),
            Reloc::ElfX86_64TlsGd => {
                assert_eq!(
                    self.object.format(),
                    object::BinaryFormat::Elf,
                    "ElfX86_64TlsGd is not supported for this file format"
                );
                (
                    RelocationKind::Elf(object::elf::R_X86_64_TLSGD),
                    RelocationEncoding::Generic,
                    32,
                )
            }
            Reloc::MachOX86_64Tlv => {
                assert_eq!(
                    self.object.format(),
                    object::BinaryFormat::MachO,
                    "MachOX86_64Tlv is not supported for this file format"
                );
                addend += 4; // X86_64_RELOC_TLV has an implicit addend of -4
                (
                    RelocationKind::MachO {
                        value: object::macho::X86_64_RELOC_TLV,
                        relative: true,
                    },
                    RelocationEncoding::Generic,
                    32,
                )
            }
            // FIXME
            _ => unimplemented!(),
        };
        ObjectRelocRecord {
            offset: record.offset,
            name: record.name.clone(),
            kind,
            encoding,
            size,
            addend,
        }
    }
}

fn translate_linkage(linkage: Linkage) -> (SymbolScope, bool) {
    let scope = match linkage {
        Linkage::Import => SymbolScope::Unknown,
        Linkage::Local => SymbolScope::Compilation,
        Linkage::Hidden => SymbolScope::Linkage,
        Linkage::Export | Linkage::Preemptible => SymbolScope::Dynamic,
    };
    // TODO: this matches rustc_codegen_cranelift, but may be wrong.
    let weak = linkage == Linkage::Preemptible;
    (scope, weak)
}

/// This is the output of `Module`'s
/// [`finish`](../cranelift_module/struct.Module.html#method.finish) function.
/// It contains the generated `Object` and other information produced during
/// compilation.
pub struct ObjectProduct {
    /// Object artifact with all functions and data from the module defined.
    pub object: Object,
    /// Symbol IDs for functions (both declared and defined).
    pub functions: SecondaryMap<FuncId, Option<(SymbolId, bool)>>,
    /// Symbol IDs for data objects (both declared and defined).
    pub data_objects: SecondaryMap<DataId, Option<(SymbolId, bool)>>,
}

impl ObjectProduct {
    /// Return the `SymbolId` for the given function.
    #[inline]
    pub fn function_symbol(&self, id: FuncId) -> SymbolId {
        self.functions[id].unwrap().0
    }

    /// Return the `SymbolId` for the given data object.
    #[inline]
    pub fn data_symbol(&self, id: DataId) -> SymbolId {
        self.data_objects[id].unwrap().0
    }

    /// Write the object bytes in memory.
    #[inline]
    pub fn emit(self) -> Result<Vec<u8>, object::write::Error> {
        self.object.write()
    }
}

#[derive(Clone)]
struct SymbolRelocs {
    section: SectionId,
    offset: u64,
    relocs: Vec<ObjectRelocRecord>,
}

#[derive(Clone)]
struct ObjectRelocRecord {
    offset: CodeOffset,
    name: ir::ExternalName,
    kind: RelocationKind,
    encoding: RelocationEncoding,
    size: u8,
    addend: Addend,
}

#[derive(Default)]
struct ObjectRelocSink {
    relocs: Vec<RelocRecord>,
}

impl RelocSink for ObjectRelocSink {
    fn reloc_block(&mut self, _offset: CodeOffset, _reloc: Reloc, _block_offset: CodeOffset) {
        unimplemented!();
    }

    fn reloc_external(
        &mut self,
        offset: CodeOffset,
        _srcloc: ir::SourceLoc,
        reloc: Reloc,
        name: &ir::ExternalName,
        addend: Addend,
    ) {
        self.relocs.push(RelocRecord {
            offset,
            reloc,
            addend,
            name: name.clone(),
        })
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
