//! Object file generation.
//!
//! Creates ELF image based on `Compilation` information. The ELF contains
//! functions and trampolines in the ".text" section. It also contains all
//! relocation records for linking stage. If DWARF sections exist, their
//! content will be written as well.
//!
//! The object file has symbols for each function and trampoline, as well as
//! symbols that refer libcalls.
//!
//! The function symbol names have format "_wasm_function_N", where N is
//! `FuncIndex`. The defined wasm function symbols refer to a JIT compiled
//! function body, the imported wasm function do not. The trampolines symbol
//! names have format "_trampoline_N", where N is `SignatureIndex`.

use super::trampoline::build_trampoline;
use cranelift_codegen::binemit::Reloc;
use cranelift_codegen::ir::{JumpTableOffsets, LibCall};
use cranelift_frontend::FunctionBuilderContext;
use object::write::{
    Object, Relocation as ObjectRelocation, SectionId, StandardSegment, Symbol, SymbolId,
    SymbolSection,
};
use object::{
    elf, Architecture, BinaryFormat, Endianness, RelocationEncoding, RelocationKind, SectionKind,
    SymbolFlags, SymbolKind, SymbolScope,
};
use std::collections::HashMap;
use wasmtime_debug::{DwarfSection, DwarfSectionRelocTarget};
use wasmtime_environ::entity::{EntityRef, PrimaryMap};
use wasmtime_environ::isa::{unwind::UnwindInfo, TargetIsa};
use wasmtime_environ::wasm::{DefinedFuncIndex, FuncIndex, SignatureIndex};
use wasmtime_environ::{Compilation, Module, Relocation, RelocationTarget, Relocations};

fn to_object_relocations<'a>(
    it: impl Iterator<Item = &'a Relocation> + 'a,
    off: u64,
    module: &'a Module,
    funcs: &'a PrimaryMap<FuncIndex, SymbolId>,
    libcalls: &'a HashMap<LibCall, SymbolId>,
    jt_offsets: &'a PrimaryMap<DefinedFuncIndex, JumpTableOffsets>,
) -> impl Iterator<Item = ObjectRelocation> + 'a {
    it.filter_map(move |r| {
        let (symbol, symbol_offset) = match r.reloc_target {
            RelocationTarget::UserFunc(index) => (funcs[index], 0),
            RelocationTarget::LibCall(call) => (libcalls[&call], 0),
            RelocationTarget::JumpTable(f, jt) => {
                let df = module.local.defined_func_index(f).unwrap();
                let offset = *jt_offsets
                    .get(df)
                    .and_then(|ofs| ofs.get(jt))
                    .expect("func jump table");
                (funcs[f], offset)
            }
        };
        let (kind, encoding, size) = match r.reloc {
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
            Reloc::ElfX86_64TlsGd => (
                RelocationKind::Elf(elf::R_X86_64_TLSGD),
                RelocationEncoding::Generic,
                32,
            ),
            Reloc::X86PCRelRodata4 => {
                return None;
            }
            Reloc::Arm64Call => (
                RelocationKind::Elf(elf::R_AARCH64_CALL26),
                RelocationEncoding::Generic,
                32,
            ),
            other => unimplemented!("Unimplemented relocation {:?}", other),
        };
        Some(ObjectRelocation {
            offset: off + r.offset as u64,
            size,
            kind,
            encoding,
            symbol,
            addend: r.addend.wrapping_add(symbol_offset as i64),
        })
    })
}

fn to_object_architecture(
    arch: target_lexicon::Architecture,
) -> Result<Architecture, anyhow::Error> {
    use target_lexicon::Architecture::*;
    Ok(match arch {
        I386 | I586 | I686 => Architecture::I386,
        X86_64 => Architecture::X86_64,
        Arm(_) => Architecture::Arm,
        Aarch64(_) => Architecture::Aarch64,
        architecture => {
            anyhow::bail!("target architecture {:?} is unsupported", architecture,);
        }
    })
}

const TEXT_SECTION_NAME: &[u8] = b".text";

/// Unwind information for object files functions (including trampolines).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectUnwindInfo {
    Func(FuncIndex, UnwindInfo),
    Trampoline(SignatureIndex, UnwindInfo),
}

fn process_unwind_info(info: &UnwindInfo, obj: &mut Object, code_section: SectionId) {
    if let UnwindInfo::WindowsX64(info) = &info {
        // Windows prefers Unwind info after the code -- writing it here.
        let unwind_size = info.emit_size();
        let mut unwind_info = vec![0; unwind_size];
        info.emit(&mut unwind_info);
        let _off = obj.append_section_data(code_section, &unwind_info, 4);
    }
}

