//! Memory management for tables.
//!
//! `Table` is to WebAssembly tables what `LinearMemory` is to WebAssembly linear memories.

use crate::vmcontext::{VMCallerCheckedAnyfunc, VMTableDefinition};
use crate::{ResourceLimiter, Trap, VMExternRef};
use anyhow::{bail, Result};
use std::cell::{Cell, RefCell};
use std::cmp::min;
use std::convert::TryInto;
use std::ops::Range;
use std::ptr;
use std::rc::Rc;
use wasmtime_environ::wasm::TableElementType;
use wasmtime_environ::{ir, TablePlan};

/// An element going into or coming out of a table.
///
/// Table elements are stored as pointers and are default-initialized with `ptr::null_mut`.
#[derive(Clone)]
pub enum TableElement {
    /// A `funcref`.
    FuncRef(*mut VMCallerCheckedAnyfunc),
    /// An `exrernref`.
    ExternRef(Option<VMExternRef>),
}

impl TableElement {
    /// Consumes the given raw pointer into a table element.
    ///
    /// # Safety
    ///
    /// This is unsafe as it will *not* clone any externref, leaving the reference count unchanged.
    ///
    /// This should only be used if the raw pointer is no longer in use.
    unsafe fn from_raw(ty: TableElementType, ptr: *mut u8) -> Self {
        match ty {
            TableElementType::Func => Self::FuncRef(ptr as _),
            TableElementType::Val(_) => Self::ExternRef(if ptr.is_null() {
                None
            } else {
                Some(VMExternRef::from_raw(ptr))
            }),
        }
    }

    /// Clones a table element from the underlying raw pointer.
    ///
    /// # Safety
    ///
    /// This is unsafe as it will clone any externref, incrementing the reference count.
    unsafe fn clone_from_raw(ty: TableElementType, ptr: *mut u8) -> Self {
        match ty {
            TableElementType::Func => Self::FuncRef(ptr as _),
            TableElementType::Val(_) => Self::ExternRef(if ptr.is_null() {
                None
            } else {
                Some(VMExternRef::clone_from_raw(ptr))
            }),
        }
    }

    /// Consumes a table element into a raw pointer.
    ///
    /// # Safety
    ///
    /// This is unsafe as it will consume any underlying externref into a raw pointer without modifying
    /// the reference count.
    ///
    /// Use `from_raw` to properly drop any table elements stored as raw pointers.
    unsafe fn into_raw(self) -> *mut u8 {
        match self {
            Self::FuncRef(e) => e as _,
            Self::ExternRef(e) => e.map(|e| e.into_raw()).unwrap_or(ptr::null_mut()),
        }
    }
}

impl From<*mut VMCallerCheckedAnyfunc> for TableElement {
    fn from(f: *mut VMCallerCheckedAnyfunc) -> TableElement {
        TableElement::FuncRef(f)
    }
}

impl From<Option<VMExternRef>> for TableElement {
    fn from(x: Option<VMExternRef>) -> TableElement {
        TableElement::ExternRef(x)
    }
}

impl From<VMExternRef> for TableElement {
    fn from(x: VMExternRef) -> TableElement {
        TableElement::ExternRef(Some(x))
    }
}

enum TableStorage {
    Static {
        data: *mut *mut u8,
        size: Cell<u32>,
        ty: TableElementType,
        maximum: u32,
    },
    Dynamic {
        elements: RefCell<Vec<*mut u8>>,
        ty: TableElementType,
        maximum: Option<u32>,
    },
}

/// Represents an instance's table.
pub struct Table {
    storage: TableStorage,
    limiter: Option<Rc<dyn ResourceLimiter>>,
}

impl Table {
    /// Create a new dynamic (movable) table instance for the specified table plan.
    pub fn new_dynamic(
        plan: &TablePlan,
        limiter: Option<&Rc<dyn ResourceLimiter>>,
    ) -> Result<Self> {
        let elements = RefCell::new(vec![ptr::null_mut(); plan.table.minimum as usize]);
        let ty = plan.table.ty.clone();
        let maximum = plan.table.maximum;

        let storage = TableStorage::Dynamic {
            elements,
            ty,
            maximum,
        };

        Self::new(plan, storage, limiter)
    }

    /// Create a new static (immovable) table instance for the specified table plan.
    pub fn new_static(
        plan: &TablePlan,
        data: *mut *mut u8,
        maximum: u32,
        limiter: Option<&Rc<dyn ResourceLimiter>>,
    ) -> Result<Self> {
        let size = Cell::new(plan.table.minimum);
        let ty = plan.table.ty.clone();
        let maximum = min(plan.table.maximum.unwrap_or(maximum), maximum);

        let storage = TableStorage::Static {
            data,
            size,
            ty,
            maximum,
        };

        Self::new(plan, storage, limiter)
    }

