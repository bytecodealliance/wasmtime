use super::Resource;
use std::any::Any;
use std::collections::{BTreeSet, HashMap};

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

impl std::fmt::Display for ResourceTableError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Full => write!(f, "resource table has no free keys"),
            Self::NotPresent => write!(f, "resource not present"),
            Self::WrongType => write!(f, "resource is of another type"),
            Self::HasChildren => write!(f, "resource has children"),
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

#[derive(Debug, PartialEq, Eq)]
struct SlotIdentity(Option<std::ptr::NonNull<dyn Any + Send>>);

// SAFETY: the pointer is never dereferenced.
unsafe impl Send for SlotIdentity {}

impl SlotIdentity {
    fn from(data: &Box<dyn Any + Send>) -> SlotIdentity {
        let as_const = data.as_ref() as *const (dyn Any + Send);
        let as_mut = as_const as *mut (dyn Any + Send);
        Self(std::ptr::NonNull::new(as_mut))
    }

    fn none() -> SlotIdentity {
        Self(None)
    }
}

#[derive(Debug)]
enum Slot {
    /// The resource is present in the table, ready for use.
    Present(Box<dyn Any + Send>),
    /// The resource is temporarily leased out for external mutation.
    /// To ensure we're getting back the same resource as the one we've handed
    /// out, we remember the raw address of the box and check for pointer
    /// equality on restore.
    LeasedOut(SlotIdentity),
}

impl Slot {
    fn unwrap_ref(&self) -> &(dyn Any + Send + 'static) {
        match self {
            Slot::Present(data) => data.as_ref(),
            Slot::LeasedOut(_) => panic!("taken"),
        }
    }

    fn unwrap_mut(&mut self) -> &mut (dyn Any + Send + 'static) {
        match self {
            Slot::Present(data) => data.as_mut(),
            Slot::LeasedOut(_) => panic!("taken"),
        }
    }

    fn take(&mut self) -> Box<dyn Any + Send> {
        match std::mem::replace(self, Slot::LeasedOut(SlotIdentity::none())) {
            Slot::Present(data) => {
                *self = Slot::LeasedOut(SlotIdentity::from(&data));
                data
            }
            Slot::LeasedOut(_) => panic!("already taken"),
        }
    }

    fn restore(&mut self, data: Box<dyn Any + Send>) {
        match std::mem::replace(self, Slot::LeasedOut(SlotIdentity::none())) {
            Slot::Present(_) => panic!("already present"),
            Slot::LeasedOut(id) => {
                assert_eq!(
                    id,
                    SlotIdentity::from(&data),
                    "expecting different resource"
                );
                *self = Slot::Present(data);
            }
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

/// Represents temporary ownership of an entry in the [ResourceTable].
/// For more information, see [ResourceTable::take].
///
/// # Panics
/// To prevent silent memory leaks, dropping a lease without manually handing it
/// back using [ResourceTable::restore] is considered a logic error and panics.
#[must_use]
#[derive(Debug)]
pub struct Lease<T: 'static>(Option<(Resource<T>, Box<T>)>);

impl<T> Lease<T> {
    fn new(resource: Resource<T>, data: Box<T>) -> Self {
        Self(Some((resource, data)))
    }

    fn destruct(mut self) -> (Resource<T>, Box<T>) {
        self.0.take().unwrap()
    }
}

impl<T> Drop for Lease<T> {
    fn drop(&mut self) {
        if self.0.is_some() && !std::thread::panicking() {
            panic!("lease dropped unexpectedly")
        }
    }
}

impl<T> AsRef<T> for Lease<T> {
    fn as_ref(&self) -> &T {
        &self.0.as_ref().unwrap().1
    }
}

impl<T> AsMut<T> for Lease<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.0.as_mut().unwrap().1
    }
}

impl<T> std::borrow::Borrow<T> for Lease<T> {
    fn borrow(&self) -> &T {
        self.as_ref()
    }
}

impl<T> std::borrow::BorrowMut<T> for Lease<T> {
    fn borrow_mut(&mut self) -> &mut T {
        self.as_mut()
    }
}

impl<T> std::ops::Deref for Lease<T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.as_ref()
    }
}

