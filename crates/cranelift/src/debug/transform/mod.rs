use self::refs::DebugInfoRefsMap;
use self::simulate::generate_simulated_dwarf;
use self::unit::clone_unit;
use crate::debug::gc::build_dependencies;
use crate::debug::ModuleMemoryOffset;
use crate::CompiledFunctionsMetadata;
use anyhow::{Context, Error};
use cranelift_codegen::isa::TargetIsa;
use gimli::{
    write, DebugAddr, DebugLine, DebugStr, Dwarf, DwarfPackage, LittleEndian, LocationLists,
    RangeLists, Section, Unit, UnitSectionOffset,
};
use std::{collections::HashSet, fmt::Debug, result::Result};
use thiserror::Error;
use wasmtime_environ::{DebugInfoData, ModuleTranslation, Tunables};

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

fn load_dwp<'data>(
    translation: ModuleTranslation<'data>,
    buffer: &'data Vec<u8>,
) -> anyhow::Result<DwarfPackage<gimli::EndianSlice<'data, gimli::LittleEndian>>> {
    let endian_slice = gimli::EndianSlice::new(buffer, LittleEndian);

    let dwarf_package = DwarfPackage::load(
        |id| -> anyhow::Result<_> {
            let slice = match id {
                gimli::SectionId::DebugAbbrev => {
                    translation.debuginfo.dwarf.debug_abbrev.reader().slice()
                }
                gimli::SectionId::DebugInfo => {
                    translation.debuginfo.dwarf.debug_info.reader().slice()
                }
                gimli::SectionId::DebugLine => {
                    translation.debuginfo.dwarf.debug_line.reader().slice()
                }
                gimli::SectionId::DebugStr => {
                    translation.debuginfo.dwarf.debug_str.reader().slice()
                }
                gimli::SectionId::DebugStrOffsets => translation
                    .debuginfo
                    .dwarf
                    .debug_str_offsets
                    .reader()
                    .slice(),
                gimli::SectionId::DebugLoc => translation.debuginfo.debug_loc.reader().slice(),
                gimli::SectionId::DebugLocLists => {
                    translation.debuginfo.debug_loclists.reader().slice()
                }
                gimli::SectionId::DebugRngLists => {
                    translation.debuginfo.debug_rnglists.reader().slice()
                }
                gimli::SectionId::DebugTypes => {
                    translation.debuginfo.dwarf.debug_types.reader().slice()
                }
                gimli::SectionId::DebugCuIndex => {
                    translation.debuginfo.debug_cu_index.reader().slice()
                }
                gimli::SectionId::DebugTuIndex => {
                    translation.debuginfo.debug_tu_index.reader().slice()
                }
                _ => &buffer,
            };

            Ok(gimli::EndianSlice::new(slice, gimli::LittleEndian))
        },
        endian_slice,
    )?;

    Ok(dwarf_package)
}

/// Attempts to load a DWARF package using the passed bytes.
fn read_dwarf_package_from_bytes<'data>(
    dwp_bytes: &'data [u8],
    buffer: &'data Vec<u8>,
    tunables: &Tunables,
) -> Option<DwarfPackage<gimli::EndianSlice<'data, gimli::LittleEndian>>> {
    let mut validator = wasmparser::Validator::new();
    let parser = wasmparser::Parser::new(0);
    let mut types = wasmtime_environ::ModuleTypesBuilder::new(&validator);
    let translation =
        wasmtime_environ::ModuleEnvironment::new(tunables, &mut validator, &mut types)
            .translate(parser, dwp_bytes)
            .context("failed to parse WebAssembly DWARF package")
            .unwrap();

    match load_dwp(translation, buffer) {
        Ok(package) => Some(package),
        Err(err) => {
            log::warn!("Failed to load Dwarf package {}", err);
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
    tunables: &Tunables,
) -> Result<write::Dwarf, Error> {
    let addr_tr = AddressTransform::new(funcs, &di.wasm_file);

    let buffer = Vec::new();

    let dwarf_package = dwarf_package_bytes
        .map(
            |bytes| -> Option<DwarfPackage<gimli::EndianSlice<'_, gimli::LittleEndian>>> {
                read_dwarf_package_from_bytes(bytes, &buffer, tunables)
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
