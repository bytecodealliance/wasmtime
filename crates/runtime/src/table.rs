//! Memory management for tables.
//!
//! `Table` is to WebAssembly tables what `LinearMemory` is to WebAssembly linear memories.

use crate::vmcontext::{VMCallerCheckedAnyfunc, VMTableDefinition};
use std::cell::RefCell;
use std::convert::{TryFrom, TryInto};
use wasmtime_environ::wasm::TableElementType;
use wasmtime_environ::{TablePlan, TableStyle};

/// A table instance.
#[derive(Debug)]
pub struct Table {
    vec: RefCell<Vec<VMCallerCheckedAnyfunc>>,
    maximum: Option<u32>,
}

impl Table {
    /// Create a new table instance with specified minimum and maximum number of elements.
    pub fn new(plan: &TablePlan) -> Self {
        match plan.table.ty {
            TableElementType::Func => (),
            TableElementType::Val(ty) => {
                unimplemented!("tables of types other than anyfunc ({})", ty)
            }
        };
        match plan.style {
            TableStyle::CallerChecksSignature => Self {
                vec: RefCell::new(vec![
                    VMCallerCheckedAnyfunc::default();
                    usize::try_from(plan.table.minimum).unwrap()
                ]),
                maximum: plan.table.maximum,
            },
        }
    }

    /// Returns the number of allocated elements.
    pub fn size(&self) -> u32 {
        self.vec.borrow().len().try_into().unwrap()
    }

    /// Grow table by the specified amount of elements.
    ///
    /// Returns `None` if table can't be grown by the specified amount
    /// of elements.
    pub fn grow(&self, delta: u32) -> Option<u32> {
        let new_len = match self.size().checked_add(delta) {
            Some(len) => {
                if let Some(max) = self.maximum {
                    if len > max {
                        return None;
                    }
                }
                len
            }
            None => {
                return None;
            }
        };
        self.vec.borrow_mut().resize(
            usize::try_from(new_len).unwrap(),
            VMCallerCheckedAnyfunc::default(),
        );
        Some(new_len)
    }

    /// Get reference to the specified element.
    ///
    /// Returns `None` if the index is out of bounds.
    pub fn get(&self, index: u32) -> Option<VMCallerCheckedAnyfunc> {
        self.vec.borrow().get(index as usize).cloned()
    }

    /// Set reference to the specified element.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of bounds.
    pub fn set(&self, index: u32, func: VMCallerCheckedAnyfunc) -> Result<(), ()> {
        match self.vec.borrow_mut().get_mut(index as usize) {
            Some(slot) => {
                *slot = func;
                Ok(())
            }
            None => Err(()),
        }
    }

    /// Return a `VMTableDefinition` for exposing the table to compiled wasm code.
    pub fn vmtable(&self) -> VMTableDefinition {
        let mut vec = self.vec.borrow_mut();
        VMTableDefinition {
            base: vec.as_mut_ptr() as *mut u8,
            current_elements: vec.len().try_into().unwrap(),
        }
    }
}
