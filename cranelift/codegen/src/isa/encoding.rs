//! The `Encoding` struct.

use crate::binemit::CodeOffset;
use crate::ir::{Function, Inst};
use crate::isa::constraints::{BranchRange, RecipeConstraints};
use crate::regalloc::RegDiversions;
use core::fmt;

/// Bits needed to encode an instruction as binary machine code.
///
/// The encoding consists of two parts, both specific to the target ISA: An encoding *recipe*, and
/// encoding *bits*. The recipe determines the native instruction format and the mapping of
/// operands to encoded bits. The encoding bits provide additional information to the recipe,
/// typically parts of the opcode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Encoding {
    recipe: u16,
    bits: u16,
}

impl Encoding {
    /// Create a new `Encoding` containing `(recipe, bits)`.
    pub fn new(recipe: u16, bits: u16) -> Self {
        Self { recipe, bits }
    }

    /// Get the recipe number in this encoding.
    pub fn recipe(self) -> usize {
        self.recipe as usize
    }

    /// Get the recipe-specific encoding bits.
    pub fn bits(self) -> u16 {
        self.bits
    }

    /// Is this a legal encoding, or the default placeholder?
    pub fn is_legal(self) -> bool {
        self != Self::default()
    }
}

/// The default encoding is the illegal one.
impl Default for Encoding {
    fn default() -> Self {
        Self::new(0xffff, 0xffff)
    }
}

/// ISA-independent display of an encoding.
impl fmt::Display for Encoding {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_legal() {
            write!(f, "{}#{:02x}", self.recipe, self.bits)
        } else {
            write!(f, "-")
        }
    }
}

/// Temporary object that holds enough context to properly display an encoding.
/// This is meant to be created by `EncInfo::display()`.
pub struct DisplayEncoding {
    pub encoding: Encoding,
    pub recipe_names: &'static [&'static str],
}

impl fmt::Display for DisplayEncoding {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.encoding.is_legal() {
            write!(
                f,
                "{}#{:02x}",
                self.recipe_names[self.encoding.recipe()],
                self.encoding.bits
            )
        } else {
            write!(f, "-")
        }
    }
}

type SizeCalculatorFn = fn(&RecipeSizing, Encoding, Inst, &RegDiversions, &Function) -> u8;

/// Returns the base size of the Recipe, assuming it's fixed. This is the default for most
/// encodings; others can be variable and longer than this base size, depending on the registers
/// they're using and use a different function, specific per platform.
pub fn base_size(
    sizing: &RecipeSizing,
    _: Encoding,
    _: Inst,
    _: &RegDiversions,
    _: &Function,
) -> u8 {
    sizing.base_size
}

/// Code size information for an encoding recipe.
///
/// Encoding recipes may have runtime-determined instruction size.
pub struct RecipeSizing {
    /// Minimum size in bytes of instructions encoded with this recipe.
    pub base_size: u8,

    /// Method computing the instruction's real size, given inputs and outputs.
    pub compute_size: SizeCalculatorFn,

    /// Allowed branch range in this recipe, if any.
    ///
    /// All encoding recipes for branches have exact branch range information.
    pub branch_range: Option<BranchRange>,
}

/// Information about all the encodings in this ISA.
#[derive(Clone)]
pub struct EncInfo {
    /// Constraints on value operands per recipe.
    pub constraints: &'static [RecipeConstraints],

    /// Code size information per recipe.
    pub sizing: &'static [RecipeSizing],

    /// Names of encoding recipes.
    pub names: &'static [&'static str],
}

impl EncInfo {
    /// Get the value operand constraints for `enc` if it is a legal encoding.
    pub fn operand_constraints(&self, enc: Encoding) -> Option<&'static RecipeConstraints> {
        self.constraints.get(enc.recipe())
    }

    /// Create an object that can display an ISA-dependent encoding properly.
    pub fn display(&self, enc: Encoding) -> DisplayEncoding {
        DisplayEncoding {
            encoding: enc,
            recipe_names: self.names,
        }
    }

    /// Get the size in bytes of `inst`, if it were encoded with `enc`.
    ///
    /// Returns 0 for illegal encodings.
    pub fn byte_size(
        &self,
        enc: Encoding,
        inst: Inst,
        divert: &RegDiversions,
        func: &Function,
    ) -> CodeOffset {
        self.sizing.get(enc.recipe()).map_or(0, |s| {
            let compute_size = s.compute_size;
            CodeOffset::from(compute_size(&s, enc, inst, divert, func))
        })
    }

    /// Get the branch range that is supported by `enc`, if any.
    ///
    /// This will never return `None` for a legal branch encoding.
    pub fn branch_range(&self, enc: Encoding) -> Option<BranchRange> {
        self.sizing.get(enc.recipe()).and_then(|s| s.branch_range)
    }
}
