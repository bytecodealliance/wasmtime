//! Source locations.
//!
//! Cranelift tracks the original source location of each instruction, and preserves the source
//! location when instructions are transformed.

use core::fmt;
#[cfg(feature = "enable-serde")]
use serde_derive::{Deserialize, Serialize};

/// A source location.
///
/// This is an opaque 31-bit number attached to each Cranelift IR instruction. Cranelift does not
/// interpret source locations in any way, they are simply preserved from the input to the output.
///
/// The default source location uses the bit pattern `0x7fff_ffff`. It is used for instructions
/// that can't be given a real source location. The high bit is reserved for
/// distinguishing relative and absolute locations in [`MaybeRelSourceLoc`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub struct SourceLoc(u32);

impl SourceLoc {
    /// Create a new source location with the given bits.
    pub fn new(bits: u32) -> Self {
        Self(bits)
    }

    /// Is this the default source location?
    pub fn is_default(self) -> bool {
        self == Default::default()
    }

    /// Read the bits of this source location.
    pub fn bits(self) -> u32 {
        self.0
    }
}

impl Default for SourceLoc {
    fn default() -> Self {
        Self(0x7fff_ffff)
    }
}

impl fmt::Display for SourceLoc {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_default() {
            write!(f, "@-")
        } else {
            write!(f, "@{:04x}", self.0)
        }
    }
}

/// Source location relative to another base source location.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub struct RelSourceLoc(u32);

impl RelSourceLoc {
    /// Create a new relative source location with the given bits.
    pub fn new(bits: u32) -> Self {
        Self(bits)
    }

    /// Creates a new `RelSourceLoc` based on the given base and offset.
    pub fn from_base_offset(base: SourceLoc, offset: SourceLoc) -> Self {
        if base.is_default() || offset.is_default() {
            Self::default()
        } else {
            // Wrap within the 31-bit source-location range, reserving the
            // high bit for MaybeRelSourceLoc's relative/absolute tag.
            Self(offset.bits().wrapping_sub(base.bits()) & MaybeRelSourceLoc::MASK)
        }
    }

    /// Expands the relative source location into an absolute one, using the given base.
    pub fn expand(&self, base: SourceLoc) -> SourceLoc {
        if self.is_default() || base.is_default() {
            Default::default()
        } else {
            SourceLoc::new(self.0.wrapping_add(base.bits()) & MaybeRelSourceLoc::MASK)
        }
    }

    /// Is this the default relative source location?
    pub fn is_default(self) -> bool {
        self == Default::default()
    }
}

impl Default for RelSourceLoc {
    fn default() -> Self {
        Self(0x7fff_ffff)
    }
}

impl fmt::Display for RelSourceLoc {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_default() {
            write!(f, "@-")
        } else {
            write!(f, "@+{:04x}", self.0)
        }
    }
}

/// A source location that is either a `RelSourceLoc` or `SourceLoc`.
///
/// This is used to represent a source location in the `MachBuffer`
/// that is initially relative to some base and is later relocated. We
/// do this to permit better code caching during incremental
/// compilation: the MachBuffer records the first SourceLoc it is
/// given as a base, and if the same function IR is later compiled but
/// with a different starting SourceLoc, we can reuse the cached
/// compilation result and just relocate (offset) the SourceLocs.
///
/// This relocation is an in-place update pass, so we want a "union"
/// type, essentially, but we don't want to pay the overhead of a true
/// `enum` (8 bytes rather than 4 in a large array), so we bitpack
/// this representation.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "enable-serde",
    derive(serde_derive::Serialize, serde_derive::Deserialize)
)]
pub struct MaybeRelSourceLoc(u32);

impl MaybeRelSourceLoc {
    const REL_BIT: u32 = 0x8000_0000;
    const MASK: u32 = !Self::REL_BIT;

    /// Create a relative SourceLoc.
    pub fn rel(loc: RelSourceLoc) -> Self {
        debug_assert!(loc.0 & Self::MASK == loc.0);
        MaybeRelSourceLoc(loc.0 | Self::REL_BIT)
    }

    /// Create an absolute SourceLoc.
    pub fn abs(loc: SourceLoc) -> Self {
        debug_assert!(loc.0 & Self::MASK == loc.0);
        MaybeRelSourceLoc(loc.0)
    }

    /// Is this a relative SourceLoc?
    pub fn is_rel(&self) -> bool {
        self.0 & Self::REL_BIT != 0
    }

    /// Is this an absolute SourceLoc?
    pub fn is_abs(&self) -> bool {
        self.0 & Self::REL_BIT == 0
    }

    /// Unwrap a relative SourceLoc.
    ///
    /// # Panics
    ///
    /// Panics if this is not a relative SourceLoc.
    pub fn as_rel(&self) -> RelSourceLoc {
        assert!(self.is_rel());
        RelSourceLoc(self.0 & Self::MASK)
    }

    /// Unwrap an absolute SourceLoc.
    ///
    /// # Panics
    ///
    /// Panics if this is not an absolute SourceLoc.
    pub fn as_abs(&self) -> SourceLoc {
        assert!(self.is_abs());
        SourceLoc(self.0)
    }

    /// Map a relative to an absolute SourceLoc, given another
    /// absolute SourceLoc as a base.
    ///
    /// # Panics
    ///
    /// Panics if this is not a relative SourceLoc.
    pub fn relocate(&self, base: SourceLoc) -> SourceLoc {
        self.as_rel().expand(base)
    }
}

#[cfg(test)]
mod tests {
    use crate::ir::SourceLoc;
    use alloc::string::ToString;

    #[test]
    fn display() {
        assert_eq!(SourceLoc::default().to_string(), "@-");
        assert_eq!(SourceLoc::new(0).to_string(), "@0000");
        assert_eq!(SourceLoc::new(16).to_string(), "@0010");
        assert_eq!(SourceLoc::new(0xabcdef).to_string(), "@abcdef");
    }
}
