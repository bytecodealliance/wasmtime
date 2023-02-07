use crate::{Error, ErrorExt};
use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// The `Table` type is designed to map u32 handles to resources. The table is now part of the
/// public interface to a `WasiCtx` - it is reference counted so that it can be shared beyond a
/// `WasiCtx` with other WASI proposals (e.g. `wasi-crypto` and `wasi-nn`) to manage their
/// resources. Elements in the `Table` are `Any` typed.
///
/// The `Table` type is intended to model how the Interface Types concept of Resources is shaping
/// up. Right now it is just an approximation.
pub struct Table(RwLock<Inner>);

struct Inner {
    map: HashMap<u32, Arc<dyn Any + Send + Sync>>,
    next_key: u32,
}

impl Table {
    /// Create an empty table. New insertions will begin at 3, above stdio.
    pub fn new() -> Self {
        Table(RwLock::new(Inner {
            map: HashMap::new(),
            next_key: 3, // 0, 1 and 2 are reserved for stdio
        }))
    }

    /// Insert a resource at a certain index.
    pub fn insert_at<T: Any + Send + Sync>(&self, key: u32, a: Arc<T>) {
        self.0.write().unwrap().map.insert(key, a);
    }

    /// Insert a resource at the next available index.
    pub fn push<T: Any + Send + Sync>(&self, a: Arc<T>) -> Result<u32, Error> {
        let mut inner = self.0.write().unwrap();
        // NOTE: The performance of this new key calculation could be very bad once keys wrap
        // around.
        if inner.map.len() == u32::MAX as usize {
            return Err(Error::trap(anyhow::Error::msg("table has no free keys")));
        }
        loop {
            let key = inner.next_key;
            inner.next_key += 1;
            if inner.map.contains_key(&key) {
                continue;
            }
            inner.map.insert(key, a);
            return Ok(key);
        }
    }

    /// Check if the table has a resource at the given index.
    pub fn contains_key(&self, key: u32) -> bool {
        self.0.read().unwrap().map.contains_key(&key)
    }

    /// Check if the resource at a given index can be downcast to a given type.
    /// Note: this will always fail if the resource is already borrowed.
    pub fn is<T: Any + Sized>(&self, key: u32) -> bool {
        if let Some(r) = self.0.read().unwrap().map.get(&key) {
            r.is::<T>()
        } else {
            false
        }
    }

    /// Get an Arc reference to a resource of a given type at a given index. Multiple
    /// immutable references can be borrowed at any given time.
    pub fn get<T: Any + Send + Sync + Sized>(&self, key: u32) -> Result<Arc<T>, Error> {
        if let Some(r) = self.0.read().unwrap().map.get(&key).cloned() {
            r.downcast::<T>()
                .map_err(|_| Error::badf().context("element is a different type"))
        } else {
            Err(Error::badf().context("key not in table"))
        }
    }

    /// Get a mutable reference to a resource of a given type at a given index.
    /// Only one such reference can be borrowed at any given time.
    pub fn get_mut<T: Any>(&mut self, key: u32) -> Result<&mut T, Error> {
        let entry = match self.0.get_mut().unwrap().map.get_mut(&key) {
            Some(entry) => entry,
            None => return Err(Error::badf().context("key not in table")),
        };
        let entry = match Arc::get_mut(entry) {
            Some(entry) => entry,
            None => return Err(Error::badf().context("cannot mutably borrow shared file")),
        };
        entry
            .downcast_mut::<T>()
            .ok_or_else(|| Error::badf().context("element is a different type"))
    }

    /// Remove a resource at a given index from the table. Returns the resource
    /// if it was present.
    pub fn delete<T: Any + Send + Sync>(&self, key: u32) -> Option<Arc<T>> {
        self.0
            .write()
            .unwrap()
            .map
            .remove(&key)
            .map(|r| r.downcast::<T>().unwrap())
    }

    /// Remove a resource at a given index from the table. Returns the resource
    /// if it was present.
    pub fn renumber(&self, from: u32, to: u32) -> Result<(), Error> {
        let map = &mut self.0.write().unwrap().map;
        let from_entry = map.remove(&from).ok_or(Error::badf())?;
        map.insert(to, from_entry);
        Ok(())
    }
}
