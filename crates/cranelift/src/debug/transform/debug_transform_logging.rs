use crate::debug::Reader;
use core::fmt;
use gimli::{
    write, AttributeValue, DebuggingInformationEntry, Dwarf, LittleEndian, Unit, UnitSectionOffset,
};

macro_rules! dbi_log {
    ($($tt:tt)*) => {
        if cfg!(any(feature = "trace-log", debug_assertions)) {
            ::log::trace!(target: "debug-info-transform", $($tt)*);
        }
    };
}
pub(crate) use dbi_log;

pub struct CompileUnitSummary<'a> {
    unit: &'a Unit<Reader<'a>, usize>,
}

impl<'a> fmt::Debug for CompileUnitSummary<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let unit = self.unit;
        let offs = get_offset_value(unit.header.offset());
        write!(f, "0x{offs:08x} [")?;
        let comp_dir = match unit.comp_dir {
            Some(dir) => &dir.to_string_lossy(),
            None => "None",
        };
        write!(f, "\"{comp_dir}\"")?;
        let name = match unit.name {
            Some(name) => &name.to_string_lossy(),
            None => "None",
        };
        write!(f, ", \"{name}\"]")
    }
}

pub fn log_get_cu_summary<'a>(unit: &'a Unit<Reader<'a>, usize>) -> CompileUnitSummary<'a> {
    CompileUnitSummary { unit }
}

struct DieDetailedSummary<'a> {
    dwarf: &'a Dwarf<Reader<'a>>,
    unit: &'a Unit<Reader<'a>, usize>,
    die: &'a DebuggingInformationEntry<'a, 'a, Reader<'a>>,
}

pub fn log_begin_input_die(
    dwarf: &Dwarf<Reader<'_>>,
    unit: &Unit<Reader<'_>, usize>,
    die: &DebuggingInformationEntry<Reader<'_>>,
    depth: isize,
) {
    dbi_log!(
        "=== Begin DIE at 0x{:08x} (depth = {}):\n{:?}",
        get_offset_value(die.offset().to_unit_section_offset(unit)),
        depth,
        DieDetailedSummary { dwarf, unit, die }
    );
}

impl<'a> fmt::Debug for DieDetailedSummary<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let die = self.die;
        let unit = self.unit;
        let dwarf = self.dwarf;
        write!(f, "{}\n", die.tag())?;

        let mut attrs = die.attrs();
        while let Some(attr) = attrs.next().unwrap_or(None) {
            write!(f, "  {} (", attr.name())?;
            let attr_value = attr.value();
            match attr_value {
                AttributeValue::Addr(addr) => {
                    write!(f, "{addr:08x}")
                }
                AttributeValue::DebugAddrIndex(index) => {
                    if let Some(addr) = dwarf.address(unit, index).ok() {
                        write!(f, "{addr:08x}")
                    } else {
                        write!(f, "<error reading address at index: {}>", index.0)
                    }
                }
                AttributeValue::Block(d) => write!(f, "{d:?}"),
                AttributeValue::Udata(d) => write!(f, "{d}"),
                AttributeValue::Data1(d) => write!(f, "{d}"),
                AttributeValue::Data2(d) => write!(f, "{d}"),
                AttributeValue::Data4(d) => write!(f, "{d}"),
                AttributeValue::Data8(d) => write!(f, "{d}"),
                AttributeValue::Sdata(d) => write!(f, "{d}"),
                AttributeValue::Flag(d) => write!(f, "{d}"),
                AttributeValue::DebugLineRef(offset) => write!(f, "0x{:08x}", offset.0),
                AttributeValue::FileIndex(index) => write!(f, "0x{index:08x}"),
                AttributeValue::String(_)
                | AttributeValue::DebugStrRef(_)
                | AttributeValue::DebugStrOffsetsIndex(_) => {
                    if let Ok(s) = dwarf.attr_string(unit, attr_value) {
                        write!(f, "\"{}\"", &s.to_string_lossy())
                    } else {
                        write!(f, "<error reading string>")
                    }
                }
                AttributeValue::RangeListsRef(_) | AttributeValue::DebugRngListsIndex(_) => {
                    let _ = dwarf.attr_ranges_offset(unit, attr_value);
                    write!(f, "<TODO: rnglist dump>")
                }
                AttributeValue::LocationListsRef(_) | AttributeValue::DebugLocListsIndex(_) => {
                    let _ = dwarf.attr_locations_offset(unit, attr_value);
                    write!(f, "<TODO: loclist dump>")
                }
                AttributeValue::Exprloc(_) => {
                    write!(f, "<TODO: exprloc dump>")
                }
                AttributeValue::Encoding(value) => write!(f, "{value}"),
                AttributeValue::DecimalSign(value) => write!(f, "{value}"),
                AttributeValue::Endianity(value) => write!(f, "{value}"),
                AttributeValue::Accessibility(value) => write!(f, "{value}"),
                AttributeValue::Visibility(value) => write!(f, "{value}"),
                AttributeValue::Virtuality(value) => write!(f, "{value}"),
                AttributeValue::Language(value) => write!(f, "{value}"),
                AttributeValue::AddressClass(value) => write!(f, "{value}"),
                AttributeValue::IdentifierCase(value) => write!(f, "{value}"),
                AttributeValue::CallingConvention(value) => write!(f, "{value}"),
                AttributeValue::Inline(value) => write!(f, "{value}"),
                AttributeValue::Ordering(value) => write!(f, "{value}"),
                AttributeValue::UnitRef(offset) => write!(f, "0x{:08x}", offset.0),
                AttributeValue::DebugInfoRef(offset) => write!(f, "0x{:08x}", offset.0),
                unexpected_attr => write!(f, "<unexpected attr: {unexpected_attr:?}>"),
            }?;
            write!(f, ")\n")?;
        }
        Ok(())
    }
}

