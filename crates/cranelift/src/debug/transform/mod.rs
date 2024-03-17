use self::refs::DebugInfoRefsMap;
use self::simulate::generate_simulated_dwarf;
use self::unit::clone_unit;
use crate::debug::gc::build_dependencies;
use crate::debug::ModuleMemoryOffset;
use crate::CompiledFunctionsMetadata;
use anyhow::Error;
use cranelift_codegen::isa::TargetIsa;
use gimli::{
    write, DebugAddr, DebugLine, DebugStr, Dwarf, DwarfPackage, LittleEndian, LocationLists,
    RangeLists, Unit, UnitSectionOffset,
};
use object::{File, Object, ObjectSection, ObjectSymbol};
use std::borrow::Cow;
use std::{collections::HashMap, collections::HashSet, fmt::Debug, result::Result};

use thiserror::Error;
use typed_arena::Arena;
use wasmtime_environ::DebugInfoData;

pub use address_transform::AddressTransform;

mod address_transform;
mod attr;
mod expression;
mod line_program;
mod range_info_builder;
mod refs;
mod simulate;
mod unit;
mod utils;

pub(crate) trait Reader: gimli::Reader<Offset = usize> + Send + Sync {}

type RelocationMap = HashMap<usize, object::Relocation>;

impl<'input, Endian> Reader for gimli::EndianSlice<'input, Endian> where
    Endian: gimli::Endianity + Send + Sync
{
}

#[derive(Error, Debug)]
#[error("Debug info transform error: {0}")]
pub struct TransformError(&'static str);

pub(crate) struct DebugInputContext<'a, R>
where
    R: Reader,
{
    debug_str: &'a DebugStr<R>,
    debug_line: &'a DebugLine<R>,
    debug_addr: &'a DebugAddr<R>,
    rnglists: &'a RangeLists<R>,
    loclists: &'a LocationLists<R>,
    reachable: &'a HashSet<UnitSectionOffset>,
}

fn add_relocations(
    relocations: &mut RelocationMap,
    file: &object::File,
    section: &object::Section,
) {
    for (offset64, mut relocation) in section.relocations() {
        let offset = offset64 as usize;
        if offset as u64 != offset64 {
            continue;
        }
        match relocation.kind() {
            object::RelocationKind::Absolute => {
                match relocation.target() {
                    object::RelocationTarget::Symbol(symbol_idx) => {
                        match file.symbol_by_index(symbol_idx) {
                            Ok(symbol) => {
                                let addend =
                                    symbol.address().wrapping_add(relocation.addend() as u64);
                                relocation.set_addend(addend as i64);
                            }
                            Err(_) => {
                                eprintln!(
                                    "Relocation with invalid symbol for section {} at offset 0x{:08x}",
                                    section.name().unwrap(),
                                    offset
                                );
                            }
                        }
                    }
                    _ => {}
                }
                if relocations.insert(offset, relocation).is_some() {
                    eprintln!(
                        "Multiple relocations for section {} at offset 0x{:08x}",
                        section.name().unwrap(),
                        offset
                    );
                }
            }
            _ => {
                eprintln!(
                    "Unsupported relocation for section {} at offset 0x{:08x}",
                    section.name().unwrap(),
                    offset
                );
            }
        }
    }
}

fn load_file_section<'input, 'arena, Endian: gimli::Endianity>(
    id: gimli::SectionId,
    file: &object::File<'input>,
    endian: Endian,
    is_dwo: bool,
    arena_data: &'arena Arena<Cow<'input, [u8]>>,
) -> anyhow::Result<gimli::EndianSlice<'input, Endian>>
where
    'arena: 'input,
{
    let mut relocations = RelocationMap::default();
    let name = if is_dwo {
        id.dwo_name()
    } else if file.format() == object::BinaryFormat::Xcoff {
        id.xcoff_name()
    } else {
        Some(id.name())
    };

    let data = match name.and_then(|name| file.section_by_name(&name)) {
        Some(ref section) => {
            // DWO sections never have relocations, so don't bother.
            if !is_dwo {
                add_relocations(&mut relocations, file, section);
            }
            section.uncompressed_data()?
        }
        // Use a non-zero capacity so that `ReaderOffsetId`s are unique.
        None => Cow::Owned(Vec::with_capacity(1)),
    };
    let data_ref = arena_data.alloc(data);
    let reader = gimli::EndianSlice::new(data_ref, endian);
    let section = reader;
    Ok(section)
}

fn load_dwp<'data>(
    file: &File<'data>,
    arena_data: &'data Arena<Cow<'data, [u8]>>,
    buffer: &'data [u8],
) -> anyhow::Result<DwarfPackage<gimli::EndianSlice<'data, gimli::LittleEndian>>> {
    // Read the file contents into a Vec<u8>
    // let file_contents = std::fs::read(file_path)?;

    // Create a gimli::EndianSlice from the file contents

    let endian_slice = gimli::EndianSlice::new(buffer, LittleEndian);

    let mut load_section = |id: gimli::SectionId| -> anyhow::Result<_> {
        load_file_section(id, &file, gimli::LittleEndian, true, arena_data)
    };

    // Load the DwarfPackage from the EndianSlice
    let dwarf_package = DwarfPackage::load(&mut load_section, endian_slice)?;

    Ok(dwarf_package)

    // let empty = Module::empty_file_section(&arena_relocations);
    // gimli::DwarfPackage::load(&mut load_section, empty)
}

