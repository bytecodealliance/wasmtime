use crate::debug::Reader;
use crate::debug::transform::AddressTransform;
use gimli::constants;
use gimli::read;
use gimli::write;

pub fn build_dependencies(
    filter: &mut write::FilterUnitSection<'_, Reader<'_>>,
    at: &AddressTransform,
) -> write::ConvertResult<()> {
    while let Some(mut unit) = filter.read_unit()? {
        build_die_dependencies(&mut unit, at)?;
    }
    Ok(())
}

fn has_valid_code_range(
    die: &write::FilterUnitEntry<'_, Reader<'_>>,
    at: &AddressTransform,
) -> read::Result<bool> {
    let unit = die.read_unit;
    match die.tag {
        constants::DW_TAG_subprogram => {
            if let Some(ranges_attr) = die.attr_value(constants::DW_AT_ranges) {
                let offset = match ranges_attr {
                    read::AttributeValue::RangeListsRef(val) => unit.ranges_offset_from_raw(val),
                    read::AttributeValue::DebugRngListsIndex(index) => unit.ranges_offset(index)?,
                    _ => return Ok(false),
                };
                let mut has_valid_base = if let Some(read::AttributeValue::Addr(low_pc)) =
                    die.attr_value(constants::DW_AT_low_pc)
                {
                    Some(at.can_translate_address(low_pc))
                } else {
                    None
                };
                let mut it = unit.raw_ranges(offset)?;
                while let Some(range) = it.next()? {
                    // If at least one of the range addresses can be converted,
                    // declaring code range as valid.
                    match range {
                        read::RawRngListEntry::AddressOrOffsetPair { .. }
                            if has_valid_base.is_some() =>
                        {
                            if has_valid_base.unwrap() {
                                return Ok(true);
                            }
                        }
                        read::RawRngListEntry::StartEnd { begin, .. }
                        | read::RawRngListEntry::StartLength { begin, .. }
                        | read::RawRngListEntry::AddressOrOffsetPair { begin, .. } => {
                            if at.can_translate_address(begin) {
                                return Ok(true);
                            }
                        }
                        read::RawRngListEntry::StartxEndx { begin, .. }
                        | read::RawRngListEntry::StartxLength { begin, .. } => {
                            let addr = unit.address(begin)?;
                            if at.can_translate_address(addr) {
                                return Ok(true);
                            }
                        }
                        read::RawRngListEntry::BaseAddress { addr } => {
                            has_valid_base = Some(at.can_translate_address(addr));
                        }
                        read::RawRngListEntry::BaseAddressx { addr } => {
                            let addr = unit.address(addr)?;
                            has_valid_base = Some(at.can_translate_address(addr));
                        }
                        read::RawRngListEntry::OffsetPair { .. } => (),
                    }
                }
                return Ok(false);
            } else if let Some(low_pc) = die.attr_value(constants::DW_AT_low_pc) {
                if let read::AttributeValue::Addr(a) = low_pc {
                    return Ok(at.can_translate_address(a));
                } else if let read::AttributeValue::DebugAddrIndex(i) = low_pc {
                    let a = unit.address(i)?;
                    return Ok(at.can_translate_address(a));
                }
            }
        }
        _ => (),
    }
    Ok(false)
}

fn build_die_dependencies(
    unit: &mut write::FilterUnit<'_, Reader<'_>>,
    at: &AddressTransform,
) -> write::ConvertResult<()> {
    let mut die = write::FilterUnitEntry::null(unit.read_unit);
    while unit.read_entry(&mut die)? {
        if has_valid_code_range(&die, at)? {
            unit.require_entry(die.offset);
        }
    }
    Ok(())
}
