//! Data structures describing the registers in an ISA.

use crate::entity::EntityRef;
use core::fmt;

/// Register units are the smallest units of register allocation.
///
/// Normally there is a 1-1 correspondence between registers and register units, but when an ISA
/// has aliasing registers, the aliasing can be modeled with registers that cover multiple
/// register units.
///
/// The register allocator will enforce that each register unit only gets used for one thing.
pub type RegUnit = u16;

/// A bit mask indexed by register units.
///
/// The size of this type is determined by the target ISA that has the most register units defined.
/// Currently that is arm32 which has 64+16 units.
///
/// This type should be coordinated with meta/src/cdsl/regs.rs.
pub type RegUnitMask = [u32; 3];

/// A bit mask indexed by register classes.
///
/// The size of this type is determined by the ISA with the most register classes.
///
/// This type should be coordinated with meta/src/cdsl/regs.rs.
pub type RegClassMask = u32;

/// Guaranteed maximum number of top-level register classes with pressure tracking in any ISA.
///
/// This can be increased, but should be coordinated with meta/src/cdsl/regs.rs.
pub const MAX_TRACKED_TOPRCS: usize = 4;

/// The register units in a target ISA are divided into disjoint register banks. Each bank covers a
/// contiguous range of register units.
///
/// The `RegBank` struct provides a static description of a register bank.
pub struct RegBank {
    /// The name of this register bank as defined in the ISA's DSL definition.
    pub name: &'static str,

    /// The first register unit in this bank.
    pub first_unit: RegUnit,

    /// The total number of register units in this bank.
    pub units: RegUnit,

    /// Array of specially named register units. This array can be shorter than the number of units
    /// in the bank.
    pub names: &'static [&'static str],

    /// Name prefix to use for those register units in the bank not covered by the `names` array.
    /// The remaining register units will be named this prefix followed by their decimal offset in
    /// the bank. So with a prefix `r`, registers will be named `r8`, `r9`, ...
    pub prefix: &'static str,

    /// Index of the first top-level register class in this bank.
    pub first_toprc: usize,

    /// Number of top-level register classes in this bank.
    ///
    /// The top-level register classes in a bank are guaranteed to be numbered sequentially from
    /// `first_toprc`, and all top-level register classes across banks come before any sub-classes.
    pub num_toprcs: usize,

    /// Is register pressure tracking enabled for this bank?
    pub pressure_tracking: bool,
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

/// A register class reference.
///
/// All register classes are statically defined in tables generated from the meta descriptions.
pub type RegClass = &'static RegClassData;

/// Data about a register class.
///
/// A register class represents a subset of the registers in a bank. It describes the set of
/// permitted registers for a register operand in a given encoding of an instruction.
///
/// A register class can be a subset of another register class. The top-level register classes are
/// disjoint.
pub struct RegClassData {
    /// The name of the register class.
    pub name: &'static str,

    /// The index of this class in the ISA's RegInfo description.
    pub index: u8,

    /// How many register units to allocate per register.
    pub width: u8,

    /// Index of the register bank this class belongs to.
    pub bank: u8,

    /// Index of the top-level register class contains this one.
    pub toprc: u8,

    /// The first register unit in this class.
    pub first: RegUnit,

    /// Bit-mask of sub-classes of this register class, including itself.
    ///
    /// Bits correspond to RC indexes.
    pub subclasses: RegClassMask,

    /// Mask of register units in the class. If `width > 1`, the mask only has a bit set for the
    /// first register unit in each allocatable register.
    pub mask: RegUnitMask,

    /// The global `RegInfo` instance containing this register class.
    pub info: &'static RegInfo,

    /// The "pinned" register of the associated register bank.
    ///
    /// This register must be non-volatile (callee-preserved) and must not be the fixed
    /// output register of any instruction.
    pub pinned_reg: Option<RegUnit>,
}

impl RegClassData {
    /// Get the register class index corresponding to the intersection of `self` and `other`.
    ///
    /// This register class is guaranteed to exist if the register classes overlap. If the register
    /// classes don't overlap, returns `None`.
    pub fn intersect_index(&self, other: RegClass) -> Option<RegClassIndex> {
        // Compute the set of common subclasses.
        let mask = self.subclasses & other.subclasses;

        if mask == 0 {
            // No overlap.
            None
        } else {
            // Register class indexes are topologically ordered, so the largest common subclass has
            // the smallest index.
            Some(RegClassIndex(mask.trailing_zeros() as u8))
        }
    }

    /// Get the intersection of `self` and `other`.
    pub fn intersect(&self, other: RegClass) -> Option<RegClass> {
        self.intersect_index(other).map(|rci| self.info.rc(rci))
    }

