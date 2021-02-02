use crate::{Error, ErrorExt};
use std::any::Any;
use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;

/// The `Table` type is designed to map u32 handles to resources. The table is now part of the
/// public interface to a `WasiCtx` - it is reference counted so that it can be shared beyond a
/// `WasiCtx` with other WASI proposals (e.g. `wasi-crypto` and `wasi-nn`) to manage their
/// resources. Elements in the `Table` are `Any` typed.
///
/// The `Table` type is intended to model how the Interface Types concept of Resources is shaping
/// up. Right now it is just an approximation.
pub struct Table {
    map: HashMap<u32, RefCell<Box<dyn Any>>>,
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
    pub fn insert_at(&mut self, key: u32, a: Box<dyn Any>) {
        self.map.insert(key, RefCell::new(a));
    }

    /// Insert a resource at the next available index.
    pub fn push(&mut self, a: Box<dyn Any>) -> Result<u32, Error> {
        loop {
            let key = self.next_key;
            // XXX this is not correct. The table may still have empty entries, but our
            // linear search strategy is quite bad
            self.next_key = self
                .next_key
                .checked_add(1)
                .ok_or_else(|| Error::trap("out of keys in table"))?;
            if self.map.contains_key(&key) {
                continue;
            }
            self.map.insert(key, RefCell::new(a));
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
        if let Some(refcell) = self.map.get(&key) {
            if let Ok(refmut) = refcell.try_borrow_mut() {
                refmut.is::<T>()
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Get an immutable reference to a resource of a given type at a given index. Multiple
    /// immutable references can be borrowed at any given time. Borrow failure
    /// results in a trapping error.
    pub fn get<T: Any + Sized>(&self, key: u32) -> Result<Ref<T>, Error> {
        if let Some(refcell) = self.map.get(&key) {
            if let Ok(r) = refcell.try_borrow() {
                if r.is::<T>() {
                    Ok(Ref::map(r, |r| r.downcast_ref::<T>().unwrap()))
                } else {
                    Err(Error::badf().context("element is a different type"))
                }
            } else {
                Err(Error::trap("table get of mutably borrowed element"))
            }
        } else {
            Err(Error::badf().context("key not in table"))
        }
    }

    /// Get a mutable reference to a resource of a given type at a given index. Only one mutable
    /// reference can be borrowed at any given time. Borrow failure results in a trapping error.
    pub fn get_mut<T: Any + Sized>(&self, key: u32) -> Result<RefMut<T>, Error> {
        if let Some(refcell) = self.map.get(&key) {
            if let Ok(r) = refcell.try_borrow_mut() {
                if r.is::<T>() {
                    Ok(RefMut::map(r, |r| r.downcast_mut::<T>().unwrap()))
                } else {
                    Err(Error::badf().context("element is a different type"))
                }
            } else {
                Err(Error::trap("table get_mut of borrowed element"))
            }
        } else {
            Err(Error::badf().context("key not in table"))
        }
    }

    /// Remove a resource at a given index from the table. Returns the resource
    /// if it was present.
    pub fn delete(&mut self, key: u32) -> Option<Box<dyn Any>> {
        self.map.remove(&key).map(|rc| RefCell::into_inner(rc))
    }
}
