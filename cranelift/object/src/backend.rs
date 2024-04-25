//! Defines `ObjectModule`.

use anyhow::anyhow;
use cranelift_codegen::binemit::{Addend, CodeOffset, Reloc};
use cranelift_codegen::entity::SecondaryMap;
use cranelift_codegen::isa::{OwnedTargetIsa, TargetIsa};
use cranelift_codegen::{ir, FinalizedMachReloc};
use cranelift_control::ControlPlane;
use cranelift_module::{
    DataDescription, DataId, FuncId, Init, Linkage, Module, ModuleDeclarations, ModuleError,
    ModuleReloc, ModuleRelocTarget, ModuleResult,
};
use log::info;
use object::write::{
    Object, Relocation, SectionId, StandardSection, Symbol, SymbolId, SymbolSection,
};
use object::{
    RelocationEncoding, RelocationFlags, RelocationKind, SectionKind, SymbolFlags, SymbolKind,
    SymbolScope,
};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::mem;
use target_lexicon::PointerWidth;

/// A builder for `ObjectModule`.
pub struct ObjectBuilder {
    isa: OwnedTargetIsa,
    binary_format: object::BinaryFormat,
    architecture: object::Architecture,
    flags: object::FileFlags,
    endian: object::Endianness,
    name: Vec<u8>,
    libcall_names: Box<dyn Fn(ir::LibCall) -> String + Send + Sync>,
    per_function_section: bool,
}

impl ObjectBuilder {
    /// Create a new `ObjectBuilder` using the given Cranelift target, that
    /// can be passed to [`ObjectModule::new`].
    ///
    /// The `libcall_names` function provides a way to translate `cranelift_codegen`'s [`ir::LibCall`]
    /// enum to symbols. LibCalls are inserted in the IR as part of the legalization for certain
    /// floating point instructions, and for stack probes. If you don't know what to use for this
    /// argument, use [`cranelift_module::default_libcall_names`].
    pub fn new<V: Into<Vec<u8>>>(
        isa: OwnedTargetIsa,
        name: V,
        libcall_names: Box<dyn Fn(ir::LibCall) -> String + Send + Sync>,
    ) -> ModuleResult<Self> {
        let mut file_flags = object::FileFlags::None;
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
            target_lexicon::Architecture::Riscv64(_) => {
                if binary_format != object::BinaryFormat::Elf {
                    return Err(ModuleError::Backend(anyhow!(
                        "binary format {:?} is not supported for riscv64",
                        binary_format,
                    )));
                }

                // FIXME(#4994): Get the right float ABI variant from the TargetIsa
                let mut eflags = object::elf::EF_RISCV_FLOAT_ABI_DOUBLE;

                // Set the RVC eflag if we have the C extension enabled.
                let has_c = isa
                    .isa_flags()
                    .iter()
                    .filter(|f| f.name == "has_zca" || f.name == "has_zcd")
                    .all(|f| f.as_bool().unwrap_or_default());
                if has_c {
                    eflags |= object::elf::EF_RISCV_RVC;
                }

                file_flags = object::FileFlags::Elf {
                    os_abi: object::elf::ELFOSABI_NONE,
                    abi_version: 0,
                    e_flags: eflags,
                };
                object::Architecture::Riscv64
            }
            target_lexicon::Architecture::S390x => object::Architecture::S390x,
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
            flags: file_flags,
            endian,
            name: name.into(),
            libcall_names,
            per_function_section: false,
        })
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
    isa: OwnedTargetIsa,
    object: Object<'static>,
    declarations: ModuleDeclarations,
    functions: SecondaryMap<FuncId, Option<(SymbolId, bool)>>,
    data_objects: SecondaryMap<DataId, Option<(SymbolId, bool)>>,
    relocs: Vec<SymbolRelocs>,
    libcalls: HashMap<ir::LibCall, SymbolId>,
    libcall_names: Box<dyn Fn(ir::LibCall) -> String + Send + Sync>,
    known_symbols: HashMap<ir::KnownSymbol, SymbolId>,
    known_labels: HashMap<(FuncId, CodeOffset), SymbolId>,
    per_function_section: bool,
}

impl ObjectModule {
    /// Create a new `ObjectModule` using the given Cranelift target.
    pub fn new(builder: ObjectBuilder) -> Self {
        let mut object = Object::new(builder.binary_format, builder.architecture, builder.endian);
        object.flags = builder.flags;
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
            known_symbols: HashMap::new(),
            known_labels: HashMap::new(),
            per_function_section: builder.per_function_section,
        }
    }
}

