use crate::{Error, ErrorExt};
use std::any::Any;
use std::collections::HashMap;

/// The `Table` type is designed to map u32 handles to resources. The table is now part of the
/// public interface to a `WasiCtx` - it is reference counted so that it can be shared beyond a
/// `WasiCtx` with other WASI proposals (e.g. `wasi-crypto` and `wasi-nn`) to manage their
/// resources. Elements in the `Table` are `Any` typed.
///
/// The `Table` type is intended to model how the Interface Types concept of Resources is shaping
/// up. Right now it is just an approximation.
pub struct Table {
    map: HashMap<u32, Box<dyn Any + Send + Sync>>,
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

    /// Insert a resource at a certain index.
    pub fn insert_at(&mut self, key: u32, a: Box<dyn Any + Send + Sync>) {
        self.map.insert(key, a);
    }

    /// Insert a resource at the next available index.
    pub fn push(&mut self, a: Box<dyn Any + Send + Sync>) -> Result<u32, Error> {
        // NOTE: The performance of this new key calculation could be very bad once keys wrap
        // around.
        if self.map.len() == u32::MAX as usize {
            return Err(Error::trap("table has no free keys"));
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
    pub fn get<T: Any + Sized>(&self, key: u32) -> Result<&T, Error> {
        if let Some(r) = self.map.get(&key) {
            r.downcast_ref::<T>()
                .ok_or_else(|| Error::badf().context("element is a different type"))
        } else {
            Err(Error::badf().context("key not in table"))
        }
    }

    /// Get a mutable reference to a resource of a given type at a given index. Only one mutable
    /// reference can be borrowed at any given time. Borrow failure results in a trapping error.
    pub fn get_mut<T: Any + Sized>(&mut self, key: u32) -> Result<&mut T, Error> {
        if let Some(r) = self.map.get_mut(&key) {
            r.downcast_mut::<T>()
                .ok_or_else(|| Error::badf().context("element is a different type"))
        } else {
            Err(Error::badf().context("key not in table"))
        }
    }

    /// Remove a resource at a given index from the table. Returns the resource
    /// if it was present.
    pub fn delete(&mut self, key: u32) -> Option<Box<dyn Any + Send + Sync>> {
        self.map.remove(&key)
    }
}
