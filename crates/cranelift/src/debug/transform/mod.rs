use self::refs::DebugInfoRefsMap;
use self::simulate::generate_simulated_dwarf;
use self::unit::clone_unit;
use crate::debug::gc::build_dependencies;
use crate::debug::ModuleMemoryOffset;
use crate::CompiledFunctionsMetadata;
use anyhow::Error;
use cranelift_codegen::isa::TargetIsa;
use fallible_iterator::FallibleIterator;
use gimli::{
    write, DebugAddr, DebugLine, DebugLineStr, DebugStr, DebugStrOffsets, Dwarf, DwarfPackage,
    DwoId, EndianSlice, LocationLists, RangeLists, Unit, UnitSectionOffset,
};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    fmt::Debug,
    fs, mem,
    path::PathBuf,
    result::Result,
};
use thiserror::Error;
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
    debug_str_offsets: &'a DebugStrOffsets<R>,
    debug_line_str: &'a DebugLineStr<R>,
    debug_line: &'a DebugLine<R>,
    debug_addr: &'a DebugAddr<R>,
    rnglists: &'a RangeLists<R>,
    loclists: &'a LocationLists<R>,
    reachable: &'a HashSet<UnitSectionOffset>,
}

pub fn transform_dwarf(
    isa: &dyn TargetIsa,
    di: &DebugInfoData,
    funcs: &CompiledFunctionsMetadata,
    memory_offset: &ModuleMemoryOffset,
) -> Result<write::Dwarf, Error> {
    let addr_tr = AddressTransform::new(funcs, &di.wasm_file);
    let reachable = build_dependencies(&di.dwarf, &di.dwarf_package, &addr_tr)?.get_reachable();

    let context = DebugInputContext {
        debug_str: &di.dwarf.debug_str,
        debug_str_offsets: &di.dwarf.debug_str_offsets,
        debug_line_str: &di.dwarf.debug_line_str,
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

        if let gimli::UnitType::Skeleton(dwo_id) = unit.header.type_() {
            if di.dwarf_package.is_some() {
                if let Some((fused, fused_dwarf)) = replace_unit_from_split_dwarf(
                    &unit,
                    di.dwarf_package.as_ref().unwrap(),
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
                        return None;
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
