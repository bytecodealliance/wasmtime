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
    map: HashMap<u32, TableEntry>,
    next_key: u32,
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
            map: HashMap::new(),
            // 0, 1 and 2 are formerly (preview 1) for stdio. To prevent users from assuming these
            // indicies are still valid ways to access stdio, they are deliberately left empty.
            // Once we have a full implementation of resources, this confusion should hopefully be
            // impossible :)
            next_key: 3,
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

    fn push_(&mut self, e: TableEntry) -> Result<u32, TableError> {
        // NOTE: The performance of this new key calculation could be very bad once keys wrap
        // around.
        if self.map.len() == u32::MAX as usize {
            return Err(TableError::Full);
        }
        loop {
            let key = self.next_key;
            self.next_key = self.next_key.wrapping_add(1);
            if self.map.contains_key(&key) {
                continue;
            }
            self.map.insert(key, e);
            return Ok(key);
        }
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
        let idx = self.push_child_(Box::new(entry), parent.rep())?;
        Ok(Resource::new_own(idx))
    }

    fn push_child_(
        &mut self,
        entry: Box<dyn Any + Send + Sync>,
        parent: u32,
    ) -> Result<u32, TableError> {
        if !self.map.contains_key(&parent) {
            return Err(TableError::NotPresent);
        }
        let child = self.push_(TableEntry::new(entry, Some(parent)))?;
        self.map
            .get_mut(&parent)
            .expect("parent existence assured above")
            .add_child(child);
        Ok(child)
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
        let r = self.map.get(&key).ok_or(TableError::NotPresent)?;
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
        let r = self.map.get_mut(&key).ok_or(TableError::NotPresent)?;
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
        if !self
            .map
            .get(&key)
            .ok_or(TableError::NotPresent)?
            .children
            .is_empty()
        {
            return Err(TableError::HasChildren);
        }
        let e = self.map.remove(&key).unwrap();
        if let Some(parent) = e.parent {
            // Remove deleted resource from parent's child list.
            // Parent must still be present because it cant be deleted while still having
            // children:
            self.map
                .get_mut(&parent)
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
                .map
                .get_mut(&k)
                .map(|e| Box::as_mut(&mut e.entry))
                // Safety: extending the lifetime of the mutable reference.
                .map(|item| unsafe { &mut *(item as *mut dyn Any) })
                .ok_or(TableError::NotPresent);
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
        let parent_entry = self.map.get(&parent.rep()).ok_or(TableError::NotPresent)?;
        Ok(parent_entry.children.iter().map(|child_index| {
            let child = self.map.get(child_index).expect("missing child");
            child.entry.as_ref()
        }))
    }
}

impl Default for Table {
    fn default() -> Self {
        Table::new()
    }
}
