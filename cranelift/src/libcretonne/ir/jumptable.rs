//! Jump table representation.
//!
//! Jump tables are declared in the preamble and assigned an `ir::entities::JumpTable` reference.
//! The actual table of destinations is stored in a `JumpTableData` struct defined in this module.

use ir::entities::{Ebb, NO_EBB};
use std::iter;
use std::slice;
use std::fmt::{self, Display, Formatter};

/// Contents of a jump table.
///
/// All jump tables use 0-based indexing and are expected to be densely populated. They don't need
/// to be completely populated, though. Individual entries can be missing.
pub struct JumpTableData {
    // Table entries, using NO_EBB as a placeholder for missing entries.
    table: Vec<Ebb>,

    // How many `NO_EBB` holes in table?
    holes: usize,
}

impl JumpTableData {
    /// Create a new empty jump table.
    pub fn new() -> JumpTableData {
        JumpTableData {
            table: Vec::new(),
            holes: 0,
        }
    }

    /// Set a table entry.
    ///
    /// The table will grow as needed to fit 'idx'.
    pub fn set_entry(&mut self, idx: usize, dest: Ebb) {
        assert!(dest != NO_EBB);
        // Resize table to fit `idx`.
        if idx >= self.table.len() {
            self.holes += idx - self.table.len();
            self.table.resize(idx + 1, NO_EBB);
        } else if self.table[idx] == NO_EBB {
            // We're filling in an existing hole.
            self.holes -= 1;
        }
        self.table[idx] = dest;
    }

    /// Clear a table entry.
    ///
    /// The `br_table` instruction will fall through if given an index corresponding to a cleared
    /// table entry.
    pub fn clear_entry(&mut self, idx: usize) {
        if idx < self.table.len() && self.table[idx] != NO_EBB {
            self.holes += 1;
            self.table[idx] = NO_EBB;
        }
    }

    /// Get the entry for `idx`, or `None`.
    pub fn get_entry(&self, idx: usize) -> Option<Ebb> {
        if idx < self.table.len() && self.table[idx] != NO_EBB {
            Some(self.table[idx])
        } else {
            None
        }
    }

    /// Enumerate over all `(idx, dest)` pairs in the table in order.
    ///
    /// This returns an iterator that skips any empty slots in the table.
    pub fn entries<'a>(&'a self) -> Entries {
        Entries(self.table.iter().cloned().enumerate())
    }

    /// Access the whole table as a mutable slice.
    pub fn as_mut_slice(&mut self) -> &mut [Ebb] {
        self.table.as_mut_slice()
    }
}

/// Enumerate `(idx, dest)` pairs in order.
pub struct Entries<'a>(iter::Enumerate<iter::Cloned<slice::Iter<'a, Ebb>>>);

impl<'a> Iterator for Entries<'a> {
    type Item = (usize, Ebb);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some((idx, dest)) = self.0.next() {
                if dest != NO_EBB {
                    return Some((idx, dest));
                }
            } else {
                return None;
            }
        }
    }
}

impl Display for JumpTableData {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        let first = self.table.first().cloned().unwrap_or_default();
        if first == NO_EBB {
            try!(write!(fmt, "jump_table 0"));
        } else {
            try!(write!(fmt, "jump_table {}", first));
        }

        for dest in self.table.iter().cloned().skip(1) {
            if dest == NO_EBB {
                try!(write!(fmt, ", 0"));
            } else {
                try!(write!(fmt, ", {}", dest));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::JumpTableData;
    use ir::Ebb;
    use entity_map::EntityRef;

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

        assert_eq!(jt.to_string(),
                   "jump_table ebb2, 0, 0, 0, 0, 0, 0, 0, 0, 0, ebb1");

        let v: Vec<(usize, Ebb)> = jt.entries().collect();
        assert_eq!(v, [(0, e2), (10, e1)]);
    }
}
