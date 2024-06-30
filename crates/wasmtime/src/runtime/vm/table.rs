//! Memory management for tables.
//!
//! `Table` is to WebAssembly tables what `LinearMemory` is to WebAssembly linear memories.

#![cfg_attr(feature = "gc", allow(irrefutable_let_patterns))]

use crate::prelude::*;
use crate::runtime::vm::vmcontext::{VMFuncRef, VMTableDefinition};
use crate::runtime::vm::{GcStore, SendSyncPtr, Store, VMGcRef};
use core::cmp;
use core::ops::Range;
use core::ptr::{self, NonNull};
use core::slice;
use sptr::Strict;
use wasmtime_environ::{
    TablePlan, TableStyle, Trap, WasmHeapTopType, WasmRefType, FUNCREF_INIT_BIT, FUNCREF_MASK,
};

/// An element going into or coming out of a table.
///
/// Table elements are stored as pointers and are default-initialized with
/// `ptr::null_mut`.
pub enum TableElement {
    /// A `funcref`.
    FuncRef(*mut VMFuncRef),

    /// A GC reference.
    GcRef(Option<VMGcRef>),

    /// An uninitialized funcref value. This should never be exposed
    /// beyond the `wasmtime` crate boundary; the upper-level code
    /// (which has access to the info needed for lazy initialization)
    /// will replace it when fetched.
    UninitFunc,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum TableElementType {
    Func,
    GcRef,
}

impl TableElementType {
    fn matches(&self, val: &TableElement) -> bool {
        match (val, self) {
            (TableElement::FuncRef(_), TableElementType::Func) => true,
            (TableElement::GcRef(_), TableElementType::GcRef) => true,
            _ => false,
        }
    }
}

// The usage of `*mut VMFuncRef` is safe w.r.t. thread safety, this just relies
// on thread-safety of `VMGcRef` itself.
unsafe impl Send for TableElement where VMGcRef: Send {}
unsafe impl Sync for TableElement where VMGcRef: Sync {}

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
    pub(crate) unsafe fn into_func_ref_asserting_initialized(self) -> *mut VMFuncRef {
        match self {
            Self::FuncRef(e) => e,
            Self::UninitFunc => panic!("Uninitialized table element value outside of table slot"),
            Self::GcRef(_) => panic!("GC reference is not a function reference"),
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

impl From<Option<VMGcRef>> for TableElement {
    fn from(x: Option<VMGcRef>) -> TableElement {
        TableElement::GcRef(x)
    }
}

impl From<VMGcRef> for TableElement {
    fn from(x: VMGcRef) -> TableElement {
        TableElement::GcRef(Some(x))
    }
}

#[derive(Copy, Clone)]
#[repr(transparent)]
struct TaggedFuncRef(*mut VMFuncRef);

impl TaggedFuncRef {
    const UNINIT: TaggedFuncRef = TaggedFuncRef(ptr::null_mut());

    /// Converts the given `ptr`, a valid funcref pointer, into a tagged pointer
    /// by adding in the `FUNCREF_INIT_BIT`.
    fn from(ptr: *mut VMFuncRef, lazy_init: bool) -> Self {
        if lazy_init {
            let masked = Strict::map_addr(ptr, |a| a | FUNCREF_INIT_BIT);
            TaggedFuncRef(masked)
        } else {
            TaggedFuncRef(ptr)
        }
    }

    /// Converts a tagged pointer into a `TableElement`, returning `UninitFunc`
    /// for null (not a tagged value) or `FuncRef` for otherwise tagged values.
    fn into_table_element(self, lazy_init: bool) -> TableElement {
        let ptr = self.0;
        if lazy_init && ptr.is_null() {
            TableElement::UninitFunc
        } else {
            // Masking off the tag bit is harmless whether the table uses lazy
            // init or not.
            let unmasked = Strict::map_addr(ptr, |a| a & FUNCREF_MASK);
            TableElement::FuncRef(unmasked)
        }
    }
}

pub type FuncTableElem = Option<SendSyncPtr<VMFuncRef>>;

pub enum StaticTable {
    Func(StaticFuncTable),
    GcRef(StaticGcRefTable),
}

impl From<StaticFuncTable> for StaticTable {
    fn from(value: StaticFuncTable) -> Self {
        Self::Func(value)
    }
}

impl From<StaticGcRefTable> for StaticTable {
    fn from(value: StaticGcRefTable) -> Self {
        Self::GcRef(value)
    }
}

pub struct StaticFuncTable {
    /// Where data for this table is stored. The length of this list is the
    /// maximum size of the table.
    data: SendSyncPtr<[FuncTableElem]>,
    /// The current size of the table.
    size: u32,
    /// Whether elements of this table are initialized lazily.
    lazy_init: bool,
}

pub struct StaticGcRefTable {
    /// Where data for this table is stored. The length of this list is the
    /// maximum size of the table.
    data: SendSyncPtr<[Option<VMGcRef>]>,
    /// The current size of the table.
    size: u32,
}

pub enum DynamicTable {
    Func(DynamicFuncTable),
    GcRef(DynamicGcRefTable),
}

impl From<DynamicFuncTable> for DynamicTable {
    fn from(value: DynamicFuncTable) -> Self {
        Self::Func(value)
    }
}

impl From<DynamicGcRefTable> for DynamicTable {
    fn from(value: DynamicGcRefTable) -> Self {
        Self::GcRef(value)
    }
}

pub struct DynamicFuncTable {
    /// Dynamically managed storage space for this table. The length of this
    /// vector is the current size of the table.
    elements: Vec<FuncTableElem>,
    /// Maximum size that `elements` can grow to.
    maximum: Option<u32>,
    /// Whether elements of this table are initialized lazily.
    lazy_init: bool,
}

pub struct DynamicGcRefTable {
    /// Dynamically managed storage space for this table. The length of this
    /// vector is the current size of the table.
    elements: Vec<Option<VMGcRef>>,
    /// Maximum size that `elements` can grow to.
    maximum: Option<u32>,
}

/// Represents an instance's table.
pub enum Table {
    /// A "static" table where storage space is managed externally, currently
    /// used with the pooling allocator.
    Static(StaticTable),
    /// A "dynamic" table where table storage space is dynamically allocated via
    /// `malloc` (aka Rust's `Vec`).
    Dynamic(DynamicTable),
}

impl From<StaticTable> for Table {
    fn from(value: StaticTable) -> Self {
        Self::Static(value)
    }
}

impl From<StaticFuncTable> for Table {
    fn from(value: StaticFuncTable) -> Self {
        let t: StaticTable = value.into();
        t.into()
    }
}

impl From<StaticGcRefTable> for Table {
    fn from(value: StaticGcRefTable) -> Self {
        let t: StaticTable = value.into();
        t.into()
    }
}

impl From<DynamicTable> for Table {
    fn from(value: DynamicTable) -> Self {
        Self::Dynamic(value)
    }
}

impl From<DynamicFuncTable> for Table {
    fn from(value: DynamicFuncTable) -> Self {
        let t: DynamicTable = value.into();
        t.into()
    }
}

impl From<DynamicGcRefTable> for Table {
    fn from(value: DynamicGcRefTable) -> Self {
        let t: DynamicTable = value.into();
        t.into()
    }
}

fn wasm_to_table_type(ty: WasmRefType) -> TableElementType {
    match ty.heap_type.top() {
        WasmHeapTopType::Func => TableElementType::Func,
        WasmHeapTopType::Any | WasmHeapTopType::Extern => TableElementType::GcRef,
    }
}

impl Table {
    /// Create a new dynamic (movable) table instance for the specified table plan.
    pub fn new_dynamic(plan: &TablePlan, store: &mut dyn Store) -> Result<Self> {
        Self::limit_new(plan, store)?;
        let TableStyle::CallerChecksSignature { lazy_init } = plan.style;
        match wasm_to_table_type(plan.table.wasm_ty) {
            TableElementType::Func => Ok(Self::from(DynamicFuncTable {
                elements: vec![None; usize::try_from(plan.table.minimum).unwrap()],
                maximum: plan.table.maximum,
                lazy_init,
            })),
            TableElementType::GcRef => Ok(Self::from(DynamicGcRefTable {
                elements: (0..usize::try_from(plan.table.minimum).unwrap())
                    .map(|_| None)
                    .collect(),
                maximum: plan.table.maximum,
            })),
        }
    }

    /// Create a new static (immovable) table instance for the specified table plan.
    pub unsafe fn new_static(
        plan: &TablePlan,
        data: SendSyncPtr<[u8]>,
        store: &mut dyn Store,
    ) -> Result<Self> {
        Self::limit_new(plan, store)?;

        let size = plan.table.minimum;
        let max = plan
            .table
            .maximum
            .map_or(usize::MAX, |x| usize::try_from(x).unwrap());

        match wasm_to_table_type(plan.table.wasm_ty) {
            TableElementType::Func => {
                let len = {
                    let data = data.as_non_null().as_ref();
                    let (before, data, after) = data.align_to::<FuncTableElem>();
                    assert!(before.is_empty());
                    assert!(after.is_empty());
                    data.len()
                };
                ensure!(
                    usize::try_from(plan.table.minimum).unwrap() <= len,
                    "initial table size of {} exceeds the pooling allocator's \
                     configured maximum table size of {len} elements",
                    plan.table.minimum,
                );
                let data = SendSyncPtr::new(NonNull::slice_from_raw_parts(
                    data.as_non_null().cast::<FuncTableElem>(),
                    cmp::min(len, max),
                ));
                let TableStyle::CallerChecksSignature { lazy_init } = plan.style;
                Ok(Self::from(StaticFuncTable {
                    data,
                    size,
                    lazy_init,
                }))
            }
            TableElementType::GcRef => {
                let len = {
                    let data = data.as_non_null().as_ref();
                    let (before, data, after) = data.align_to::<Option<VMGcRef>>();
                    assert!(before.is_empty());
                    assert!(after.is_empty());
                    data.len()
                };
                ensure!(
                    usize::try_from(plan.table.minimum).unwrap() <= len,
                    "initial table size of {} exceeds the pooling allocator's \
                     configured maximum table size of {len} elements",
                    plan.table.minimum,
                );
                let data = SendSyncPtr::new(NonNull::slice_from_raw_parts(
                    data.as_non_null().cast::<Option<VMGcRef>>(),
                    cmp::min(len, max),
                ));
                Ok(Self::from(StaticGcRefTable { data, size }))
            }
        }
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
            Table::Static(StaticTable::Func(_)) | Table::Dynamic(DynamicTable::Func(_)) => {
                TableElementType::Func
            }
            Table::Static(StaticTable::GcRef(_)) | Table::Dynamic(DynamicTable::GcRef(_)) => {
                TableElementType::GcRef
            }
        }
    }

    /// Returns whether or not the underlying storage of the table is "static".
    #[cfg(feature = "pooling-allocator")]
    pub(crate) fn is_static(&self) -> bool {
        matches!(self, Table::Static(_))
    }

    /// Returns the number of allocated elements.
    pub fn size(&self) -> u32 {
        match self {
            Table::Static(StaticTable::Func(StaticFuncTable { size, .. })) => *size,
            Table::Static(StaticTable::GcRef(StaticGcRefTable { size, .. })) => *size,
            Table::Dynamic(DynamicTable::Func(DynamicFuncTable { elements, .. })) => {
                elements.len().try_into().unwrap()
            }
            Table::Dynamic(DynamicTable::GcRef(DynamicGcRefTable { elements, .. })) => {
                elements.len().try_into().unwrap()
            }
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
            Table::Static(StaticTable::Func(StaticFuncTable { data, .. })) => {
                Some(u32::try_from(data.len()).unwrap())
            }
            Table::Static(StaticTable::GcRef(StaticGcRefTable { data, .. })) => {
                Some(u32::try_from(data.len()).unwrap())
            }
            Table::Dynamic(DynamicTable::Func(DynamicFuncTable { maximum, .. })) => *maximum,
            Table::Dynamic(DynamicTable::GcRef(DynamicGcRefTable { maximum, .. })) => *maximum,
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

        let (funcrefs, lazy_init) = self.funcrefs_mut();
        let elements = funcrefs
            .get_mut(dst..)
            .and_then(|s| s.get_mut(..items.len()))
            .ok_or(Trap::TableOutOfBounds)?;

        for (item, slot) in items.zip(elements) {
            *slot = TaggedFuncRef::from(item, lazy_init);
        }
        Ok(())
    }

    /// Fill `table[dst..]` with values from `items`
    ///
    /// Returns a trap error on out-of-bounds accesses.
    pub fn init_gc_refs(
        &mut self,
        dst: u32,
        items: impl ExactSizeIterator<Item = Option<VMGcRef>>,
    ) -> Result<(), Trap> {
        let dst = usize::try_from(dst).map_err(|_| Trap::TableOutOfBounds)?;

        let elements = self
            .gc_refs_mut()
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
    pub fn fill(
        &mut self,
        gc_store: &mut GcStore,
        dst: u32,
        val: TableElement,
        len: u32,
    ) -> Result<(), Trap> {
        let start = dst as usize;
        let end = start
            .checked_add(len as usize)
            .ok_or_else(|| Trap::TableOutOfBounds)?;

        if end > self.size() as usize {
            return Err(Trap::TableOutOfBounds);
        }

        match val {
            TableElement::FuncRef(f) => {
                let (funcrefs, lazy_init) = self.funcrefs_mut();
                funcrefs[start..end].fill(TaggedFuncRef::from(f, lazy_init));
            }
            TableElement::GcRef(r) => {
                // Clone the init GC reference into each table slot.
                for slot in &mut self.gc_refs_mut()[start..end] {
                    gc_store.write_gc_ref(slot, r.as_ref());
                }

                // Drop the init GC reference, since we aren't holding onto this
                // reference anymore, only the clones in the table.
                if let Some(r) = r {
                    gc_store.drop_gc_ref(r);
                }
            }
            TableElement::UninitFunc => {
                let (funcrefs, _lazy_init) = self.funcrefs_mut();
                funcrefs[start..end].fill(TaggedFuncRef::UNINIT);
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
            Table::Static(StaticTable::Func(StaticFuncTable { data, size, .. })) => {
                unsafe {
                    debug_assert!(data.as_ref()[*size as usize..new_size as usize]
                        .iter()
                        .all(|x| x.is_none()));
                }
                *size = new_size;
            }
            Table::Static(StaticTable::GcRef(StaticGcRefTable { data, size })) => {
                unsafe {
                    debug_assert!(data.as_ref()[*size as usize..new_size as usize]
                        .iter()
                        .all(|x| x.is_none()));
                }
                *size = new_size;
            }

            // These calls to `resize` could move the base address of
            // `elements`. If this table's limits declare it to be fixed-size,
            // then during AOT compilation we may have promised Cranelift that
            // the table base address won't change, so it is allowed to optimize
            // loading the base address. However, in that case the above checks
            // that delta is non-zero and the new size doesn't exceed the
            // maximum mean we can't get here.
            Table::Dynamic(DynamicTable::Func(DynamicFuncTable { elements, .. })) => {
                elements.resize(usize::try_from(new_size).unwrap(), None);
            }
            Table::Dynamic(DynamicTable::GcRef(DynamicGcRefTable { elements, .. })) => {
                elements.resize_with(usize::try_from(new_size).unwrap(), || None);
            }
        }

        self.fill(store.gc_store(), old_size, init_value, delta)
            .expect("table should not be out of bounds");

        Ok(Some(old_size))
    }

    /// Get reference to the specified element.
    ///
    /// Returns `None` if the index is out of bounds.
    pub fn get(&self, gc_store: &mut GcStore, index: u32) -> Option<TableElement> {
        let index = usize::try_from(index).ok()?;
        match self.element_type() {
            TableElementType::Func => {
                let (funcrefs, lazy_init) = self.funcrefs();
                funcrefs
                    .get(index)
                    .copied()
                    .map(|e| e.into_table_element(lazy_init))
            }
            TableElementType::GcRef => self.gc_refs().get(index).map(|r| {
                let r = r.as_ref().map(|r| gc_store.clone_gc_ref(r));
                TableElement::GcRef(r)
            }),
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
                let (funcrefs, lazy_init) = self.funcrefs_mut();
                *funcrefs.get_mut(index).ok_or(())? = TaggedFuncRef::from(f, lazy_init);
            }
            TableElement::UninitFunc => {
                let (funcrefs, _lazy_init) = self.funcrefs_mut();
                *funcrefs.get_mut(index).ok_or(())? = TaggedFuncRef::UNINIT;
            }
            TableElement::GcRef(e) => {
                *self.gc_refs_mut().get_mut(index).ok_or(())? = e;
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
        gc_store: &mut GcStore,
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
            (*dst_table).copy_elements_within(gc_store, dst_range, src_range);
        } else {
            Self::copy_elements(gc_store, &mut *dst_table, &*src_table, dst_range, src_range);
        }

        Ok(())
    }

    /// Return a `VMTableDefinition` for exposing the table to compiled wasm code.
    pub fn vmtable(&mut self) -> VMTableDefinition {
        match self {
            Table::Static(StaticTable::Func(StaticFuncTable { data, size, .. })) => {
                VMTableDefinition {
                    base: data.as_ptr().cast(),
                    current_elements: *size,
                }
            }
            Table::Static(StaticTable::GcRef(StaticGcRefTable { data, size })) => {
                VMTableDefinition {
                    base: data.as_ptr().cast(),
                    current_elements: *size,
                }
            }
            Table::Dynamic(DynamicTable::Func(DynamicFuncTable { elements, .. })) => {
                VMTableDefinition {
                    base: elements.as_mut_ptr().cast(),
                    current_elements: elements.len().try_into().unwrap(),
                }
            }
            Table::Dynamic(DynamicTable::GcRef(DynamicGcRefTable { elements, .. })) => {
                VMTableDefinition {
                    base: elements.as_mut_ptr().cast(),
                    current_elements: elements.len().try_into().unwrap(),
                }
            }
        }
    }

    fn type_matches(&self, val: &TableElement) -> bool {
        self.element_type().matches(val)
    }

    fn funcrefs(&self) -> (&[TaggedFuncRef], bool) {
        assert_eq!(self.element_type(), TableElementType::Func);
        match self {
            Self::Dynamic(DynamicTable::Func(DynamicFuncTable {
                elements,
                lazy_init,
                ..
            })) => (
                unsafe { slice::from_raw_parts(elements.as_ptr().cast(), elements.len()) },
                *lazy_init,
            ),
            Self::Static(StaticTable::Func(StaticFuncTable {
                data,
                size,
                lazy_init,
            })) => (
                unsafe {
                    slice::from_raw_parts(data.as_ptr().cast(), usize::try_from(*size).unwrap())
                },
                *lazy_init,
            ),
            _ => unreachable!(),
        }
    }

    fn funcrefs_mut(&mut self) -> (&mut [TaggedFuncRef], bool) {
        assert_eq!(self.element_type(), TableElementType::Func);
        match self {
            Self::Dynamic(DynamicTable::Func(DynamicFuncTable {
                elements,
                lazy_init,
                ..
            })) => (
                unsafe { slice::from_raw_parts_mut(elements.as_mut_ptr().cast(), elements.len()) },
                *lazy_init,
            ),
            Self::Static(StaticTable::Func(StaticFuncTable {
                data,
                size,
                lazy_init,
            })) => (
                unsafe {
                    slice::from_raw_parts_mut(data.as_ptr().cast(), usize::try_from(*size).unwrap())
                },
                *lazy_init,
            ),
            _ => unreachable!(),
        }
    }

    fn gc_refs(&self) -> &[Option<VMGcRef>] {
        assert_eq!(self.element_type(), TableElementType::GcRef);
        match self {
            Self::Dynamic(DynamicTable::GcRef(DynamicGcRefTable { elements, .. })) => elements,
            Self::Static(StaticTable::GcRef(StaticGcRefTable { data, size })) => unsafe {
                &data.as_non_null().as_ref()[..usize::try_from(*size).unwrap()]
            },
            _ => unreachable!(),
        }
    }

    /// Get this table's GC references as a slice.
    ///
    /// Panics if this is not a table of GC references.
    pub fn gc_refs_mut(&mut self) -> &mut [Option<VMGcRef>] {
        assert_eq!(self.element_type(), TableElementType::GcRef);
        match self {
            Self::Dynamic(DynamicTable::GcRef(DynamicGcRefTable { elements, .. })) => elements,
            Self::Static(StaticTable::GcRef(StaticGcRefTable { data, size })) => unsafe {
                &mut data.as_non_null().as_mut()[..usize::try_from(*size).unwrap()]
            },
            _ => unreachable!(),
        }
    }

    fn copy_elements(
        gc_store: &mut GcStore,
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
                let (dst_funcrefs, _lazy_init) = dst_table.funcrefs_mut();
                let (src_funcrefs, _lazy_init) = src_table.funcrefs();
                dst_funcrefs[dst_range].copy_from_slice(&src_funcrefs[src_range]);
            }
            TableElementType::GcRef => {
                assert_eq!(
                    dst_range.end - dst_range.start,
                    src_range.end - src_range.start
                );
                assert!(dst_range.end <= dst_table.gc_refs().len());
                assert!(src_range.end <= src_table.gc_refs().len());
                for (dst, src) in dst_range.zip(src_range) {
                    gc_store.write_gc_ref(
                        &mut dst_table.gc_refs_mut()[dst],
                        src_table.gc_refs()[src].as_ref(),
                    );
                }
            }
        }
    }

    fn copy_elements_within(
        &mut self,
        gc_store: &mut GcStore,
        dst_range: Range<usize>,
        src_range: Range<usize>,
    ) {
        assert_eq!(
            dst_range.end - dst_range.start,
            src_range.end - src_range.start
        );

        // This is a no-op.
        if src_range.start == dst_range.start {
            return;
        }

        let ty = self.element_type();
        match ty {
            TableElementType::Func => {
                // `funcref` are `Copy`, so just do a memmove
                let (funcrefs, _lazy_init) = self.funcrefs_mut();
                funcrefs.copy_within(src_range, dst_range.start);
            }
            TableElementType::GcRef => {
                // We need to clone each `externref` while handling overlapping
                // ranges
                let elements = self.gc_refs_mut();
                if dst_range.start < src_range.start {
                    for (d, s) in dst_range.zip(src_range) {
                        let (ds, ss) = elements.split_at_mut(s);
                        let dst = &mut ds[d];
                        let src = ss[0].as_ref();
                        gc_store.write_gc_ref(dst, src);
                    }
                } else {
                    for (s, d) in src_range.rev().zip(dst_range.rev()) {
                        let (ss, ds) = elements.split_at_mut(d);
                        let dst = &mut ds[0];
                        let src = ss[s].as_ref();
                        gc_store.write_gc_ref(dst, src);
                    }
                }
            }
        }
    }
}

// The default table representation is an empty funcref table that cannot grow.
impl Default for Table {
    fn default() -> Self {
        Self::from(StaticFuncTable {
            data: SendSyncPtr::new(NonNull::from(&mut [])),
            size: 0,
            lazy_init: false,
        })
    }
}
