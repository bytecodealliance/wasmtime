use std::collections::HashMap;
use wasmparser::{ModuleReader, SectionCode};

use gimli;

use gimli::{
    DebugAbbrev, DebugAddr, DebugInfo, DebugLine, DebugLineStr, DebugLoc, DebugLocLists,
    DebugRanges, DebugRngLists, DebugStr, DebugStrOffsets, DebugTypes, EndianSlice, LittleEndian,
    LocationLists, RangeLists,
};

trait Reader: gimli::Reader<Offset = usize, Endian = LittleEndian> {}

impl<'input> Reader for gimli::EndianSlice<'input, LittleEndian> {}

pub type Dwarf<'input> = gimli::Dwarf<gimli::EndianSlice<'input, LittleEndian>>;

#[derive(Debug)]
pub struct WasmFileInfo {
    pub code_section_offset: u64,
}

#[derive(Debug)]
pub struct DebugInfoData<'a> {
    pub dwarf: Dwarf<'a>,
    pub wasm_file: WasmFileInfo,
}

fn convert_sections<'a>(sections: HashMap<&str, &'a [u8]>) -> Dwarf<'a> {
    let endian = LittleEndian;
    let debug_str = DebugStr::new(sections[".debug_str"], endian);
    let debug_abbrev = DebugAbbrev::new(sections[".debug_abbrev"], endian);
    let debug_info = DebugInfo::new(sections[".debug_info"], endian);
    let debug_line = DebugLine::new(sections[".debug_line"], endian);

    if sections.contains_key(".debug_addr") {
        panic!("Unexpected .debug_addr");
    }

    let debug_addr = DebugAddr::from(EndianSlice::new(&[], endian));

    if sections.contains_key(".debug_line_str") {
        panic!("Unexpected .debug_line_str");
    }

    let debug_line_str = DebugLineStr::from(EndianSlice::new(&[], endian));
    let debug_str_sup = DebugStr::from(EndianSlice::new(&[], endian));

    if sections.contains_key(".debug_rnglists") {
        panic!("Unexpected .debug_rnglists");
    }

    let debug_ranges = match sections.get(".debug_ranges") {
        Some(section) => DebugRanges::new(section, endian),
        None => DebugRanges::new(&[], endian),
    };
    let debug_rnglists = DebugRngLists::new(&[], endian);
    let ranges = RangeLists::new(debug_ranges, debug_rnglists);

    if sections.contains_key(".debug_loclists") {
        panic!("Unexpected .debug_loclists");
    }

    let debug_loc = match sections.get(".debug_loc") {
        Some(section) => DebugLoc::new(section, endian),
        None => DebugLoc::new(&[], endian),
    };
    let debug_loclists = DebugLocLists::new(&[], endian);
    let locations = LocationLists::new(debug_loc, debug_loclists);

    if sections.contains_key(".debug_str_offsets") {
        panic!("Unexpected .debug_str_offsets");
    }

    let debug_str_offsets = DebugStrOffsets::from(EndianSlice::new(&[], endian));

    if sections.contains_key(".debug_types") {
        panic!("Unexpected .debug_types");
    }

    let debug_types = DebugTypes::from(EndianSlice::new(&[], endian));

    Dwarf {
        debug_abbrev,
        debug_addr,
        debug_info,
        debug_line,
        debug_line_str,
        debug_str,
        debug_str_offsets,
        debug_str_sup,
        debug_types,
        locations,
        ranges,
    }
}

pub fn read_debuginfo(data: &[u8]) -> DebugInfoData {
    let mut reader = ModuleReader::new(data).expect("reader");
    let mut sections = HashMap::new();
    let mut code_section_offset = 0;
    while !reader.eof() {
        let section = reader.read().expect("section");
        if let SectionCode::Custom { name, .. } = section.code {
            if name.starts_with(".debug_") {
                let mut reader = section.get_binary_reader();
                let len = reader.bytes_remaining();
                sections.insert(name, reader.read_bytes(len).expect("bytes"));
            }
        }
        if let SectionCode::Code = section.code {
            code_section_offset = section.range().start as u64;
        }
    }
    DebugInfoData {
        dwarf: convert_sections(sections),
        wasm_file: WasmFileInfo {
            code_section_offset,
        },
    }
}
