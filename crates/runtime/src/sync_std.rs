#![allow(missing_docs)]

use once_cell::sync::OnceCell;
use std::ops::{Deref, DerefMut};

pub struct OnceLock<T>(OnceCell<T>);

impl<T> OnceLock<T> {
    pub const fn new() -> OnceLock<T> {
        OnceLock(OnceCell::new())
    }

    pub fn get_or_init(&self, f: impl FnOnce() -> T) -> &T {
        self.0.get_or_init(f)
    }

    pub fn get_or_try_init<E>(&self, f: impl FnOnce() -> Result<T, E>) -> Result<&T, E> {
        self.0.get_or_try_init(f)
    }
}

#[derive(Debug, Default)]
pub struct RwLock<T>(std::sync::RwLock<T>);

impl<T> RwLock<T> {
    pub const fn new(val: T) -> RwLock<T> {
        RwLock(std::sync::RwLock::new(val))
    }

    pub fn read(&self) -> impl Deref<Target = T> + '_ {
        self.0.read().unwrap()
    }

    pub fn write(&self) -> impl DerefMut<Target = T> + '_ {
        self.0.write().unwrap()
    }
}
