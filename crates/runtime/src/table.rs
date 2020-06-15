//! Memory management for tables.
//!
//! `Table` is to WebAssembly tables what `LinearMemory` is to WebAssembly linear memories.

use crate::vmcontext::{VMCallerCheckedAnyfunc, VMTableDefinition};
use crate::{Trap, VMExternRef};
use std::cell::RefCell;
use std::convert::{TryFrom, TryInto};
use wasmtime_environ::wasm::TableElementType;
use wasmtime_environ::{ir, TablePlan, TableStyle};

/// A table instance.
#[derive(Debug)]
pub struct Table {
    elements: RefCell<TableElements>,
    maximum: Option<u32>,
}

/// An element going into or coming out of a table.
#[derive(Clone, Debug)]
pub enum TableElement {
    /// A `funcref`.
    FuncRef(VMCallerCheckedAnyfunc),
    /// An `exrernref`.
    ExternRef(Option<VMExternRef>),
}

#[derive(Debug)]
enum TableElements {
    FuncRefs(Vec<VMCallerCheckedAnyfunc>),
    ExternRefs(Vec<Option<VMExternRef>>),
}

impl Table {
    /// Create a new table instance with specified minimum and maximum number of elements.
    pub fn new(plan: &TablePlan) -> Self {
        let elements =
            RefCell::new(match plan.table.ty {
                TableElementType::Func => TableElements::FuncRefs(vec![
                    VMCallerCheckedAnyfunc::default();
                    usize::try_from(plan.table.minimum).unwrap()
                ]),
                TableElementType::Val(ty)
                    if (cfg!(target_pointer_width = "64") && ty == ir::types::R64)
                        || (cfg!(target_pointer_width = "32") && ty == ir::types::R32) =>
                {
                    let min = usize::try_from(plan.table.minimum).unwrap();
                    TableElements::ExternRefs(vec![None; min])
                }
                TableElementType::Val(ty) => unimplemented!("unsupported table type ({})", ty),
            });
        match plan.style {
            TableStyle::CallerChecksSignature => Self {
                elements,
                maximum: plan.table.maximum,
            },
        }
    }

    /// Returns the number of allocated elements.
    pub fn size(&self) -> u32 {
        match &*self.elements.borrow() {
            TableElements::FuncRefs(x) => x.len().try_into().unwrap(),
            TableElements::ExternRefs(x) => x.len().try_into().unwrap(),
        }
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
        let new_len = usize::try_from(new_len).unwrap();
        match &mut *self.elements.borrow_mut() {
            TableElements::FuncRefs(x) => x.resize(new_len, VMCallerCheckedAnyfunc::default()),
            TableElements::ExternRefs(x) => x.resize(new_len, None),
        }
        Some(size)
    }

    /// Get reference to the specified element.
    ///
    /// Returns `None` if the index is out of bounds.
    pub fn get(&self, index: u32) -> Option<TableElement> {
        match &*self.elements.borrow() {
            TableElements::FuncRefs(x) => x.get(index as usize).cloned().map(TableElement::FuncRef),
            TableElements::ExternRefs(x) => {
                x.get(index as usize).cloned().map(TableElement::ExternRef)
            }
        }
    }

    /// Set reference to the specified element.
    ///
    /// # Errors
    ///
    /// Returns an error if `index` is out of bounds or if this table type does
    /// not match the element type.
    pub fn set(&self, index: u32, elem: TableElement) -> Result<(), ()> {
        let mut elems = self.elements.borrow_mut();
        match &mut *elems {
            TableElements::FuncRefs(x) => {
                let slot = x.get_mut(index as usize).ok_or(())?;
                *slot = elem.try_into().or(Err(()))?;
            }
            TableElements::ExternRefs(x) => {
                let slot = x.get_mut(index as usize).ok_or(())?;
                *slot = elem.try_into().or(Err(()))?;
            }
        }
        Ok(())
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
        match &*self.elements.borrow() {
            TableElements::FuncRefs(x) => VMTableDefinition {
                base: x.as_ptr() as *const u8 as *mut u8,
                current_elements: x.len().try_into().unwrap(),
            },
            TableElements::ExternRefs(x) => VMTableDefinition {
                base: x.as_ptr() as *const u8 as *mut u8,
                current_elements: x.len().try_into().unwrap(),
            },
        }
    }
}

impl TryFrom<TableElement> for VMCallerCheckedAnyfunc {
    type Error = TableElement;

    fn try_from(e: TableElement) -> Result<Self, Self::Error> {
        match e {
            TableElement::FuncRef(f) => Ok(f),
            _ => Err(e),
        }
    }
}

impl TryFrom<TableElement> for Option<VMExternRef> {
    type Error = TableElement;

    fn try_from(e: TableElement) -> Result<Self, Self::Error> {
        match e {
            TableElement::ExternRef(x) => Ok(x),
            _ => Err(e),
        }
    }
}
