use super::Resource;
use crate::prelude::*;
use alloc::collections::{BTreeMap, BTreeSet};
use core::any::Any;
use core::fmt;
use core::mem;

#[derive(Debug)]
/// Errors returned by operations on `ResourceTable`
pub enum ResourceTableError {
    /// ResourceTable has no free keys
    Full,
    /// Resource not present in table
    NotPresent,
    /// Resource present in table, but with a different type
    WrongType,
    /// Resource cannot be deleted because child resources exist in the table. Consult wit docs for
    /// the particular resource to see which methods may return child resources.
    HasChildren,
}

impl fmt::Display for ResourceTableError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Full => write!(f, "resource table has no free keys"),
            Self::NotPresent => write!(f, "resource not present"),
            Self::WrongType => write!(f, "resource is of another type"),
            Self::HasChildren => write!(f, "resource has children"),
        }
    }
}

impl core::error::Error for ResourceTableError {}

/// The `ResourceTable` type maps a `Resource<T>` to its `T`.
pub struct ResourceTable {
    entries: Vec<Entry>,
    free_head: Option<usize>,
}

#[derive(Debug)]
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

struct Tombstone;

// Change this to `true` to assist with handle debugging in development if
// necessary.
const DELETE_WITH_TOMBSTONE: bool = false;

/// This structure tracks parent and child relationships for a given table entry.
///
/// Parents and children are referred to by table index. We maintain the
/// following invariants:
/// * the parent must exist when adding a child.
/// * whenever a child is created, its index is added to children.
/// * whenever a child is deleted, its index is removed from children.
/// * an entry with children may not be deleted.
#[derive(Debug)]
struct TableEntry {
    /// The entry in the table, as a boxed dynamically-typed object
    entry: Box<dyn Any + Send>,
    /// The index of the parent of this entry, if it has one.
    parent: Option<u32>,
    /// The indices of any children of this entry.
    children: BTreeSet<u32>,
}

impl TableEntry {
    fn new(entry: Box<dyn Any + Send>, parent: Option<u32>) -> Self {
        Self {
            entry,
            parent,
            children: BTreeSet::new(),
        }
    }
    fn add_child(&mut self, child: u32) {
        debug_assert!(!self.children.contains(&child));
        self.children.insert(child);
    }
    fn remove_child(&mut self, child: u32) {
        let was_removed = self.children.remove(&child);
        debug_assert!(was_removed);
    }
}

impl ResourceTable {
    /// Create an empty table
    pub fn new() -> Self {
        ResourceTable {
            entries: Vec::new(),
            free_head: None,
        }
    }

    /// Returns whether or not this table is empty.
    ///
    /// Note that this is an `O(n)` operation, where `n` is the number of
    /// entries in the backing `Vec`.
    pub fn is_empty(&self) -> bool {
        self.entries.iter().all(|entry| match entry {
            Entry::Free { .. } => true,
            Entry::Occupied { entry } => entry.entry.downcast_ref::<Tombstone>().is_some(),
        })
    }

