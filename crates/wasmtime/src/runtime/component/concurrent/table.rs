// TODO: This duplicates a lot of resource_table.rs; consider reducing that
// duplication.
//
// The main difference between this and resource_table.rs is that the key type,
// `TableId<T>` implements `Copy`, making them much easier to work with than
// `Resource<T>`.  I've also added a `Table::delete_any` function, useful for
// implementing `subtask.drop`.

use std::{any::Any, boxed::Box, collections::BTreeSet, marker::PhantomData, vec::Vec};

pub struct TableId<T> {
    rep: u32,
    _marker: PhantomData<fn() -> T>,
}

impl<T> TableId<T> {
    pub fn new(rep: u32) -> Self {
        Self {
            rep,
            _marker: PhantomData,
        }
    }
}

impl<T> Clone for TableId<T> {
    fn clone(&self) -> Self {
        Self::new(self.rep)
    }
}

impl<T> Copy for TableId<T> {}

impl<T> TableId<T> {
    pub fn rep(&self) -> u32 {
        self.rep
    }
}

#[derive(Debug)]
/// Errors returned by operations on `Table`
pub enum TableError {
    /// Table has no free keys
    Full,
    /// Entry not present in table
    NotPresent,
    /// Resource present in table, but with a different type
    WrongType,
    /// Entry cannot be deleted because child entrys exist in the table.
    HasChildren,
}

impl std::fmt::Display for TableError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Full => write!(f, "table has no free keys"),
            Self::NotPresent => write!(f, "entry not present"),
            Self::WrongType => write!(f, "entry is of another type"),
            Self::HasChildren => write!(f, "entry has children"),
        }
    }
}
impl std::error::Error for TableError {}

/// The `Table` type maps a `TableId` to its entry.
#[derive(Default)]
pub struct Table {
    entries: Vec<Entry>,
    free_head: Option<usize>,
}

enum Entry {
    Free { next: Option<usize> },
    Occupied { entry: TableEntry },
}

impl Entry {
    pub fn occupied(&self) -> Option<&TableEntry> {
        match self {
            Self::Occupied { entry } => Some(entry),
            Self::Free { .. } => None,
        }
    }

    pub fn occupied_mut(&mut self) -> Option<&mut TableEntry> {
        match self {
            Self::Occupied { entry } => Some(entry),
            Self::Free { .. } => None,
        }
    }
}

/// This structure tracks parent and child relationships for a given table entry.
///
/// Parents and children are referred to by table index. We maintain the
/// following invariants to prevent orphans and cycles:
/// * parent can only be assigned on creating the entry.
/// * parent, if some, must exist when creating the entry.
/// * whenever a child is created, its index is added to children.
/// * whenever a child is deleted, its index is removed from children.
/// * an entry with children may not be deleted.
struct TableEntry {
    /// The entry in the table
    entry: Box<dyn Any + Send + Sync>,
    /// The index of the parent of this entry, if it has one.
    parent: Option<u32>,
    /// The indicies of any children of this entry.
    children: BTreeSet<u32>,
}

impl TableEntry {
    fn new(entry: Box<dyn Any + Send + Sync>, parent: Option<u32>) -> Self {
        Self {
            entry,
            parent,
            children: BTreeSet::new(),
        }
    }
    fn add_child(&mut self, child: u32) {
        assert!(self.children.insert(child));
    }
    fn remove_child(&mut self, child: u32) {
        assert!(self.children.remove(&child));
    }
}

impl Table {
    /// Create an empty table
    pub fn new() -> Self {
        let mut me = Self {
            entries: Vec::new(),
            free_head: None,
        };

        // TODO: remove this once we've stopped exposing these indexes to guest code:
        me.push(()).unwrap();

        me
    }