impl<T> std::ops::DerefMut for Lease<T> {
    fn deref_mut(&mut self) -> &mut T {
        self.as_mut()
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
        Ok(r.slot.unwrap_ref())
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
        Ok(r.slot.unwrap_mut())
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
        let data = entry.slot.take();
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

    /// Zip the values of the map with mutable references to table entries corresponding to each
    /// key. As the keys in the [HashMap] are unique, this iterator can give mutable references
    /// with the same lifetime as the mutable reference to the [ResourceTable].
    pub fn iter_entries<'a, T>(
        &'a mut self,
        map: HashMap<u32, T>,
    ) -> impl Iterator<Item = (Result<&'a mut dyn Any, ResourceTableError>, T)> {
        map.into_iter().map(move |(k, v)| {
            let item = self
                .occupied_mut(k)
                .map(|e| e.slot.unwrap_mut())
                // Safety: extending the lifetime of the mutable reference.
                .map(|item| unsafe { &mut *(item as *mut dyn Any) });
            (item, v)
        })
    }

    /// Iterate over all children belonging to the provided parent
    pub fn iter_children<T>(
        &self,
        parent: &Resource<T>,
    ) -> Result<impl Iterator<Item = &(dyn Any + Send)>, ResourceTableError>
    where
        T: 'static,
    {
        let parent_entry = self.occupied(parent.rep())?;
        Ok(parent_entry.children.iter().map(|child_index| {
            let child = self.occupied(*child_index).expect("missing child");
            child.slot.unwrap_ref()
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
    /// # Panics
    /// - It's the caller's responsibility to put the resource back using
    ///   [ResourceTable::restore]. Dropping the Lease without doing so will panic.
    /// - While a resource is leased out, any attempt to access that resource's
    ///   index through the table is considered a logic error and will panic.
    pub fn take<T>(&mut self, resource: Resource<T>) -> Result<Lease<T>, ResourceTableError>
    where
        T: Any + Send + 'static,
    {
        let entry = self.occupied_mut(resource.rep())?;
        match entry.slot.take().downcast() {
            Ok(data) => Ok(Lease::new(resource, data)),
            Err(data) => {
                entry.slot.restore(data);
                Err(ResourceTableError::WrongType)
            }
        }
    }

    /// Put the resource back into the table. This returns the resource handle
    /// originally passed to [ResourceTable::take].
    ///
    /// # Panics
    /// Panics when the provided lease did not originate from this table.
    pub fn restore<T>(&mut self, lease: Lease<T>) -> Resource<T>
    where
        T: Any + Send + 'static,
    {
        let (resource, data) = lease.destruct();
        let entry = self.occupied_mut(resource.rep()).expect("wrong table");
        entry.slot.restore(data);
        resource
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
fn test_slot_identity() {
    let a: Box<dyn Any + Send> = Box::new(42u32);
    let b: Box<dyn Any + Send> = Box::new(42u32);

    assert_eq!(SlotIdentity::from(&a), SlotIdentity::from(&a));
    assert_ne!(SlotIdentity::from(&a), SlotIdentity::from(&b));
}

#[test]
fn test_slot() {
    let mut a = Slot::Present(Box::new(42u32));
    let _ = a.unwrap_ref();
    let _ = a.unwrap_mut();
    let a_data = a.take();
    assert_eq!(*a_data.downcast_ref::<u32>().unwrap(), 42u32);
    a.restore(a_data);
    let _ = a.unwrap_ref();
    let _ = a.unwrap_mut();
}

#[test]
fn test_take_restore() {
    let mut table = ResourceTable::new();
    let a = table.push(()).unwrap();
    let a_rep = a.rep();
    let l = table.take(a).unwrap();
    table.push(()).unwrap();
    let a = table.restore(l);
    assert_eq!(a.rep(), a_rep);
}

#[test]
#[should_panic]
fn test_get_taken() {
    let mut table = ResourceTable::new();
    let a = table.push(42u32).unwrap();
    let a_bad: Resource<u32> = Resource::new_borrow(a.rep());
    let _ = table.take(a).unwrap();

    // Should panic:
    let _ = table.get(&a_bad);
}

#[test]
#[should_panic]
fn test_restore_wrong_table() {
    let mut table_a = ResourceTable::new();
    let mut table_b = ResourceTable::new();

    let a = table_a.push(42u32).unwrap();
    let b = table_b.push(42u32).unwrap();

    let lease_a = table_a.take(a).unwrap();
    let lease_b = table_b.take(b).unwrap();

    // Should panic:
    table_a.restore(lease_b);
    table_b.restore(lease_a);
}

#[test]
#[should_panic]
fn test_lease_drop() {
    let mut table = ResourceTable::new();
    let a = table.push(()).unwrap();
    let lease_a = table.take(a).unwrap();

    // Should panic:
    drop(lease_a)
}