    /// Create an empty table with at least the specified capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        ResourceTable {
            entries: Vec::with_capacity(capacity),
            free_head: None,
        }
    }

    /// Inserts a new value `T` into this table, returning a corresponding
    /// `Resource<T>` which can be used to refer to it after it was inserted.
    pub fn push<T>(&mut self, entry: T) -> Result<Resource<T>, ResourceTableError>
    where
        T: Send + 'static,
    {
        let idx = self.push_(TableEntry::new(Box::new(entry), None))?;
        Ok(Resource::new_own(idx))
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
    fn free_entry(&mut self, ix: usize, debug: bool) -> TableEntry {
        if debug {
            // Instead of making this entry available for reuse, we leave a
            // tombstone in debug mode.  This helps detect use-after-delete and
            // double-delete bugs.
            match mem::replace(
                &mut self.entries[ix],
                Entry::Occupied {
                    entry: TableEntry {
                        entry: Box::new(Tombstone),
                        parent: None,
                        children: BTreeSet::new(),
                    },
                },
            ) {
                Entry::Occupied { entry } => entry,
                Entry::Free { .. } => unreachable!(),
            }
        } else {
            let entry = match core::mem::replace(
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
    }

    /// Push a new entry into the table, returning its handle. This will prefer to use free entries
    /// if they exist, falling back on pushing new entries onto the end of the table.
    fn push_(&mut self, e: TableEntry) -> Result<u32, ResourceTableError> {
        if let Some(free) = self.pop_free_list() {
            self.entries[free] = Entry::Occupied { entry: e };
            Ok(free.try_into().unwrap())
        } else {
            let ix = self
                .entries
                .len()
                .try_into()
                .map_err(|_| ResourceTableError::Full)?;
            self.entries.push(Entry::Occupied { entry: e });
            Ok(ix)
        }
    }

    fn occupied(&self, key: u32) -> Result<&TableEntry, ResourceTableError> {
        self.entries
            .get(key as usize)
            .and_then(Entry::occupied)
            .ok_or(ResourceTableError::NotPresent)
    }

    fn occupied_mut(&mut self, key: u32) -> Result<&mut TableEntry, ResourceTableError> {
        self.entries
            .get_mut(key as usize)
            .and_then(Entry::occupied_mut)
            .ok_or(ResourceTableError::NotPresent)
    }

    /// Insert a resource at the next available index, and track that it has a
    /// parent resource.
    ///
    /// The parent must exist to create a child. All children resources must
    /// be destroyed before a parent can be destroyed - otherwise
    /// [`ResourceTable::delete`] will fail with
    /// [`ResourceTableError::HasChildren`].
    ///
    /// Parent-child relationships are tracked inside the table to ensure that
    /// a parent resource is not deleted while it has live children. This
    /// allows child resources to hold "references" to a parent by table
    /// index, to avoid needing e.g. an `Arc<Mutex<parent>>` and the associated
    /// locking overhead and design issues, such as child existence extending
    /// lifetime of parent referent even after parent resource is destroyed,
    /// possibility for deadlocks.
    pub fn push_child<T, U>(
        &mut self,
        entry: T,
        parent: &Resource<U>,
    ) -> Result<Resource<T>, ResourceTableError>
    where
        T: Send + 'static,
        U: 'static,
    {
        let parent = parent.rep();
        self.occupied(parent)?;
        let child = self.push_(TableEntry::new(Box::new(entry), Some(parent)))?;
        self.occupied_mut(parent)?.add_child(child);
        Ok(Resource::new_own(child))
    }

    /// Add an already-resident child to a resource.
    pub fn add_child<T: 'static, U: 'static>(
        &mut self,
        child: Resource<T>,
        parent: Resource<U>,
    ) -> Result<(), ResourceTableError> {
        let entry = self.occupied_mut(child.rep())?;
        assert!(entry.parent.is_none());
        entry.parent = Some(parent.rep());
        self.occupied_mut(parent.rep())?.add_child(child.rep());
        Ok(())
    }

    /// Remove a child to from a resource (but leave it in the table).
    pub fn remove_child<T: 'static, U: 'static>(
        &mut self,
        child: Resource<T>,
        parent: Resource<U>,
    ) -> Result<(), ResourceTableError> {
        let entry = self.occupied_mut(child.rep())?;
        assert_eq!(entry.parent, Some(parent.rep()));
        entry.parent = None;
        self.occupied_mut(parent.rep())?.remove_child(child.rep());
        Ok(())
    }

    /// Get an immutable reference to a resource of a given type at a given
    /// index.
    ///
    /// Multiple shared references can be borrowed at any given time.
    pub fn get<T: Any + Sized>(&self, key: &Resource<T>) -> Result<&T, ResourceTableError> {
        self.get_(key.rep())?
            .downcast_ref()
            .ok_or(ResourceTableError::WrongType)
    }

    fn get_(&self, key: u32) -> Result<&dyn Any, ResourceTableError> {
        let r = self.occupied(key)?;
        Ok(&*r.entry)
    }

    /// Get an mutable reference to a resource of a given type at a given
    /// index.
    pub fn get_mut<T: Any + Sized>(
        &mut self,
        key: &Resource<T>,
    ) -> Result<&mut T, ResourceTableError> {
        self.get_any_mut(key.rep())?
            .downcast_mut()
            .ok_or(ResourceTableError::WrongType)
    }

    /// Returns the raw `Any` at the `key` index provided.
    pub fn get_any_mut(&mut self, key: u32) -> Result<&mut dyn Any, ResourceTableError> {
        let r = self.occupied_mut(key)?;
        Ok(&mut *r.entry)
    }

    /// Remove the specified entry from the table.
    pub fn delete<T>(&mut self, resource: Resource<T>) -> Result<T, ResourceTableError>
    where
        T: Any,
    {
        self.delete_maybe_debug(resource, DELETE_WITH_TOMBSTONE)
    }

    fn delete_maybe_debug<T>(
        &mut self,
        resource: Resource<T>,
        debug: bool,
    ) -> Result<T, ResourceTableError>
    where
        T: Any,
    {
        debug_assert!(resource.owned());
        let entry = self.delete_entry(resource.rep(), debug)?;
        match entry.entry.downcast() {
            Ok(t) => Ok(*t),
            Err(_e) => Err(ResourceTableError::WrongType),
        }
    }

    fn delete_entry(&mut self, key: u32, debug: bool) -> Result<TableEntry, ResourceTableError> {
        if !self.occupied(key)?.children.is_empty() {
            return Err(ResourceTableError::HasChildren);
        }
        let e = self.free_entry(key as usize, debug);
        if let Some(parent) = e.parent {
            // Remove deleted resource from parent's child list.
            // Parent must still be present because it can't be deleted while still having
            // children:
            self.occupied_mut(parent)
                .expect("missing parent")
                .remove_child(key);
        }
        Ok(e)
    }

    /// Zip the values of the map with mutable references to table entries corresponding to each
    /// key. As the keys in the `BTreeMap` are unique, this iterator can give mutable references
    /// with the same lifetime as the mutable reference to the [ResourceTable].
    pub fn iter_entries<'a, T>(
        &'a mut self,
        map: BTreeMap<u32, T>,
    ) -> impl Iterator<Item = (Result<&'a mut dyn Any, ResourceTableError>, T)> {
        map.into_iter().map(move |(k, v)| {
            let item = self
                .occupied_mut(k)
                .map(|e| Box::as_mut(&mut e.entry))
                // Safety: extending the lifetime of the mutable reference.
                .map(|item| unsafe { &mut *(item as *mut dyn Any) });
            (item, v)
        })
    }

    /// Iterate over all children belonging to the provided parent
    pub fn iter_children<T>(
        &self,
        parent: &Resource<T>,
    ) -> Result<impl Iterator<Item = &(dyn Any + Send)> + use<'_, T>, ResourceTableError>
    where
        T: 'static,
    {
        let parent_entry = self.occupied(parent.rep())?;
        Ok(parent_entry.children.iter().map(|child_index| {
            let child = self.occupied(*child_index).expect("missing child");
            child.entry.as_ref()
        }))
    }

    /// Iterate over all the entries in this table.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut (dyn Any + Send)> {
        self.entries.iter_mut().filter_map(|entry| match entry {
            Entry::Occupied { entry } => Some(&mut *entry.entry),
            Entry::Free { .. } => None,
        })
    }
}