    /// Returns true if `other` is a subclass of this register class.
    /// A register class is considered to be a subclass of itself.
    pub fn has_subclass<RCI: Into<RegClassIndex>>(&self, other: RCI) -> bool {
        self.subclasses & (1 << other.into().0) != 0
    }

    /// Get the top-level register class containing this class.
    pub fn toprc(&self) -> RegClass {
        self.info.rc(RegClassIndex(self.toprc))
    }

    /// Get a specific register unit in this class.
    pub fn unit(&self, offset: usize) -> RegUnit {
        let uoffset = offset * usize::from(self.width);
        self.first + uoffset as RegUnit
    }

    /// Does this register class contain `regunit`?
    pub fn contains(&self, regunit: RegUnit) -> bool {
        self.mask[(regunit / 32) as usize] & (1u32 << (regunit % 32)) != 0
    }

    /// If the pinned register is used, is the given regunit the pinned register of this class?
    #[inline]
    pub fn is_pinned_reg(&self, enabled: bool, regunit: RegUnit) -> bool {
        enabled
            && self
                .pinned_reg
                .map_or(false, |pinned_reg| pinned_reg == regunit)
    }
}

impl fmt::Display for RegClassData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.name)
    }
}

impl fmt::Debug for RegClassData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.name)
    }
}

/// Within an ISA, register classes are uniquely identified by their index.
impl PartialEq for RegClassData {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}

/// A small reference to a register class.
///
/// Use this when storing register classes in compact data structures. The `RegInfo::rc()` method
/// can be used to get the real register class reference back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RegClassIndex(u8);

impl EntityRef for RegClassIndex {
    fn new(idx: usize) -> Self {
        RegClassIndex(idx as u8)
    }

    fn index(self) -> usize {
        usize::from(self.0)
    }
}

impl From<RegClass> for RegClassIndex {
    fn from(rc: RegClass) -> Self {
        RegClassIndex(rc.index)
    }
}

impl fmt::Display for RegClassIndex {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "rci{}", self.0)
    }
}

/// Test of two registers overlap.
///
/// A register is identified as a `(RegClass, RegUnit)` pair. The register class is needed to
/// determine the width (in regunits) of the register.
pub fn regs_overlap(rc1: RegClass, reg1: RegUnit, rc2: RegClass, reg2: RegUnit) -> bool {
    let end1 = reg1 + RegUnit::from(rc1.width);
    let end2 = reg2 + RegUnit::from(rc2.width);
    !(end1 <= reg2 || end2 <= reg1)
}

/// Information about the registers in an ISA.
///
/// The `RegUnit` data structure collects all relevant static information about the registers in an
/// ISA.
#[derive(Clone)]
pub struct RegInfo {
    /// All register banks, ordered by their `first_unit`. The register banks are disjoint, but
    /// there may be holes of unused register unit numbers between banks due to alignment.
    pub banks: &'static [RegBank],

    /// All register classes ordered topologically so a sub-class always follows its parent.
    pub classes: &'static [RegClass],
}

impl RegInfo {
    /// Get the register bank holding `regunit`.
    pub fn bank_containing_regunit(&self, regunit: RegUnit) -> Option<&RegBank> {
        // We could do a binary search, but most ISAs have only two register banks...
        self.banks.iter().find(|b| b.contains(regunit))
    }

    /// Try to parse a regunit name. The name is not expected to begin with `%`.
    pub fn parse_regunit(&self, name: &str) -> Option<RegUnit> {
        self.banks
            .iter()
            .filter_map(|b| b.parse_regunit(name))
            .next()
    }

    /// Make a temporary object that can display a register unit.
    pub fn display_regunit(&self, regunit: RegUnit) -> DisplayRegUnit {
        DisplayRegUnit {
            regunit,
            reginfo: self,
        }
    }

    /// Get the register class corresponding to `idx`.
    pub fn rc(&self, idx: RegClassIndex) -> RegClass {
        self.classes[idx.index()]
    }

    /// Get the top-level register class containing the `idx` class.
    pub fn toprc(&self, idx: RegClassIndex) -> RegClass {
        self.classes[self.rc(idx).toprc as usize]
    }
}

/// Temporary object that holds enough information to print a register unit.
pub struct DisplayRegUnit<'a> {
    regunit: RegUnit,
    reginfo: &'a RegInfo,
}

impl<'a> fmt::Display for DisplayRegUnit<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.reginfo.bank_containing_regunit(self.regunit) {
            Some(b) => b.write_regunit(f, self.regunit),
            None => write!(f, "%INVALID{}", self.regunit),
        }
    }
}
