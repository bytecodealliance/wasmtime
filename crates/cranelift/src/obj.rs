//! Object file builder.
//!
//! Creates ELF image based on `Compilation` information. The ELF contains
//! functions and trampolines in the ".text" section. It also contains all
//! relocation records for the linking stage. If DWARF sections exist, their
//! content will be written as well.
//!
//! The object file has symbols for each function and trampoline, as well as
//! symbols that refer to libcalls.
//!
//! The function symbol names have format "_wasm_function_N", where N is
//! `FuncIndex`. The defined wasm function symbols refer to a JIT compiled
//! function body, the imported wasm function do not. The trampolines symbol
//! names have format "_trampoline_N", where N is `SignatureIndex`.

use crate::debug::{DwarfSection, DwarfSectionRelocTarget};
use crate::{CompiledFunction, Relocation, RelocationTarget};
use anyhow::Result;
use cranelift_codegen::binemit::Reloc;
use cranelift_codegen::ir::{JumpTableOffsets, LibCall};
use cranelift_codegen::isa::{
    unwind::{systemv, UnwindInfo},
    TargetIsa,
};
use gimli::write::{Address, EhFrame, EndianVec, FrameTable, Writer};
use gimli::RunTimeEndian;
use object::write::{
    Object, Relocation as ObjectRelocation, SectionId, StandardSegment, Symbol, SymbolId,
    SymbolSection,
};
use object::{
    elf, Architecture, BinaryFormat, Endianness, RelocationEncoding, RelocationKind, SectionKind,
    SymbolFlags, SymbolKind, SymbolScope,
};
use std::collections::HashMap;
use std::convert::TryFrom;
use wasmtime_environ::obj;
use wasmtime_environ::{
    DefinedFuncIndex, EntityRef, FuncIndex, Module, PrimaryMap, SignatureIndex,
};

fn to_object_architecture(
    arch: target_lexicon::Architecture,
) -> Result<Architecture, anyhow::Error> {
    use target_lexicon::Architecture::*;
    Ok(match arch {
        X86_32(_) => Architecture::I386,
        X86_64 => Architecture::X86_64,
        Arm(_) => Architecture::Arm,
        Aarch64(_) => Architecture::Aarch64,
        S390x => Architecture::S390x,
        architecture => {
            anyhow::bail!("target architecture {:?} is unsupported", architecture,);
        }
    })
}

const TEXT_SECTION_NAME: &[u8] = b".text";

/// Iterates through all `LibCall` members and all runtime exported functions.
#[macro_export]
macro_rules! for_each_libcall {
    ($op:ident) => {
        $op![
            (UdivI64, wasmtime_i64_udiv),
            (UdivI64, wasmtime_i64_udiv),
            (SdivI64, wasmtime_i64_sdiv),
            (UremI64, wasmtime_i64_urem),
            (SremI64, wasmtime_i64_srem),
            (IshlI64, wasmtime_i64_ishl),
            (UshrI64, wasmtime_i64_ushr),
            (SshrI64, wasmtime_i64_sshr),
            (CeilF32, wasmtime_f32_ceil),
            (FloorF32, wasmtime_f32_floor),
            (TruncF32, wasmtime_f32_trunc),
            (NearestF32, wasmtime_f32_nearest),
            (CeilF64, wasmtime_f64_ceil),
            (FloorF64, wasmtime_f64_floor),
            (TruncF64, wasmtime_f64_trunc),
            (NearestF64, wasmtime_f64_nearest)
        ];
    };
}

fn write_libcall_symbols(obj: &mut Object) -> HashMap<LibCall, SymbolId> {
    let mut libcalls = HashMap::new();
    macro_rules! add_libcall_symbol {
        [$(($libcall:ident, $export:ident)),*] => {{
            $(
                let symbol_id = obj.add_symbol(Symbol {
                    name: stringify!($export).as_bytes().to_vec(),
                    value: 0,
                    size: 0,
                    kind: SymbolKind::Text,
                    scope: SymbolScope::Linkage,
                    weak: true,
                    section: SymbolSection::Undefined,
                    flags: SymbolFlags::None,
                });
                libcalls.insert(LibCall::$libcall, symbol_id);
            )+
        }};
    }
    for_each_libcall!(add_libcall_symbol);

    libcalls
}

