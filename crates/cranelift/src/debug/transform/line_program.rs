use super::address_transform::AddressTransform;
use super::attr::clone_attr_string;
use super::{Reader, TransformError};
use anyhow::{Context, Error};
use gimli::{
    write, AttributeValue::DebugLineRef, DebugLine, DebugLineOffset, DebugStr,
    DebuggingInformationEntry, LineEncoding, Unit,
};
use wasmtime_environ::DefinedFuncIndex;

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
    EndOfSequence,
}

#[derive(Debug)]
struct FuncRows {
    index: DefinedFuncIndex,
    sorted_rows: Vec<(u64, SavedLineProgramRow)>,
}

#[derive(Debug, Eq, PartialEq)]
enum ReadLineProgramState {
    SequenceEnded,
    ReadSequence(DefinedFuncIndex),
    IgnoreSequence,
}

pub(crate) fn clone_line_program<R>(
    dwarf: &gimli::Dwarf<R>,
    skeleton_dwarf: &gimli::Dwarf<R>,
    unit: &Unit<R, R::Offset>,
    root: &DebuggingInformationEntry<R>,
    skeleton_die: Option<&DebuggingInformationEntry<R>>,
    addr_tr: &AddressTransform,
    out_encoding: gimli::Encoding,
    debug_str: &DebugStr<R>,
    debug_line: &DebugLine<R>,
    out_strings: &mut write::StringTable,
) -> Result<(write::LineProgram, DebugLineOffset, Vec<write::FileId>, u64), Error>
where
    R: Reader,
{
    // Where are the "location" attributes
    let (location_die, location_dwarf) = match skeleton_die {
        Some(die) => (die, skeleton_dwarf),
        _ => (root, dwarf),
    };

    let offset = match location_die.attr_value(gimli::DW_AT_stmt_list)? {
        Some(DebugLineRef(offset)) => offset,
        Some(gimli::AttributeValue::SecOffset(offset)) => DebugLineOffset(offset),
        _ => {
            return Err(TransformError("Debug line offset is not found").into());
        }
    };
    let comp_dir = location_die.attr_value(gimli::DW_AT_comp_dir)?;
    let comp_name = root.attr_value(gimli::DW_AT_name)?;
    let out_comp_dir = match &comp_dir {
        Some(comp_dir) => Some(clone_attr_string(
            comp_dir,
            gimli::DW_FORM_strp,
            unit,
            location_dwarf,
            out_strings,
        )?),
        None => None,
    };
    let out_comp_name = match comp_name {
        Some(_) => clone_attr_string(
            comp_name
                .as_ref()
                .context("failed to read DW_AT_name attribute")?,
            gimli::DW_FORM_strp,
            unit,
            dwarf,
            out_strings,
        )?,
        _ => gimli::write::LineString::String("missing DW_AT_name attribute".into()),
    };

    let program = debug_line.program(
        offset,
        unit.header.address_size(),
        comp_dir.and_then(|val| val.string_value(&debug_str)),
        comp_name.and_then(|val| val.string_value(&debug_str)),
    );
    if let Ok(program) = program {
        let header = program.header();
        let file_index_base = if header.version() < 5 { 1 } else { 0 };
        assert!(header.version() <= 5, "not supported 6");
        let line_encoding = LineEncoding {
            minimum_instruction_length: header.minimum_instruction_length(),
            maximum_operations_per_instruction: header.maximum_operations_per_instruction(),
            default_is_stmt: header.default_is_stmt(),
            line_base: header.line_base(),
            line_range: header.line_range(),
        };
        let mut out_program = write::LineProgram::new(
            out_encoding,
            line_encoding,
            out_comp_dir.unwrap_or_else(|| write::LineString::String(Vec::new())),
            out_comp_name,
            None,
        );
        let mut dirs = Vec::new();
        dirs.push(out_program.default_directory());
        for dir_attr in header.include_directories() {
            let dir_id = out_program.add_directory(clone_attr_string(
                dir_attr,
                gimli::DW_FORM_string,
                unit,
                location_dwarf,
                out_strings,
            )?);
            dirs.push(dir_id);
        }
        let mut files = Vec::new();
        // Since we are outputting DWARF-4, perform base change.
        let directory_index_correction = if header.version() >= 5 { 1 } else { 0 };
        for file_entry in header.file_names() {
            let dir_index = file_entry.directory_index() + directory_index_correction;
            let dir_id = dirs[dir_index as usize];
            let file_id = out_program.add_file(
                clone_attr_string(
                    &file_entry.path_name(),
                    gimli::DW_FORM_string,
                    unit,
                    location_dwarf,
                    out_strings,
                )?,
                dir_id,
                None,
            );
            files.push(file_id);
        }

        let mut rows = program.rows();
        let mut func_rows = Vec::new();
        let mut saved_rows: Vec<(u64, SavedLineProgramRow)> = Vec::new();
        let mut state = ReadLineProgramState::SequenceEnded;
        while let Some((_header, row)) = rows.next_row()? {
            if state == ReadLineProgramState::IgnoreSequence {
                if row.end_sequence() {
                    state = ReadLineProgramState::SequenceEnded;
                }
                continue;
            }
            let saved_row = if row.end_sequence() {
                let index = match state {
                    ReadLineProgramState::ReadSequence(index) => index,
                    _ => panic!(),
                };
                saved_rows.sort_by_key(|r| r.0);
                func_rows.push(FuncRows {
                    index,
                    sorted_rows: saved_rows,
                });

                saved_rows = Vec::new();
                state = ReadLineProgramState::SequenceEnded;
                SavedLineProgramRow::EndOfSequence
            } else {
                if state == ReadLineProgramState::SequenceEnded {
                    // Discard sequences for non-existent code.
                    if row.address() == 0 {
                        state = ReadLineProgramState::IgnoreSequence;
                        continue;
                    }
                    match addr_tr.find_func_index(row.address()) {
                        Some(index) => {
                            state = ReadLineProgramState::ReadSequence(index);
                        }
                        None => {
                            // Some non-existent address found.
                            state = ReadLineProgramState::IgnoreSequence;
                            continue;
                        }
                    }
                }
                SavedLineProgramRow::Normal {
                    address: row.address(),
                    op_index: row.op_index(),
                    file_index: row.file_index(),
                    line: row.line().map(|nonzero| nonzero.get()).unwrap_or(0),
                    column: match row.column() {
                        gimli::ColumnType::LeftEdge => 0,
                        gimli::ColumnType::Column(val) => val.get(),
                    },
                    discriminator: row.discriminator(),
                    is_stmt: row.is_stmt(),
                    basic_block: row.basic_block(),
                    prologue_end: row.prologue_end(),
                    epilogue_begin: row.epilogue_begin(),
                    isa: row.isa(),
                }
            };
            saved_rows.push((row.address(), saved_row));
        }

        for FuncRows {
            index,
            sorted_rows: saved_rows,
        } in func_rows
        {
            let map = match addr_tr.map().get(index) {
                Some(map) if map.len > 0 => map,
                _ => {
                    continue; // no code generated
                }
            };
            let symbol = map.symbol;
            let base_addr = map.offset;
            out_program.begin_sequence(Some(write::Address::Symbol { symbol, addend: 0 }));
            // TODO track and place function declaration line here
            let mut last_address = None;
            for addr_map in map.addresses.iter() {
                let saved_row = match saved_rows.binary_search_by_key(&addr_map.wasm, |i| i.0) {
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
                        out_program.row().file = files[(file_index - file_index_base) as usize];
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
            let end_addr = (map.offset + map.len) as u64;
            out_program.end_sequence(end_addr);
        }
        Ok((out_program, offset, files, file_index_base))
    } else {
        Err(TransformError("Valid line program not found").into())
    }
}