fn validate_symbol(name: &str) -> ModuleResult<()> {
    // null bytes are not allowed in symbol names and will cause the `object`
    // crate to panic. Let's return a clean error instead.
    if name.contains("\0") {
        return Err(ModuleError::Backend(anyhow::anyhow!(
            "Symbol {:?} has a null byte, which is disallowed",
            name
        )));
    }
    Ok(())
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
        validate_symbol(name)?;

        let (id, linkage) = self
            .declarations
            .declare_function(name, linkage, signature)?;

        let (scope, weak) = translate_linkage(linkage);

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

    fn declare_anonymous_function(&mut self, signature: &ir::Signature) -> ModuleResult<FuncId> {
        let id = self.declarations.declare_anonymous_function(signature)?;

        let symbol_id = self.object.add_symbol(Symbol {
            name: self
                .declarations
                .get_function_decl(id)
                .linkage_name(id)
                .into_owned()
                .into_bytes(),
            value: 0,
            size: 0,
            kind: SymbolKind::Text,
            scope: SymbolScope::Compilation,
            weak: false,
            section: SymbolSection::Undefined,
            flags: SymbolFlags::None,
        });
        self.functions[id] = Some((symbol_id, false));

        Ok(id)
    }

    fn declare_data(
        &mut self,
        name: &str,
        linkage: Linkage,
        writable: bool,
        tls: bool,
    ) -> ModuleResult<DataId> {
        validate_symbol(name)?;

        let (id, linkage) = self
            .declarations
            .declare_data(name, linkage, writable, tls)?;

        // Merging declarations with conflicting values for tls is not allowed, so it is safe to use
        // the passed in tls value here.
        let kind = if tls {
            SymbolKind::Tls
        } else {
            SymbolKind::Data
        };
        let (scope, weak) = translate_linkage(linkage);

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

    fn declare_anonymous_data(&mut self, writable: bool, tls: bool) -> ModuleResult<DataId> {
        let id = self.declarations.declare_anonymous_data(writable, tls)?;

        let kind = if tls {
            SymbolKind::Tls
        } else {
            SymbolKind::Data
        };

        let symbol_id = self.object.add_symbol(Symbol {
            name: self
                .declarations
                .get_data_decl(id)
                .linkage_name(id)
                .into_owned()
                .into_bytes(),
            value: 0,
            size: 0,
            kind,
            scope: SymbolScope::Compilation,
            weak: false,
            section: SymbolSection::Undefined,
            flags: SymbolFlags::None,
        });
        self.data_objects[id] = Some((symbol_id, false));

        Ok(id)
    }

    fn define_function_with_control_plane(
        &mut self,
        func_id: FuncId,
        ctx: &mut cranelift_codegen::Context,
        ctrl_plane: &mut ControlPlane,
    ) -> ModuleResult<()> {
        info!("defining function {}: {}", func_id, ctx.func.display());
        let mut code: Vec<u8> = Vec::new();

        let res = ctx.compile_and_emit(self.isa(), &mut code, ctrl_plane)?;
        let alignment = res.buffer.alignment as u64;

        self.define_function_bytes(
            func_id,
            &ctx.func,
            alignment,
            &code,
            ctx.compiled_code().unwrap().buffer.relocs(),
        )
    }

    fn define_function_bytes(
        &mut self,
        func_id: FuncId,
        func: &ir::Function,
        alignment: u64,
        bytes: &[u8],
        relocs: &[FinalizedMachReloc],
    ) -> ModuleResult<()> {
        info!("defining function {} with bytes", func_id);
        let decl = self.declarations.get_function_decl(func_id);
        let decl_name = decl.linkage_name(func_id);
        if !decl.linkage.is_definable() {
            return Err(ModuleError::InvalidImportDefinition(decl_name.into_owned()));
        }

        let &mut (symbol, ref mut defined) = self.functions[func_id].as_mut().unwrap();
        if *defined {
            return Err(ModuleError::DuplicateDefinition(decl_name.into_owned()));
        }
        *defined = true;

        let align = alignment
            .max(self.isa.function_alignment().minimum.into())
            .max(self.isa.symbol_alignment());
        let (section, offset) = if self.per_function_section {
            let symbol_name = self.object.symbol(symbol).name.clone();
            let (section, offset) =
                self.object
                    .add_subsection(StandardSection::Text, &symbol_name, bytes, align);
            self.object.symbol_mut(symbol).section = SymbolSection::Section(section);
            self.object.symbol_mut(symbol).value = offset;
            (section, offset)
        } else {
            let section = self.object.section_id(StandardSection::Text);
            let offset = self.object.add_symbol_data(symbol, section, bytes, align);
            (section, offset)
        };

        if !relocs.is_empty() {
            let relocs = relocs
                .iter()
                .map(|record| {
                    self.process_reloc(&ModuleReloc::from_mach_reloc(&record, func, func_id))
                })
                .collect();
            self.relocs.push(SymbolRelocs {
                section,
                offset,
                relocs,
            });
        }

        Ok(())
    }

    fn define_data(&mut self, data_id: DataId, data: &DataDescription) -> ModuleResult<()> {
        let decl = self.declarations.get_data_decl(data_id);
        if !decl.linkage.is_definable() {
            return Err(ModuleError::InvalidImportDefinition(
                decl.linkage_name(data_id).into_owned(),
            ));
        }

        let &mut (symbol, ref mut defined) = self.data_objects[data_id].as_mut().unwrap();
        if *defined {
            return Err(ModuleError::DuplicateDefinition(
                decl.linkage_name(data_id).into_owned(),
            ));
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
        } = data;

        let pointer_reloc = match self.isa.triple().pointer_width().unwrap() {
            PointerWidth::U16 => unimplemented!("16bit pointers"),
            PointerWidth::U32 => Reloc::Abs4,
            PointerWidth::U64 => Reloc::Abs8,
        };
        let relocs = data
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
                    SectionKind::ReadOnlyDataWithRel
                },
            )
        };

        let align = std::cmp::max(align.unwrap_or(1), self.isa.symbol_alignment());
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
                flags,
                addend,
            } in &symbol.relocs
            {
                let target_symbol = self.get_symbol(name);
                self.object
                    .add_relocation(
                        symbol.section,
                        Relocation {
                            offset: symbol.offset + u64::from(offset),
                            flags,
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
    fn get_symbol(&mut self, name: &ModuleRelocTarget) -> SymbolId {
        match *name {
            ModuleRelocTarget::User { .. } => {
                if ModuleDeclarations::is_function(name) {
                    let id = FuncId::from_name(name);
                    self.functions[id].unwrap().0
                } else {
                    let id = DataId::from_name(name);
                    self.data_objects[id].unwrap().0
                }
            }
            ModuleRelocTarget::LibCall(ref libcall) => {
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
            // These are "magic" names well-known to the linker.
            // They require special treatment.
            ModuleRelocTarget::KnownSymbol(ref known_symbol) => {
                if let Some(symbol) = self.known_symbols.get(known_symbol) {
                    *symbol
                } else {
                    let symbol = self.object.add_symbol(match known_symbol {
                        ir::KnownSymbol::ElfGlobalOffsetTable => Symbol {
                            name: b"_GLOBAL_OFFSET_TABLE_".to_vec(),
                            value: 0,
                            size: 0,
                            kind: SymbolKind::Data,
                            scope: SymbolScope::Unknown,
                            weak: false,
                            section: SymbolSection::Undefined,
                            flags: SymbolFlags::None,
                        },
                        ir::KnownSymbol::CoffTlsIndex => Symbol {
                            name: b"_tls_index".to_vec(),
                            value: 0,
                            size: 32,
                            kind: SymbolKind::Tls,
                            scope: SymbolScope::Unknown,
                            weak: false,
                            section: SymbolSection::Undefined,
                            flags: SymbolFlags::None,
                        },
                    });
                    self.known_symbols.insert(*known_symbol, symbol);
                    symbol
                }
            }

            ModuleRelocTarget::FunctionOffset(func_id, offset) => {
                match self.known_labels.entry((func_id, offset)) {
                    Entry::Occupied(o) => *o.get(),
                    Entry::Vacant(v) => {
                        let func_symbol_id = self.functions[func_id].unwrap().0;
                        let func_symbol = self.object.symbol(func_symbol_id);

                        let name = format!(".L{}_{}", func_id.as_u32(), offset);
                        let symbol_id = self.object.add_symbol(Symbol {
                            name: name.as_bytes().to_vec(),
                            value: func_symbol.value + offset as u64,
                            size: 0,
                            kind: SymbolKind::Label,
                            scope: SymbolScope::Compilation,
                            weak: false,
                            section: SymbolSection::Section(func_symbol.section.id().unwrap()),
                            flags: SymbolFlags::None,
                        });

                        v.insert(symbol_id);
                        symbol_id
                    }
                }
            }
        }
    }

    fn process_reloc(&self, record: &ModuleReloc) -> ObjectRelocRecord {
        let flags = match record.kind {
            Reloc::Abs4 => RelocationFlags::Generic {
                kind: RelocationKind::Absolute,
                encoding: RelocationEncoding::Generic,
                size: 32,
            },
            Reloc::Abs8 => RelocationFlags::Generic {
                kind: RelocationKind::Absolute,
                encoding: RelocationEncoding::Generic,
                size: 64,
            },
            Reloc::X86PCRel4 => RelocationFlags::Generic {
                kind: RelocationKind::Relative,
                encoding: RelocationEncoding::Generic,
                size: 32,
            },
            Reloc::X86CallPCRel4 => RelocationFlags::Generic {
                kind: RelocationKind::Relative,
                encoding: RelocationEncoding::X86Branch,
                size: 32,
            },
            // TODO: Get Cranelift to tell us when we can use
            // R_X86_64_GOTPCRELX/R_X86_64_REX_GOTPCRELX.
            Reloc::X86CallPLTRel4 => RelocationFlags::Generic {
                kind: RelocationKind::PltRelative,
                encoding: RelocationEncoding::X86Branch,
                size: 32,
            },
            Reloc::X86SecRel => RelocationFlags::Generic {
                kind: RelocationKind::SectionOffset,
                encoding: RelocationEncoding::Generic,
                size: 32,
            },
            Reloc::X86GOTPCRel4 => RelocationFlags::Generic {
                kind: RelocationKind::GotRelative,
                encoding: RelocationEncoding::Generic,
                size: 32,
            },
            Reloc::Arm64Call => RelocationFlags::Generic {
                kind: RelocationKind::Relative,
                encoding: RelocationEncoding::AArch64Call,
                size: 26,
            },
            Reloc::ElfX86_64TlsGd => {
                assert_eq!(
                    self.object.format(),
                    object::BinaryFormat::Elf,
                    "ElfX86_64TlsGd is not supported for this file format"
                );
                RelocationFlags::Elf {
                    r_type: object::elf::R_X86_64_TLSGD,
                }
            }
            Reloc::MachOX86_64Tlv => {
                assert_eq!(
                    self.object.format(),
                    object::BinaryFormat::MachO,
                    "MachOX86_64Tlv is not supported for this file format"
                );
                RelocationFlags::MachO {
                    r_type: object::macho::X86_64_RELOC_TLV,
                    r_pcrel: true,
                    r_length: 2,
                }
            }
            Reloc::MachOAarch64TlsAdrPage21 => {
                assert_eq!(
                    self.object.format(),
                    object::BinaryFormat::MachO,
                    "MachOAarch64TlsAdrPage21 is not supported for this file format"
                );
                RelocationFlags::MachO {
                    r_type: object::macho::ARM64_RELOC_TLVP_LOAD_PAGE21,
                    r_pcrel: true,
                    r_length: 2,
                }
            }
            Reloc::MachOAarch64TlsAdrPageOff12 => {
                assert_eq!(
                    self.object.format(),
                    object::BinaryFormat::MachO,
                    "MachOAarch64TlsAdrPageOff12 is not supported for this file format"
                );
                RelocationFlags::MachO {
                    r_type: object::macho::ARM64_RELOC_TLVP_LOAD_PAGEOFF12,
                    r_pcrel: false,
                    r_length: 2,
                }
            }
            Reloc::Aarch64TlsDescAdrPage21 => {
                assert_eq!(
                    self.object.format(),
                    object::BinaryFormat::Elf,
                    "Aarch64TlsDescAdrPage21 is not supported for this file format"
                );
                RelocationFlags::Elf {
                    r_type: object::elf::R_AARCH64_TLSDESC_ADR_PAGE21,
                }
            }
            Reloc::Aarch64TlsDescLd64Lo12 => {
                assert_eq!(
                    self.object.format(),
                    object::BinaryFormat::Elf,
                    "Aarch64TlsDescLd64Lo12 is not supported for this file format"
                );
                RelocationFlags::Elf {
                    r_type: object::elf::R_AARCH64_TLSDESC_LD64_LO12,
                }
            }
            Reloc::Aarch64TlsDescAddLo12 => {
                assert_eq!(
                    self.object.format(),
                    object::BinaryFormat::Elf,
                    "Aarch64TlsDescAddLo12 is not supported for this file format"
                );
                RelocationFlags::Elf {
                    r_type: object::elf::R_AARCH64_TLSDESC_ADD_LO12,
                }
            }
            Reloc::Aarch64TlsDescCall => {
                assert_eq!(
                    self.object.format(),
                    object::BinaryFormat::Elf,
                    "Aarch64TlsDescCall is not supported for this file format"
                );
                RelocationFlags::Elf {
                    r_type: object::elf::R_AARCH64_TLSDESC_CALL,
                }
            }

            Reloc::Aarch64AdrGotPage21 => match self.object.format() {
                object::BinaryFormat::Elf => RelocationFlags::Elf {
                    r_type: object::elf::R_AARCH64_ADR_GOT_PAGE,
                },
                object::BinaryFormat::MachO => RelocationFlags::MachO {
                    r_type: object::macho::ARM64_RELOC_GOT_LOAD_PAGE21,
                    r_pcrel: true,
                    r_length: 2,
                },
                _ => unimplemented!("Aarch64AdrGotPage21 is not supported for this file format"),
            },
            Reloc::Aarch64Ld64GotLo12Nc => match self.object.format() {
                object::BinaryFormat::Elf => RelocationFlags::Elf {
                    r_type: object::elf::R_AARCH64_LD64_GOT_LO12_NC,
                },
                object::BinaryFormat::MachO => RelocationFlags::MachO {
                    r_type: object::macho::ARM64_RELOC_GOT_LOAD_PAGEOFF12,
                    r_pcrel: false,
                    r_length: 2,
                },
                _ => unimplemented!("Aarch64Ld64GotLo12Nc is not supported for this file format"),
            },
            Reloc::S390xPCRel32Dbl => RelocationFlags::Generic {
                kind: RelocationKind::Relative,
                encoding: RelocationEncoding::S390xDbl,
                size: 32,
            },
            Reloc::S390xPLTRel32Dbl => RelocationFlags::Generic {
                kind: RelocationKind::PltRelative,
                encoding: RelocationEncoding::S390xDbl,
                size: 32,
            },
            Reloc::S390xTlsGd64 => {
                assert_eq!(
                    self.object.format(),
                    object::BinaryFormat::Elf,
                    "S390xTlsGd64 is not supported for this file format"
                );
                RelocationFlags::Elf {
                    r_type: object::elf::R_390_TLS_GD64,
                }
            }
            Reloc::S390xTlsGdCall => {
                assert_eq!(
                    self.object.format(),
                    object::BinaryFormat::Elf,
                    "S390xTlsGdCall is not supported for this file format"
                );
                RelocationFlags::Elf {
                    r_type: object::elf::R_390_TLS_GDCALL,
                }
            }
            Reloc::RiscvCallPlt => {
                assert_eq!(
                    self.object.format(),
                    object::BinaryFormat::Elf,
                    "RiscvCallPlt is not supported for this file format"
                );
                RelocationFlags::Elf {
                    r_type: object::elf::R_RISCV_CALL_PLT,
                }
            }
            Reloc::RiscvTlsGdHi20 => {
                assert_eq!(
                    self.object.format(),
                    object::BinaryFormat::Elf,
                    "RiscvTlsGdHi20 is not supported for this file format"
                );
                RelocationFlags::Elf {
                    r_type: object::elf::R_RISCV_TLS_GD_HI20,
                }
            }
            Reloc::RiscvPCRelLo12I => {
                assert_eq!(
                    self.object.format(),
                    object::BinaryFormat::Elf,
                    "RiscvPCRelLo12I is not supported for this file format"
                );
                RelocationFlags::Elf {
                    r_type: object::elf::R_RISCV_PCREL_LO12_I,
                }
            }
            Reloc::RiscvGotHi20 => {
                assert_eq!(
                    self.object.format(),
                    object::BinaryFormat::Elf,
                    "RiscvGotHi20 is not supported for this file format"
                );
                RelocationFlags::Elf {
                    r_type: object::elf::R_RISCV_GOT_HI20,
                }
            }
            // FIXME
            reloc => unimplemented!("{:?}", reloc),
        };

        ObjectRelocRecord {
            offset: record.offset,
            name: record.name.clone(),
            flags,
            addend: record.addend,
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

/// This is the output of `ObjectModule`'s
/// [`finish`](../struct.ObjectModule.html#method.finish) function.
/// It contains the generated `Object` and other information produced during
/// compilation.
pub struct ObjectProduct {
    /// Object artifact with all functions and data from the module defined.
    pub object: Object<'static>,
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
    name: ModuleRelocTarget,
    flags: RelocationFlags,
    addend: Addend,
}
