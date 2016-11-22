//! Data structures describing the registers in an ISA.

use std::fmt;

/// Register units are the smallest units of register allocation.
///
/// Normally there is a 1-1 correspondence between registers and register units, but when an ISA
/// has aliasing registers, the aliasing can be modeled with registers that cover multiple
/// register units.
///
/// The register allocator will enforce that each register unit only gets used for one thing.
pub type RegUnit = u16;

/// The register units in a target ISA are divided into disjoint register banks. Each bank covers a
/// contiguous range of register units.
///
/// The `RegBank` struct provides a static description of a register bank.
pub struct RegBank {
    /// The name of this register bank as defined in the ISA's `registers.py` file.
    pub name: &'static str,

    /// The first register unit in this bank.
    pub first_unit: RegUnit,

    /// The total number of register units in this bank.
    pub units: u16,

    /// Array of specially named register units. This array can be shorter than the number of units
    /// in the bank.
    pub names: &'static [&'static str],

    /// Name prefix to use for those register units in the bank not covered by the `names` array.
    /// The remaining register units will be named this prefix followed by their decimal offset in
    /// the bank. So with a prefix `r`, registers will be named `r8`, `r9`, ...
    pub prefix: &'static str,
}

impl RegBank {
    /// Does this bank contain `regunit`?
    fn contains(&self, regunit: RegUnit) -> bool {
        regunit >= self.first_unit && regunit - self.first_unit < self.units
    }

    /// Try to parse a regunit name. The name is not expected to begin with `%`.
    fn parse_regunit(&self, name: &str) -> Option<RegUnit> {
        match self.names.iter().position(|&x| x == name) {
                Some(offset) => {
                    // This is one of the special-cased names.
                    Some(offset as RegUnit)
                }
                None => {
                    // Try a regular prefixed name.
                    if name.starts_with(self.prefix) {
                        name[self.prefix.len()..].parse().ok()
                    } else {
                        None
                    }
                }
            }
            .and_then(|offset| {
                if offset < self.units {
                    Some(offset + self.first_unit)
                } else {
                    None
                }
            })
    }

    /// Write `regunit` to `w`, assuming that it belongs to this bank.
    /// All regunits are written with a `%` prefix.
    fn write_regunit(&self, f: &mut fmt::Formatter, regunit: RegUnit) -> fmt::Result {
        let offset = regunit - self.first_unit;
        assert!(offset < self.units);
        if (offset as usize) < self.names.len() {
            write!(f, "%{}", self.names[offset as usize])
        } else {
            write!(f, "%{}{}", self.prefix, offset)
        }
    }
}

/// Information about the registers in an ISA.
///
/// The `RegUnit` data structure collects all relevant static information about the registers in an
/// ISA.
pub struct RegInfo {
    /// All register banks, ordered by their `first_unit`. The register banks are disjoint, but
    /// there may be holes of unused register unit numbers between banks due to alignment.
    pub banks: &'static [RegBank],
}

impl RegInfo {
    /// Get the register bank holding `regunit`.
    pub fn bank_containing_regunit(&self, regunit: RegUnit) -> Option<&RegBank> {
        // We could do a binary search, but most ISAs have only two register banks...
        self.banks.iter().find(|b| b.contains(regunit))
    }

    /// Try to parse a regunit name. The name is not expected to begin with `%`.
    pub fn parse_regunit(&self, name: &str) -> Option<RegUnit> {
        self.banks.iter().filter_map(|b| b.parse_regunit(name)).next()
    }

    /// Make a temporary object that can display a register unit.
    pub fn display_regunit(&self, regunit: RegUnit) -> DisplayRegUnit {
        DisplayRegUnit {
            regunit: regunit,
            reginfo: self,
        }
    }
}

/// Temporary object that holds enough information to print a register unit.
pub struct DisplayRegUnit<'a> {
    pub regunit: RegUnit,
    pub reginfo: &'a RegInfo,
}

impl<'a> fmt::Display for DisplayRegUnit<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.reginfo.bank_containing_regunit(self.regunit) {
            Some(b) => b.write_regunit(f, self.regunit),
            None => write!(f, "%INVALID{}", self.regunit),
        }
    }
}
