use cranelift_bitset::CompoundBitSet;
use serde_derive::{Deserialize, Serialize};

/// A map for determining where live GC references live in a stack frame.
///
/// Note that this is currently primarily documented as cranelift's
/// `binemit::StackMap`, so for detailed documentation about this please read
/// the docs over there.
#[derive(Debug, Serialize, Deserialize)]
pub struct StackMap {
    bits: CompoundBitSet,
    mapped_words: u32,
}

impl StackMap {
    /// Creates a new `StackMap`, typically from a preexisting
    /// `binemit::StackMap`.
    pub fn new(mapped_words: u32, bits: CompoundBitSet) -> StackMap {
        StackMap { bits, mapped_words }
    }

    /// Returns a specified bit.
    pub fn get_bit(&self, bit_index: usize) -> bool {
        self.bits.contains(bit_index)
    }

    /// Returns the number of words represented by this stack map.
    pub fn mapped_words(&self) -> u32 {
        self.mapped_words
    }
}
