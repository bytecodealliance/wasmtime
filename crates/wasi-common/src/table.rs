use crate::{Error, ErrorExt};
use std::any::Any;
use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;

pub struct Table {
    map: HashMap<u32, RefCell<Box<dyn Any>>>,
    next_key: u32,
}

impl Table {
    pub fn new() -> Self {
        Table {
            map: HashMap::new(),
            next_key: 3, // 0, 1 and 2 are reserved for stdio
        }
    }

    pub fn insert_at(&mut self, key: u32, a: Box<dyn Any>) {
        self.map.insert(key, RefCell::new(a));
    }

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

    pub fn contains_key(&self, key: u32) -> bool {
        self.map.contains_key(&key)
    }

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

    pub fn delete(&mut self, key: u32) -> Option<Box<dyn Any>> {
        self.map.remove(&key).map(|rc| RefCell::into_inner(rc))
    }

    pub fn update_in_place<T, F>(&mut self, key: u32, f: F) -> Result<(), Error>
    where
        T: Any + Sized,
        F: FnOnce(T) -> Result<T, Error>,
    {
        let entry = self.delete(key).ok_or(Error::badf())?;
        let downcast = entry
            .downcast::<T>()
            .map_err(|_| Error::exist().context("element is a different type"))?;
        let new = f(*downcast)?;
        self.insert_at(key, Box::new(new));
        Ok(())
    }
}