    /// Inserts a new entry into this table, returning a corresponding
    /// `TableId<T>` which can be used to refer to it after it was inserted.
    pub fn push<T: Send + Sync + 'static>(&mut self, entry: T) -> Result<TableId<T>, TableError> {
        let idx = self.push_(TableEntry::new(Box::new(entry), None))?;
        Ok(TableId::new(idx))
    }

    /// Pop an index off of the free list, if it's not empty.
    fn pop_free_list(&mut self) -> Option<usize> {
        if let Some(ix) = self.free_head {
            // Advance free_head to the next entry if one is available.
            match &self.entries[ix] {
                Entry::Free { next } => self.free_head = *next,
                Entry::Occupied { .. } => unreachable!(),
            }
            Some(ix)
        } else {
            None
        }
    }

    /// Free an entry in the table, returning its [`TableEntry`]. Add the index to the free list.
    fn free_entry(&mut self, ix: usize) -> TableEntry {
        let entry = match std::mem::replace(
            &mut self.entries[ix],
            Entry::Free {
                next: self.free_head,
            },
        ) {
            Entry::Occupied { entry } => entry,
            Entry::Free { .. } => unreachable!(),
        };

        self.free_head = Some(ix);

        entry
    }

    /// Push a new entry into the table, returning its handle. This will prefer to use free entries
    /// if they exist, falling back on pushing new entries onto the end of the table.
    fn push_(&mut self, e: TableEntry) -> Result<u32, TableError> {
        if let Some(free) = self.pop_free_list() {
            self.entries[free] = Entry::Occupied { entry: e };
            Ok(u32::try_from(free).unwrap())
        } else {
            let ix = self
                .entries
                .len()
                .try_into()
                .map_err(|_| TableError::Full)?;
            self.entries.push(Entry::Occupied { entry: e });
            Ok(ix)
        }
    }

    fn occupied(&self, key: u32) -> Result<&TableEntry, TableError> {
        self.entries
            .get(key as usize)
            .and_then(Entry::occupied)
            .ok_or(TableError::NotPresent)
    }

    fn occupied_mut(&mut self, key: u32) -> Result<&mut TableEntry, TableError> {
        self.entries
            .get_mut(key as usize)
            .and_then(Entry::occupied_mut)
            .ok_or(TableError::NotPresent)
    }

    /// Insert a entry at the next available index, and track that it has a
    /// parent entry.
    ///
    /// The parent must exist to create a child. All child entrys must be
    /// destroyed before a parent can be destroyed - otherwise [`Table::delete`]
    /// will fail with [`TableError::HasChildren`].
    ///
    /// Parent-child relationships are tracked inside the table to ensure that a
    /// parent is not deleted while it has live children. This allows children
    /// to hold "references" to a parent by table index, to avoid needing
    /// e.g. an `Arc<Mutex<parent>>` and the associated locking overhead and
    /// design issues, such as child existence extending lifetime of parent
    /// referent even after parent is destroyed, possibility for deadlocks.
    ///
    /// Parent-child relationships may not be modified once created. There is no
    /// way to observe these relationships through the [`Table`] methods except
    /// for erroring on deletion, or the [`std::fmt::Debug`] impl.
    pub fn push_child<T: Send + Sync + 'static, U>(
        &mut self,
        entry: T,
        parent: TableId<U>,
    ) -> Result<TableId<T>, TableError> {
        let parent = parent.rep();
        self.occupied(parent)?;
        let child = self.push_(TableEntry::new(Box::new(entry), Some(parent)))?;
        self.occupied_mut(parent)?.add_child(child);
        Ok(TableId::new(child))
    }

    pub fn add_child<T, U>(
        &mut self,
        child: TableId<T>,
        parent: TableId<U>,
    ) -> Result<(), TableError> {
        let entry = self.occupied_mut(child.rep())?;
        assert!(entry.parent.is_none());
        entry.parent = Some(parent.rep());
        self.occupied_mut(parent.rep())?.add_child(child.rep());
        Ok(())
    }

    pub fn remove_child<T, U>(
        &mut self,
        child: TableId<T>,
        parent: TableId<U>,
    ) -> Result<(), TableError> {
        let entry = self.occupied_mut(child.rep())?;
        assert_eq!(entry.parent, Some(parent.rep()));
        entry.parent = None;
        self.occupied_mut(parent.rep())?.remove_child(child.rep());
        Ok(())
    }

    /// Get an immutable reference to a task of a given type at a given index.
    ///
    /// Multiple shared references can be borrowed at any given time.
    pub fn get<T: 'static>(&self, key: TableId<T>) -> Result<&T, TableError> {
        self.get_(key.rep())?
            .downcast_ref()
            .ok_or(TableError::WrongType)
    }

    fn get_(&self, key: u32) -> Result<&dyn Any, TableError> {
        let r = self.occupied(key)?;
        Ok(&*r.entry)
    }

    /// Get an mutable reference to a task of a given type at a given index.
    pub fn get_mut<T: 'static>(&mut self, key: TableId<T>) -> Result<&mut T, TableError> {
        self.get_mut_(key.rep())?
            .downcast_mut()
            .ok_or(TableError::WrongType)
    }

    pub fn get_mut_(&mut self, key: u32) -> Result<&mut dyn Any, TableError> {
        let r = self.occupied_mut(key)?;
        Ok(&mut *r.entry)
    }

    /// Delete the specified task
    pub fn delete<T: 'static>(&mut self, key: TableId<T>) -> Result<T, TableError> {
        self.delete_entry(key.rep())?
            .entry
            .downcast()
            .map(|v| *v)
            .map_err(|_| TableError::WrongType)
    }

    pub fn delete_any(&mut self, key: u32) -> Result<Box<dyn Any + Send + Sync>, TableError> {
        Ok(self.delete_entry(key)?.entry)
    }

    fn delete_entry(&mut self, key: u32) -> Result<TableEntry, TableError> {
        if !self.occupied(key)?.children.is_empty() {
            return Err(TableError::HasChildren);
        }
        let e = self.free_entry(key as usize);
        if let Some(parent) = e.parent {
            // Remove deleted task from parent's child list.  Parent must still
            // be present because it cant be deleted while still having
            // children:
            self.occupied_mut(parent)
                .expect("missing parent")
                .remove_child(key);
        }
        Ok(e)
    }
}
