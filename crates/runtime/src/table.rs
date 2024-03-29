//! Memory management for tables.
//!
//! `Table` is to WebAssembly tables what `LinearMemory` is to WebAssembly linear memories.

#![cfg_attr(feature = "gc", allow(irrefutable_let_patterns))]

use crate::gc::VMExternRef;
use crate::prelude::*;
use crate::vmcontext::{VMFuncRef, VMTableDefinition};
use crate::{SendSyncPtr, Store};
use anyhow::{bail, format_err, Error, Result};
use core::ops::Range;
use core::ptr::{self, NonNull};
use sptr::Strict;
use wasmtime_environ::{
    TablePlan, Trap, WasmHeapType, WasmRefType, FUNCREF_INIT_BIT, FUNCREF_MASK,
};

/// An element going into or coming out of a table.
///
/// Table elements are stored as pointers and are default-initialized with `ptr::null_mut`.
#[derive(Clone)]
pub enum TableElement {
    /// A `funcref`.
    FuncRef(*mut VMFuncRef),

    /// An `exrernref`.
    ExternRef(Option<VMExternRef>),

    /// An uninitialized funcref value. This should never be exposed
    /// beyond the `wasmtime` crate boundary; the upper-level code
    /// (which has access to the info needed for lazy initialization)
    /// will replace it when fetched.
    UninitFunc,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum TableElementType {
    Func,
    Extern,
}

impl TableElementType {
    fn matches(&self, val: &TableElement) -> bool {
        match (val, self) {
            (TableElement::FuncRef(_), TableElementType::Func) => true,
            (TableElement::ExternRef(_), TableElementType::Extern) => true,
            _ => false,
        }
    }
}

// The usage of `*mut VMFuncRef` is safe w.r.t. thread safety, this
// just relies on thread-safety of `VMExternRef` itself.
unsafe impl Send for TableElement where VMExternRef: Send {}
unsafe impl Sync for TableElement where VMExternRef: Sync {}

impl TableElement {
    /// Consumes a table element into a pointer/reference, as it
    /// exists outside the table itself. This strips off any tag bits
    /// or other information that only lives inside the table.
    ///
    /// Can only be done to an initialized table element; lazy init
    /// must occur first. (In other words, lazy values do not survive
    /// beyond the table, as every table read path initializes them.)
    ///
    /// # Safety
    ///
    /// The same warnings as for `into_table_values()` apply.
    pub(crate) unsafe fn into_ref_asserting_initialized(self) -> *mut u8 {
        match self {
            Self::FuncRef(e) => e.cast(),
            Self::UninitFunc => panic!("Uninitialized table element value outside of table slot"),
            Self::ExternRef(e) => e.map_or(ptr::null_mut(), |e| e.into_raw()),
        }
    }

