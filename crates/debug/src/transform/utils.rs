use super::address_transform::AddressTransform;
use super::expression::{CompiledExpression, FunctionFrameInfo};
use anyhow::Error;
use gimli::write;
use wasmtime_environ::wasm::DefinedFuncIndex;
use wasmtime_environ::{ModuleMemoryOffset, ModuleVmctxInfo, ValueLabelsRanges};

pub(crate) fn add_internal_types(
    comp_unit: &mut write::Unit,
    root_id: write::UnitEntryId,
    out_strings: &mut write::StringTable,
    module_info: &ModuleVmctxInfo,
) -> (write::UnitEntryId, write::UnitEntryId) {
    let wp_die_id = comp_unit.add(root_id, gimli::DW_TAG_base_type);
    let wp_die = comp_unit.get_mut(wp_die_id);
    wp_die.set(
        gimli::DW_AT_name,
        write::AttributeValue::StringRef(out_strings.add("WebAssemblyPtr")),
    );
    wp_die.set(gimli::DW_AT_byte_size, write::AttributeValue::Data1(4));
    wp_die.set(
        gimli::DW_AT_encoding,
        write::AttributeValue::Encoding(gimli::DW_ATE_unsigned),
    );

    let memory_byte_die_id = comp_unit.add(root_id, gimli::DW_TAG_base_type);
    let memory_byte_die = comp_unit.get_mut(memory_byte_die_id);
    memory_byte_die.set(
        gimli::DW_AT_name,
        write::AttributeValue::StringRef(out_strings.add("u8")),
    );
    memory_byte_die.set(
        gimli::DW_AT_encoding,
        write::AttributeValue::Encoding(gimli::DW_ATE_unsigned),
    );
    memory_byte_die.set(gimli::DW_AT_byte_size, write::AttributeValue::Data1(1));

    let memory_bytes_die_id = comp_unit.add(root_id, gimli::DW_TAG_pointer_type);
    let memory_bytes_die = comp_unit.get_mut(memory_bytes_die_id);
    memory_bytes_die.set(
        gimli::DW_AT_name,
        write::AttributeValue::StringRef(out_strings.add("u8*")),
    );
    memory_bytes_die.set(
        gimli::DW_AT_type,
        write::AttributeValue::ThisUnitEntryRef(memory_byte_die_id),
    );

    // Create artificial VMContext type and its reference for convinience viewing
    // its fields (such as memory ref) in a debugger.
    let vmctx_die_id = comp_unit.add(root_id, gimli::DW_TAG_structure_type);
    let vmctx_die = comp_unit.get_mut(vmctx_die_id);
    vmctx_die.set(
        gimli::DW_AT_name,
        write::AttributeValue::StringRef(out_strings.add("WasmtimeVMContext")),
    );

    match module_info.memory_offset {
        ModuleMemoryOffset::Defined(memory_offset) => {
            // The context has defined memory: extend the WasmtimeVMContext size
            // past the "memory" field.
            const MEMORY_FIELD_SIZE_PLUS_PADDING: u32 = 8;
            vmctx_die.set(
                gimli::DW_AT_byte_size,
                write::AttributeValue::Data4(memory_offset + MEMORY_FIELD_SIZE_PLUS_PADDING),
            );

            // Define the "memory" field which is a direct pointer to allocated Wasm memory.
            let m_die_id = comp_unit.add(vmctx_die_id, gimli::DW_TAG_member);
            let m_die = comp_unit.get_mut(m_die_id);
            m_die.set(
                gimli::DW_AT_name,
                write::AttributeValue::StringRef(out_strings.add("memory")),
            );
            m_die.set(
                gimli::DW_AT_type,
                write::AttributeValue::ThisUnitEntryRef(memory_bytes_die_id),
            );
            m_die.set(
                gimli::DW_AT_data_member_location,
                write::AttributeValue::Udata(memory_offset as u64),
            );
        }
        ModuleMemoryOffset::Imported(_) => {
            // TODO implement convinience pointer to and additional types for VMMemoryImport.
        }
        ModuleMemoryOffset::None => (),
    }

    let vmctx_ptr_die_id = comp_unit.add(root_id, gimli::DW_TAG_pointer_type);
    let vmctx_ptr_die = comp_unit.get_mut(vmctx_ptr_die_id);
    vmctx_ptr_die.set(
        gimli::DW_AT_name,
        write::AttributeValue::StringRef(out_strings.add("WasmtimeVMContext*")),
    );
    vmctx_ptr_die.set(
        gimli::DW_AT_type,
        write::AttributeValue::ThisUnitEntryRef(vmctx_die_id),
    );

    (wp_die_id, vmctx_ptr_die_id)
}

pub(crate) fn append_vmctx_info(
    comp_unit: &mut write::Unit,
    parent_id: write::UnitEntryId,
    vmctx_die_id: write::UnitEntryId,
    addr_tr: &AddressTransform,
    frame_info: Option<&FunctionFrameInfo>,
    scope_ranges: &[(u64, u64)],
    out_strings: &mut write::StringTable,
) -> Result<(), Error> {
    let loc = {
        let endian = gimli::RunTimeEndian::Little;

        let expr = CompiledExpression::vmctx();
        let mut locs = Vec::new();
        for (begin, length, data) in
            expr.build_with_locals(scope_ranges, addr_tr, frame_info, endian)
        {
            locs.push(write::Location::StartLength {
                begin,
                length,
                data,
            });
        }
        let list_id = comp_unit.locations.add(write::LocationList(locs));
        write::AttributeValue::LocationListRef(list_id)
    };

    let var_die_id = comp_unit.add(parent_id, gimli::DW_TAG_variable);
    let var_die = comp_unit.get_mut(var_die_id);
    var_die.set(
        gimli::DW_AT_name,
        write::AttributeValue::StringRef(out_strings.add("__vmctx")),
    );
    var_die.set(
        gimli::DW_AT_type,
        write::AttributeValue::ThisUnitEntryRef(vmctx_die_id),
    );
    var_die.set(gimli::DW_AT_location, loc);

    Ok(())
}

pub(crate) fn get_function_frame_info<'a, 'b, 'c>(
    module_info: &'b ModuleVmctxInfo,
    func_index: DefinedFuncIndex,
    value_ranges: &'c ValueLabelsRanges,
) -> Option<FunctionFrameInfo<'a>>
where
    'b: 'a,
    'c: 'a,
{
    if let Some(value_ranges) = value_ranges.get(func_index) {
        let frame_info = FunctionFrameInfo {
            value_ranges,
            memory_offset: module_info.memory_offset.clone(),
            stack_slots: &module_info.stack_slots[func_index],
        };
        Some(frame_info)
    } else {
        None
    }
}
