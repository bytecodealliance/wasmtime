//! Memory management for tables.
//!
//! `Table` is to WebAssembly tables what `LinearMemory` is to WebAssembly linear memories.

use cranelift_wasm::TableElementType;
use std::ptr;
use vmcontext::VMTable;
use wasmtime_environ::{TablePlan, TableStyle};

#[derive(Debug, Clone)]
#[repr(C)]
pub struct AnyFunc {
    pub func_ptr: *const u8,
    pub type_id: usize,
}

impl Default for AnyFunc {
    fn default() -> Self {
        Self {
            func_ptr: ptr::null(),
            type_id: 0,
        }
    }
}

/// A table instance.
#[derive(Debug)]
pub struct Table {
    vec: Vec<AnyFunc>,
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
            TableStyle::CallerChecksSignature => {
                let mut vec = Vec::new();
                vec.resize(plan.table.minimum as usize, AnyFunc::default());

                Self {
                    vec,
                    maximum: plan.table.maximum,
                }
            }
        }
    }

    pub fn vmtable(&mut self) -> VMTable {
        VMTable::new(self.vec.as_mut_ptr() as *mut u8, self.vec.len())
    }
}

impl AsRef<[AnyFunc]> for Table {
    fn as_ref(&self) -> &[AnyFunc] {
        self.vec.as_slice()
    }
}

impl AsMut<[AnyFunc]> for Table {
    fn as_mut(&mut self) -> &mut [AnyFunc] {
        self.vec.as_mut_slice()
    }
}
