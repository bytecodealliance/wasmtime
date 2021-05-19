use std::convert::TryFrom;

/// A renumbering of indices.
///
/// Keeps track of which indices in the original Wasm map to which indices in
/// the rewritten Wasm.
///
/// Only supports injective renumberings: every old index has a corresponding
/// unique new index, but each new index does not need to have a corresponding
/// old index.
#[derive(Default, Debug)]
pub struct Renumbering {
    old_count: u32,
    new_count: u32,
    old_to_new: Vec<u32>,
}

impl Renumbering {
    /// Define an entry in both the old and new index spaces. Returns a tuple of
    /// the old and new indices.
    pub fn define_both(&mut self) -> (u32, u32) {
        debug_assert_eq!(
            self.old_to_new.len(),
            usize::try_from(self.old_count).unwrap()
        );
        self.old_to_new.push(self.new_count);
        self.old_count += 1;
        self.new_count += 1;
        (self.old_count - 1, self.new_count - 1)
    }

    /// Add an import to both the old and new index spaces. Returns a tuple of
    /// the old and new indices.
    pub fn add_import(&mut self) -> (u32, u32) {
        self.define_both()
    }

    /// Add an alias to both the old and new index spaces. Returns a tuple of
    /// the old and new indices.
    pub fn add_alias(&mut self) -> (u32, u32) {
        self.define_both()
    }

    /// Add an entry to the new index space. Returns the entry's index in the
    /// new index space.
    pub fn define_new(&mut self) -> u32 {
        self.new_count += 1;
        self.new_count - 1
    }

    /// Get the new index for the given old index.
    ///
    /// Panics when `old` is not in the old index space.
    pub fn lookup(&self, old: u32) -> u32 {
        self.old_to_new[usize::try_from(old).unwrap()]
    }
}
