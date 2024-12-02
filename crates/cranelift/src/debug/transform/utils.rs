use super::address_transform::AddressTransform;
use super::expression::{CompiledExpression, FunctionFrameInfo};
use anyhow::Error;
use cranelift_codegen::isa::TargetIsa;
use gimli::write;

pub(crate) fn append_vmctx_info(
    comp_unit: &mut write::Unit,
    parent_id: write::UnitEntryId,
    vmctx_ptr_die_ref: write::Reference,
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
        write::AttributeValue::DebugInfoRef(vmctx_ptr_die_ref),
    );
    var_die.set(gimli::DW_AT_location, loc);

    Ok(())
}