pub struct ObjectBuilderTarget {
    pub(crate) binary_format: BinaryFormat,
    pub(crate) architecture: Architecture,
    pub(crate) endianness: Endianness,
}

impl ObjectBuilderTarget {
    pub fn elf(arch: target_lexicon::Architecture) -> Result<Self> {
        Ok(Self {
            binary_format: BinaryFormat::Elf,
            architecture: to_object_architecture(arch)?,
            endianness: match arch.endianness().unwrap() {
                target_lexicon::Endianness::Little => object::Endianness::Little,
                target_lexicon::Endianness::Big => object::Endianness::Big,
            },
        })
    }
}

pub struct ObjectBuilder<'a> {
    obj: Object,
    module: &'a Module,
    text_section: SectionId,
    func_symbols: PrimaryMap<FuncIndex, SymbolId>,
    jump_tables: PrimaryMap<DefinedFuncIndex, &'a JumpTableOffsets>,
    libcalls: HashMap<LibCall, SymbolId>,
    pending_relocations: Vec<(u64, &'a [Relocation])>,
    windows_unwind_info: Vec<RUNTIME_FUNCTION>,
    systemv_unwind_info: Vec<(u64, &'a systemv::UnwindInfo)>,
}

// This is a mirror of `RUNTIME_FUNCTION` in the Windows API, but defined here
// to ensure everything is always `u32` and to have it available on all
// platforms. Note that all of these specifiers here are relative to a "base
// address" which we define as the base of where the text section is eventually
// loaded.
#[allow(non_camel_case_types)]
struct RUNTIME_FUNCTION {
    begin: u32,
    end: u32,
    unwind_address: u32,
}

impl<'a> ObjectBuilder<'a> {
    pub fn new(target: ObjectBuilderTarget, module: &'a Module) -> Self {
        let mut obj = Object::new(target.binary_format, target.architecture, target.endianness);

        // Entire code (functions and trampolines) will be placed
        // in the ".text" section.
        let text_section = obj.add_section(
            obj.segment_name(StandardSegment::Text).to_vec(),
            TEXT_SECTION_NAME.to_vec(),
            SectionKind::Text,
        );

        // Create symbols for imports -- needed during linking.
        let mut func_symbols = PrimaryMap::with_capacity(module.functions.len());
        for index in 0..module.num_imported_funcs {
            let symbol_id = obj.add_symbol(Symbol {
                name: obj::func_symbol_name(FuncIndex::new(index))
                    .as_bytes()
                    .to_vec(),
                value: 0,
                size: 0,
                kind: SymbolKind::Text,
                scope: SymbolScope::Linkage,
                weak: false,
                section: SymbolSection::Undefined,
                flags: SymbolFlags::None,
            });
            func_symbols.push(symbol_id);
        }

        let libcalls = write_libcall_symbols(&mut obj);

        Self {
            obj,
            module,
            text_section,
            func_symbols,
            libcalls,
            pending_relocations: Vec::new(),
            jump_tables: PrimaryMap::with_capacity(module.functions.len()),
            windows_unwind_info: Vec::new(),
            systemv_unwind_info: Vec::new(),
        }
    }

    fn append_func(&mut self, name: Vec<u8>, func: &'a CompiledFunction) -> SymbolId {
        let off = self
            .obj
            .append_section_data(self.text_section, &func.body, 1);
        let symbol_id = self.obj.add_symbol(Symbol {
            name,
            value: off,
            size: func.body.len() as u64,
            kind: SymbolKind::Text,
            scope: SymbolScope::Compilation,
            weak: false,
            section: SymbolSection::Section(self.text_section),
            flags: SymbolFlags::None,
        });

        match &func.unwind_info {
            // Windows unwind information is preferred to come after the code
            // itself. The information is appended here just after the function,
            // aligned to 4-bytes as required by Windows.
            //
            // The location of the unwind info, and the function it describes,
            // is then recorded in an unwind info table to get embedded into the
            // object at the end of compilation.
            Some(UnwindInfo::WindowsX64(info)) => {
                // Windows prefers Unwind info after the code -- writing it here.
                let unwind_size = info.emit_size();
                let mut unwind_info = vec![0; unwind_size];
                info.emit(&mut unwind_info);
                let unwind_off = self
                    .obj
                    .append_section_data(self.text_section, &unwind_info, 4);
                self.windows_unwind_info.push(RUNTIME_FUNCTION {
                    begin: u32::try_from(off).unwrap(),
                    end: u32::try_from(off + func.body.len() as u64).unwrap(),
                    unwind_address: u32::try_from(unwind_off).unwrap(),
                });
            }

            // System-V is different enough that we just record the unwinding
            // information to get processed at a later time.
            Some(UnwindInfo::SystemV(info)) => {
                self.systemv_unwind_info.push((off, info));
            }

            Some(_) => panic!("some unwind info isn't handled here"),
            None => {}
        }
        if !func.relocations.is_empty() {
            self.pending_relocations.push((off, &func.relocations));
        }
        symbol_id
    }

    pub fn func(&mut self, index: DefinedFuncIndex, func: &'a CompiledFunction) {
        assert_eq!(self.jump_tables.push(&func.jt_offsets), index);
        let index = self.module.func_index(index);
        let name = obj::func_symbol_name(index);
        let symbol_id = self.append_func(name.into_bytes(), func);
        assert_eq!(self.func_symbols.push(symbol_id), index);
    }

    pub fn trampoline(&mut self, sig: SignatureIndex, func: &'a CompiledFunction) {
        let name = obj::trampoline_symbol_name(sig);
        self.append_func(name.into_bytes(), func);
    }

    pub fn align_text_to(&mut self, align: u64) {
        self.obj.append_section_data(self.text_section, &[], align);
    }

    pub fn dwarf_sections(&mut self, sections: &[DwarfSection]) -> Result<()> {
        // If we have DWARF data, write it in the object file.
        let (debug_bodies, debug_relocs): (Vec<_>, Vec<_>) = sections
            .iter()
            .map(|s| ((s.name, &s.body), (s.name, &s.relocs)))
            .unzip();
        let mut dwarf_sections_ids = HashMap::new();
        for (name, body) in debug_bodies {
            let segment = self.obj.segment_name(StandardSegment::Debug).to_vec();
            let section_id =
                self.obj
                    .add_section(segment, name.as_bytes().to_vec(), SectionKind::Debug);
            dwarf_sections_ids.insert(name, section_id);
            self.obj.append_section_data(section_id, &body, 1);
        }

        // Write all debug data relocations.
        for (name, relocs) in debug_relocs {
            let section_id = *dwarf_sections_ids.get(name).unwrap();
            for reloc in relocs {
                let target_symbol = match reloc.target {
                    DwarfSectionRelocTarget::Func(index) => {
                        self.func_symbols[self.module.func_index(DefinedFuncIndex::new(index))]
                    }
                    DwarfSectionRelocTarget::Section(name) => {
                        self.obj.section_symbol(dwarf_sections_ids[name])
                    }
                };
                self.obj.add_relocation(
                    section_id,
                    ObjectRelocation {
                        offset: u64::from(reloc.offset),
                        size: reloc.size << 3,
                        kind: RelocationKind::Absolute,
                        encoding: RelocationEncoding::Generic,
                        symbol: target_symbol,
                        addend: i64::from(reloc.addend),
                    },
                )?;
            }
        }

        Ok(())
    }

    pub fn finish(&mut self, isa: &dyn TargetIsa) -> Result<Vec<u8>> {
        self.append_relocations()?;
        if self.windows_unwind_info.len() > 0 {
            self.append_windows_unwind_info();
        }
        if self.systemv_unwind_info.len() > 0 {
            self.append_systemv_unwind_info(isa);
        }
        Ok(self.obj.write()?)
    }

    fn append_relocations(&mut self) -> Result<()> {
        for (off, relocations) in self.pending_relocations.iter() {
            for r in relocations.iter() {
                let (symbol, symbol_offset) = match r.reloc_target {
                    RelocationTarget::UserFunc(index) => (self.func_symbols[index], 0),
                    RelocationTarget::LibCall(call) => (self.libcalls[&call], 0),
                    RelocationTarget::JumpTable(f, jt) => {
                        let df = self.module.defined_func_index(f).unwrap();
                        let offset = *self
                            .jump_tables
                            .get(df)
                            .and_then(|t| t.get(jt))
                            .expect("func jump table");
                        (self.func_symbols[f], offset)
                    }
                };
                let (kind, encoding, size) = match r.reloc {
                    Reloc::Abs4 => (RelocationKind::Absolute, RelocationEncoding::Generic, 32),
                    Reloc::Abs8 => (RelocationKind::Absolute, RelocationEncoding::Generic, 64),
                    Reloc::X86PCRel4 => (RelocationKind::Relative, RelocationEncoding::Generic, 32),
                    Reloc::X86CallPCRel4 => {
                        (RelocationKind::Relative, RelocationEncoding::X86Branch, 32)
                    }
                    // TODO: Get Cranelift to tell us when we can use
                    // R_X86_64_GOTPCRELX/R_X86_64_REX_GOTPCRELX.
                    Reloc::X86CallPLTRel4 => (
                        RelocationKind::PltRelative,
                        RelocationEncoding::X86Branch,
                        32,
                    ),
                    Reloc::X86GOTPCRel4 => {
                        (RelocationKind::GotRelative, RelocationEncoding::Generic, 32)
                    }
                    Reloc::ElfX86_64TlsGd => (
                        RelocationKind::Elf(elf::R_X86_64_TLSGD),
                        RelocationEncoding::Generic,
                        32,
                    ),
                    Reloc::X86PCRelRodata4 => {
                        continue;
                    }
                    Reloc::Arm64Call => (
                        RelocationKind::Elf(elf::R_AARCH64_CALL26),
                        RelocationEncoding::Generic,
                        32,
                    ),
                    Reloc::S390xPCRel32Dbl => {
                        (RelocationKind::Relative, RelocationEncoding::S390xDbl, 32)
                    }
                    other => unimplemented!("Unimplemented relocation {:?}", other),
                };
                self.obj.add_relocation(
                    self.text_section,
                    ObjectRelocation {
                        offset: off + r.offset as u64,
                        size,
                        kind,
                        encoding,
                        symbol,
                        addend: r.addend.wrapping_add(symbol_offset as i64),
                    },
                )?;
            }
        }
        Ok(())
    }

    /// This function appends a nonstandard section to the object which is only
    /// used during `CodeMemory::allocate_for_object`.
    ///
    /// This custom section effectively stores a `[RUNTIME_FUNCTION; N]` into
    /// the object file itself. This way registration of unwind info can simply
    /// pass this slice to the OS itself and there's no need to recalculate
    /// anything on the other end of loading a module from a precompiled object.
    fn append_windows_unwind_info(&mut self) {
        // Currently the binary format supported here only supports
        // little-endian for x86_64, or at least that's all where it's tested.
        // This may need updates for other platforms.
        assert_eq!(self.obj.architecture(), Architecture::X86_64);

        // Page-align the text section so the unwind info can reside on a
        // separate page that doesn't need executable permissions.
        self.obj.append_section_data(self.text_section, &[], 0x1000);

        let segment = self.obj.segment_name(StandardSegment::Data).to_vec();
        let section_id = self.obj.add_section(
            segment,
            b"_wasmtime_winx64_unwind".to_vec(),
            SectionKind::ReadOnlyData,
        );
        let mut unwind_info = Vec::with_capacity(self.windows_unwind_info.len() * 3 * 4);
        for info in self.windows_unwind_info.iter() {
            unwind_info.extend_from_slice(&info.begin.to_le_bytes());
            unwind_info.extend_from_slice(&info.end.to_le_bytes());
            unwind_info.extend_from_slice(&info.unwind_address.to_le_bytes());
        }
        self.obj.append_section_data(section_id, &unwind_info, 1);
    }

    /// This function appends a nonstandard section to the object which is only
    /// used during `CodeMemory::allocate_for_object`.
    ///
    /// This will generate a `.eh_frame` section, but not one that can be
    /// naively loaded. The goal of this section is that we can create the
    /// section once here and never again does it need to change. To describe
    /// dynamically loaded functions though each individual FDE needs to talk
    /// about the function's absolute address that it's referencing. Naturally
    /// we don't actually know the function's absolute address when we're
    /// creating an object here.
    ///
    /// To solve this problem the FDE address encoding mode is set to
    /// `DW_EH_PE_pcrel`. This means that the actual effective address that the
    /// FDE describes is a relative to the address of the FDE itself. By
    /// leveraging this relative-ness we can assume that the relative distance
    /// between the FDE and the function it describes is constant, which should
    /// allow us to generate an FDE ahead-of-time here.
    ///
    /// For now this assumes that all the code of functions will start at a
    /// page-aligned address when loaded into memory. The eh_frame encoded here
    /// then assumes that the text section is itself page aligned to its size
    /// and the eh_frame will follow just after the text section. This means
    /// that the relative offsets we're using here is the FDE going backwards
    /// into the text section itself.
    ///
    /// Note that the library we're using to create the FDEs, `gimli`, doesn't
    /// actually encode addresses relative to the FDE itself. Instead the
    /// addresses are encoded relative to the start of the `.eh_frame` section.
    /// This makes it much easier for us where we provide the relative offset
    /// from the start of `.eh_frame` to the function in the text section, which
    /// given our layout basically means the offset of the function in the text
    /// section from the end of the text section.
    ///
    /// A final note is that the reason we page-align the text section's size is
    /// so the .eh_frame lives on a separate page from the text section itself.
    /// This allows `.eh_frame` to have different virtual memory permissions,
    /// such as being purely read-only instead of read/execute like the code
    /// bits.
    fn append_systemv_unwind_info(&mut self, isa: &dyn TargetIsa) {
        let segment = self.obj.segment_name(StandardSegment::Data).to_vec();
        let section_id = self.obj.add_section(
            segment,
            b"_wasmtime_eh_frame".to_vec(),
            SectionKind::ReadOnlyData,
        );
        let mut cie = isa
            .create_systemv_cie()
            .expect("must be able to create a CIE for system-v unwind info");
        let mut table = FrameTable::default();
        cie.fde_address_encoding = gimli::constants::DW_EH_PE_pcrel;
        let cie_id = table.add_cie(cie);

        // This write will align the text section to a page boundary (0x1000)
        // and then return the offset at that point. This gives us the full size
        // of the text section at that point, after alignment.
        let text_section_size = self.obj.append_section_data(self.text_section, &[], 0x1000);
        for (text_section_off, unwind_info) in self.systemv_unwind_info.iter() {
            let backwards_off = text_section_size - text_section_off;
            let actual_offset = -i64::try_from(backwards_off).unwrap();
            // Note that gimli wants an unsigned 64-bit integer here, but
            // unwinders just use this constant for a relative addition with the
            // address of the FDE, which means that the sign doesn't actually
            // matter.
            let fde = unwind_info.to_fde(Address::Constant(actual_offset as u64));
            table.add_fde(cie_id, fde);
        }
        let endian = match isa.triple().endianness().unwrap() {
            target_lexicon::Endianness::Little => RunTimeEndian::Little,
            target_lexicon::Endianness::Big => RunTimeEndian::Big,
        };
        let mut eh_frame = EhFrame(MyVec(EndianVec::new(endian)));
        table.write_eh_frame(&mut eh_frame).unwrap();

        // Some unwinding implementations expect a terminating "empty" length so
        // a 0 is written at the end of the table for those implementations.
        let mut endian_vec = (eh_frame.0).0;
        endian_vec.write_u32(0).unwrap();
        self.obj
            .append_section_data(section_id, endian_vec.slice(), 1);

        use gimli::constants;
        use gimli::write::Error;

        struct MyVec(EndianVec<RunTimeEndian>);

        impl Writer for MyVec {
            type Endian = RunTimeEndian;

            fn endian(&self) -> RunTimeEndian {
                self.0.endian()
            }

            fn len(&self) -> usize {
                self.0.len()
            }

            fn write(&mut self, buf: &[u8]) -> Result<(), Error> {
                self.0.write(buf)
            }

            fn write_at(&mut self, pos: usize, buf: &[u8]) -> Result<(), Error> {
                self.0.write_at(pos, buf)
            }

            // FIXME(gimli-rs/gimli#576) this is the definition we want for
            // `write_eh_pointer` but the default implementation, at the time
            // of this writing, uses `offset - val` instead of `val - offset`.
            // A PR has been merged to fix this but until that's published we
            // can't use it.
            fn write_eh_pointer(
                &mut self,
                address: Address,
                eh_pe: constants::DwEhPe,
                size: u8,
            ) -> Result<(), Error> {
                let val = match address {
                    Address::Constant(val) => val,
                    Address::Symbol { .. } => unreachable!(),
                };
                assert_eq!(eh_pe.application(), constants::DW_EH_PE_pcrel);
                let offset = self.len() as u64;
                let val = val.wrapping_sub(offset);
                self.write_eh_pointer_data(val, eh_pe.format(), size)
            }
        }
    }
}
