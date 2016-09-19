//! The `Encoding` struct.

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
    pub fn new(recipe: u16, bits: u16) -> Encoding {
        Encoding {
            recipe: recipe,
            bits: bits,
        }
    }

    /// Get the recipe number in this encoding.
    pub fn recipe(self) -> usize {
        self.recipe as usize
    }

    /// Get the recipe-specific encoding bits.
    pub fn bits(self) -> u16 {
        self.bits
    }
}
