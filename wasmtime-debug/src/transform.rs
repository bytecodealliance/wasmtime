use crate::address_transform::AddressTransform;
use crate::gc::build_dependencies;
pub use crate::read_debuginfo::DebugInfoData;
use cranelift_codegen::ir;
use cranelift_codegen::isa::TargetFrontendConfig;
use cranelift_entity::{EntityRef, PrimaryMap};
use cranelift_wasm::DefinedFuncIndex;
use failure::Error;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::iter::FromIterator;

use gimli;

use gimli::{
    AttributeValue, DebugAddr, DebugAddrBase, DebugLine, DebugLineOffset, DebugStr,
    DebuggingInformationEntry, LineEncoding, LocationLists, RangeLists, Unit, UnitOffset,
    UnitSectionOffset,
};

use gimli::write;

trait Reader: gimli::Reader<Offset = usize> {}

impl<'input, Endian> Reader for gimli::EndianSlice<'input, Endian> where Endian: gimli::Endianity {}

#[derive(Fail, Debug)]
#[fail(display = "Debug info transform error: {}", _0)]
pub struct TransformError(&'static str);

/// Single wasm source location to generated address mapping.
#[derive(Debug, Clone)]
pub struct InstructionAddressMap {
    /// Original source location.
    pub srcloc: ir::SourceLoc,

    /// Generated instructions offset.
    pub code_offset: usize,

    /// Generated instructions length.
    pub code_len: usize,
}

/// Function and its instructions addresses mappings.
#[derive(Debug, Clone)]
pub struct FunctionAddressMap {
    /// Instructions maps.
    /// The array is sorted by the InstructionAddressMap::code_offset field.
    pub instructions: Vec<InstructionAddressMap>,

    /// Generated function body offset if applicable, otherwise 0.
    pub body_offset: usize,

    /// Generated function body length.
    pub body_len: usize,
}

/// Module functions addresses mappings.
pub type ModuleAddressMap = PrimaryMap<DefinedFuncIndex, FunctionAddressMap>;

/// Module `vmctx` related info.
pub struct ModuleVmctxInfo {
    pub memory_offset: i64,
    pub stack_slots: PrimaryMap<DefinedFuncIndex, ir::StackSlots>,
}

/// Value ranges for functions.
pub type ValueLabelsRanges = PrimaryMap<DefinedFuncIndex, cranelift_codegen::ValueLabelsRanges>;

struct DebugInputContext<'a, R>
where
    R: Reader,
{
    debug_str: &'a DebugStr<R>,
    debug_line: &'a DebugLine<R>,
    debug_addr: &'a DebugAddr<R>,
    debug_addr_base: DebugAddrBase<R::Offset>,
    rnglists: &'a RangeLists<R>,
    loclists: &'a LocationLists<R>,
    reachable: &'a HashSet<UnitSectionOffset>,
}

type PendingDieRef = (write::UnitEntryId, gimli::DwAt, UnitOffset);

enum FileAttributeContext<'a> {
    Root(Option<DebugLineOffset>),
    Children(&'a Vec<write::FileId>),
}

fn clone_die_attributes<'a, R>(
    entry: &DebuggingInformationEntry<R>,
    context: &DebugInputContext<R>,
    addr_tr: &'a AddressTransform,
    unit_encoding: &gimli::Encoding,
    current_scope: &mut write::DebuggingInformationEntry,
    current_scope_id: write::UnitEntryId,
    subprogram_range: Option<(write::Address, u64)>,
    out_strings: &mut write::StringTable,
    die_ref_map: &HashMap<UnitOffset, write::UnitEntryId>,
    pending_die_refs: &mut Vec<PendingDieRef>,
    file_context: FileAttributeContext<'a>,
) -> Result<(), Error>
where
    R: Reader,
{
    let _tag = &entry.tag();
    let mut attrs = entry.attrs();
    let mut low_pc = None;
    while let Some(attr) = attrs.next()? {
        let attr_value = match attr.value() {
            AttributeValue::Addr(_)
                if attr.name() == gimli::DW_AT_low_pc && subprogram_range.is_some() =>
            {
                write::AttributeValue::Address(subprogram_range.unwrap().0)
            }
            AttributeValue::Udata(_)
                if attr.name() == gimli::DW_AT_high_pc && subprogram_range.is_some() =>
            {
                write::AttributeValue::Udata(subprogram_range.unwrap().1)
            }
            AttributeValue::Addr(u) => {
                let addr = addr_tr.translate(u).unwrap_or(write::Address::Constant(0));
                if attr.name() == gimli::DW_AT_low_pc {
                    low_pc = Some((u, addr));
                }
                write::AttributeValue::Address(addr)
            }
            AttributeValue::Udata(u) => {
                if attr.name() != gimli::DW_AT_high_pc || low_pc.is_none() {
                    write::AttributeValue::Udata(u)
                } else {
                    let u = addr_tr.delta(low_pc.unwrap().0, u).unwrap_or(0);
                    write::AttributeValue::Udata(u)
                }
            }
            AttributeValue::Data1(d) => write::AttributeValue::Data1(d),
            AttributeValue::Data2(d) => write::AttributeValue::Data2(d),
            AttributeValue::Data4(d) => write::AttributeValue::Data4(d),
            AttributeValue::Sdata(d) => write::AttributeValue::Sdata(d),
            AttributeValue::Flag(f) => write::AttributeValue::Flag(f),
            AttributeValue::DebugLineRef(line_program_offset) => {
                if let FileAttributeContext::Root(o) = file_context {
                    if o != Some(line_program_offset) {
                        return Err(TransformError("invalid debug_line offset").into());
                    }
                    write::AttributeValue::LineProgramRef
                } else {
                    return Err(TransformError("unexpected debug_line index attribute").into());
                }
            }
            AttributeValue::FileIndex(i) => {
                if let FileAttributeContext::Children(file_map) = file_context {
                    write::AttributeValue::FileIndex(Some(file_map[(i - 1) as usize]))
                } else {
                    return Err(TransformError("unexpected file index attribute").into());
                }
            }
            AttributeValue::DebugStrRef(str_offset) => {
                let s = context.debug_str.get_str(str_offset)?.to_slice()?.to_vec();
                write::AttributeValue::StringRef(out_strings.add(s))
            }
            AttributeValue::RangeListsRef(r) => {
                let low_pc = 0;
                let mut ranges = context.rnglists.ranges(
                    r,
                    *unit_encoding,
                    low_pc,
                    &context.debug_addr,
                    context.debug_addr_base,
                )?;
                let mut _result = Vec::new();
                while let Some(range) = ranges.next()? {
                    assert!(range.begin <= range.end);
                    _result.push((range.begin as i64, range.end as i64));
                }
                // FIXME _result contains invalid code offsets; translate_address
                continue; // ignore attribute
            }
            AttributeValue::LocationListsRef(r) => {
                let low_pc = 0;
                let mut locs = context.loclists.locations(
                    r,
                    *unit_encoding,
                    low_pc,
                    &context.debug_addr,
                    context.debug_addr_base,
                )?;
                let mut _result = Vec::new();
                while let Some(loc) = locs.next()? {
                    _result.push((loc.range.begin as i64, loc.range.end as i64, loc.data.0));
                }
                // FIXME _result contains invalid expressions and code offsets
                continue; // ignore attribute
            }
            AttributeValue::Exprloc(ref _expr) => {
                // FIXME _expr contains invalid expression
                continue; // ignore attribute
            }
            AttributeValue::Encoding(e) => write::AttributeValue::Encoding(e),
            AttributeValue::DecimalSign(e) => write::AttributeValue::DecimalSign(e),
            AttributeValue::Endianity(e) => write::AttributeValue::Endianity(e),
            AttributeValue::Accessibility(e) => write::AttributeValue::Accessibility(e),
            AttributeValue::Visibility(e) => write::AttributeValue::Visibility(e),
            AttributeValue::Virtuality(e) => write::AttributeValue::Virtuality(e),
            AttributeValue::Language(e) => write::AttributeValue::Language(e),
            AttributeValue::AddressClass(e) => write::AttributeValue::AddressClass(e),
            AttributeValue::IdentifierCase(e) => write::AttributeValue::IdentifierCase(e),
            AttributeValue::CallingConvention(e) => write::AttributeValue::CallingConvention(e),
            AttributeValue::Inline(e) => write::AttributeValue::Inline(e),
            AttributeValue::Ordering(e) => write::AttributeValue::Ordering(e),
            AttributeValue::UnitRef(ref offset) => {
                if let Some(unit_id) = die_ref_map.get(offset) {
                    write::AttributeValue::ThisUnitEntryRef(*unit_id)
                } else {
                    pending_die_refs.push((current_scope_id, attr.name(), *offset));
                    continue;
                }
            }
            // AttributeValue::DebugInfoRef(_) => {
            //     continue;
            // }
            _ => panic!(), //write::AttributeValue::StringRef(out_strings.add("_")),
        };
        current_scope.set(attr.name(), attr_value);
    }
    Ok(())
}