    fn new(
        plan: &TablePlan,
        storage: TableStorage,
        limiter: Option<&Rc<dyn ResourceLimiter>>,
    ) -> Result<Self> {
        if let Some(limiter) = limiter {
            if !limiter.table_growing(0, plan.table.minimum, plan.table.maximum) {
                bail!(
                    "table minimum size of {} elements exceeds table limits",
                    plan.table.minimum
                );
            }
        }

        Ok(Self {
            storage,
            limiter: limiter.cloned(),
        })
    }

    /// Returns the type of the elements in this table.
    pub fn element_type(&self) -> TableElementType {
        match &self.storage {
            TableStorage::Static { ty, .. } => *ty,
            TableStorage::Dynamic { ty, .. } => *ty,
        }
    }

    /// Returns whether or not the underlying storage of the table is "static".
    pub(crate) fn is_static(&self) -> bool {
        if let TableStorage::Static { .. } = &self.storage {
            true
        } else {
            false
        }
    }

    /// Returns the number of allocated elements.
    pub fn size(&self) -> u32 {
        match &self.storage {
            TableStorage::Static { size, .. } => size.get(),
            TableStorage::Dynamic { elements, .. } => elements.borrow().len().try_into().unwrap(),
        }
    }

    /// Returns the maximum number of elements at runtime.
    ///
    /// Returns `None` if the table is unbounded.
    ///
    /// The runtime maximum may not be equal to the maximum from the table's Wasm type
    /// when it is being constrained by an instance allocator.
    pub fn maximum(&self) -> Option<u32> {
        match &self.storage {
            TableStorage::Static { maximum, .. } => Some(*maximum),
            TableStorage::Dynamic { maximum, .. } => maximum.clone(),
        }
    }

    /// Fill `table[dst..dst + len]` with `val`.
    ///
    /// Returns a trap error on out-of-bounds accesses.
    pub fn fill(&self, dst: u32, val: TableElement, len: u32) -> Result<(), Trap> {
        let start = dst as usize;
        let end = start
            .checked_add(len as usize)
            .ok_or_else(|| Trap::wasm(ir::TrapCode::TableOutOfBounds))?;

        if end > self.size() as usize {
            return Err(Trap::wasm(ir::TrapCode::TableOutOfBounds));
        }

        debug_assert!(self.type_matches(&val));

        self.with_elements_mut(|elements| {
            if let Some((last, elements)) = elements[start..end].split_last_mut() {
                let ty = self.element_type();

                for e in elements {
                    Self::set_raw(ty, e, val.clone());
                }

                Self::set_raw(self.element_type(), last, val);
            }

            Ok(())
        })
    }

    /// Grow table by the specified amount of elements.
    ///
    /// Returns the previous size of the table if growth is successful.
    ///
    /// Returns `None` if table can't be grown by the specified amount of
    /// elements, or if the `init_value` is the wrong kind of table element.
    ///
    /// # Unsafety
    ///
    /// Resizing the table can reallocate its internal elements buffer. This
    /// table's instance's `VMContext` has raw pointers to the elements buffer
    /// that are used by Wasm, and they need to be fixed up before we call into
    /// Wasm again. Failure to do so will result in use-after-free inside Wasm.
    ///
    /// Generally, prefer using `InstanceHandle::table_grow`, which encapsulates
    /// this unsafety.
    pub unsafe fn grow(&self, delta: u32, init_value: TableElement) -> Option<u32> {
        let old_size = self.size();
        let new_size = old_size.checked_add(delta)?;

        if let Some(limiter) = &self.limiter {
            if !limiter.table_growing(old_size, new_size, self.maximum()) {
                return None;
            }
        }

        if let Some(max) = self.maximum() {
            if new_size > max {
                return None;
            }
        }

        debug_assert!(self.type_matches(&init_value));

        // First resize the storage and then fill with the init value
        match &self.storage {
            TableStorage::Static { size, .. } => {
                size.set(new_size);
            }
            TableStorage::Dynamic { elements, .. } => {
                let mut elements = elements.borrow_mut();
                elements.resize(new_size as usize, ptr::null_mut());
            }
        }

        self.fill(old_size, init_value, delta)
            .expect("table should not be out of bounds");

        Some(old_size)
    }

    /// Get reference to the specified element.
    ///
    /// Returns `None` if the index is out of bounds.
    pub fn get(&self, index: u32) -> Option<TableElement> {
        self.with_elements(|elements| {
            elements
                .get(index as usize)
                .map(|p| unsafe { TableElement::clone_from_raw(self.element_type(), *p) })
        })
    }

    /// Set reference to the specified element.
    ///
    /// # Errors
    ///
    /// Returns an error if `index` is out of bounds or if this table type does
    /// not match the element type.
    pub fn set(&self, index: u32, elem: TableElement) -> Result<(), ()> {
        if !self.type_matches(&elem) {
            return Err(());
        }

        self.with_elements_mut(|elements| {
            let e = elements.get_mut(index as usize).ok_or(())?;
            Self::set_raw(self.element_type(), e, elem);
            Ok(())
        })
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

        debug_assert!(
            dst_table.element_type() == src_table.element_type(),
            "table element type mismatch"
        );

        let src_range = src_index as usize..src_index as usize + len as usize;
        let dst_range = dst_index as usize..dst_index as usize + len as usize;

        // Check if the tables are the same as we cannot mutably borrow and also borrow the same `RefCell`
        if ptr::eq(dst_table, src_table) {
            Self::copy_elements_within(dst_table, dst_range, src_range);
        } else {
            Self::copy_elements(dst_table, src_table, dst_range, src_range);
        }

        Ok(())
    }

