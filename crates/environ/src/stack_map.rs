use serde::{Deserialize, Serialize};

/// A map for determining where live GC references live in a stack frame.
///
/// Note that this is currently primarily documented as cranelift's
/// `binemit::StackMap`, so for detailed documentation about this please read
/// the docs over there.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StackMap {
    bits: Box<[u32]>,
    mapped_words: u32,
}

impl StackMap {
    /// Creates a new `StackMap`, typically from a preexisting
    /// `binemit::StackMap`.
    pub fn new(mapped_words: u32, bits: impl Iterator<Item = u32>) -> StackMap {
        StackMap {
            bits: bits.collect(),
            mapped_words,
        }
    }

    /// Returns a specified bit.
    pub fn get_bit(&self, bit_index: usize) -> bool {
        assert!(bit_index < 32 * self.bits.len());
        let word_index = bit_index / 32;
        let word_offset = bit_index % 32;
        (self.bits[word_index] & (1 << word_offset)) != 0
    }

    /// Returns the number of words represented by this stack map.
    pub fn mapped_words(&self) -> u32 {
        self.mapped_words
    }
}