fn clone_attr_string<R>(
    attr_value: &AttributeValue<R>,
    form: gimli::DwForm,
    debug_str: &DebugStr<R>,
    out_strings: &mut write::StringTable,
) -> Result<write::LineString, gimli::Error>
where
    R: Reader,
{
    let content = match attr_value {
        AttributeValue::DebugStrRef(str_offset) => {
            debug_str.get_str(*str_offset)?.to_slice()?.to_vec()
        }
        AttributeValue::String(b) => b.to_slice()?.to_vec(),
        _ => panic!("Unexpected attribute value"),
    };
    Ok(match form {
        gimli::DW_FORM_strp => {
            let id = out_strings.add(content);
            write::LineString::StringRef(id)
        }
        gimli::DW_FORM_string => write::LineString::String(content),
        _ => panic!("DW_FORM_line_strp or other not supported"),
    })
}

#[derive(Debug)]
enum SavedLineProgramRow {
    Normal {
        address: u64,
        op_index: u64,
        file_index: u64,
        line: u64,
        column: u64,
        discriminator: u64,
        is_stmt: bool,
        basic_block: bool,
        prologue_end: bool,
        epilogue_begin: bool,
        isa: u64,
    },
    EndOfSequence(u64),
}

#[derive(Debug, Eq, PartialEq)]
enum ReadLineProgramState {
    SequenceEnded,
    ReadSequence,
    IgnoreSequence,
}

