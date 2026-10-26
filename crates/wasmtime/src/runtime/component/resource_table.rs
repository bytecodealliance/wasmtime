use super::Resource;
use super::resources::{FixedHostHeapUsage, HostHeapUsage};
use crate::prelude::*;
use alloc::collections::{BTreeMap, BTreeSet};
use core::any::Any;
use core::fmt;
use core::marker::PhantomData;
use core::mem;
use sealed::{DefaultTombstone, DeleteWithTombstone};

/// Errors returned by operations on `ResourceTable`
#[derive(Debug)]
#[non_exhaustive]
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
    /// The host heap memory limit for this table has been exceeded.
    HostMemoryLimitExceeded,
}

impl fmt::Display for ResourceTableError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Full => write!(f, "resource table has no free keys"),
            Self::NotPresent => write!(f, "resource not present"),
            Self::WrongType => write!(f, "resource is of another type"),
            Self::HasChildren => write!(f, "resource has children"),
            Self::HostMemoryLimitExceeded => {
                write!(f, "host heap memory limit exceeded for resource table")
            }
        }
    }
}

impl core::error::Error for ResourceTableError {}

/// The `ResourceTable` type maps a `Resource<T>` to its `T`.
pub struct ResourceTable<D: DeleteWithTombstone = DefaultTombstone> {
    entries: Vec<Entry>,
    free_head: Option<usize>,
    // Number of reusable slots on the free list, excluding debug tombstones.
    free_count: usize,
    max_capacity: usize,
    live: usize,
    /// The current total host heap usage, in bytes, of all resources in the table.
    current_host_heap_usage: usize,
    /// An optional maximum limit on total host heap usage, in bytes. If `Some`,
    /// any `push` or `update` that would exceed this limit will result in
    /// [`ResourceTableError::HostMemoryLimitExceeded`].
    max_host_heap_usage: Option<usize>,
    _tombstone: PhantomData<D>,
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

mod sealed {
    /// Configures whether `ResourceTable` leaves tombstones on delete.
    ///
    /// This trait is public but unnameable outside of this module, so it can't
    /// be implemented elsewhere.
    pub trait DeleteWithTombstone {
        const DELETE_WITH_TOMBSTONE: bool;
    }

    pub struct DefaultTombstone;

    impl DeleteWithTombstone for DefaultTombstone {
        // Change this to `true` to assist with handle debugging in development if
        // necessary.
        const DELETE_WITH_TOMBSTONE: bool = false;
    }
}

/// Default setting for `ResourceTable::max_capacity`, chosen to be high
/// enough that it doesn't need changing all that often but low enough that
/// exhausting it isn't a massive problem for the host.
const DEFAULT_MAX_CAPACITY: usize = 1_000_000;

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
    /// The host heap usage, in bytes, currently charged to the table for this
    /// entry.
    ///
    /// `update` uses this to compute the change in usage and `delete` releases
    /// it, so the table total stays consistent even if the entry was mutated
    /// through an untracked path such as `get_any_mut`.
    host_heap_usage: usize,
}

