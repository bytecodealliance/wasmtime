use std::any::Any;
use std::collections::{BTreeSet, HashMap};
use wasmtime::component::Resource;

#[derive(thiserror::Error, Debug)]
pub enum TableError {
    #[error("table has no free keys")]
    Full,
    #[error("value not present")]
    NotPresent,
    #[error("value is of another type")]
    WrongType,
    #[error("entry still has children")]
    HasChildren,
}

/// The `Table` type is designed to map u32 handles to resources. The table is now part of the
/// public interface to a `WasiCtx` - it is reference counted so that it can be shared beyond a
/// `WasiCtx` with other WASI proposals (e.g. `wasi-crypto` and `wasi-nn`) to manage their
/// resources. Elements in the `Table` are `Any` typed.
///
/// The `Table` type is intended to model how the Interface Types concept of Resources is shaping
/// up. Right now it is just an approximation.
#[derive(Debug)]
pub struct Table {
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

/// This structure tracks parent and child relationships for a given table entry.
///
/// Parents and children are referred to by table index. We maintain the
/// following invariants to prevent orphans and cycles:
/// * parent can only be assigned on creating the entry.
/// * parent, if some, must exist when creating the entry.
/// * whenever a child is created, its index is added to children.
/// * whenever a child is deleted, its index is removed from children.
/// * an entry with children may not be deleted.
#[derive(Debug)]
struct TableEntry {
    /// The entry in the table, as a boxed dynamically-typed object
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
        debug_assert!(!self.children.contains(&child));
        self.children.insert(child);
    }
    fn remove_child(&mut self, child: u32) {
        let was_removed = self.children.remove(&child);
        debug_assert!(was_removed);
    }
}

impl Table {
    /// Create an empty table
    pub fn new() -> Self {
        Table {
            entries: Vec::new(),
            free_head: None,
        }
    }

    /// Create an empty table with at least the specified capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Table {
            entries: Vec::with_capacity(capacity),
            free_head: None,
        }
    }

    /// Inserts a new value `T` into this table, returning a corresponding
    /// `Resource<T>` which can be used to refer to it after it was inserted.
    pub fn push<T>(&mut self, entry: T) -> Result<Resource<T>, TableError>
    where
        T: Send + Sync + 'static,
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
            Ok(free as u32)
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

    /// Insert a resource at the next available index, and track that it has a
    /// parent resource.
    ///
    /// The parent must exist to create a child. All children resources must
    /// be destroyed before a parent can be destroyed - otherwise [`Table::delete`]
    /// will fail with [`TableError::HasChildren`].
    ///
    /// Parent-child relationships are tracked inside the table to ensure that
    /// a parent resource is not deleted while it has live children. This
    /// allows child resources to hold "references" to a parent by table
    /// index, to avoid needing e.g. an `Arc<Mutex<parent>>` and the associated
    /// locking overhead and design issues, such as child existence extending
    /// lifetime of parent referent even after parent resource is destroyed,
    /// possibility for deadlocks.
    ///
    /// Parent-child relationships may not be modified once created. There
    /// is no way to observe these relationships through the [`Table`] methods
    /// except for erroring on deletion, or the [`std::fmt::Debug`] impl.
    pub fn push_child<T, U>(
        &mut self,
        entry: T,
        parent: &Resource<U>,
    ) -> Result<Resource<T>, TableError>
    where
        T: Send + Sync + 'static,
        U: 'static,
    {
        let parent = parent.rep();
        self.occupied(parent)?;
        let child = self.push_(TableEntry::new(Box::new(entry), Some(parent)))?;
        self.occupied_mut(parent)?.add_child(child);
        Ok(Resource::new_own(child))
    }

    /// Get an immutable reference to a resource of a given type at a given
    /// index.
    ///
    /// Multiple shared references can be borrowed at any given time.
    pub fn get<T: Any + Sized>(&self, key: &Resource<T>) -> Result<&T, TableError> {
        self.get_(key.rep())?
            .downcast_ref()
            .ok_or(TableError::WrongType)
    }

    fn get_(&self, key: u32) -> Result<&dyn Any, TableError> {
        let r = self.occupied(key)?;
        Ok(&*r.entry)
    }

    /// Get an mutable reference to a resource of a given type at a given
    /// index.
    pub fn get_mut<T: Any + Sized>(&mut self, key: &Resource<T>) -> Result<&mut T, TableError> {
        self.get_any_mut(key.rep())?
            .downcast_mut()
            .ok_or(TableError::WrongType)
    }

    /// Returns the raw `Any` at the `key` index provided.
    pub fn get_any_mut(&mut self, key: u32) -> Result<&mut dyn Any, TableError> {
        let r = self.occupied_mut(key)?;
        Ok(&mut *r.entry)
    }

    /// Same as `delete`, but typed
    pub fn delete<T>(&mut self, resource: Resource<T>) -> Result<T, TableError>
    where
        T: Any,
    {
        debug_assert!(resource.owned());
        let entry = self.delete_entry(resource.rep())?;
        match entry.entry.downcast() {
            Ok(t) => Ok(*t),
            Err(_e) => Err(TableError::WrongType),
        }
    }

    fn delete_entry(&mut self, key: u32) -> Result<TableEntry, TableError> {
        if !self.occupied(key)?.children.is_empty() {
            return Err(TableError::HasChildren);
        }
        let e = self.free_entry(key as usize);
        if let Some(parent) = e.parent {
            // Remove deleted resource from parent's child list.
            // Parent must still be present because it cant be deleted while still having
            // children:
            self.occupied_mut(parent)
                .expect("missing parent")
                .remove_child(key);
        }
        Ok(e)
    }

    /// Zip the values of the map with mutable references to table entries corresponding to each
    /// key. As the keys in the [HashMap] are unique, this iterator can give mutable references
    /// with the same lifetime as the mutable reference to the [Table].
    pub fn iter_entries<'a, T>(
        &'a mut self,
        map: HashMap<u32, T>,
    ) -> impl Iterator<Item = (Result<&'a mut dyn Any, TableError>, T)> {
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
    ) -> Result<impl Iterator<Item = &(dyn Any + Send + Sync)>, TableError>
    where
        T: 'static,
    {
        let parent_entry = self.occupied(parent.rep())?;
        Ok(parent_entry.children.iter().map(|child_index| {
            let child = self.occupied(*child_index).expect("missing child");
            child.entry.as_ref()
        }))
    }
}

impl Default for Table {
    fn default() -> Self {
        Table::new()
    }
}

#[test]
pub fn test_free_list() {
    let mut table = Table::new();

    let x = table.push(()).unwrap();
    assert_eq!(x.rep(), 0);

    let y = table.push(()).unwrap();
    assert_eq!(y.rep(), 1);

    // Deleting x should put it on the free list, so the next entry should have the same rep.
    table.delete(x).unwrap();
    let x = table.push(()).unwrap();
    assert_eq!(x.rep(), 0);

    // Deleting x and then y should yield indices 1 and then 0 for new entries.
    table.delete(x).unwrap();
    table.delete(y).unwrap();

    let y = table.push(()).unwrap();
    assert_eq!(y.rep(), 1);

    let x = table.push(()).unwrap();
    assert_eq!(x.rep(), 0);

    // As the free list is empty, this entry will have a new id.
    let x = table.push(()).unwrap();
    assert_eq!(x.rep(), 2);
}
