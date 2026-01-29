use crate::{debug::Reader, translate::get_vmctx_value_label};
use core::fmt;
use cranelift_codegen::{LabelValueLoc, ValueLabelsRanges, ir::ValueLabel, isa::TargetIsa};
use gimli::{AttributeValue, LittleEndian, UnitRef, write};

macro_rules! dbi_log_enabled {
    () => {
        cfg!(any(feature = "trace-log", debug_assertions))
            && ::log::log_enabled!(target: "debug-info-transform", ::log::Level::Trace)
    };
}
macro_rules! dbi_log {
    ($($tt:tt)*) => {
        if cfg!(any(feature = "trace-log", debug_assertions)) {
            ::log::trace!(target: "debug-info-transform", $($tt)*);
        }
    };
}
pub(crate) use dbi_log;
pub(crate) use dbi_log_enabled;

pub struct CompileUnitSummary<'a, 'r> {
    unit: UnitRef<'a, Reader<'r>>,
}

impl<'a, 'r> fmt::Debug for CompileUnitSummary<'a, 'r> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let unit = self.unit;
        write!(f, "0x{:08x} [", unit.header.offset().0)?;
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

pub fn log_get_cu_summary<'a, 'r>(unit: UnitRef<'a, Reader<'r>>) -> CompileUnitSummary<'a, 'r> {
    CompileUnitSummary { unit }
}

pub struct DieRefSummary<'a, 'r> {
    entry: &'a write::ConvertUnitEntry<'a, Reader<'r>>,
}

impl<'a, 'r> fmt::Debug for DieRefSummary<'a, 'r> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let section_offs = self
            .entry
            .offset
            .to_unit_section_offset(&self.entry.read_unit);
        write!(f, "0x{:08x}", section_offs.0)
    }
}

pub fn log_get_die_ref<'a, 'r>(
    entry: &'a write::ConvertUnitEntry<'a, Reader<'r>>,
) -> DieRefSummary<'a, 'r> {
    DieRefSummary { entry }
}

struct DieDetailedSummary<'a, 'r> {
    entry: &'a write::ConvertUnitEntry<'a, Reader<'r>>,
}

pub fn log_begin_input_die<'a, 'r>(entry: &'a write::ConvertUnitEntry<'a, Reader<'r>>) {
    dbi_log!(
        "=== Begin DIE at {:?} (depth = {}):\n{:?}",
        log_get_die_ref(entry),
        entry.depth,
        DieDetailedSummary { entry }
    );
}

impl<'a, 'r> fmt::Debug for DieDetailedSummary<'a, 'r> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let unit = self.entry.read_unit;
        write!(f, "{}\n", self.entry.tag)?;

        for attr in &self.entry.attrs {
            write!(f, "  {} (", attr.name())?;
            let attr_value = attr.value();
            match attr_value {
                AttributeValue::Addr(addr) => {
                    write!(f, "{addr:08x}")
                }
                AttributeValue::DebugAddrIndex(index) => {
                    if let Some(addr) = unit.address(index).ok() {
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
                    if let Ok(s) = unit.attr_string(attr_value) {
                        write!(f, "\"{}\"", &s.to_string_lossy())
                    } else {
                        write!(f, "<error reading string>")
                    }
                }
                AttributeValue::RangeListsRef(_) | AttributeValue::DebugRngListsIndex(_) => {
                    let _ = unit.attr_ranges_offset(attr_value);
                    write!(f, "<TODO: rnglist dump>")
                }
                AttributeValue::LocationListsRef(_) | AttributeValue::DebugLocListsIndex(_) => {
                    let _ = unit.attr_locations_offset(attr_value);
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
                AttributeValue::UnitRef(offset) => {
                    write!(f, "0x{:08x}", offset.to_unit_section_offset(&unit).0)
                }
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
                    write::DebugInfoRef::Symbol(index) => write!(f, "symbol #{index}>"),
                    write::DebugInfoRef::Entry(unit_id, die_id) => {
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
    die_id: write::UnitEntryId,
    entry: &write::ConvertUnitEntry<'_, Reader<'_>>,
    unit: &write::ConvertUnit<'_, Reader<'_>>,
) {
    dbi_log!(
        "=== End DIE at {:?} (depth = {}):\n{:?}",
        log_get_die_ref(entry),
        entry.depth,
        OutDieDetailedSummary {
            die_id,
            unit: &unit.unit,
            strings: &unit.strings,
        }
    );
}

pub fn log_end_output_die_skipped(entry: &write::ConvertUnitEntry<'_, Reader<'_>>, reason: &str) {
    dbi_log!(
        "=== End DIE at {:?} (depth = {}):\n  Skipped as {}\n",
        log_get_die_ref(entry),
        entry.depth,
        reason
    );
}

pub fn log_get_value_name(value: ValueLabel) -> ValueNameSummary {
    ValueNameSummary { value }
}

pub struct ValueNameSummary {
    value: ValueLabel,
}

impl fmt::Debug for ValueNameSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.value == get_vmctx_value_label() {
            f.pad("VMCTX")
        } else {
            f.pad(&format!("L#{}", self.value.as_u32()))
        }
    }
}

pub fn log_get_value_loc(loc: LabelValueLoc, isa: &dyn TargetIsa) -> ValueLocSummary<'_> {
    ValueLocSummary { loc, isa }
}

pub struct ValueLocSummary<'a> {
    loc: LabelValueLoc,
    isa: &'a dyn TargetIsa,
}

impl<'a> fmt::Debug for ValueLocSummary<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let LabelValueLoc::Reg(reg) = self.loc {
            let reg_name = self.isa.pretty_print_reg(reg, self.isa.pointer_bytes());
            return write!(f, "{reg_name}");
        }

        write!(f, "{:?}", self.loc)
    }
}

pub fn log_get_value_ranges<'a>(
    ranges: Option<&'a ValueLabelsRanges>,
    isa: &'a dyn TargetIsa,
) -> ValueRangesSummary<'a> {
    ValueRangesSummary { ranges, isa }
}

pub struct ValueRangesSummary<'a> {
    ranges: Option<&'a ValueLabelsRanges>,
    isa: &'a dyn TargetIsa,
}

impl<'a> fmt::Debug for ValueRangesSummary<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ranges) = self.ranges {
            // Sort the output first for nicer display.
            let mut locals = Vec::new();
            for value in ranges {
                locals.push(*value.0);
            }
            locals.sort_by_key(|n| n.as_u32());

            for i in 0..locals.len() {
                let name = locals[i];
                write!(f, "{:<6?}:", log_get_value_name(name))?;
                for range in ranges.get(&name).unwrap() {
                    write!(f, " {:?}", log_get_value_loc(range.loc, self.isa))?;
                    write!(f, "@[{}..{})", range.start, range.end)?;
                }
                if i != locals.len() - 1 {
                    writeln!(f)?;
                }
            }
        }
        Ok(())
    }
}
