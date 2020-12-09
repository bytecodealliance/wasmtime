//! Memory management for tables.
//!
//! `Table` is to WebAssembly tables what `LinearMemory` is to WebAssembly linear memories.

use crate::vmcontext::{VMCallerCheckedAnyfunc, VMTableDefinition};
use crate::{Trap, VMExternRef};
use std::cell::{Cell, RefCell};
use std::cmp::min;
use std::convert::{TryFrom, TryInto};
use std::ptr;
use wasmtime_environ::wasm::TableElementType;
use wasmtime_environ::{ir, TablePlan, TableStyle};

/// An element going into or coming out of a table.
#[derive(Clone, Debug)]
pub enum TableElement {
    /// A `funcref`.
    FuncRef(*mut VMCallerCheckedAnyfunc),
    /// An `exrernref`.
    ExternRef(Option<VMExternRef>),
}

impl TryFrom<TableElement> for *mut VMCallerCheckedAnyfunc {
    type Error = ();

    fn try_from(e: TableElement) -> Result<Self, Self::Error> {
        match e {
            TableElement::FuncRef(f) => Ok(f),
            _ => Err(()),
        }
    }
}

impl TryFrom<TableElement> for Option<VMExternRef> {
    type Error = ();

    fn try_from(e: TableElement) -> Result<Self, Self::Error> {
        match e {
            TableElement::ExternRef(x) => Ok(x),
            _ => Err(()),
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

#[derive(Debug)]
enum TableElements {
    FuncRefs(Vec<*mut VMCallerCheckedAnyfunc>),
    ExternRefs(Vec<Option<VMExternRef>>),
}

// Ideally this should be static assertion that table elements are pointer-sized
#[inline(always)]
pub(crate) fn max_table_element_size() -> usize {
    debug_assert_eq!(
        std::mem::size_of::<*mut VMCallerCheckedAnyfunc>(),
        std::mem::size_of::<*const ()>()
    );
    debug_assert_eq!(
        std::mem::size_of::<Option<VMExternRef>>(),
        std::mem::size_of::<*const ()>()
    );
    std::mem::size_of::<*const ()>()
}

#[derive(Debug)]
enum TableStorage {
    Static {
        data: *mut u8,
        size: Cell<u32>,
        ty: TableElementType,
        maximum: u32,
    },
    Dynamic {
        elements: RefCell<TableElements>,
        maximum: Option<u32>,
    },
}

/// Represents an instance's table.
#[derive(Debug)]
pub struct Table {
    storage: TableStorage,
}

impl Table {
    /// Create a new dynamic (movable) table instance for the specified table plan.
    pub fn new_dynamic(plan: &TablePlan) -> Self {
        let min = usize::try_from(plan.table.minimum).unwrap();
        let elements = RefCell::new(match plan.table.ty {
            TableElementType::Func => TableElements::FuncRefs(vec![ptr::null_mut(); min]),
            TableElementType::Val(ty) => {
                debug_assert_eq!(ty, crate::ref_type());
                TableElements::ExternRefs(vec![None; min])
            }
        });

        match plan.style {
            TableStyle::CallerChecksSignature => Self {
                storage: TableStorage::Dynamic {
                    elements,
                    maximum: plan.table.maximum,
                },
            },
        }
    }

    /// Create a new static (immovable) table instance for the specified table plan.
    pub fn new_static(plan: &TablePlan, data: *mut u8, maximum: u32) -> Self {
        match plan.style {
            TableStyle::CallerChecksSignature => Self {
                storage: TableStorage::Static {
                    data,
                    size: Cell::new(plan.table.minimum),
                    ty: plan.table.ty.clone(),
                    maximum: min(plan.table.maximum.unwrap_or(maximum), maximum),
                },
            },
        }
    }

    /// Returns the type of the elements in this table.
    pub fn element_type(&self) -> TableElementType {
        match &self.storage {
            TableStorage::Static { ty, .. } => *ty,
            TableStorage::Dynamic { elements, .. } => match &*elements.borrow() {
                TableElements::FuncRefs(_) => TableElementType::Func,
                TableElements::ExternRefs(_) => TableElementType::Val(crate::ref_type()),
            },
        }
    }

    /// Returns the number of allocated elements.
    pub fn size(&self) -> u32 {
        match &self.storage {
            TableStorage::Static { size, .. } => size.get(),
            TableStorage::Dynamic { elements, .. } => match &*elements.borrow() {
                TableElements::FuncRefs(x) => x.len().try_into().unwrap(),
                TableElements::ExternRefs(x) => x.len().try_into().unwrap(),
            },
        }
    }

    /// Returns the maximum number of elements.
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
        let start = dst;
        let end = start
            .checked_add(len)
            .ok_or_else(|| Trap::wasm(ir::TrapCode::TableOutOfBounds))?;

        if end > self.size() {
            return Err(Trap::wasm(ir::TrapCode::TableOutOfBounds));
        }

        match val {
            TableElement::FuncRef(r) => {
                unsafe {
                    self.with_funcrefs_mut(move |elements| {
                        let elements = elements.unwrap();

                        // TODO: replace this with slice::fill (https://github.com/rust-lang/rust/issues/70758) when stabilized
                        for e in &mut elements[start as usize..end as usize] {
                            *e = r;
                        }
                    });
                }
            }
            TableElement::ExternRef(r) => {
                unsafe {
                    self.with_externrefs_mut(move |elements| {
                        let elements = elements.unwrap();

                        // TODO: replace this with slice::fill (https://github.com/rust-lang/rust/issues/70758) when stabilized
                        for e in &mut elements[start as usize..end as usize] {
                            *e = r.clone();
                        }
                    });
                }
            }
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
    pub unsafe fn grow(&self, delta: u32, init_value: TableElement) -> Option<u32> {
        let old_size = self.size();

        let new_size = old_size.checked_add(delta)?;
        if let Some(max) = self.maximum() {
            if new_size > max {
                return None;
            }
        }

        match &self.storage {
            TableStorage::Static { size, .. } => {
                size.set(new_size);
                self.fill(old_size, init_value, delta)
                    .ok()
                    .map(|_| old_size)
            }
            TableStorage::Dynamic { elements, .. } => {
                let new_len = usize::try_from(new_size).unwrap();

                match &mut *elements.borrow_mut() {
                    TableElements::FuncRefs(x) => x.resize(new_len, init_value.try_into().ok()?),
                    TableElements::ExternRefs(x) => x.resize(new_len, init_value.try_into().ok()?),
                }

                Some(old_size)
            }
        }
    }

    /// Get reference to the specified element.
    ///
    /// Returns `None` if the index is out of bounds.
    pub fn get(&self, index: u32) -> Option<TableElement> {
        unsafe {
            match self.element_type() {
                TableElementType::Func => self.with_funcrefs(|elements| {
                    elements.and_then(|e| e.get(index as usize).cloned().map(TableElement::FuncRef))
                }),
                TableElementType::Val(_) => self.with_externrefs(|elements| {
                    elements
                        .and_then(|e| e.get(index as usize).cloned().map(TableElement::ExternRef))
                }),
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
        unsafe {
            match self.element_type() {
                TableElementType::Func => self.with_funcrefs_mut(move |elements| {
                    let elements = elements.ok_or(())?;
                    let e = elements.get_mut(index as usize).ok_or(())?;
                    *e = elem.try_into()?;
                    Ok(())
                }),
                TableElementType::Val(_) => self.with_externrefs_mut(move |elements| {
                    let elements = elements.ok_or(())?;
                    let e = elements.get_mut(index as usize).ok_or(())?;
                    *e = elem.try_into()?;
                    Ok(())
                }),
            }
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

        // Check if the source and destination are the same table
        // This ensures we don't `borrow` and `borrow_mut` the same underlying RefCell
        let same_table = ptr::eq(dst_table, src_table);

        let src_range = src_index as usize..src_index as usize + len as usize;
        let dst_range = dst_index as usize..dst_index as usize + len as usize;

        unsafe {
            match dst_table.element_type() {
                TableElementType::Func => dst_table.with_funcrefs_mut(|dst| {
                    let dst = dst.unwrap();

                    if same_table {
                        dst.copy_within(src_range, dst_index as usize);
                    } else {
                        src_table.with_funcrefs(|src| {
                            let src = src.unwrap();
                            dst[dst_range].copy_from_slice(&src[src_range]);
                        })
                    }
                }),
                TableElementType::Val(_) => dst_table.with_externrefs_mut(|dst| {
                    let dst = dst.unwrap();

                    if same_table {
                        // As there's no `slice::clone_within` because cloning can't be done with memmove, use a loop
                        if dst_index <= src_index {
                            for (s, d) in (src_range).zip(dst_range) {
                                dst[d] = dst[s].clone();
                            }
                        } else {
                            for (s, d) in src_range.rev().zip(dst_range.rev()) {
                                dst[d] = dst[s].clone();
                            }
                        }
                    } else {
                        src_table.with_externrefs(|src| {
                            let src = src.unwrap();
                            dst[dst_range].clone_from_slice(&src[src_range]);
                        })
                    }
                }),
            }
        }

        Ok(())
    }

    /// Return a `VMTableDefinition` for exposing the table to compiled wasm code.
    pub fn vmtable(&self) -> VMTableDefinition {
        match &self.storage {
            TableStorage::Static { data, size, .. } => VMTableDefinition {
                base: *data,
                current_elements: size.get(),
            },
            TableStorage::Dynamic { elements, .. } => match &*elements.borrow() {
                TableElements::FuncRefs(x) => VMTableDefinition {
                    base: x.as_ptr() as *const u8 as _,
                    current_elements: x.len().try_into().unwrap(),
                },
                TableElements::ExternRefs(x) => VMTableDefinition {
                    base: x.as_ptr() as *const u8 as _,
                    current_elements: x.len().try_into().unwrap(),
                },
            },
        }
    }

    unsafe fn with_funcrefs<F, R>(&self, with: F) -> R
    where
        F: FnOnce(Option<&[*mut VMCallerCheckedAnyfunc]>) -> R,
    {
        match &self.storage {
            TableStorage::Static { data, size, ty, .. } => match ty {
                TableElementType::Func => with(Some(std::slice::from_raw_parts(
                    *data as *const _,
                    size.get() as usize,
                ))),
                _ => with(None),
            },
            TableStorage::Dynamic { elements, .. } => match &*elements.borrow() {
                TableElements::FuncRefs(x) => with(Some(x.as_slice())),
                _ => with(None),
            },
        }
    }

    unsafe fn with_funcrefs_mut<F, R>(&self, with: F) -> R
    where
        F: FnOnce(Option<&mut [*mut VMCallerCheckedAnyfunc]>) -> R,
    {
        match &self.storage {
            TableStorage::Static { data, size, ty, .. } => match ty {
                TableElementType::Func => with(Some(std::slice::from_raw_parts_mut(
                    *data as *mut _,
                    size.get() as usize,
                ))),
                _ => with(None),
            },
            TableStorage::Dynamic { elements, .. } => match &mut *elements.borrow_mut() {
                TableElements::FuncRefs(x) => with(Some(x.as_mut_slice())),
                _ => with(None),
            },
        }
    }

    unsafe fn with_externrefs<F, R>(&self, with: F) -> R
    where
        F: FnOnce(Option<&[Option<VMExternRef>]>) -> R,
    {
        match &self.storage {
            TableStorage::Static { data, size, ty, .. } => match ty {
                TableElementType::Val(_) => with(Some(std::slice::from_raw_parts(
                    *data as *const _,
                    size.get() as usize,
                ))),
                _ => with(None),
            },
            TableStorage::Dynamic { elements, .. } => match &*elements.borrow() {
                TableElements::ExternRefs(x) => with(Some(x.as_slice())),
                _ => with(None),
            },
        }
    }

    unsafe fn with_externrefs_mut<F, R>(&self, with: F) -> R
    where
        F: FnOnce(Option<&mut [Option<VMExternRef>]>) -> R,
    {
        match &self.storage {
            TableStorage::Static { data, size, ty, .. } => match ty {
                TableElementType::Val(_) => with(Some(std::slice::from_raw_parts_mut(
                    *data as *mut _,
                    size.get() as usize,
                ))),
                _ => with(None),
            },
            TableStorage::Dynamic { elements, .. } => match &mut *elements.borrow_mut() {
                TableElements::ExternRefs(x) => with(Some(x.as_mut_slice())),
                _ => with(None),
            },
        }
    }
}