    /// Return a `VMTableDefinition` for exposing the table to compiled wasm code.
    pub fn vmtable(&self) -> VMTableDefinition {
        match &self.storage {
            TableStorage::Static { data, size, .. } => VMTableDefinition {
                base: *data as _,
                current_elements: size.get(),
            },
            TableStorage::Dynamic { elements, .. } => {
                let elements = elements.borrow();
                VMTableDefinition {
                    base: elements.as_ptr() as _,
                    current_elements: elements.len().try_into().unwrap(),
                }
            }
        }
    }

    fn type_matches(&self, val: &TableElement) -> bool {
        match (&val, self.element_type()) {
            (TableElement::FuncRef(_), TableElementType::Func) => true,
            (TableElement::ExternRef(_), TableElementType::Val(_)) => true,
            _ => false,
        }
    }

    fn with_elements<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&[*mut u8]) -> R,
    {
        match &self.storage {
            TableStorage::Static { data, size, .. } => unsafe {
                f(std::slice::from_raw_parts(*data, size.get() as usize))
            },
            TableStorage::Dynamic { elements, .. } => {
                let elements = elements.borrow();
                f(elements.as_slice())
            }
        }
    }

    fn with_elements_mut<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut [*mut u8]) -> R,
    {
        match &self.storage {
            TableStorage::Static { data, size, .. } => unsafe {
                f(std::slice::from_raw_parts_mut(*data, size.get() as usize))
            },
            TableStorage::Dynamic { elements, .. } => {
                let mut elements = elements.borrow_mut();
                f(elements.as_mut_slice())
            }
        }
    }

    fn set_raw(ty: TableElementType, elem: &mut *mut u8, val: TableElement) {
        unsafe {
            let old = *elem;
            *elem = val.into_raw();

            // Drop the old element
            let _ = TableElement::from_raw(ty, old);
        }
    }

    fn copy_elements(
        dst_table: &Self,
        src_table: &Self,
        dst_range: Range<usize>,
        src_range: Range<usize>,
    ) {
        // This can only be used when copying between different tables
        debug_assert!(!ptr::eq(dst_table, src_table));

        let ty = dst_table.element_type();

        match ty {
            TableElementType::Func => {
                // `funcref` are `Copy`, so just do a mempcy
                dst_table.with_elements_mut(|dst| {
                    src_table.with_elements(|src| dst[dst_range].copy_from_slice(&src[src_range]))
                });
            }
            TableElementType::Val(_) => {
                // We need to clone each `externref`
                dst_table.with_elements_mut(|dst| {
                    src_table.with_elements(|src| {
                        for (s, d) in src_range.zip(dst_range) {
                            let elem = unsafe { TableElement::clone_from_raw(ty, src[s]) };
                            Self::set_raw(ty, &mut dst[d], elem);
                        }
                    })
                });
            }
        }
    }

    fn copy_elements_within(table: &Self, dst_range: Range<usize>, src_range: Range<usize>) {
        let ty = table.element_type();

        match ty {
            TableElementType::Func => {
                // `funcref` are `Copy`, so just do a memmove
                table.with_elements_mut(|dst| dst.copy_within(src_range, dst_range.start));
            }
            TableElementType::Val(_) => {
                // We need to clone each `externref` while handling overlapping ranges
                table.with_elements_mut(|dst| {
                    if dst_range.start <= src_range.start {
                        for (s, d) in src_range.zip(dst_range) {
                            let elem = unsafe { TableElement::clone_from_raw(ty, dst[s]) };
                            Self::set_raw(ty, &mut dst[d], elem);
                        }
                    } else {
                        for (s, d) in src_range.rev().zip(dst_range.rev()) {
                            let elem = unsafe { TableElement::clone_from_raw(ty, dst[s]) };
                            Self::set_raw(ty, &mut dst[d], elem);
                        }
                    }
                });
            }
        }
    }
}

impl Drop for Table {
    fn drop(&mut self) {
        let ty = self.element_type();

        // funcref tables can skip this
        if let TableElementType::Func = ty {
            return;
        }

        // Properly drop any table elements stored in the table
        self.with_elements(|elements| {
            for element in elements.iter() {
                let _ = unsafe { TableElement::from_raw(ty, *element) };
            }
        });
    }
}

// The default table representation is an empty funcref table that cannot grow.
impl Default for Table {
    fn default() -> Self {
        Self {
            storage: TableStorage::Static {
                data: std::ptr::null_mut(),
                size: Cell::new(0),
                ty: TableElementType::Func,
                maximum: 0,
            },
            limiter: None,
        }
    }
}
