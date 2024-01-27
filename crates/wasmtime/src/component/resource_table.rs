use super::Resource;
use std::any::{Any, TypeId};
use std::collections::BTreeSet;

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
    /// Resource has been temporarily taken from the table.
    Taken,
    /// Resource is not taken. This can happen when attempting to restore a resource
    /// that has already been restored, or was never taken in the first place.
    NotTaken,
}

impl std::fmt::Display for ResourceTableError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Full => write!(f, "resource table has no free keys"),
            Self::NotPresent => write!(f, "resource not present"),
            Self::WrongType => write!(f, "resource is of another type"),
            Self::HasChildren => write!(f, "resource has children"),
            Self::Taken => write!(f, "resource is taken"),
            Self::NotTaken => write!(f, "resource is not taken"),
        }
    }
}
impl std::error::Error for ResourceTableError {}

/// The `ResourceTable` type maps a `Resource<T>` to its `T`.
#[derive(Debug)]
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

#[derive(Debug)]
enum Slot {
    /// The resource is present in the table, ready for use.
    Present(Box<dyn Any + Send>),
    /// The resource is temporarily taken out for external mutation.
    /// To ensure we're getting back the same type of resource as the one we've
    /// handed out, we remember the TypeId of the data and validate it on restore.
    Taken(TypeId),
}

impl Slot {
    fn get(&self) -> Result<&(dyn Any + Send + 'static), ResourceTableError> {
        match self {
            Slot::Present(data) => Ok(data.as_ref()),
            Slot::Taken(_) => Err(ResourceTableError::Taken),
        }
    }

    fn get_mut(&mut self) -> Result<&mut (dyn Any + Send + 'static), ResourceTableError> {
        match self {
            Slot::Present(data) => Ok(data.as_mut()),
            Slot::Taken(_) => Err(ResourceTableError::Taken),
        }
    }

    fn take(&mut self) -> Result<Box<dyn Any + Send>, ResourceTableError> {
        let (result, replacement) = match std::mem::replace(self, Slot::Taken(TypeId::of::<()>())) {
            Slot::Present(data) => {
                let type_id = data.as_ref().type_id();
                (Ok(data), Slot::Taken(type_id))
            }
            Slot::Taken(id) => (Err(ResourceTableError::Taken), Slot::Taken(id)),
        };
        *self = replacement;
        result
    }

