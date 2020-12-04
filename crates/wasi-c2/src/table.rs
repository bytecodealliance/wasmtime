use crate::Error;
use std::any::Any;
use std::cell::{RefCell, RefMut};
use std::collections::HashMap;

pub struct Table {
    map: HashMap<u32, RefCell<Box<dyn Any>>>,
    next_key: u32,
}

impl Table {
    pub fn new() -> Self {
        Table {
            map: HashMap::new(),
            next_key: 0,
        }
    }

    pub fn insert_at(&mut self, key: u32, a: impl Any + Sized) {
        self.map.insert(key, RefCell::new(Box::new(a)));
    }

    pub fn push(&mut self, a: impl Any + Sized) -> u32 {
        loop {
            let key = self.next_key;
            self.next_key += 1;
            if self.map.contains_key(&key) {
                continue;
            }
            self.map.insert(key, RefCell::new(Box::new(a)));
            return key;
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
    // Todo: we can refine these errors and translate them to Exist at abi
    pub fn get<T: Any + Sized>(&self, key: u32) -> Result<RefMut<T>, Error> {
        if let Some(refcell) = self.map.get(&key) {
            if let Ok(refmut) = refcell.try_borrow_mut() {
                if refmut.is::<T>() {
                    Ok(RefMut::map(refmut, |r| r.downcast_mut::<T>().unwrap()))
                } else {
                    Err(Error::Exist) // Exists at another type
                }
            } else {
                Err(Error::Exist) // Does exist, but borrowed
            }
        } else {
            Err(Error::Exist) // Does not exist
        }
    }

    pub fn delete(&mut self, key: u32) -> Option<Box<dyn Any>> {
        self.map.remove(&key).map(|rc| RefCell::into_inner(rc))
    }
}
