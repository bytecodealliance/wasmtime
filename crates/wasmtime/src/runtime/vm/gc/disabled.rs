//! Dummy GC types for when the `gc` cargo feature is disabled.

use super::VMGcRef;

pub enum VMExternRef {}

pub enum VMStructRef {}

pub enum VMArrayRef {}

pub enum VMExnRef {}

impl VMGcRef {
    pub fn into_structref_unchecked(self) -> VMStructRef {
        unreachable!()
    }

    pub fn into_exnref_unchecked(self) -> VMExnRef {
        unreachable!()
    }
}

impl From<VMStructRef> for VMGcRef {
    fn from(s: VMStructRef) -> VMGcRef {
        match s {}
    }
}

impl From<VMExnRef> for VMGcRef {
    fn from(e: VMExnRef) -> VMGcRef {
        match e {}
    }
}