/// Attempts to load a DWARF package using the passed bytes.
fn read_dwarf_package_from_bytes<'data>(
    dwp_bytes: &'data [u8],
    buffer: &'data mut Vec<u8>,
    arena_data: &'data Arena<Cow<'data, [u8]>>,
) -> Option<DwarfPackage<gimli::EndianSlice<'data, gimli::LittleEndian>>> {
    let object_file = match object::File::parse(dwp_bytes) {
        Ok(file) => file,
        Err(err) => {
            eprintln!("Failed to parse file {}", err);
            return None;
        }
    };

    match load_dwp(&object_file, &arena_data, buffer) {
        Ok(package) => Some(package),
        Err(err) => {
            eprintln!("Failed to load Dwarf package {}", err);
            None
        }
    }
}

pub fn transform_dwarf<'data>(
    isa: &dyn TargetIsa,
    di: &DebugInfoData,
    funcs: &CompiledFunctionsMetadata,
    memory_offset: &ModuleMemoryOffset,
    dwarf_package_bytes: Option<&[u8]>,
) -> Result<write::Dwarf, Error> {
    let addr_tr = AddressTransform::new(funcs, &di.wasm_file);

    let arena_data = Arena::new();
    let mut buffer = Vec::new();
    let dwarf_package = dwarf_package_bytes
        .map(
            |bytes| -> Option<DwarfPackage<gimli::EndianSlice<'_, gimli::LittleEndian>>> {
                read_dwarf_package_from_bytes(bytes, &mut buffer, &arena_data)
            },
        )
        .flatten();

    let reachable = build_dependencies(&di.dwarf, &dwarf_package, &addr_tr)?.get_reachable();

    let context = DebugInputContext {
        debug_str: &di.dwarf.debug_str,
        debug_line: &di.dwarf.debug_line,
        debug_addr: &di.dwarf.debug_addr,
        rnglists: &di.dwarf.ranges,
        loclists: &di.dwarf.locations,
        reachable: &reachable,
    };

    let out_encoding = gimli::Encoding {
        format: gimli::Format::Dwarf32,
        // TODO: this should be configurable
        version: 4,
        address_size: isa.pointer_bytes(),
    };

    let mut out_strings = write::StringTable::default();
    let mut out_units = write::UnitTable::default();

    let out_line_strings = write::LineStringTable::default();
    let mut pending_di_refs = Vec::new();
    let mut di_ref_map = DebugInfoRefsMap::new();

    let mut translated = HashSet::new();
    let mut iter = di.dwarf.debug_info.units();

    while let Some(header) = iter.next().unwrap_or(None) {
        let unit = di.dwarf.unit(header)?;

        let resolved_unit;

        let mut split_dwarf = None;

        if let gimli::UnitType::Skeleton(_dwo_id) = unit.header.type_() {
            if dwarf_package.is_some() {
                if let Some((fused, fused_dwarf)) = replace_unit_from_split_dwarf(
                    &unit,
                    &dwarf_package.as_ref().unwrap(),
                    &di.dwarf,
                ) {
                    resolved_unit = Some(fused);
                    split_dwarf = Some(fused_dwarf);
                } else {
                    resolved_unit = None;
                }
            } else {
                resolved_unit = None;
            }
        } else {
            resolved_unit = None;
        }

        if let Some((id, ref_map, pending_refs)) = clone_unit(
            &di.dwarf,
            &unit,
            resolved_unit.as_ref(),
            split_dwarf.as_ref(),
            &context,
            &addr_tr,
            funcs,
            memory_offset,
            out_encoding,
            &mut out_units,
            &mut out_strings,
            &mut translated,
            isa,
        )? {
            di_ref_map.insert(&header, id, ref_map);
            pending_di_refs.push((id, pending_refs));
        }
    }
    di_ref_map.patch(pending_di_refs.into_iter(), &mut out_units);

    generate_simulated_dwarf(
        &addr_tr,
        di,
        memory_offset,
        funcs,
        &translated,
        out_encoding,
        &mut out_units,
        &mut out_strings,
        isa,
    )?;

    Ok(write::Dwarf {
        units: out_units,
        line_programs: vec![],
        line_strings: out_line_strings,
        strings: out_strings,
    })
}

fn replace_unit_from_split_dwarf<'a>(
    unit: &'a Unit<gimli::EndianSlice<'a, gimli::LittleEndian>, usize>,
    dwp: &DwarfPackage<gimli::EndianSlice<'a, gimli::LittleEndian>>,
    parent: &Dwarf<gimli::EndianSlice<'a, gimli::LittleEndian>>,
) -> Option<(
    Unit<gimli::EndianSlice<'a, gimli::LittleEndian>, usize>,
    Dwarf<gimli::EndianSlice<'a, gimli::LittleEndian>>,
)> {
    if let Some(dwo_id) = unit.dwo_id {
        return match dwp.find_cu(dwo_id, parent) {
            Ok(cu) => match cu {
                Some(split_unit_dwarf) => match split_unit_dwarf.debug_info.units().next() {
                    Ok(Some(unit_header)) => Some((
                        split_unit_dwarf.unit(unit_header).unwrap(),
                        split_unit_dwarf,
                    )),
                    Err(err) => {
                        eprintln!("Failed to get unit header from compilation unit {}", err);
                        None
                    }
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        };
    }

    None
}