struct OutDieDetailedSummary<'a> {
    die_id: write::UnitEntryId,
    unit: &'a write::Unit,
    strings: &'a write::StringTable,
}

impl<'a> fmt::Debug for OutDieDetailedSummary<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let die = self.unit.get(self.die_id);
        write!(f, "{}\n", die.tag())?;
        for attr in die.attrs() {
            write!(f, "  {} (", attr.name())?;
            let attr_value = attr.get();
            match attr_value {
                write::AttributeValue::Address(addr) => match addr {
                    write::Address::Constant(addr_value) => write!(f, "{addr_value:08x}"),
                    write::Address::Symbol { symbol, addend } => {
                        write!(f, "symbol #{symbol}+{addend}")
                    }
                },
                write::AttributeValue::Block(d) => {
                    write!(f, "{:?}", Reader::new(d.as_slice(), LittleEndian))
                }
                write::AttributeValue::Udata(d) => write!(f, "{d}"),
                write::AttributeValue::Data1(d) => write!(f, "{d}"),
                write::AttributeValue::Data2(d) => write!(f, "{d}"),
                write::AttributeValue::Data4(d) => write!(f, "{d}"),
                write::AttributeValue::Data8(d) => write!(f, "{d}"),
                write::AttributeValue::Sdata(d) => write!(f, "{d}"),
                write::AttributeValue::Flag(d) => write!(f, "{d}"),
                write::AttributeValue::LineProgramRef => write!(f, "LineProgramRef"),
                write::AttributeValue::FileIndex(index) => match index {
                    Some(id) => write!(f, "{id:?}"),
                    None => write!(f, "<file index missing>"),
                },
                write::AttributeValue::String(s) => {
                    write!(f, "\"{}\"", &String::from_utf8_lossy(s))
                }
                write::AttributeValue::StringRef(id) => {
                    write!(f, "\"{}\"", &String::from_utf8_lossy(self.strings.get(*id)))
                }
                write::AttributeValue::RangeListRef(_) => {
                    write!(f, "<TODO: out rnglist dump>")
                }
                write::AttributeValue::LocationListRef(_) => {
                    write!(f, "<TODO: out loclist dump>")
                }
                write::AttributeValue::Exprloc(_) => {
                    write!(f, "<TODO: out exprloc dump>")
                }
                write::AttributeValue::Encoding(value) => write!(f, "{value}"),
                write::AttributeValue::DecimalSign(value) => write!(f, "{value}"),
                write::AttributeValue::Endianity(value) => write!(f, "{value}"),
                write::AttributeValue::Accessibility(value) => write!(f, "{value}"),
                write::AttributeValue::Visibility(value) => write!(f, "{value}"),
                write::AttributeValue::Virtuality(value) => write!(f, "{value}"),
                write::AttributeValue::Language(value) => write!(f, "{value}"),
                write::AttributeValue::AddressClass(value) => write!(f, "{value}"),
                write::AttributeValue::IdentifierCase(value) => write!(f, "{value}"),
                write::AttributeValue::CallingConvention(value) => write!(f, "{value}"),
                write::AttributeValue::Inline(value) => write!(f, "{value}"),
                write::AttributeValue::Ordering(value) => write!(f, "{value}"),
                write::AttributeValue::UnitRef(unit_ref) => write!(f, "{unit_ref:?}>"),
                write::AttributeValue::DebugInfoRef(reference) => match reference {
                    write::Reference::Symbol(index) => write!(f, "symbol #{index}>"),
                    write::Reference::Entry(unit_id, die_id) => {
                        write!(f, "{die_id:?} in {unit_id:?}>")
                    }
                },
                unexpected_attr => write!(f, "<unexpected attr: {unexpected_attr:?}>"),
            }?;
            write!(f, ")\n")?;
        }
        Ok(())
    }
}

pub fn log_end_output_die(
    input_die: &DebuggingInformationEntry<Reader<'_>>,
    input_unit: &Unit<Reader<'_>, usize>,
    die_id: write::UnitEntryId,
    unit: &write::Unit,
    strings: &write::StringTable,
    depth: isize,
) {
    dbi_log!(
        "=== End DIE at 0x{:08x} (depth = {}):\n{:?}",
        get_offset_value(input_die.offset().to_unit_section_offset(input_unit)),
        depth,
        OutDieDetailedSummary {
            die_id,
            unit,
            strings
        }
    );
}

pub fn log_end_output_die_skipped(
    input_die: &DebuggingInformationEntry<Reader<'_>>,
    input_unit: &Unit<Reader<'_>, usize>,
    reason: &str,
    depth: isize,
) {
    dbi_log!(
        "=== End DIE at 0x{:08x} (depth = {}):\n  Skipped as {}\n",
        get_offset_value(input_die.offset().to_unit_section_offset(input_unit)),
        depth,
        reason
    );
}

fn get_offset_value(offset: UnitSectionOffset) -> usize {
    match offset {
        UnitSectionOffset::DebugInfoOffset(offs) => offs.0,
        UnitSectionOffset::DebugTypesOffset(offs) => offs.0,
    }
}
