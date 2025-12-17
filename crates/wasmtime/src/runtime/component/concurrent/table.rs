use crate::component::Resource;
use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

/// Represents a `ResourceTable` entry for a `waitable` or `waitable-set`.
///
/// This is equivalent to a `Resource<T>`, except without any tracking of borrow
/// status (since neither `waitable`s nor `waitable-set`s can be borrowed) or
/// other resource-specific bookkeeping.
pub struct TableId<T> {
    rep: u32,
    _marker: PhantomData<fn() -> T>,
}

pub trait TableDebug {
    fn type_name() -> &'static str;
}

impl<T: TableDebug> fmt::Debug for TableId<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}({})", T::type_name(), self.rep)
    }
}

impl<T> Hash for TableId<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.rep.hash(state)
    }
}

impl<T> PartialEq for TableId<T> {
    fn eq(&self, other: &Self) -> bool {
        self.rep == other.rep
    }
}

impl<T> Eq for TableId<T> {}

impl<T> PartialOrd for TableId<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.rep.partial_cmp(&other.rep)
    }
}

impl<T> Ord for TableId<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.rep.cmp(&other.rep)
    }
}

impl<T> TableId<T> {
    pub fn new(rep: u32) -> Self {
        Self {
            rep,
            _marker: PhantomData,
        }
    }
}

impl<T> Clone for TableId<T> {
    fn clone(&self) -> Self {
        Self::new(self.rep)
    }
}

impl<T> Copy for TableId<T> {}

impl<T> TableId<T> {
    pub fn rep(&self) -> u32 {
        self.rep
    }
}

impl<T: 'static> From<Resource<T>> for TableId<T> {
    fn from(value: Resource<T>) -> Self {
        Self {
            rep: value.rep(),
            _marker: PhantomData,
        }
    }
}

impl<T: 'static> From<TableId<T>> for Resource<T> {
    fn from(value: TableId<T>) -> Self {
        Resource::new_own(value.rep)
    }
}
