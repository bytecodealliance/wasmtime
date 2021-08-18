use super::address_transform::AddressTransform;
use super::expression::{CompiledExpression, FunctionFrameInfo};
use crate::CompiledFunctions;
use anyhow::Error;
use cranelift_codegen::isa::TargetIsa;
use gimli::write;
use wasmtime_environ::{DefinedFuncIndex, ModuleMemoryOffset};

/// Adds internal Wasm utility types DIEs such as WebAssemblyPtr and
/// WasmtimeVMContext.
///
/// For unwrapping Wasm pointer, the WasmtimeVMContext has the `set()` method
/// that allows to control current Wasm memory to inspect.
/// Notice that "set_vmctx_memory" is an external/builtin subprogram that
/// is not part of Wasm code.
pub(crate) fn add_internal_types(
    comp_unit: &mut write::Unit,
    root_id: write::UnitEntryId,
    out_strings: &mut write::StringTable,
    memory_offset: &ModuleMemoryOffset,
) -> (write::UnitEntryId, write::UnitEntryId) {
    const WASM_PTR_LEN: u8 = 4;

    macro_rules! add_tag {
        ($parent_id:ident, $tag:expr => $die:ident as $die_id:ident { $($a:path = $v:expr),* }) => {
            let $die_id = comp_unit.add($parent_id, $tag);
            let $die = comp_unit.get_mut($die_id);
            $( $die.set($a, $v); )*
        };
    }

    // Build DW_TAG_base_type for generic `WebAssemblyPtr`.
    //  .. DW_AT_name = "WebAssemblyPtr"
    //  .. DW_AT_byte_size = 4
    //  .. DW_AT_encoding = DW_ATE_unsigned
    // let wp_die_id = comp_unit.add(root_id, gimli::DW_TAG_base_type);
    // let wp_die = comp_unit.get_mut(wp_die_id);
    add_tag!(root_id, gimli::DW_TAG_base_type => wp_die as wp_die_id {
        gimli::DW_AT_name = write::AttributeValue::StringRef(out_strings.add("WebAssemblyPtr")),
        gimli::DW_AT_byte_size = write::AttributeValue::Data1(WASM_PTR_LEN),
        gimli::DW_AT_encoding = write::AttributeValue::Encoding(gimli::DW_ATE_unsigned)
    });

    // Build DW_TAG_base_type for Wasm byte:
    //  .. DW_AT_name = u8
    //  .. DW_AT_encoding = DW_ATE_unsigned
    //  .. DW_AT_byte_size = 1
    add_tag!(root_id, gimli::DW_TAG_base_type => memory_byte_die as memory_byte_die_id {
        gimli::DW_AT_name = write::AttributeValue::StringRef(out_strings.add("u8")),
        gimli::DW_AT_encoding = write::AttributeValue::Encoding(gimli::DW_ATE_unsigned),
        gimli::DW_AT_byte_size = write::AttributeValue::Data1(1)
    });

    // Build DW_TAG_pointer_type that references Wasm bytes:
    //  .. DW_AT_name = "u8*"
    //  .. DW_AT_type = <memory_byte_die>
    add_tag!(root_id, gimli::DW_TAG_pointer_type => memory_bytes_die as memory_bytes_die_id {
        gimli::DW_AT_name = write::AttributeValue::StringRef(out_strings.add("u8*")),
        gimli::DW_AT_type = write::AttributeValue::UnitRef(memory_byte_die_id)
    });

    // Create artificial VMContext type and its reference for convinience viewing
    // its fields (such as memory ref) in a debugger. Build DW_TAG_structure_type:
    //   .. DW_AT_name = "WasmtimeVMContext"
    let vmctx_die_id = comp_unit.add(root_id, gimli::DW_TAG_structure_type);
    let vmctx_die = comp_unit.get_mut(vmctx_die_id);
    vmctx_die.set(
        gimli::DW_AT_name,
        write::AttributeValue::StringRef(out_strings.add("WasmtimeVMContext")),
    );

    // TODO multiple memories
    match *memory_offset {
        ModuleMemoryOffset::Defined(memory_offset) => {
            // The context has defined memory: extend the WasmtimeVMContext size
            // past the "memory" field.
            const MEMORY_FIELD_SIZE_PLUS_PADDING: u32 = 8;
            vmctx_die.set(
                gimli::DW_AT_byte_size,
                write::AttributeValue::Data4(memory_offset + MEMORY_FIELD_SIZE_PLUS_PADDING),
            );

            // Define the "memory" field which is a direct pointer to allocated Wasm memory.
            // Build DW_TAG_member:
            //  .. DW_AT_name = "memory"
            //  .. DW_AT_type = <memory_bytes_die>
            //  .. DW_AT_data_member_location = `memory_offset`
            add_tag!(vmctx_die_id, gimli::DW_TAG_member => m_die as m_die_id {
                gimli::DW_AT_name = write::AttributeValue::StringRef(out_strings.add("memory")),
                gimli::DW_AT_type = write::AttributeValue::UnitRef(memory_bytes_die_id),
                gimli::DW_AT_data_member_location = write::AttributeValue::Udata(memory_offset as u64)
            });
        }
        ModuleMemoryOffset::Imported(_) => {
            // TODO implement convinience pointer to and additional types for VMMemoryImport.
        }
        ModuleMemoryOffset::None => (),
    }

    // Build DW_TAG_pointer_type for `WasmtimeVMContext*`:
    //  .. DW_AT_name = "WasmtimeVMContext*"
    //  .. DW_AT_type = <vmctx_die>
    add_tag!(root_id, gimli::DW_TAG_pointer_type => vmctx_ptr_die as vmctx_ptr_die_id {
        gimli::DW_AT_name = write::AttributeValue::StringRef(out_strings.add("WasmtimeVMContext*")),
        gimli::DW_AT_type = write::AttributeValue::UnitRef(vmctx_die_id)
    });

    // Build vmctx_die's DW_TAG_subprogram for `set` method:
    //  .. DW_AT_linkage_name = "set_vmctx_memory"
    //  .. DW_AT_name = "set"
    //  .. DW_TAG_formal_parameter
    //  ..  .. DW_AT_type = <vmctx_ptr_die>
    //  ..  .. DW_AT_artificial = 1
    add_tag!(vmctx_die_id, gimli::DW_TAG_subprogram => vmctx_set as vmctx_set_id {
        gimli::DW_AT_linkage_name = write::AttributeValue::StringRef(out_strings.add("set_vmctx_memory")),
        gimli::DW_AT_name = write::AttributeValue::StringRef(out_strings.add("set"))
    });
    add_tag!(vmctx_set_id, gimli::DW_TAG_formal_parameter => vmctx_set_this_param as vmctx_set_this_param_id {
        gimli::DW_AT_type = write::AttributeValue::UnitRef(vmctx_ptr_die_id),
        gimli::DW_AT_artificial = write::AttributeValue::Flag(true)
    });

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
    isa: &dyn TargetIsa,
) -> Result<(), Error> {
    let loc = {
        let expr = CompiledExpression::vmctx();
        let locs = expr
            .build_with_locals(scope_ranges, addr_tr, frame_info, isa)
            .map(|i| {
                i.map(|(begin, length, data)| write::Location::StartLength {
                    begin,
                    length,
                    data,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
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
        write::AttributeValue::UnitRef(vmctx_die_id),
    );
    var_die.set(gimli::DW_AT_location, loc);

    Ok(())
}

pub(crate) fn get_function_frame_info<'a, 'b, 'c>(
    memory_offset: &ModuleMemoryOffset,
    funcs: &'b CompiledFunctions,
    func_index: DefinedFuncIndex,
) -> Option<FunctionFrameInfo<'a>>
where
    'b: 'a,
    'c: 'a,
{
    if let Some(func) = funcs.get(func_index) {
        let frame_info = FunctionFrameInfo {
            value_ranges: &func.value_labels_ranges,
            memory_offset: memory_offset.clone(),
            stack_slots: &func.stack_slots,
        };
        Some(frame_info)
    } else {
        None
    }
}
