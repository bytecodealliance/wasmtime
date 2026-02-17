use self::debug_transform_logging::dbi_log;
use self::simulate::generate_simulated_dwarf;
use self::unit::clone_unit;
use crate::debug::Compilation;
use crate::debug::gc::build_dependencies;
use cranelift_codegen::isa::TargetIsa;
use gimli::{DwarfPackage, LittleEndian, Section, write};
use std::collections::HashSet;
use synthetic::ModuleSyntheticUnit;
use wasmtime_environ::error::Error;
use wasmtime_environ::{
    DefinedFuncIndex, ModuleTranslation, PrimaryMap, StaticModuleIndex, Tunables, prelude::*,
};

pub use address_transform::AddressTransform;

mod address_transform;
mod attr;
mod debug_transform_logging;
mod expression;
mod line_program;
mod range_info_builder;
mod simulate;
mod synthetic;
mod unit;
mod utils;

impl<'a> Compilation<'a> {
    fn function_frame_info(
        &mut self,
        module: StaticModuleIndex,
        func: DefinedFuncIndex,
    ) -> expression::FunctionFrameInfo<'a> {
        let (_, func) = self.function(module, func);

        expression::FunctionFrameInfo {
            value_ranges: &func.value_labels_ranges,
            memory_offset: self.module_memory_offsets[module].clone(),
        }
    }
}

fn load_dwp<'data>(
    translation: ModuleTranslation<'data>,
    buffer: &'data [u8],
) -> Result<DwarfPackage<gimli::EndianSlice<'data, gimli::LittleEndian>>> {
    let endian_slice = gimli::EndianSlice::new(buffer, LittleEndian);

    let dwarf_package = DwarfPackage::load(
        |id| -> Result<_> {
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
    buffer: &'data [u8],
    tunables: &Tunables,
) -> Option<DwarfPackage<gimli::EndianSlice<'data, gimli::LittleEndian>>> {
    let mut validator = wasmparser::Validator::new();
    let parser = wasmparser::Parser::new(0);
    let mut types = wasmtime_environ::ModuleTypesBuilder::new(&validator);
    let translation = match wasmtime_environ::ModuleEnvironment::new(
        tunables,
        &mut validator,
        &mut types,
        StaticModuleIndex::from_u32(0),
    )
    .translate(parser, dwp_bytes)
    {
        Ok(translation) => translation,
        Err(e) => {
            log::warn!("failed to parse wasm dwarf package: {e:?}");
            return None;
        }
    };

    match load_dwp(translation, buffer) {
        Ok(package) => Some(package),
        Err(err) => {
            log::warn!("Failed to load Dwarf package {err}");
            None
        }
    }
}

pub fn transform_dwarf(
    isa: &dyn TargetIsa,
    compilation: &mut Compilation<'_>,
) -> Result<write::Dwarf, Error> {
    dbi_log!("Commencing DWARF transform for {:?}", compilation);

    let mut transforms = PrimaryMap::new();
    for (i, _) in compilation.translations.iter() {
        transforms.push(AddressTransform::new(compilation, i));
    }

    let buffer = Vec::new();

    let dwarf_package = compilation
        .dwarf_package_bytes
        .map(
            |bytes| -> Option<DwarfPackage<gimli::EndianSlice<'_, gimli::LittleEndian>>> {
                read_dwarf_package_from_bytes(bytes, &buffer, compilation.tunables)
            },
        )
        .flatten();

    let out_encoding = gimli::Encoding {
        format: gimli::Format::Dwarf32,
        version: 4, // TODO: this should be configurable
        address_size: isa.pointer_bytes(),
    };
    let mut out_dwarf = write::Dwarf::default();

    let mut vmctx_ptr_die_refs = PrimaryMap::new();

    let mut translated = HashSet::new();

    for (module, translation) in compilation.translations.iter() {
        dbi_log!("[== Transforming CUs for module #{} ==]", module.as_u32());

        let addr_tr = &transforms[module];
        let di = &translation.debuginfo;

        let out_module_synthetic_unit = ModuleSyntheticUnit::new(
            module,
            compilation,
            out_encoding,
            &mut out_dwarf.units,
            &mut out_dwarf.strings,
        );
        // TODO-DebugInfo-Cleanup: move the simulation code to be per-module and delete this map.
        vmctx_ptr_die_refs.push(out_module_synthetic_unit.vmctx_ptr_die_ref());

        let mut filter = write::FilterUnitSection::new(&di.dwarf)?;
        build_dependencies(&mut filter, addr_tr)?;
        let mut convert = out_dwarf.convert_with_filter(filter)?;
        while let Some((mut unit, root_entry)) = convert.read_unit()? {
            if let Some(dwp) = dwarf_package.as_ref()
                && let Some(dwo_id) = unit.read_unit.dwo_id
                && let Ok(Some(split_dwarf)) = dwp.find_cu(dwo_id, unit.read_unit.dwarf)
            {
                let mut split_filter =
                    write::FilterUnitSection::new_split(&split_dwarf, unit.read_unit)?;
                build_dependencies(&mut split_filter, addr_tr)?;
                let mut convert_split = unit.convert_split_with_filter(split_filter)?;
                let (mut split_unit, split_root_entry) = convert_split.read_unit()?;
                split_unit.unit.set_encoding(out_encoding);
                clone_unit(
                    compilation,
                    module,
                    &mut split_unit,
                    &split_root_entry,
                    Some(&root_entry),
                    &addr_tr,
                    &out_module_synthetic_unit,
                    &mut translated,
                    isa,
                )?;
            } else {
                unit.unit.set_encoding(out_encoding);
                clone_unit(
                    compilation,
                    module,
                    &mut unit,
                    &root_entry,
                    None,
                    &addr_tr,
                    &out_module_synthetic_unit,
                    &mut translated,
                    isa,
                )?;
            }
        }
    }

    generate_simulated_dwarf(
        compilation,
        &transforms,
        &translated,
        out_encoding,
        &vmctx_ptr_die_refs,
        &mut out_dwarf.units,
        &mut out_dwarf.strings,
        isa,
    )?;

    Ok(out_dwarf)
}