fn clone_line_program<R>(
    unit: &Unit<R, R::Offset>,
    root: &DebuggingInformationEntry<R>,
    addr_tr: &AddressTransform,
    out_encoding: &gimli::Encoding,
    debug_str: &DebugStr<R>,
    debug_line: &DebugLine<R>,
    out_strings: &mut write::StringTable,
) -> Result<(write::LineProgram, DebugLineOffset, Vec<write::FileId>), Error>
where
    R: Reader,
{
    let offset = match root.attr_value(gimli::DW_AT_stmt_list)? {
        Some(gimli::AttributeValue::DebugLineRef(offset)) => offset,
        _ => {
            return Err(TransformError("Debug line offset is not found").into());
        }
    };
    let comp_dir = root.attr_value(gimli::DW_AT_comp_dir)?;
    let comp_name = root.attr_value(gimli::DW_AT_name)?;
    let out_comp_dir = clone_attr_string(
        comp_dir.as_ref().expect("comp_dir"),
        gimli::DW_FORM_strp,
        debug_str,
        out_strings,
    )?;
    let out_comp_name = clone_attr_string(
        comp_name.as_ref().expect("comp_name"),
        gimli::DW_FORM_strp,
        debug_str,
        out_strings,
    )?;

    let program = debug_line.program(
        offset,
        unit.header.address_size(),
        comp_dir.and_then(|val| val.string_value(&debug_str)),
        comp_name.and_then(|val| val.string_value(&debug_str)),
    );
    if let Ok(program) = program {
        let header = program.header();
        assert!(header.version() <= 4, "not supported 5");
        let line_encoding = LineEncoding {
            minimum_instruction_length: header.minimum_instruction_length(),
            maximum_operations_per_instruction: header.maximum_operations_per_instruction(),
            default_is_stmt: header.default_is_stmt(),
            line_base: header.line_base(),
            line_range: header.line_range(),
        };
        let mut out_program = write::LineProgram::new(
            *out_encoding,
            line_encoding,
            out_comp_dir,
            out_comp_name,
            None,
        );
        let mut dirs = Vec::new();
        dirs.push(out_program.default_directory());
        for dir_attr in header.include_directories() {
            let dir_id = out_program.add_directory(clone_attr_string(
                dir_attr,
                gimli::DW_FORM_string,
                debug_str,
                out_strings,
            )?);
            dirs.push(dir_id);
        }
        let mut files = Vec::new();
        for file_entry in header.file_names() {
            let dir_id = dirs[file_entry.directory_index() as usize];
            let file_id = out_program.add_file(
                clone_attr_string(
                    &file_entry.path_name(),
                    gimli::DW_FORM_string,
                    debug_str,
                    out_strings,
                )?,
                dir_id,
                None,
            );
            files.push(file_id);
        }

        let mut rows = program.rows();
        let mut saved_rows = BTreeMap::new();
        let mut state = ReadLineProgramState::SequenceEnded;
        while let Some((_header, row)) = rows.next_row()? {
            if state == ReadLineProgramState::IgnoreSequence {
                if row.end_sequence() {
                    state = ReadLineProgramState::SequenceEnded;
                }
                continue;
            }
            let saved_row = if row.end_sequence() {
                state = ReadLineProgramState::SequenceEnded;
                SavedLineProgramRow::EndOfSequence(row.address())
            } else {
                if state == ReadLineProgramState::SequenceEnded {
                    // Discard sequences for non-existent code.
                    if row.address() == 0 {
                        state = ReadLineProgramState::IgnoreSequence;
                        continue;
                    }
                    state = ReadLineProgramState::ReadSequence;
                }
                SavedLineProgramRow::Normal {
                    address: row.address(),
                    op_index: row.op_index(),
                    file_index: row.file_index(),
                    line: row.line().unwrap_or(0),
                    column: match row.column() {
                        gimli::ColumnType::LeftEdge => 0,
                        gimli::ColumnType::Column(val) => val,
                    },
                    discriminator: row.discriminator(),
                    is_stmt: row.is_stmt(),
                    basic_block: row.basic_block(),
                    prologue_end: row.prologue_end(),
                    epilogue_begin: row.epilogue_begin(),
                    isa: row.isa(),
                }
            };
            saved_rows.insert(row.address(), saved_row);
        }

        let saved_rows = Vec::from_iter(saved_rows.into_iter());

        for (i, map) in addr_tr.map() {
            let symbol = i.index();
            let base_addr = map.offset;
            out_program.begin_sequence(Some(write::Address::Symbol { symbol, addend: 0 }));
            // TODO track and place function declaration line here
            let mut last_address = None;
            for addr_map in map.addresses.iter() {
                let saved_row =
                    match saved_rows.binary_search_by(|entry| entry.0.cmp(&addr_map.wasm)) {
                        Ok(i) => Some(&saved_rows[i].1),
                        Err(i) => {
                            if i > 0 {
                                Some(&saved_rows[i - 1].1)
                            } else {
                                None
                            }
                        }
                    };
                if let Some(SavedLineProgramRow::Normal {
                    address,
                    op_index,
                    file_index,
                    line,
                    column,
                    discriminator,
                    is_stmt,
                    basic_block,
                    prologue_end,
                    epilogue_begin,
                    isa,
                }) = saved_row
                {
                    // Ignore duplicates
                    if Some(*address) != last_address {
                        let address_offset = if last_address.is_none() {
                            // Extend first entry to the function declaration
                            // TODO use the function declaration line instead
                            0
                        } else {
                            (addr_map.generated - base_addr) as u64
                        };
                        out_program.row().address_offset = address_offset;
                        out_program.row().op_index = *op_index;
                        out_program.row().file = files[(file_index - 1) as usize];
                        out_program.row().line = *line;
                        out_program.row().column = *column;
                        out_program.row().discriminator = *discriminator;
                        out_program.row().is_statement = *is_stmt;
                        out_program.row().basic_block = *basic_block;
                        out_program.row().prologue_end = *prologue_end;
                        out_program.row().epilogue_begin = *epilogue_begin;
                        out_program.row().isa = *isa;
                        out_program.generate_row();
                        last_address = Some(*address);
                    }
                }
            }
            let end_addr = (map.offset + map.len - 1) as u64;
            out_program.end_sequence(end_addr);
        }
        Ok((out_program, offset, files))
    } else {
        Err(TransformError("Valid line program not found").into())
    }
}

