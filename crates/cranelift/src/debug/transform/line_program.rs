use super::address_transform::AddressTransform;
use crate::debug::Reader;
use gimli::write;
use wasmtime_environ::error::Error;

pub(crate) fn clone_line_program(
    mut transform: write::ConvertLineProgram<'_, Reader<'_>>,
    addr_tr: &AddressTransform,
) -> Result<(write::LineProgram, Vec<write::FileId>), Error> {
    while let Some(write::ConvertLineSequence {
        start,
        rows: saved_rows,
        ..
    }) = transform.read_sequence()?
    {
        let Some(start) = start else {
            continue;
        };
        if start == 0 {
            continue;
        }
        let Some(index) = addr_tr.find_func_index(start) else {
            // Some non-existent address found.
            continue;
        };
        let Some(map) = addr_tr.map().get(index) else {
            continue; // no code generated
        };
        let symbol = map.symbol;
        let base_addr = map.offset;
        transform.begin_sequence(Some(write::Address::Symbol { symbol, addend: 0 }));
        // TODO track and place function declaration line here
        let mut last_address = None;
        for addr_map in map.addresses.iter() {
            let Some(wasm_offset) = addr_map.wasm.checked_sub(start) else {
                continue;
            };
            let mut saved_row =
                match saved_rows.binary_search_by_key(&wasm_offset, |i| i.address_offset) {
                    Ok(i) => saved_rows[i],
                    Err(i) => {
                        if i > 0 {
                            saved_rows[i - 1]
                        } else {
                            continue;
                        }
                    }
                };
            // Ignore duplicates
            if Some(saved_row.address_offset) != last_address {
                let address_offset = if last_address.is_none() {
                    // Extend first entry to the function declaration
                    // TODO use the function declaration line instead
                    0
                } else {
                    (addr_map.generated - base_addr) as u64
                };
                last_address = Some(saved_row.address_offset);
                saved_row.address_offset = address_offset;
                transform.generate_row(saved_row);
            }
        }
        transform.end_sequence(map.len as u64);
    }
    let (out_program, files) = transform.program();
    Ok((out_program, files))
}
