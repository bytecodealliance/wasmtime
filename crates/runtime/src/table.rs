//! Memory management for tables.
//!
//! `Table` is to WebAssembly tables what `LinearMemory` is to WebAssembly linear memories.

use crate::vmcontext::{VMCallerCheckedAnyfunc, VMTableDefinition};
use crate::{Store, Trap, VMExternRef};
use anyhow::{bail, Result};
use std::convert::{TryFrom, TryInto};
use std::ops::Range;
use std::ptr;
use wasmtime_environ::{TablePlan, TrapCode, WasmType};

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

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum TableElementType {
    Func,
    Extern,
}

// The usage of `*mut VMCallerCheckedAnyfunc` is safe w.r.t. thread safety, this
// just relies on thread-safety of `VMExternRef` itself.
unsafe impl Send for TableElement where VMExternRef: Send {}
unsafe impl Sync for TableElement where VMExternRef: Sync {}

impl TableElement {
    /// Consumes the given raw pointer into a table element.
    ///
    /// # Safety
    ///
    /// This is unsafe as it will *not* clone any externref, leaving the reference count unchanged.
    ///
    /// This should only be used if the raw pointer is no longer in use.
    unsafe fn from_raw(ty: TableElementType, ptr: usize) -> Self {
        match ty {
            TableElementType::Func => Self::FuncRef(ptr as _),
            TableElementType::Extern => Self::ExternRef(if ptr == 0 {
                None
            } else {
                Some(VMExternRef::from_raw(ptr as *mut u8))
            }),
        }
    }

