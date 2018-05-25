//! The `Encoding` struct.

use binemit::CodeOffset;
use isa::constraints::{BranchRange, RecipeConstraints};
use std::fmt;

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

/// Code size information for an encoding recipe.
///
/// All encoding recipes correspond to an exact instruction size.
pub struct RecipeSizing {
    /// Size in bytes of instructions encoded with this recipe.
    pub bytes: u8,

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

    /// Get the exact size in bytes of instructions encoded with `enc`.
    ///
    /// Returns 0 for illegal encodings.
    pub fn bytes(&self, enc: Encoding) -> CodeOffset {
        self.sizing
            .get(enc.recipe())
            .map_or(0, |s| CodeOffset::from(s.bytes))
    }

    /// Get the branch range that is supported by `enc`, if any.
    ///
    /// This will never return `None` for a legal branch encoding.
    pub fn branch_range(&self, enc: Encoding) -> Option<BranchRange> {
        self.sizing.get(enc.recipe()).and_then(|s| s.branch_range)
    }
}
