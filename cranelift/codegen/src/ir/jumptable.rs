//! Jump table representation.
//!
//! Jump tables are declared in the preamble and assigned an `ir::entities::JumpTable` reference.
//! The actual table of destinations is stored in a `JumpTableData` struct defined in this module.

use crate::ir::entities::Block;
use alloc::vec::Vec;
use core::fmt::{self, Display, Formatter};
use core::slice::{Iter, IterMut};

#[cfg(feature = "enable-serde")]
use serde::{Deserialize, Serialize};

/// Contents of a jump table.
///
/// All jump tables use 0-based indexing and are densely populated.
///
/// The default block for the jump table is stored as the last element of the underlying vector,
/// and is not included in the length of the jump table. It can be accessed through the
/// `default_block` function. The default block is iterated via the `iter` and `iter_mut` methods.
#[derive(Clone, PartialEq, Hash)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub struct JumpTableData {
    // Table entries.
    table: Vec<Block>,
}

impl JumpTableData {
    /// Create a new empty jump table.
    pub fn new(def: Block) -> Self {
        Self { table: vec![def] }
    }

    /// Create a new jump table with the provided blocks
    pub fn with_blocks(def: Block, mut table: Vec<Block>) -> Self {
        table.push(def);
        Self { table }
    }

    /// Fetch the default block for this jump table.
    pub fn default_block(&self) -> Block {
        *self.table.last().unwrap()
    }

    /// Get the number of table entries.
    pub fn len(&self) -> usize {
        self.table.len() - 1
    }

    /// Append a table entry.
    pub fn push_entry(&mut self, dest: Block) {
        let last = self.table.len();
        self.table.push(dest);

        // Ensure that the default block stays as the final element in the table.
        self.table.swap(last - 1, last);
    }

    /// Checks if any of the entries branch to `block`. The default block will be considered when
    /// checking for a possible branch.
    pub fn branches_to(&self, block: Block) -> bool {
        self.table.iter().any(|target_block| *target_block == block)
    }

    /// Access the jump table as a slice, excluding the default block.
    pub fn table_slice(&self) -> &[Block] {
        let last = self.len();
        &self.table.as_slice()[0..last]
    }

    /// Access the jump table as a slice, excluding the default block.
    pub fn table_slice_mut(&mut self) -> &mut [Block] {
        let last = self.len();
        &mut self.table.as_mut_slice()[0..last]
    }

    /// Access the entire jump table as a slice.
    pub fn as_slice(&self) -> &[Block] {
        self.table.as_slice()
    }

    /// Access the whole table as a mutable slice, excluding the default block.
    pub fn as_mut_slice(&mut self) -> &mut [Block] {
        self.table.as_mut_slice()
    }

    /// Returns an iterator over the table.
    pub fn iter(&self) -> Iter<Block> {
        self.table.iter()
    }

    /// Returns an iterator that allows modifying each value, including the default block.
    pub fn iter_mut(&mut self) -> IterMut<Block> {
        self.table.iter_mut()
    }

    /// Clears all entries in this jump table, except for the default block.
    pub fn clear(&mut self) {
        self.table.drain(0..self.table.len() - 1);
    }
}

impl Display for JumpTableData {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        write!(fmt, "{}, [", self.default_block())?;
        if let Some((first, rest)) = self.table_slice().split_first() {
            write!(fmt, "{}", first)?;
            for block in rest {
                write!(fmt, ", {}", block)?;
            }
        }
        write!(fmt, "]")
    }
}

#[cfg(test)]
mod tests {
    use super::JumpTableData;
    use crate::entity::EntityRef;
    use crate::ir::Block;
    use alloc::string::ToString;

    #[test]
    fn empty() {
        let def = Block::new(0);

        let jt = JumpTableData::new(def);

        assert_eq!(jt.table_slice().get(0), None);

        assert_eq!(jt.as_slice().get(0), Some(&def));
        assert_eq!(jt.as_slice().get(10), None);

        assert_eq!(jt.to_string(), "block0, []");

        assert_eq!(jt.as_slice(), [def]);

        assert_eq!(jt.table_slice(), []);
    }

    #[test]
    fn insert() {
        let def = Block::new(0);
        let e1 = Block::new(1);
        let e2 = Block::new(2);

        let mut jt = JumpTableData::new(def);

        jt.push_entry(e1);
        jt.push_entry(e2);
        jt.push_entry(e1);

        assert_eq!(jt.default_block(), def);
        assert_eq!(jt.to_string(), "block0, [block1, block2, block1]");

        assert_eq!(jt.as_slice(), [e1, e2, e1, def]);
        assert_eq!(jt.table_slice(), [e1, e2, e1]);

        jt.clear();
        assert_eq!(jt.default_block(), def);
    }
}
