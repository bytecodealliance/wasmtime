use std::any::Any;
use std::collections::HashMap;

#[derive(thiserror::Error, Debug)]
pub enum TableError {
    #[error("table has no free keys")]
    Full,
    #[error("value not present")]
    NotPresent,
    #[error("value is of another type")]
    WrongType,
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
    pub(crate) map: HashMap<u32, Box<dyn Any + Send + Sync>>,
    next_key: u32,
}

impl Table {
    /// Create an empty table. New insertions will begin at 3, above stdio.
    pub fn new() -> Self {
        Table {
            map: HashMap::new(),
            next_key: 3, // 0, 1 and 2 are reserved for stdio
        }
    }

    /// Insert a resource at the next available index.
    pub fn push(&mut self, a: Box<dyn Any + Send + Sync>) -> Result<u32, TableError> {
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
            self.map.insert(key, a);
            return Ok(key);
        }
    }

    /// Check if the table has a resource at the given index.
    pub fn contains_key(&self, key: u32) -> bool {
        self.map.contains_key(&key)
    }

    /// Check if the resource at a given index can be downcast to a given type.
    /// Note: this will always fail if the resource is already borrowed.
    pub fn is<T: Any + Sized>(&self, key: u32) -> bool {
        if let Some(r) = self.map.get(&key) {
            r.is::<T>()
        } else {
            false
        }
    }

    /// Get an immutable reference to a resource of a given type at a given index. Multiple
    /// immutable references can be borrowed at any given time. Borrow failure
    /// results in a trapping error.
    pub fn get<T: Any + Sized>(&self, key: u32) -> Result<&T, TableError> {
        if let Some(r) = self.map.get(&key) {
            r.downcast_ref::<T>().ok_or_else(|| TableError::WrongType)
        } else {
            Err(TableError::NotPresent)
        }
    }

    /// Get a mutable reference to a resource of a given type at a given index. Only one mutable
    /// reference can be borrowed at any given time. Borrow failure results in a trapping error.
    pub fn get_mut<T: Any + Sized>(&mut self, key: u32) -> Result<&mut T, TableError> {
        if let Some(r) = self.map.get_mut(&key) {
            r.downcast_mut::<T>().ok_or_else(|| TableError::WrongType)
        } else {
            Err(TableError::NotPresent)
        }
    }

    /// Get an [`std::collections::hash_map::OccupiedEntry`] corresponding to
    /// a table entry, if it exists. This allows you to remove or replace the
    /// entry based on its contents.
    pub fn entry(
        &mut self,
        key: u32,
    ) -> Result<
        std::collections::hash_map::OccupiedEntry<u32, Box<dyn Any + Send + Sync + 'static>>,
        TableError,
    > {
        use std::collections::hash_map::Entry;
        match self.map.entry(key) {
            Entry::Occupied(occ) => Ok(occ),
            Entry::Vacant(_) => Err(TableError::NotPresent),
        }
    }

    /// Remove a resource at a given index from the table.
    pub fn delete<T: Any + Sized>(&mut self, key: u32) -> Result<T, TableError> {
        // Optimistically attempt to remove the value stored under key
        match self
            .map
            .remove(&key)
            .ok_or(TableError::NotPresent)?
            .downcast::<T>()
        {
            Ok(v) => Ok(*v),
            Err(v) => {
                // Insert the value back, since the downcast failed
                self.map.insert(key, v);
                Err(TableError::WrongType)
            }
        }
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
                .map(Box::as_mut)
                // Safety: extending the lifetime of the mutable reference.
                .map(|item| unsafe { &mut *(item as *mut dyn Any) })
                .ok_or(TableError::NotPresent);
            (item, v)
        })
    }
}