fn get_subprogram_range<'a, R>(
    entry: &DebuggingInformationEntry<R>,
    addr_tr: &'a AddressTransform,
) -> Result<Option<(write::Address, u64)>, Error>
where
    R: Reader,
{
    let low_pc = entry.attr_value(gimli::DW_AT_low_pc)?;
    if let Some(AttributeValue::Addr(addr)) = low_pc {
        let transformed = addr_tr.translate(addr);
        if let Some(write::Address::Symbol { symbol, .. }) = transformed {
            let range = addr_tr.func_range(symbol);
            let addr = write::Address::Symbol {
                symbol,
                addend: range.0 as i64,
            };
            let len = (range.1 - range.0) as u64;
            return Ok(Some((addr, len)));
        }
    }
    Ok(None)
}

fn clone_unit<'a, R>(
    unit: Unit<R, R::Offset>,
    context: &DebugInputContext<R>,
    addr_tr: &'a AddressTransform,
    out_encoding: &gimli::Encoding,
    out_units: &mut write::UnitTable,
    out_strings: &mut write::StringTable,
) -> Result<(), Error>
where
    R: Reader,
{
    let mut die_ref_map = HashMap::new();
    let mut pending_die_refs = Vec::new();
    let mut stack = Vec::new();

    // Iterate over all of this compilation unit's entries.
    let mut entries = unit.entries();
    let (comp_unit, file_map) = if let Some((depth_delta, entry)) = entries.next_dfs()? {
        assert!(depth_delta == 0);
        let (out_line_program, debug_line_offset, file_map) = clone_line_program(
            &unit,
            entry,
            addr_tr,
            out_encoding,
            context.debug_str,
            context.debug_line,
            out_strings,
        )?;

        if entry.tag() == gimli::DW_TAG_compile_unit {
            let unit_id = out_units.add(write::Unit::new(*out_encoding, out_line_program));
            let comp_unit = out_units.get_mut(unit_id);

            let root_id = comp_unit.root();
            die_ref_map.insert(entry.offset(), root_id);

            clone_die_attributes(
                entry,
                context,
                addr_tr,
                &unit.encoding(),
                comp_unit.get_mut(root_id),
                root_id,
                None,
                out_strings,
                &die_ref_map,
                &mut pending_die_refs,
                FileAttributeContext::Root(Some(debug_line_offset)),
            )?;

            stack.push(root_id);
            (comp_unit, file_map)
        } else {
            return Err(TransformError("Unexpected unit header").into());
        }
    } else {
        return Ok(()); // empty
    };
    let mut skip_at_depth = None;
    while let Some((depth_delta, entry)) = entries.next_dfs()? {
        let depth_delta = if let Some((depth, cached)) = skip_at_depth {
            let new_depth = depth + depth_delta;
            if new_depth > 0 {
                skip_at_depth = Some((new_depth, cached));
                continue;
            }
            skip_at_depth = None;
            new_depth + cached
        } else {
            depth_delta
        };
        if !context
            .reachable
            .contains(&entry.offset().to_unit_section_offset(&unit))
        {
            // entry is not reachable: discarding all its info.
            skip_at_depth = Some((0, depth_delta));
            continue;
        }

        let range = if entry.tag() == gimli::DW_TAG_subprogram {
            get_subprogram_range(entry, addr_tr)?
        } else {
            None
        };

        if depth_delta <= 0 {
            for _ in depth_delta..1 {
                stack.pop();
            }
        } else {
            assert!(depth_delta == 1);
        }
        let parent = stack.last().unwrap();
        let die_id = comp_unit.add(*parent, entry.tag());
        let current_scope = comp_unit.get_mut(die_id);

        stack.push(die_id);
        die_ref_map.insert(entry.offset(), die_id);

        clone_die_attributes(
            entry,
            context,
            addr_tr,
            &unit.encoding(),
            current_scope,
            die_id,
            range,
            out_strings,
            &die_ref_map,
            &mut pending_die_refs,
            FileAttributeContext::Children(&file_map),
        )?;
    }
    for (die_id, attr_name, offset) in pending_die_refs {
        let die = comp_unit.get_mut(die_id);
        // TODO we probably loosing DW_AT_abstract_origin and DW_AT_type references
        // here, find out if we drop stuff we don't need to.
        if let Some(unit_id) = die_ref_map.get(&offset) {
            die.set(attr_name, write::AttributeValue::ThisUnitEntryRef(*unit_id));
        }
    }
    Ok(())
}

