//! Memory management for tables.
//!
//! `Table` is to WebAssembly tables what `LinearMemory` is to WebAssembly linear memories.

use crate::vmcontext::{VMCallerCheckedAnyfunc, VMTableDefinition};
use crate::Trap;
use std::cell::RefCell;
use std::convert::{TryFrom, TryInto};
use wasmtime_environ::wasm::TableElementType;
use wasmtime_environ::{ir, TablePlan, TableStyle};

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
    /// of elements. Returns the previous size of the table if growth is
    /// successful.
    pub fn grow(&self, delta: u32) -> Option<u32> {
        let size = self.size();
        let new_len = match size.checked_add(delta) {
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
        Some(size)
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

    /// Copy `len` elements from `src_table[src_index..]` into `dst_table[dst_index..]`.
    ///
    /// # Errors
    ///
    /// Returns an error if the range is out of bounds of either the source or
    /// destination tables.
    pub fn copy(
        dst_table: &Self,
        src_table: &Self,
        dst_index: u32,
        src_index: u32,
        len: u32,
    ) -> Result<(), Trap> {
        // https://webassembly.github.io/bulk-memory-operations/core/exec/instructions.html#exec-table-copy

        if src_index
            .checked_add(len)
            .map_or(true, |n| n > src_table.size())
            || dst_index
                .checked_add(len)
                .map_or(true, |m| m > dst_table.size())
        {
            return Err(Trap::wasm(ir::TrapCode::TableOutOfBounds));
        }

        let srcs = src_index..src_index + len;
        let dsts = dst_index..dst_index + len;

        // Note on the unwraps: the bounds check above means that these will
        // never panic.
        //
        // TODO(#983): investigate replacing this get/set loop with a `memcpy`.
        if dst_index <= src_index {
            for (s, d) in (srcs).zip(dsts) {
                dst_table.set(d, src_table.get(s).unwrap()).unwrap();
            }
        } else {
            for (s, d) in srcs.rev().zip(dsts.rev()) {
                dst_table.set(d, src_table.get(s).unwrap()).unwrap();
            }
        }

        Ok(())
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
