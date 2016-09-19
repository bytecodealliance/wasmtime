//! The `Encoding` struct.

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
            write!(f, "#{}/{:02x}", self.recipe, self.bits)
        } else {
            write!(f, "-")
        }
    }
}

/// Temporary object that holds enough context to properly display an encoding.
/// This is meant to be created by `TargetIsa::display_enc()`.
pub struct DisplayEncoding {
    pub encoding: Encoding,
    pub recipe_names: &'static [&'static str],
}

impl fmt::Display for DisplayEncoding {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.encoding.is_legal() {
            write!(f,
                   "{}/{:02x}",
                   self.recipe_names[self.encoding.recipe()],
                   self.encoding.bits)
        } else {
            write!(f, "-")
        }
    }
}