pub fn transform_dwarf(
    target_config: &TargetFrontendConfig,
    di: &DebugInfoData,
    at: &ModuleAddressMap,
) -> Result<write::Dwarf, Error> {
    let addr_tr = AddressTransform::new(at, &di.wasm_file);
    let reachable = build_dependencies(&di.dwarf, &addr_tr)?.get_reachable();

    let context = DebugInputContext {
        debug_str: &di.dwarf.debug_str,
        debug_line: &di.dwarf.debug_line,
        debug_addr: &di.dwarf.debug_addr,
        debug_addr_base: DebugAddrBase(0),
        rnglists: &di.dwarf.ranges,
        loclists: &di.dwarf.locations,
        reachable: &reachable,
    };

    let out_encoding = gimli::Encoding {
        format: gimli::Format::Dwarf32,
        // TODO: this should be configurable
        // macOS doesn't seem to support DWARF > 3
        version: 3,
        address_size: target_config.pointer_bytes(),
    };

    let mut out_strings = write::StringTable::default();
    let mut out_units = write::UnitTable::default();

    let out_line_strings = write::LineStringTable::default();

    let mut iter = di.dwarf.debug_info.units();
    while let Some(unit) = iter.next().unwrap_or(None) {
        let unit = di.dwarf.unit(unit)?;
        clone_unit(
            unit,
            &context,
            &addr_tr,
            &out_encoding,
            &mut out_units,
            &mut out_strings,
        )?;
    }

    Ok(write::Dwarf {
        units: out_units,
        line_programs: vec![],
        line_strings: out_line_strings,
        strings: out_strings,
    })
}