    fn restore(&mut self, data: Box<dyn Any + Send>) -> Result<(), ResourceTableError> {
        let (result, replacement) = match std::mem::replace(self, Slot::Taken(TypeId::of::<()>())) {
            Slot::Present(data) => (Err(ResourceTableError::NotTaken), Slot::Present(data)),
            Slot::Taken(id) if id == data.as_ref().type_id() => (Ok(()), Slot::Present(data)),
            Slot::Taken(id) => (Err(ResourceTableError::WrongType), Slot::Taken(id)),
        };
        *self = replacement;
        result
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
    /// The entry in the table.
    slot: Slot,
    /// The index of the parent of this entry, if it has one.
    parent: Option<u32>,
    /// The indicies of any children of this entry.
    children: BTreeSet<u32>,
}

impl TableEntry {
    fn new(entry: Box<dyn Any + Send>, parent: Option<u32>) -> Self {
        Self {
            slot: Slot::Present(entry),
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
    fn push_(&mut self, e: TableEntry) -> Result<u32, ResourceTableError> {
        if let Some(free) = self.pop_free_list() {
            self.entries[free] = Entry::Occupied { entry: e };
            Ok(free as u32)
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
    ///
    /// Parent-child relationships may not be modified once created. There
    /// is no way to observe these relationships through the [`ResourceTable`]
    /// methods except for erroring on deletion, or the [`std::fmt::Debug`]
    /// impl.
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
        Ok(r.slot.get()?)
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
        Ok(r.slot.get_mut()?)
    }

    /// Same as `delete`, but typed
    pub fn delete<T>(&mut self, resource: Resource<T>) -> Result<T, ResourceTableError>
    where
        T: Any,
    {
        debug_assert!(resource.owned());
        let data = self.delete_entry(resource.rep())?;
        match data.downcast() {
            Ok(t) => Ok(*t),
            Err(_e) => Err(ResourceTableError::WrongType),
        }
    }

    fn delete_entry(&mut self, key: u32) -> Result<Box<dyn Any + Send>, ResourceTableError> {
        let entry = self.occupied_mut(key)?;
        if !entry.children.is_empty() {
            return Err(ResourceTableError::HasChildren);
        }
        let data = entry.slot.take()?;
        let e = self.free_entry(key as usize);
        if let Some(parent) = e.parent {
            // Remove deleted resource from parent's child list.
            // Parent must still be present because it cant be deleted while still having
            // children:
            self.occupied_mut(parent)
                .expect("missing parent")
                .remove_child(key);
        }
        Ok(data)
    }

    /// Iterate over all children belonging to the provided parent.
    /// This returns an iterator of results, because some children may be taken.
    pub fn iter_children<T>(
        &self,
        parent: &Resource<T>,
    ) -> Result<
        impl Iterator<Item = Result<&(dyn Any + Send), ResourceTableError>>,
        ResourceTableError,
    >
    where
        T: 'static,
    {
        let parent_entry = self.occupied(parent.rep())?;
        Ok(parent_entry.children.iter().map(|child_index| {
            let child = self.occupied(*child_index).expect("missing child");
            child.slot.get()
        }))
    }

    /// Temporarily take the resource out of the table.
    ///
    /// This is an advanced operation to allow mutating resources independent of
    /// the table's mutable reference lifetime. For simple access to the resource,
    /// try [ResourceTable::get_mut] instead.
    ///
    /// Unlike deleting the resource and pushing it back in, this method retains
    /// the resource's index in the table and the parent/children relationships.
    ///
    /// While a resource is taken out, any attempt to access that resource's
    /// index through the table returns [ResourceTableError::Taken]. It's the
    /// caller's responsibility to put the resource back in using [ResourceTable::restore].
    pub fn take<T>(&mut self, resource: &Resource<T>) -> Result<Box<T>, ResourceTableError>
    where
        T: Any + Send + 'static,
    {
        let entry = self.occupied_mut(resource.rep())?;
        let data = entry.slot.take()?;
        match data.downcast() {
            Ok(data) => Ok(data),
            Err(data) => {
                entry.slot.restore(data).expect("resource was just taken");
                Err(ResourceTableError::WrongType)
            }
        }
    }

    /// Put the resource back into the table.
    pub fn restore<T>(
        &mut self,
        resource: &Resource<T>,
        data: Box<T>,
    ) -> Result<(), ResourceTableError>
    where
        T: Any + Send + 'static,
    {
        let entry = self.occupied_mut(resource.rep())?;
        entry.slot.restore(data)?;
        Ok(())
    }
}

impl Default for ResourceTable {
    fn default() -> Self {
        ResourceTable::new()
    }
}

#[test]
fn test_free_list() {
    let mut table = ResourceTable::new();

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

#[test]
fn test_take_restore() {
    let mut table = ResourceTable::new();
    let a_u32: Resource<u32> = table.push(42).unwrap();
    let a_f64: Resource<f64> = Resource::new_borrow(a_u32.rep());

    table.take(&a_u32).unwrap();

    assert!(matches!(table.get(&a_f64), Err(ResourceTableError::Taken)));

    assert!(matches!(
        table.restore(&a_f64, Box::new(42f64)),
        Err(ResourceTableError::WrongType)
    ));
    table.restore(&a_u32, Box::new(42u32)).unwrap();
    assert!(matches!(
        table.restore(&a_u32, Box::new(42u32)),
        Err(ResourceTableError::NotTaken)
    ));
}