impl Default for ResourceTable {
    fn default() -> Self {
        ResourceTable::new()
    }
}

impl fmt::Debug for ResourceTable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[")?;
        let mut wrote = false;
        for (index, entry) in self.entries.iter().enumerate() {
            if let Entry::Occupied { entry } = entry {
                if entry.entry.downcast_ref::<Tombstone>().is_none() {
                    if wrote {
                        write!(f, ", ")?;
                    } else {
                        wrote = true;
                    }
                    write!(f, "{index}")?;
                }
            }
        }
        write!(f, "]")
    }
}

#[test]
pub fn test_free_list() {
    let mut table = ResourceTable::new();

    let x = table.push(()).unwrap();
    assert_eq!(x.rep(), 0);

    let y = table.push(()).unwrap();
    assert_eq!(y.rep(), 1);

    // Deleting x should put it on the free list, so the next entry should have the same rep.
    table.delete_maybe_debug(x, false).unwrap();
    let x = table.push(()).unwrap();
    assert_eq!(x.rep(), 0);

    // Deleting x and then y should yield indices 1 and then 0 for new entries.
    table.delete_maybe_debug(x, false).unwrap();
    table.delete_maybe_debug(y, false).unwrap();

    let y = table.push(()).unwrap();
    assert_eq!(y.rep(), 1);

    let x = table.push(()).unwrap();
    assert_eq!(x.rep(), 0);

    // As the free list is empty, this entry will have a new id.
    let x = table.push(()).unwrap();
    assert_eq!(x.rep(), 2);
}