    /// Indicates whether this value is the "uninitialized element"
    /// value.
    pub(crate) fn is_uninit(&self) -> bool {
        match self {
            Self::UninitFunc => true,
            _ => false,
        }
    }
}

impl From<*mut VMFuncRef> for TableElement {
    fn from(f: *mut VMFuncRef) -> TableElement {
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

#[derive(Copy, Clone)]
#[repr(transparent)]
struct TaggedFuncRef(*mut VMFuncRef);

impl TaggedFuncRef {
    const UNINIT: TaggedFuncRef = TaggedFuncRef(ptr::null_mut());

    /// Converts the given `ptr`, a valid funcref pointer, into a tagged pointer
    /// by adding in the `FUNCREF_INIT_BIT`.
    fn from(ptr: *mut VMFuncRef) -> Self {
        let masked = Strict::map_addr(ptr, |a| a | FUNCREF_INIT_BIT);
        TaggedFuncRef(masked)
    }

    /// Converts a tagged pointer into a `TableElement`, returning `UninitFunc`
    /// for null (not a tagged value) or `FuncRef` for otherwise tagged values.
    fn into_table_element(self) -> TableElement {
        let ptr = self.0;
        if ptr.is_null() {
            TableElement::UninitFunc
        } else {
            let unmasked = Strict::map_addr(ptr, |a| a & FUNCREF_MASK);
            TableElement::FuncRef(unmasked)
        }
    }
}

/// Represents an instance's table.
pub enum Table {
    /// A "static" table where storage space is managed externally, currently
    /// used with the pooling allocator.
    Static {
        /// Where data for this table is stored. The length of this list is the
        /// maximum size of the table.
        data: SendSyncPtr<[TableValue]>,
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
        elements: Vec<TableValue>,
        /// The type of this table.
        ty: TableElementType,
        /// Maximum size that `elements` can grow to.
        maximum: Option<u32>,
    },
}

pub type TableValue = Option<SendSyncPtr<u8>>;

fn wasm_to_table_type(ty: WasmRefType) -> TableElementType {
    match ty.heap_type {
        WasmHeapType::Func | WasmHeapType::Concrete(_) | WasmHeapType::NoFunc => {
            TableElementType::Func
        }
        WasmHeapType::Extern => TableElementType::Extern,
    }
}

impl Table {
    /// Create a new dynamic (movable) table instance for the specified table plan.
    pub fn new_dynamic(plan: &TablePlan, store: &mut dyn Store) -> Result<Self> {
        Self::limit_new(plan, store)?;
        let elements = vec![None; plan.table.minimum as usize];
        let ty = wasm_to_table_type(plan.table.wasm_ty);
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
        data: SendSyncPtr<[TableValue]>,
        store: &mut dyn Store,
    ) -> Result<Self> {
        Self::limit_new(plan, store)?;
        let size = plan.table.minimum;
        let ty = wasm_to_table_type(plan.table.wasm_ty);
        if data.len() < (plan.table.minimum as usize) {
            bail!(
                "initial table size of {} exceeds the pooling allocator's \
                 configured maximum table size of {} elements",
                plan.table.minimum,
                data.len(),
            );
        }
        let data = match plan.table.maximum {
            Some(max) if (max as usize) < data.len() => {
                let ptr = data.as_non_null();
                SendSyncPtr::new(NonNull::slice_from_raw_parts(ptr.cast(), max as usize))
            }
            _ => data,
        };

        Ok(Table::Static { data, size, ty })
    }

    fn limit_new(plan: &TablePlan, store: &mut dyn Store) -> Result<()> {
        if !store.table_growing(0, plan.table.minimum, plan.table.maximum)? {
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
    #[cfg(feature = "pooling-allocator")]
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

    /// Initializes the contents of this table to the specified function
    ///
    /// # Panics
    ///
    /// Panics if the table is not a function table.
    pub fn init_func(
        &mut self,
        dst: u32,
        items: impl ExactSizeIterator<Item = *mut VMFuncRef>,
    ) -> Result<(), Trap> {
        let dst = usize::try_from(dst).map_err(|_| Trap::TableOutOfBounds)?;

        let elements = self
            .funcrefs_mut()
            .get_mut(dst..)
            .and_then(|s| s.get_mut(..items.len()))
            .ok_or(Trap::TableOutOfBounds)?;

        for (item, slot) in items.zip(elements) {
            *slot = TaggedFuncRef::from(item);
        }
        Ok(())
    }

    /// Fill `table[dst..]` with values from `items`
    ///
    /// Returns a trap error on out-of-bounds accesses.
    pub fn init_extern(
        &mut self,
        dst: u32,
        items: impl ExactSizeIterator<Item = Option<VMExternRef>>,
    ) -> Result<(), Trap> {
        let dst = usize::try_from(dst).map_err(|_| Trap::TableOutOfBounds)?;

        let elements = self
            .externrefs_mut()
            .get_mut(dst..)
            .and_then(|s| s.get_mut(..items.len()))
            .ok_or(Trap::TableOutOfBounds)?;

        for (item, slot) in items.zip(elements) {
            *slot = item;
        }
        Ok(())
    }

    /// Fill `table[dst..dst + len]` with `val`.
    ///
    /// Returns a trap error on out-of-bounds accesses.
    ///
    /// # Panics
    ///
    /// Panics if `val` does not have a type that matches this table.
    pub fn fill(&mut self, dst: u32, val: TableElement, len: u32) -> Result<(), Trap> {
        let start = dst as usize;
        let end = start
            .checked_add(len as usize)
            .ok_or_else(|| Trap::TableOutOfBounds)?;

        if end > self.size() as usize {
            return Err(Trap::TableOutOfBounds);
        }

        match val {
            TableElement::FuncRef(f) => {
                self.funcrefs_mut()[start..end].fill(TaggedFuncRef::from(f));
            }
            TableElement::ExternRef(e) => {
                self.externrefs_mut()[start..end].fill(e);
            }
            TableElement::UninitFunc => {
                self.funcrefs_mut()[start..end].fill(TaggedFuncRef::UNINIT);
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
    /// # Panics
    ///
    /// Panics if `init_value` does not have a type that matches this table.
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
    ) -> Result<Option<u32>, Error> {
        let old_size = self.size();

        // Don't try to resize the table if its size isn't changing, just return
        // success.
        if delta == 0 {
            return Ok(Some(old_size));
        }

        let new_size = match old_size.checked_add(delta) {
            Some(s) => s,
            None => {
                store.table_grow_failed(format_err!("overflow calculating new table size"))?;
                return Ok(None);
            }
        };

        if !store.table_growing(old_size, new_size, self.maximum())? {
            return Ok(None);
        }

        // The WebAssembly spec requires failing a `table.grow` request if
        // it exceeds the declared limits of the table. We may have set lower
        // limits in the instance allocator as well.
        if let Some(max) = self.maximum() {
            if new_size > max {
                store.table_grow_failed(format_err!("Table maximum size exceeded"))?;
                return Ok(None);
            }
        }

        debug_assert!(self.type_matches(&init_value));

        // First resize the storage and then fill with the init value
        match self {
            Table::Static { size, data, .. } => {
                unsafe {
                    debug_assert!(data.as_ref()[*size as usize..new_size as usize]
                        .iter()
                        .all(|x| x.is_none()));
                }
                *size = new_size;
            }
            Table::Dynamic { elements, .. } => {
                // This call to `resize` could move the base address of
                // `elements`. If this table's limits declare it to be
                // fixed-size, then during AOT compilation we may have promised
                // Cranelift that the table base address won't change, so it
                // is allowed to optimize loading the base address. However, in
                // that case the above checks that delta is non-zero and the new
                // size doesn't exceed the maximum mean we can't get here.
                elements.resize(new_size as usize, None);
            }
        }

        self.fill(old_size, init_value, delta)
            .expect("table should not be out of bounds");

        Ok(Some(old_size))
    }

    /// Get reference to the specified element.
    ///
    /// Returns `None` if the index is out of bounds.
    pub fn get(&self, index: u32) -> Option<TableElement> {
        let index = usize::try_from(index).ok()?;
        match self.element_type() {
            TableElementType::Func => self
                .funcrefs()
                .get(index)
                .copied()
                .map(|e| e.into_table_element()),
            TableElementType::Extern => self
                .externrefs()
                .get(index)
                .cloned()
                .map(TableElement::ExternRef),
        }
    }

    /// Set reference to the specified element.
    ///
    /// # Errors
    ///
    /// Returns an error if `index` is out of bounds or if this table type does
    /// not match the element type.
    ///
    /// # Panics
    ///
    /// Panics if `elem` is not of the right type for this table.
    pub fn set(&mut self, index: u32, elem: TableElement) -> Result<(), ()> {
        let index = usize::try_from(index).map_err(|_| ())?;
        match elem {
            TableElement::FuncRef(f) => {
                *self.funcrefs_mut().get_mut(index).ok_or(())? = TaggedFuncRef::from(f);
            }
            TableElement::UninitFunc => {
                *self.funcrefs_mut().get_mut(index).ok_or(())? = TaggedFuncRef::UNINIT;
            }
            TableElement::ExternRef(e) => {
                *self.externrefs_mut().get_mut(index).ok_or(())? = e;
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
            return Err(Trap::TableOutOfBounds);
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
    pub fn vmtable(&mut self) -> VMTableDefinition {
        match self {
            Table::Static { data, size, .. } => VMTableDefinition {
                base: data.as_ptr().cast(),
                current_elements: *size,
            },
            Table::Dynamic { elements, .. } => VMTableDefinition {
                base: elements.as_mut_ptr().cast(),
                current_elements: elements.len().try_into().unwrap(),
            },
        }
    }

    fn type_matches(&self, val: &TableElement) -> bool {
        self.element_type().matches(val)
    }

    fn raw_elements(&self) -> &[TableValue] {
        match self {
            Table::Static { data, size, .. } => unsafe { &data.as_ref()[..*size as usize] },
            Table::Dynamic { elements, .. } => &elements[..],
        }
    }

    fn raw_elements_mut(&mut self) -> &mut [TableValue] {
        match self {
            Table::Static { data, size, .. } => unsafe { &mut data.as_mut()[..*size as usize] },
            Table::Dynamic { elements, .. } => &mut elements[..],
        }
    }

    fn funcrefs(&self) -> &[TaggedFuncRef] {
        assert_eq!(self.element_type(), TableElementType::Func);
        unsafe {
            let (a, b, c) = self.raw_elements().align_to();
            assert!(a.is_empty());
            assert!(c.is_empty());
            b
        }
    }

    fn funcrefs_mut(&mut self) -> &mut [TaggedFuncRef] {
        assert_eq!(self.element_type(), TableElementType::Func);
        unsafe {
            let (a, b, c) = self.raw_elements_mut().align_to_mut();
            assert!(a.is_empty());
            assert!(c.is_empty());
            b
        }
    }

    fn externrefs(&self) -> &[Option<VMExternRef>] {
        assert_eq!(self.element_type(), TableElementType::Extern);
        unsafe {
            let (a, b, c) = self.raw_elements().align_to();
            assert!(a.is_empty());
            assert!(c.is_empty());
            b
        }
    }

    fn externrefs_mut(&mut self) -> &mut [Option<VMExternRef>] {
        assert_eq!(self.element_type(), TableElementType::Extern);
        unsafe {
            let (a, b, c) = self.raw_elements_mut().align_to_mut();
            assert!(a.is_empty());
            assert!(c.is_empty());
            b
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
                dst_table.funcrefs_mut()[dst_range]
                    .copy_from_slice(&src_table.funcrefs()[src_range]);
            }
            TableElementType::Extern => {
                dst_table.externrefs_mut()[dst_range]
                    .clone_from_slice(&src_table.externrefs()[src_range]);
            }
        }
    }

    fn copy_elements_within(&mut self, dst_range: Range<usize>, src_range: Range<usize>) {
        let ty = self.element_type();
        match ty {
            TableElementType::Func => {
                // `funcref` are `Copy`, so just do a memmove
                self.funcrefs_mut().copy_within(src_range, dst_range.start);
            }
            TableElementType::Extern => {
                // We need to clone each `externref` while handling overlapping
                // ranges
                let elements = self.externrefs_mut();
                if dst_range.start <= src_range.start {
                    for (s, d) in src_range.zip(dst_range) {
                        elements[d] = elements[s].clone();
                    }
                } else {
                    for (s, d) in src_range.rev().zip(dst_range.rev()) {
                        elements[d] = elements[s].clone();
                    }
                }
            }
        }
    }
}

impl Drop for Table {
    fn drop(&mut self) {
        match self.element_type() {
            // `funcref` tables don't need drops.
            TableElementType::Func => {}

            // `externref` tables are null'd out to ensure that no strong
            // references are preserved.
            TableElementType::Extern => {
                for e in self.externrefs_mut() {
                    let _ = e.take();
                }
            }
        }
    }
}

// The default table representation is an empty funcref table that cannot grow.
impl Default for Table {
    fn default() -> Self {
        Table::Static {
            data: SendSyncPtr::new(NonNull::from(&mut [])),
            size: 0,
            ty: TableElementType::Func,
        }
    }
}
