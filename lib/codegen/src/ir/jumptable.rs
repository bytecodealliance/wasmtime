//! Jump table representation.
//!
//! Jump tables are declared in the preamble and assigned an `ir::entities::JumpTable` reference.
//! The actual table of destinations is stored in a `JumpTableData` struct defined in this module.

use ir::entities::Ebb;
use packed_option::PackedOption;
use std::fmt::{self, Display, Formatter};
use std::iter;
use std::slice;
use std::vec::Vec;

/// Contents of a jump table.
///
/// All jump tables use 0-based indexing and are expected to be densely populated. They don't need
/// to be completely populated, though. Individual entries can be missing.
#[derive(Clone)]
pub struct JumpTableData {
    // Table entries, using `None` as a placeholder for missing entries.
    table: Vec<PackedOption<Ebb>>,

    // How many `None` holes in table?
    holes: usize,
}

impl JumpTableData {
    /// Create a new empty jump table.
    pub fn new() -> Self {
        Self {
            table: Vec::new(),
            holes: 0,
        }
    }

    /// Create a new empty jump table with the specified capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            table: Vec::with_capacity(capacity),
            holes: 0,
        }
    }

    /// Get the number of table entries.
    pub fn len(&self) -> usize {
        self.table.len()
    }

    /// Set a table entry.
    ///
    /// The table will grow as needed to fit `idx`.
    pub fn set_entry(&mut self, idx: usize, dest: Ebb) {
        // Resize table to fit `idx`.
        if idx >= self.table.len() {
            self.holes += idx - self.table.len();
            self.table.resize(idx + 1, None.into());
        } else if self.table[idx].is_none() {
            // We're filling in an existing hole.
            self.holes -= 1;
        }
        self.table[idx] = dest.into();
    }

    /// Append a table entry.
    pub fn push_entry(&mut self, dest: Ebb) {
        self.table.push(dest.into())
    }

    /// Clear a table entry.
    ///
    /// The `br_table` instruction will fall through if given an index corresponding to a cleared
    /// table entry.
    pub fn clear_entry(&mut self, idx: usize) {
        if idx < self.table.len() && self.table[idx].is_some() {
            self.holes += 1;
            self.table[idx] = None.into();
        }
    }

    /// Get the entry for `idx`, or `None`.
    pub fn get_entry(&self, idx: usize) -> Option<Ebb> {
        self.table.get(idx).and_then(|e| e.expand())
    }

    /// Enumerate over all `(idx, dest)` pairs in the table in order.
    ///
    /// This returns an iterator that skips any empty slots in the table.
    pub fn entries(&self) -> Entries {
        Entries(self.table.iter().cloned().enumerate())
    }

    /// Checks if any of the entries branch to `ebb`.
    pub fn branches_to(&self, ebb: Ebb) -> bool {
        self.table
            .iter()
            .any(|target_ebb| target_ebb.expand() == Some(ebb))
    }

    /// Access the whole table as a mutable slice.
    pub fn as_mut_slice(&mut self) -> &mut [PackedOption<Ebb>] {
        self.table.as_mut_slice()
    }
}

/// Enumerate `(idx, dest)` pairs in order.
pub struct Entries<'a>(iter::Enumerate<iter::Cloned<slice::Iter<'a, PackedOption<Ebb>>>>);

impl<'a> Iterator for Entries<'a> {
    type Item = (usize, Ebb);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some((idx, dest)) = self.0.next() {
                if let Some(ebb) = dest.expand() {
                    return Some((idx, ebb));
                }
            } else {
                return None;
            }
        }
    }
}

impl Display for JumpTableData {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        match self.table.first().and_then(|e| e.expand()) {
            None => write!(fmt, "jump_table 0")?,
            Some(first) => write!(fmt, "jump_table {}", first)?,
        }

        for dest in self.table.iter().skip(1).map(|e| e.expand()) {
            match dest {
                None => write!(fmt, ", 0")?,
                Some(ebb) => write!(fmt, ", {}", ebb)?,
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::JumpTableData;
    use entity::EntityRef;
    use ir::Ebb;
    use std::string::ToString;
    use std::vec::Vec;

    #[test]
    fn empty() {
        let jt = JumpTableData::new();

        assert_eq!(jt.get_entry(0), None);
        assert_eq!(jt.get_entry(10), None);

        assert_eq!(jt.to_string(), "jump_table 0");

        let v: Vec<(usize, Ebb)> = jt.entries().collect();
        assert_eq!(v, []);
    }

    #[test]
    fn insert() {
        let e1 = Ebb::new(1);
        let e2 = Ebb::new(2);

        let mut jt = JumpTableData::new();

        jt.set_entry(0, e1);
        jt.set_entry(0, e2);
        jt.set_entry(10, e1);

        assert_eq!(
            jt.to_string(),
            "jump_table ebb2, 0, 0, 0, 0, 0, 0, 0, 0, 0, ebb1"
        );

        let v: Vec<(usize, Ebb)> = jt.entries().collect();
        assert_eq!(v, [(0, e2), (10, e1)]);
    }
}