    /// Clones a table element from the underlying raw pointer.
    ///
    /// # Safety
    ///
    /// This is unsafe as it will clone any externref, incrementing the reference count.
    unsafe fn clone_from_raw(ty: TableElementType, ptr: usize) -> Self {
        match ty {
            TableElementType::Func => Self::FuncRef(ptr as _),
            TableElementType::Extern => Self::ExternRef(if ptr == 0 {
                None
            } else {
                Some(VMExternRef::clone_from_raw(ptr as *mut u8))
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
    unsafe fn into_raw(self) -> usize {
        match self {
            Self::FuncRef(e) => e as _,
            Self::ExternRef(e) => e.map_or(0, |e| e.into_raw() as usize),
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

/// Represents an instance's table.
pub enum Table {
    /// A "static" table where storage space is managed externally, currently
    /// used with the pooling allocator.
    Static {
        /// Where data for this table is stored. The length of this list is the
        /// maximum size of the table.
        data: &'static mut [usize],
        /// The current size of the table.
        size: u32,
        /// The type of this table.
        ty: TableElementType,
    },
    /// A "dynamic" table where table storage space is dynamically allocated via
    /// `malloc` (aka Rust's `Vec`).
    Dynamic {
        /// Dynamically managed storage space for this table. The length of this
        /// vector is the current size of the table.
        elements: Vec<usize>,
        /// The type of this table.
        ty: TableElementType,
        /// Maximum size that `elements` can grow to.
        maximum: Option<u32>,
    },
}

fn wasm_to_table_type(ty: WasmType) -> Result<TableElementType> {
    match ty {
        WasmType::FuncRef => Ok(TableElementType::Func),
        WasmType::ExternRef => Ok(TableElementType::Extern),
        ty => bail!("invalid table element type {:?}", ty),
    }
}

impl Table {
    /// Create a new dynamic (movable) table instance for the specified table plan.
    pub fn new_dynamic(plan: &TablePlan, store: &mut dyn Store) -> Result<Self> {
        Self::limit_new(plan, store)?;
        let elements = vec![0; plan.table.minimum as usize];
        let ty = wasm_to_table_type(plan.table.wasm_ty)?;
        let maximum = plan.table.maximum;

        Ok(Table::Dynamic {
            elements,
            ty,
            maximum,
        })
    }

    /// Create a new static (immovable) table instance for the specified table plan.
    pub fn new_static(
        plan: &TablePlan,
        data: &'static mut [usize],
        store: &mut dyn Store,
    ) -> Result<Self> {
        Self::limit_new(plan, store)?;
        let size = plan.table.minimum;
        let ty = wasm_to_table_type(plan.table.wasm_ty)?;
        let data = match plan.table.maximum {
            Some(max) if (max as usize) < data.len() => &mut data[..max as usize],
            _ => data,
        };

        Ok(Table::Static { data, size, ty })
    }

    fn limit_new(plan: &TablePlan, store: &mut dyn Store) -> Result<()> {
        if !store.table_growing(0, plan.table.minimum, plan.table.maximum) {
            bail!(
                "table minimum size of {} elements exceeds table limits",
                plan.table.minimum
            );
        }
        Ok(())
    }

    /// Returns the type of the elements in this table.
    pub fn element_type(&self) -> TableElementType {
        match self {
            Table::Static { ty, .. } => *ty,
            Table::Dynamic { ty, .. } => *ty,
        }
    }

    /// Returns whether or not the underlying storage of the table is "static".
    pub(crate) fn is_static(&self) -> bool {
        if let Table::Static { .. } = self {
            true
        } else {
            false
        }
    }

    /// Returns the number of allocated elements.
    pub fn size(&self) -> u32 {
        match self {
            Table::Static { size, .. } => *size,
            Table::Dynamic { elements, .. } => elements.len().try_into().unwrap(),
        }
    }

    /// Returns the maximum number of elements at runtime.
    ///
    /// Returns `None` if the table is unbounded.
    ///
    /// The runtime maximum may not be equal to the maximum from the table's Wasm type
    /// when it is being constrained by an instance allocator.
    pub fn maximum(&self) -> Option<u32> {
        match self {
            Table::Static { data, .. } => Some(data.len() as u32),
            Table::Dynamic { maximum, .. } => maximum.clone(),
        }
    }

    /// Fill `table[dst..]` with values from `items`
    ///
    /// Returns a trap error on out-of-bounds accesses.
    pub fn init_funcs(
        &mut self,
        dst: u32,
        items: impl ExactSizeIterator<Item = *mut VMCallerCheckedAnyfunc>,
    ) -> Result<(), Trap> {
        assert!(self.element_type() == TableElementType::Func);

        let elements = match self
            .elements_mut()
            .get_mut(usize::try_from(dst).unwrap()..)
            .and_then(|s| s.get_mut(..items.len()))
        {
            Some(elements) => elements,
            None => return Err(Trap::wasm(TrapCode::TableOutOfBounds)),
        };

        for (item, slot) in items.zip(elements) {
            *slot = item as usize;
        }
        Ok(())
    }

    /// Fill `table[dst..dst + len]` with `val`.
    ///
    /// Returns a trap error on out-of-bounds accesses.
    pub fn fill(&mut self, dst: u32, val: TableElement, len: u32) -> Result<(), Trap> {
        let start = dst as usize;
        let end = start
            .checked_add(len as usize)
            .ok_or_else(|| Trap::wasm(TrapCode::TableOutOfBounds))?;

        if end > self.size() as usize {
            return Err(Trap::wasm(TrapCode::TableOutOfBounds));
        }

        debug_assert!(self.type_matches(&val));

        let ty = self.element_type();
        if let Some((last, elements)) = self.elements_mut()[start..end].split_last_mut() {
            for e in elements {
                Self::set_raw(ty, e, val.clone());
            }

            Self::set_raw(ty, last, val);
        }

        Ok(())
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
    pub unsafe fn grow(
        &mut self,
        delta: u32,
        init_value: TableElement,
        store: &mut dyn Store,
    ) -> Option<u32> {
        let old_size = self.size();
        let new_size = old_size.checked_add(delta)?;

        if !store.table_growing(old_size, new_size, self.maximum()) {
            return None;
        }

        if let Some(max) = self.maximum() {
            if new_size > max {
                return None;
            }
        }

        debug_assert!(self.type_matches(&init_value));

        // First resize the storage and then fill with the init value
        match self {
            Table::Static { size, data, .. } => {
                debug_assert!(data[*size as usize..new_size as usize]
                    .iter()
                    .all(|x| *x == 0));
                *size = new_size;
            }
            Table::Dynamic { elements, .. } => {
                elements.resize(new_size as usize, 0);
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
        self.elements()
            .get(index as usize)
            .map(|p| unsafe { TableElement::clone_from_raw(self.element_type(), *p) })
    }

    /// Set reference to the specified element.
    ///
    /// # Errors
    ///
    /// Returns an error if `index` is out of bounds or if this table type does
    /// not match the element type.
    pub fn set(&mut self, index: u32, elem: TableElement) -> Result<(), ()> {
        if !self.type_matches(&elem) {
            return Err(());
        }

        let ty = self.element_type();
        let e = self.elements_mut().get_mut(index as usize).ok_or(())?;
        Self::set_raw(ty, e, elem);
        Ok(())
    }

    /// Copy `len` elements from `src_table[src_index..]` into `dst_table[dst_index..]`.
    ///
    /// # Errors
    ///
    /// Returns an error if the range is out of bounds of either the source or
    /// destination tables.
    pub unsafe fn copy(
        dst_table: *mut Self,
        src_table: *mut Self,
        dst_index: u32,
        src_index: u32,
        len: u32,
    ) -> Result<(), Trap> {
        // https://webassembly.github.io/bulk-memory-operations/core/exec/instructions.html#exec-table-copy

        if src_index
            .checked_add(len)
            .map_or(true, |n| n > (*src_table).size())
            || dst_index
                .checked_add(len)
                .map_or(true, |m| m > (*dst_table).size())
        {
            return Err(Trap::wasm(TrapCode::TableOutOfBounds));
        }

        debug_assert!(
            (*dst_table).element_type() == (*src_table).element_type(),
            "table element type mismatch"
        );

        let src_range = src_index as usize..src_index as usize + len as usize;
        let dst_range = dst_index as usize..dst_index as usize + len as usize;

        // Check if the tables are the same as we cannot mutably borrow and also borrow the same `RefCell`
        if ptr::eq(dst_table, src_table) {
            (*dst_table).copy_elements_within(dst_range, src_range);
        } else {
            Self::copy_elements(&mut *dst_table, &*src_table, dst_range, src_range);
        }

        Ok(())
    }

    /// Return a `VMTableDefinition` for exposing the table to compiled wasm code.
    pub fn vmtable(&self) -> VMTableDefinition {
        match self {
            Table::Static { data, size, .. } => VMTableDefinition {
                base: data.as_ptr() as *mut _,
                current_elements: *size,
            },
            Table::Dynamic { elements, .. } => VMTableDefinition {
                base: elements.as_ptr() as _,
                current_elements: elements.len().try_into().unwrap(),
            },
        }
    }

    fn type_matches(&self, val: &TableElement) -> bool {
        match (&val, self.element_type()) {
            (TableElement::FuncRef(_), TableElementType::Func) => true,
            (TableElement::ExternRef(_), TableElementType::Extern) => true,
            _ => false,
        }
    }

    fn elements(&self) -> &[usize] {
        match self {
            Table::Static { data, size, .. } => &data[..*size as usize],
            Table::Dynamic { elements, .. } => &elements[..],
        }
    }

    fn elements_mut(&mut self) -> &mut [usize] {
        match self {
            Table::Static { data, size, .. } => &mut data[..*size as usize],
            Table::Dynamic { elements, .. } => &mut elements[..],
        }
    }

    fn set_raw(ty: TableElementType, elem: &mut usize, val: TableElement) {
        unsafe {
            let old = *elem;
            *elem = val.into_raw();

            // Drop the old element
            let _ = TableElement::from_raw(ty, old);
        }
    }

    fn copy_elements(
        dst_table: &mut Self,
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
                dst_table.elements_mut()[dst_range]
                    .copy_from_slice(&src_table.elements()[src_range]);
            }
            TableElementType::Extern => {
                // We need to clone each `externref`
                let dst = dst_table.elements_mut();
                let src = src_table.elements();
                for (s, d) in src_range.zip(dst_range) {
                    let elem = unsafe { TableElement::clone_from_raw(ty, src[s]) };
                    Self::set_raw(ty, &mut dst[d], elem);
                }
            }
        }
    }

    fn copy_elements_within(&mut self, dst_range: Range<usize>, src_range: Range<usize>) {
        let ty = self.element_type();
        let dst = self.elements_mut();
        match ty {
            TableElementType::Func => {
                // `funcref` are `Copy`, so just do a memmove
                dst.copy_within(src_range, dst_range.start);
            }
            TableElementType::Extern => {
                // We need to clone each `externref` while handling overlapping
                // ranges
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
        for element in self.elements() {
            drop(unsafe { TableElement::from_raw(ty, *element) });
        }
    }
}

// The default table representation is an empty funcref table that cannot grow.
impl Default for Table {
    fn default() -> Self {
        Table::Static {
            data: &mut [],
            size: 0,
            ty: TableElementType::Func,
        }
    }
}