impl TableEntry {
    fn new(entry: Box<dyn Any + Send>, parent: Option<u32>, host_heap_usage: usize) -> Self {
        Self {
            entry,
            parent,
            children: BTreeSet::new(),
            host_heap_usage,
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

#[inline]
fn total_host_size<T: HostHeapUsage + ?Sized>(val: &T) -> Result<usize, ResourceTableError> {
    size_of_val(val)
        .checked_add(val.heap_size_of_children())
        .ok_or(ResourceTableError::HostMemoryLimitExceeded)
}

impl ResourceTable {
    /// Create an empty table
    pub fn new() -> Self {
        ResourceTable::with_capacity(0)
    }

    /// Create an empty table with at least the specified capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        ResourceTable::empty(capacity)
    }
}

impl<D: DeleteWithTombstone> ResourceTable<D> {
    fn empty(capacity: usize) -> Self {
        ResourceTable {
            entries: Vec::with_capacity(capacity),
            free_head: None,
            free_count: 0,
            max_capacity: DEFAULT_MAX_CAPACITY,
            live: 0,
            current_host_heap_usage: 0,
            max_host_heap_usage: None,
            _tombstone: PhantomData,
        }
    }

    /// Returns the number of live resources in this table.
    ///
    /// Deleted resources, including debug tombstones, are not counted. This is
    /// an `O(1)` operation.
    pub fn len(&self) -> usize {
        self.live
    }

    /// Returns whether or not this table is empty in `O(1)` time.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Checks that a batch of pushes has enough free or appendable slots.
    ///
    /// Free slots are reused even if the maximum capacity has been lowered below
    /// the number of backing slots.
    #[cfg(any(feature = "component-model-async", test))]
    pub(crate) fn ensure_capacity(&self, additional: usize) -> Result<(), ResourceTableError> {
        let Some(to_append) = additional.checked_sub(self.free_count) else {
            return Ok(());
        };
        if to_append > self.max_capacity.saturating_sub(self.entries.len()) {
            Err(ResourceTableError::Full)
        } else {
            Ok(())
        }
    }

    /// Returns the maximum capacity of this table, in elements, before adding
    /// any more will be refused.
    pub fn max_capacity(&self) -> usize {
        self.max_capacity
    }

    /// Configures the maximum number of entries that may be present within this
    /// table.
    ///
    /// Note that this does not retroactively shrink the table nor evict
    /// existing entries should the maximum be smaller than the current size of
    /// the entry table.
    pub fn set_max_capacity(&mut self, max: usize) {
        self.max_capacity = max;
    }

    /// Returns the configured maximum host heap usage, in bytes, if any.
    pub fn max_host_heap_usage(&self) -> Option<usize> {
        self.max_host_heap_usage
    }

    /// Set the maximum allowed total host heap usage for this table.
    ///
    /// This option provides an umbrella mechanism for protecting hosts against
    /// guests that may try to make calls that allocate excessive host memory.
    /// Once the limit is hit for a given table, the guest will trap. Hosts may
    /// monitor the current usage via [`Self::current_host_heap_usage`].
    ///
    /// Pass `None` to remove any previously set limit.
    pub fn set_max_host_heap_usage(&mut self, max: Option<usize>) {
        self.max_host_heap_usage = max;
    }

    /// Total host heap usage, in bytes, of resources in this table.
    pub fn current_host_heap_usage(&self) -> usize {
        self.current_host_heap_usage
    }

    /// Charge `bytes` to the table, failing if that would exceed the limit.
    fn reserve(&mut self, bytes: usize) -> Result<(), ResourceTableError> {
        let new_usage = self
            .current_host_heap_usage
            .checked_add(bytes)
            .ok_or(ResourceTableError::HostMemoryLimitExceeded)?;
        if self.max_host_heap_usage.is_some_and(|max| new_usage > max) {
            return Err(ResourceTableError::HostMemoryLimitExceeded);
        }
        self.current_host_heap_usage = new_usage;
        Ok(())
    }

    /// Release `bytes` previously charged to the table.
    fn release(&mut self, bytes: usize) {
        debug_assert!(bytes <= self.current_host_heap_usage);
        self.current_host_heap_usage = self.current_host_heap_usage.saturating_sub(bytes);
    }

    /// Inserts a new value `T` into this table, returning a corresponding
    /// `Resource<T>` which can be used to refer to it after it was inserted.
    pub fn push<T>(&mut self, entry: T) -> Result<Resource<T>, ResourceTableError>
    where
        T: Send + 'static + HostHeapUsage,
    {
        let idx = self.push_with_parent(entry, None)?;
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
            self.free_count -= 1;
            Some(ix)
        } else {
            None
        }
    }

    /// Free an entry in the table, returning its [`TableEntry`]. Add the index to the free list.
    #[inline]
    fn free_entry(&mut self, ix: usize) -> TableEntry {
        let removed_entry = if D::DELETE_WITH_TOMBSTONE {
            // Instead of making this entry available for reuse, we leave a
            // tombstone in debug mode.  This helps detect use-after-delete and
            // double-delete bugs.
            match mem::replace(
                &mut self.entries[ix],
                Entry::Occupied {
                    entry: TableEntry::new(Box::new(Tombstone), None, 0),
                },
            ) {
                Entry::Occupied { entry } => {
                    if !entry.entry.is::<Tombstone>() {
                        self.live -= 1;
                    }
                    entry
                }
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
            self.free_count += 1;
            self.live -= 1;

            entry
        };
        self.release(removed_entry.host_heap_usage);
        removed_entry
    }

    fn push_with_parent<T>(
        &mut self,
        entry: T,
        parent: Option<u32>,
    ) -> Result<u32, ResourceTableError>
    where
        T: Send + 'static + HostHeapUsage,
    {
        let usage = total_host_size(&entry)?;
        self.reserve(usage)?;
        let e = TableEntry::new(Box::new(entry), parent, usage);
        self.push_(e).inspect_err(|_| self.release(usage))
    }

    /// Insert an entry into the table, returning its handle. This will prefer to use free entries
    /// if they exist, falling back on pushing new entries onto the end of the table.
    fn push_(&mut self, e: TableEntry) -> Result<u32, ResourceTableError> {
        if let Some(free) = self.pop_free_list() {
            self.entries[free] = Entry::Occupied { entry: e };
            self.live += 1;
            Ok(free.try_into().unwrap())
        } else {
            if self.entries.len() >= self.max_capacity {
                return Err(ResourceTableError::Full);
            }
            let ix = self
                .entries
                .len()
                .try_into()
                .map_err(|_| ResourceTableError::Full)?;
            self.entries.push(Entry::Occupied { entry: e });
            self.live += 1;
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
        T: Send + 'static + HostHeapUsage,
        U: 'static,
    {
        let parent = parent.rep();
        self.occupied(parent)?;
        let child = self.push_with_parent(entry, Some(parent))?;
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

    /// Get a mutable reference to a resource of a given type at a given index.
    ///
    /// This method is only available for types that implement
    /// [`FixedHostHeapUsage`]. For types whose heap footprint can vary with
    /// mutation, use [`update`](ResourceTable::update) instead.
    pub fn get_mut<T: Any + Sized + FixedHostHeapUsage>(
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

    /// Mutate a variable-size resource in place and recompute its host heap
    /// usage.
    ///
    /// Use this method for types that implement [`HostHeapUsage`] without
    /// [`FixedHostHeapUsage`] (for example, types with owned `Vec`, `String`,
    /// or `HashMap` fields). For fixed-size types, use [`get_mut`](ResourceTable::get_mut).
    ///
    /// After `updater` returns, the resource's size is recomputed and the
    /// table's running total is updated. If the resource grew and the table
    /// total exceeds the configured maximum,
    /// [`ResourceTableError::HostMemoryLimitExceeded`] is returned (and should
    /// be treated as a guest trap by host bindings; see
    /// [`set_max_host_heap_usage`](ResourceTable::set_max_host_heap_usage) for
    /// soft-limit behavior).
    ///
    /// ```
    /// use wasmtime::component::{HostHeapUsage, ResourceTable};
    ///
    /// #[derive(HostHeapUsage)]
    /// struct Item {
    ///     payload: Option<Vec<u8>>,
    /// }
    ///
    /// let mut table = ResourceTable::new();
    /// let id = table.push(Item { payload: Some(vec![1, 2, 3]) })?;
    /// let taken = table.update(&id, |r| r.payload.take())?;
    /// assert_eq!(taken, Some(vec![1, 2, 3]));
    /// # Ok::<(), wasmtime::component::ResourceTableError>(())
    /// ```
    pub fn update<T, F, R>(
        &mut self,
        resource: &Resource<T>,
        updater: F,
    ) -> Result<R, ResourceTableError>
    where
        T: Any + Sized + HostHeapUsage,
        F: FnOnce(&mut T) -> R,
    {
        let current_total = self.current_host_heap_usage;
        let entry = self.occupied_mut(resource.rep())?;
        let t = entry
            .entry
            .downcast_mut::<T>()
            .ok_or(ResourceTableError::WrongType)?;
        let result = updater(t);
        let raw_new_usage = total_host_size(t);
        let old_usage = entry.host_heap_usage;
        let other_usage = current_total - old_usage;
        let max_add = usize::MAX - other_usage;
        let (new_usage, overflowed) = match raw_new_usage {
            Ok(u) if u <= max_add => (u, false),
            Ok(_) | Err(_) => (max_add, true),
        };
        entry.host_heap_usage = new_usage;
        self.current_host_heap_usage = other_usage + new_usage;
        if overflowed
            || (new_usage > old_usage
                && self
                    .max_host_heap_usage
                    .is_some_and(|max| self.current_host_heap_usage > max))
        {
            return Err(ResourceTableError::HostMemoryLimitExceeded);
        }
        Ok(result)
    }

    /// Remove the specified entry from the table.
    pub fn delete<T>(&mut self, resource: Resource<T>) -> Result<T, ResourceTableError>
    where
        T: Any,
    {
        debug_assert!(resource.owned());
        let key = resource.rep();
        if !self.occupied(key)?.children.is_empty() {
            return Err(ResourceTableError::HasChildren);
        }
        let entry = self.free_entry(key as usize);
        if let Some(parent) = entry.parent {
            // Remove deleted resource from parent's child list.
            // Parent must still be present because it can't be deleted while still having
            // children:
            self.occupied_mut(parent)
                .expect("missing parent")
                .remove_child(key);
        }
        match entry.entry.downcast() {
            Ok(t) => Ok(*t),
            Err(_e) => Err(ResourceTableError::WrongType),
        }
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
    ) -> Result<impl Iterator<Item = &(dyn Any + Send)> + use<'_, T, D>, ResourceTableError>
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
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (u32, &mut (dyn Any + Send))> {
        self.entries
            .iter_mut()
            .enumerate()
            .filter_map(|(rep, entry)| match entry {
                Entry::Occupied { entry } => Some((u32::try_from(rep).unwrap(), &mut *entry.entry)),
                Entry::Free { .. } => None,
            })
    }
}

impl Default for ResourceTable {
    fn default() -> Self {
        ResourceTable::new()
    }
}

impl<D: DeleteWithTombstone> fmt::Debug for ResourceTable<D> {
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
    assert!(table.is_empty());

    let x = table.push(()).unwrap();
    assert_eq!(x.rep(), 0);
    assert_eq!(table.len(), 1);

    let y = table.push(()).unwrap();
    assert_eq!(y.rep(), 1);
    assert_eq!(table.len(), 2);

    // Deleting x should put it on the free list, so the next entry should have the same rep.
    table.delete(x).unwrap();
    assert_eq!(table.len(), 1);
    let x = table.push(()).unwrap();
    assert_eq!(x.rep(), 0);
    assert_eq!(table.len(), 2);

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
    assert_eq!(table.len(), 3);
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Tracked(usize);
    impl HostHeapUsage for Tracked {
        fn heap_size_of_children(&self) -> usize {
            self.0
        }
    }

    const TRACKED_INLINE: usize = size_of::<Tracked>();

    struct Fixed(u64);
    impl FixedHostHeapUsage for Fixed {}

    #[test]
    fn test_heap_usage_push_child_and_delete() {
        let mut table = ResourceTable::new();
        assert_eq!(table.current_host_heap_usage(), 0);

        let parent = table.push(Tracked(100)).unwrap();
        assert_eq!(table.current_host_heap_usage(), TRACKED_INLINE + 100);

        let child = table.push_child(Tracked(200), &parent).unwrap();
        assert_eq!(table.current_host_heap_usage(), 2 * TRACKED_INLINE + 300);

        table.delete(child).unwrap();
        assert_eq!(table.current_host_heap_usage(), TRACKED_INLINE + 100);

        table.delete(parent).unwrap();
        assert_eq!(table.current_host_heap_usage(), 0);

        // Fixed-size resources mutated via get_mut keep constant accounting.
        let fixed_size = size_of::<Fixed>();
        let r = table.push(Fixed(1)).unwrap();
        assert_eq!(table.current_host_heap_usage(), fixed_size);
        table.get_mut(&r).unwrap().0 = 42;
        assert_eq!(table.current_host_heap_usage(), fixed_size);
        table.delete(r).unwrap();
        assert_eq!(table.current_host_heap_usage(), 0);
    }

    #[test]
    fn test_heap_usage_limits() {
        let mut table = ResourceTable::new();
        let entry_size = TRACKED_INLINE + 100;
        table.set_max_host_heap_usage(Some(2 * entry_size));
        assert_eq!(table.max_host_heap_usage(), Some(2 * entry_size));

        let parent = table.push(Tracked(100)).unwrap();
        let child = table.push_child(Tracked(100), &parent).unwrap();
        assert_eq!(table.current_host_heap_usage(), 2 * entry_size);

        // Exceeding the limit fails for both push and push_child.
        assert!(matches!(
            table.push(Tracked(1)),
            Err(ResourceTableError::HostMemoryLimitExceeded)
        ));
        assert!(matches!(
            table.push_child(Tracked(1), &parent),
            Err(ResourceTableError::HostMemoryLimitExceeded)
        ));
        assert_eq!(table.current_host_heap_usage(), 2 * entry_size);

        // Removing the limit allows further pushes.
        table.set_max_host_heap_usage(None);
        assert_eq!(table.max_host_heap_usage(), None);
        let extra = table.push(Tracked(50)).unwrap();
        assert_eq!(
            table.current_host_heap_usage(),
            2 * entry_size + TRACKED_INLINE + 50
        );

        table.delete(extra).unwrap();
        table.delete(child).unwrap();
        table.delete(parent).unwrap();
        assert_eq!(table.current_host_heap_usage(), 0);
    }

    #[test]
    fn test_update() {
        let mut table = ResourceTable::new();
        table.set_max_host_heap_usage(Some(TRACKED_INLINE + 250));

        let r = table.push(Tracked(100)).unwrap();
        assert_eq!(table.current_host_heap_usage(), TRACKED_INLINE + 100);

        // Grow and shrink within the limit.
        table.update(&r, |t| t.0 = 200).unwrap();
        assert_eq!(table.current_host_heap_usage(), TRACKED_INLINE + 200);
        table.update(&r, |t| t.0 = 50).unwrap();
        assert_eq!(table.current_host_heap_usage(), TRACKED_INLINE + 50);

        // Growing past the limit returns HostMemoryLimitExceeded while keeping
        // the post-mutation size recorded so delete releases the exact amount.
        let err = table.update(&r, |t| t.0 = 300).unwrap_err();
        assert!(matches!(err, ResourceTableError::HostMemoryLimitExceeded));
        assert_eq!(table.current_host_heap_usage(), TRACKED_INLINE + 300);

        // Shrinking while over the limit succeeds.
        table.update(&r, |t| t.0 = 280).unwrap();
        assert_eq!(table.current_host_heap_usage(), TRACKED_INLINE + 280);

        table.delete(r).unwrap();
        assert_eq!(table.current_host_heap_usage(), 0);
    }

    #[test]
    fn test_heap_usage_error_rollbacks() {
        let mut table = ResourceTable::new();
        table.set_max_capacity(2);
        table.set_max_host_heap_usage(Some(1000));

        let a = table.push(Tracked(100)).unwrap();
        let b = table.push(Tracked(100)).unwrap();
        assert_eq!(table.current_host_heap_usage(), 2 * (TRACKED_INLINE + 100));

        // Push when table is full rolls back reserved usage.
        assert!(matches!(
            table.push(Tracked(100)),
            Err(ResourceTableError::Full)
        ));
        assert_eq!(table.current_host_heap_usage(), 2 * (TRACKED_INLINE + 100));

        // Untracked mutation via get_any_mut still releases original charged bytes.
        table
            .get_any_mut(a.rep())
            .unwrap()
            .downcast_mut::<Tracked>()
            .unwrap()
            .0 = 150;
        table.delete(a).unwrap();
        assert_eq!(table.current_host_heap_usage(), TRACKED_INLINE + 100);

        // Wrong-type delete still releases the entry's charged bytes.
        let wrong = Resource::<Fixed>::new_own(b.rep());
        assert!(matches!(
            table.delete(wrong),
            Err(ResourceTableError::WrongType)
        ));
        assert_eq!(table.current_host_heap_usage(), 0);

        // push_child with missing parent does not leak usage.
        let stale = Resource::<Tracked>::new_own(b.rep());
        assert!(matches!(
            table.push_child(Tracked(100), &stale),
            Err(ResourceTableError::NotPresent)
        ));
        assert_eq!(table.current_host_heap_usage(), 0);
    }
}

#[test]
fn test_max_capacity() {
    let mut table = ResourceTable::new();
    assert_eq!(table.max_capacity(), DEFAULT_MAX_CAPACITY);

    table.set_max_capacity(0);
    assert_eq!(table.max_capacity(), 0);
    assert!(table.push(()).is_err());

    table.set_max_capacity(1);
    assert_eq!(table.max_capacity(), 1);
    let x = table.push(()).unwrap();
    assert!(table.push(()).is_err());
    assert_eq!(table.len(), 1);

    table.set_max_capacity(0);
    assert!(table.push(()).is_err());
    table.delete(x).unwrap();
    let x = table.push(()).unwrap();
    table.delete(x).unwrap();

    table.set_max_capacity(10);

    let handles = (0..10).map(|_| table.push(()).unwrap()).collect::<Vec<_>>();
    assert!(table.push(()).is_err());
    for handle in handles {
        table.delete(handle).unwrap();
    }
    assert!(table.is_empty());

    table.push(()).unwrap();
}

#[test]
fn test_len_and_delete_errors() {
    let mut table = ResourceTable::new();
    let parent = table.push(()).unwrap();
    let child = table.push_child(1u32, &parent).unwrap();
    assert_eq!(table.len(), 2);
    table.set_max_capacity(2);
    assert!(matches!(
        table.push_child(2u32, &parent),
        Err(ResourceTableError::Full)
    ));
    assert_eq!(table.len(), 2);
    assert!(matches!(
        table.delete(Resource::<()>::new_own(parent.rep())),
        Err(ResourceTableError::HasChildren)
    ));
    assert_eq!(table.len(), 2);
    assert!(matches!(
        table.push_child((), &Resource::<()>::new_own(100)),
        Err(ResourceTableError::NotPresent)
    ));
    assert_eq!(table.len(), 2);
    assert!(matches!(
        table.delete(Resource::<()>::new_own(child.rep())),
        Err(ResourceTableError::WrongType)
    ));
    assert_eq!(table.len(), 1);
    assert!(matches!(
        table.delete(Resource::<()>::new_own(100)),
        Err(ResourceTableError::NotPresent)
    ));
    assert_eq!(table.len(), 1);
    table.delete(parent).unwrap();
    assert!(table.is_empty());
}

#[test]
fn test_len_and_tombstones() {
    struct DebugTombstone;
    impl DeleteWithTombstone for DebugTombstone {
        const DELETE_WITH_TOMBSTONE: bool = true;
    }

    let mut table = ResourceTable::<DebugTombstone>::empty(0);
    table.set_max_capacity(1);
    let value = table.push(1u32).unwrap();
    let rep = value.rep();
    table.delete(value).unwrap();
    assert_eq!(table.len(), 0);
    assert!(table.is_empty());
    assert_eq!(table.entries.len(), 1);
    assert!(matches!(
        table.ensure_capacity(1),
        Err(ResourceTableError::Full)
    ));
    assert!(matches!(
        table.delete(Resource::<u32>::new_own(rep)),
        Err(ResourceTableError::WrongType)
    ));
    assert_eq!(table.len(), 0);
    assert!(table.is_empty());
}

#[test]
fn test_batch_capacity() {
    let mut table = ResourceTable::new();
    table.set_max_capacity(3);
    table.ensure_capacity(0).unwrap();
    table.ensure_capacity(3).unwrap();
    assert!(matches!(
        table.ensure_capacity(4),
        Err(ResourceTableError::Full)
    ));
    assert_eq!(table.entries.len(), 0);
    let first = table.push(()).unwrap();
    table.ensure_capacity(2).unwrap();
    assert!(matches!(
        table.ensure_capacity(3),
        Err(ResourceTableError::Full)
    ));
    table.delete(first).unwrap();
    // A batch can combine free slots and new slots.
    table.ensure_capacity(3).unwrap();
    let handles = (0..3).map(|_| table.push(()).unwrap()).collect::<Vec<_>>();
    for handle in handles {
        table.delete(handle).unwrap();
    }
    table.set_max_capacity(0);
    // Reusing slots remains valid even after lowering the maximum capacity.
    table.ensure_capacity(3).unwrap();
    assert!(matches!(
        table.ensure_capacity(4),
        Err(ResourceTableError::Full)
    ));
    assert_eq!(table.len(), 0);
    assert_eq!(table.entries.len(), 3);
}
