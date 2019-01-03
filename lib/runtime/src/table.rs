//! Memory management for tables.
//!
//! `Table` is to WebAssembly tables what `LinearMemory` is to WebAssembly linear memories.

use crate::vmcontext::{VMCallerCheckedAnyfunc, VMTableDefinition};
use cranelift_wasm::TableElementType;
use wasmtime_environ::{TablePlan, TableStyle};

/// A table instance.
#[derive(Debug)]
pub struct Table {
    vec: Vec<VMCallerCheckedAnyfunc>,
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
                let mut vec = Vec::with_capacity(plan.table.minimum as usize);
                vec.resize(
                    plan.table.minimum as usize,
                    VMCallerCheckedAnyfunc::default(),
                );

                Self {
                    vec,
                    maximum: plan.table.maximum,
                }
            }
        }
    }

    /// Return a `VMTableDefinition` for exposing the table to compiled wasm code.
    pub fn vmtable(&mut self) -> VMTableDefinition {
        VMTableDefinition {
            base: self.vec.as_mut_ptr() as *mut u8,
            current_elements: self.vec.len(),
        }
    }
}

impl AsRef<[VMCallerCheckedAnyfunc]> for Table {
    fn as_ref(&self) -> &[VMCallerCheckedAnyfunc] {
        self.vec.as_slice()
    }
}

impl AsMut<[VMCallerCheckedAnyfunc]> for Table {
    fn as_mut(&mut self) -> &mut [VMCallerCheckedAnyfunc] {
        self.vec.as_mut_slice()
    }
}