// Builds ELF image from the module `Compilation`.
pub(crate) fn build_object(
    isa: &dyn TargetIsa,
    compilation: &Compilation,
    relocations: &Relocations,
    module: &Module,
    dwarf_sections: &[DwarfSection],
) -> Result<(Object, Vec<ObjectUnwindInfo>), anyhow::Error> {
    const CODE_SECTION_ALIGNMENT: u64 = 0x1000;
    assert_eq!(
        isa.triple().architecture.endianness(),
        Ok(target_lexicon::Endianness::Little)
    );

    let mut obj = Object::new(
        BinaryFormat::Elf,
        to_object_architecture(isa.triple().architecture)?,
        Endianness::Little,
    );
    // Entire code (functions and trampolines) will be placed
    // in the ".text" section.
    let section_id = obj.add_section(
        obj.segment_name(StandardSegment::Text).to_vec(),
        TEXT_SECTION_NAME.to_vec(),
        SectionKind::Text,
    );

    let mut unwind_info = Vec::new();

    // Create symbols for imports -- needed during linking.
    let mut func_symbols = PrimaryMap::with_capacity(compilation.len());
    for index in 0..module.local.num_imported_funcs {
        let symbol_id = obj.add_symbol(Symbol {
            name: utils::func_symbol_name(FuncIndex::new(index))
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
    // Create symbols and section data for the compiled functions.
    for (index, func) in compilation.into_iter().enumerate() {
        let off = obj.append_section_data(section_id, &func.body, 1);
        let symbol_id = obj.add_symbol(Symbol {
            name: utils::func_symbol_name(module.local.func_index(DefinedFuncIndex::new(index)))
                .as_bytes()
                .to_vec(),
            value: off,
            size: func.body.len() as u64,
            kind: SymbolKind::Text,
            scope: SymbolScope::Compilation,
            weak: false,
            section: SymbolSection::Section(section_id),
            flags: SymbolFlags::None,
        });
        func_symbols.push(symbol_id);
        // Preserve function unwind info.
        if let Some(info) = &func.unwind_info {
            process_unwind_info(info, &mut obj, section_id);
            unwind_info.push(ObjectUnwindInfo::Func(
                FuncIndex::new(module.local.num_imported_funcs + index),
                info.clone(),
            ))
        }
    }

    let mut trampoline_relocs = HashMap::new();
    let mut cx = FunctionBuilderContext::new();
    // Build trampolines for every signature.
    for (i, (_, native_sig)) in module.local.signatures.iter() {
        let (func, relocs) =
            build_trampoline(isa, &mut cx, native_sig, std::mem::size_of::<u128>())?;
        let off = obj.append_section_data(section_id, &func.body, 1);
        let symbol_id = obj.add_symbol(Symbol {
            name: utils::trampoline_symbol_name(i).as_bytes().to_vec(),
            value: off,
            size: func.body.len() as u64,
            kind: SymbolKind::Text,
            scope: SymbolScope::Compilation,
            weak: false,
            section: SymbolSection::Section(section_id),
            flags: SymbolFlags::None,
        });
        trampoline_relocs.insert(symbol_id, relocs);
        // Preserve trampoline function unwind info.
        if let Some(info) = &func.unwind_info {
            process_unwind_info(info, &mut obj, section_id);
            unwind_info.push(ObjectUnwindInfo::Trampoline(i, info.clone()))
        }
    }

    obj.append_section_data(section_id, &[], CODE_SECTION_ALIGNMENT);

    // If we have DWARF data, write it in the object file.
    let (debug_bodies, debug_relocs) = dwarf_sections
        .into_iter()
        .map(|s| ((s.name, &s.body), (s.name, &s.relocs)))
        .unzip::<_, _, Vec<_>, Vec<_>>();
    let mut dwarf_sections_ids = HashMap::new();
    for (name, body) in debug_bodies {
        let segment = obj.segment_name(StandardSegment::Debug).to_vec();
        let section_id = obj.add_section(segment, name.as_bytes().to_vec(), SectionKind::Debug);
        dwarf_sections_ids.insert(name.to_string(), section_id);
        obj.append_section_data(section_id, &body, 1);
    }

    let libcalls = write_libcall_symbols(&mut obj);

    let jt_offsets = compilation.get_jt_offsets();

    // Write all functions relocations.
    for (index, relocs) in relocations.into_iter() {
        let func_index = module.local.func_index(index);
        let (_, off) = obj
            .symbol_section_and_offset(func_symbols[func_index])
            .unwrap();
        for r in to_object_relocations(
            relocs.iter(),
            off,
            module,
            &func_symbols,
            &libcalls,
            &jt_offsets,
        ) {
            obj.add_relocation(section_id, r)?;
        }
    }
    for (symbol, relocs) in trampoline_relocs {
        let (_, off) = obj.symbol_section_and_offset(symbol).unwrap();
        for r in to_object_relocations(
            relocs.iter(),
            off,
            module,
            &func_symbols,
            &libcalls,
            &jt_offsets,
        ) {
            obj.add_relocation(section_id, r)?;
        }
    }

    // Write all debug data relocations.
    for (name, relocs) in debug_relocs {
        let section_id = *dwarf_sections_ids.get(name).unwrap();
        for reloc in relocs {
            let target_symbol = match reloc.target {
                DwarfSectionRelocTarget::Func(index) => func_symbols[FuncIndex::new(index)],
                DwarfSectionRelocTarget::Section(name) => {
                    obj.section_symbol(*dwarf_sections_ids.get(name).unwrap())
                }
            };
            obj.add_relocation(
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

    Ok((obj, unwind_info))
}

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

pub(crate) mod utils {
    use wasmtime_environ::entity::EntityRef;
    use wasmtime_environ::wasm::{FuncIndex, SignatureIndex};

    pub const FUNCTION_PREFIX: &str = "_wasm_function_";
    pub const TRAMPOLINE_PREFIX: &str = "_trampoline_";

    pub fn func_symbol_name(index: FuncIndex) -> String {
        format!("_wasm_function_{}", index.index())
    }

    pub fn try_parse_func_name(name: &str) -> Option<FuncIndex> {
        if !name.starts_with(FUNCTION_PREFIX) {
            return None;
        }
        name[FUNCTION_PREFIX.len()..]
            .parse()
            .ok()
            .map(FuncIndex::new)
    }

    pub fn trampoline_symbol_name(index: SignatureIndex) -> String {
        format!("_trampoline_{}", index.index())
    }

    pub fn try_parse_trampoline_name(name: &str) -> Option<SignatureIndex> {
        if !name.starts_with(TRAMPOLINE_PREFIX) {
            return None;
        }
        name[TRAMPOLINE_PREFIX.len()..]
            .parse()
            .ok()
            .map(SignatureIndex::new)
    }
}
