//! Support for serializing type information for a `Module`.
//!
//! Wasmtime AOT compiled artifacts are ELF files where relevant data is stored
//! in relevant sections. This module implements the serialization format for
//! type information, or the `ModuleTypes` structure.
//!
//! This structure lives in a section of the final artifact at this time. It is
//! appended after compilation has otherwise completed and additionally is
//! deserialized from the entirety of the section.
//!
//! Implementation details are "just bincode it all" right now with no further
//! clever tricks about representation. Currently this works out more-or-less
//! ok since the type information is typically relatively small per-module.

use anyhow::{anyhow, Result};
use object::write::{Object, StandardSegment};
use object::{File, Object as _, ObjectSection, SectionKind};
use wasmtime_environ::ModuleTypes;
use wasmtime_runtime::MmapVec;

const ELF_WASM_TYPES: &str = ".wasmtime.types";

pub fn append_types(types: &ModuleTypes, obj: &mut Object<'_>) {
    let section = obj.add_section(
        obj.segment_name(StandardSegment::Data).to_vec(),
        ELF_WASM_TYPES.as_bytes().to_vec(),
        SectionKind::ReadOnlyData,
    );
    let data = bincode::serialize(types).unwrap();
    obj.set_section_data(section, data, 1);
}

pub fn deserialize_types(mmap: &MmapVec) -> Result<ModuleTypes> {
    // Ideally we'd only `File::parse` once and avoid the linear
    // `section_by_name` search here but the general serialization code isn't
    // structured well enough to make this easy and additionally it's not really
    // a perf issue right now so doing that is left for another day's
    // refactoring.
    let obj = File::parse(&mmap[..])?;
    let data = obj
        .section_by_name(ELF_WASM_TYPES)
        .ok_or_else(|| anyhow!("failed to find section `{ELF_WASM_TYPES}`"))?
        .data()?;
    Ok(bincode::deserialize(data)?)
}
