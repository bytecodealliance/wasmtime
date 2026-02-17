//! Infrastructure for managing "host tables" of resources in the component
//! model.
//!
//! This module is mostly an implementation detail of `ResourceAny` where it
//! tracks the ability for the host to have a handle to any resource, be it
//! either guest-defined or host-defined. The `HostResourceData` type is
//! stored inside of a `Store<T>` and `HostResourceTables` provides temporary
//! access to this information.
//!
//! This provides operations in terms of "lift" and "lower" terminology where
//! the idea is that a resource is "lowered" to the host meaning that metadata
//! about the resource is inserted into `HostResourceData` through
//! `HostResourceTables`. This produces a `HostResourceIndex` which can
//! then be used to access it later on. The `HostResourceIndex` can later be
//! used to "lift" the resource from the table back into its constituent parts
//! or remove it from the table.
//!
//! This tracks borrow state where you can, for example, lift a borrow from a
//! `HostResourceIndex` that represents an "own". This will activate
//! borrow-tracking infrastructure to ensure that the borrow is deleted before
//! the own is removed or otherwise taken out.

use crate::prelude::*;
use crate::runtime::component::RuntimeInstance;
use crate::runtime::vm::component::{ResourceTables, TypedResource, TypedResourceIndex};
use crate::runtime::vm::{SendSyncPtr, VMFuncRef};
use crate::store::StoreOpaque;
use core::ptr::NonNull;
use wasmtime_environ::component::TypeResourceTableIndex;

/// Metadata tracking the state of resources within a `Store`.
///
/// This is a borrowed structure created from a `Store` piecemeal from below.
/// The `ResourceTables` type holds most of the raw information and this
/// structure tacks on a reference to `HostResourceData` to track generation
/// numbers of host indices.
pub struct HostResourceTables<'a> {
    tables: ResourceTables<'a>,
    host_resource_data: &'a mut HostResourceData,
}

/// Metadata for host-owned resources owned within a `Store`.
///
/// This metadata is used to prevent the ABA problem with indices handed out as
/// part of `Resource` and `ResourceAny`. Those structures are `Copy` meaning
/// that it's easy to reuse them, possibly accidentally. To prevent issues in
/// the host Wasmtime attaches both an index (within `ResourceTables`) as well
/// as a 32-bit generation counter onto each `HostResourceIndex` which the host
/// actually holds in `Resource` and `ResourceAny`.
///
/// This structure holds a list which is a parallel list to the "list of reps"
/// that's stored within `ResourceTables` elsewhere in the `Store`. This
/// parallel list holds the last known generation of each element in the table.
/// The generation is then compared on access to make sure it's the same.
///
/// Whenever a slot in the table is allocated the `cur_generation` field is
/// pushed at the corresponding index of `generation_of_table_slot`. Whenever
/// a field is accessed the current value of `generation_of_table_slot` is
/// checked against the generation of the index. Whenever a slot is deallocated
/// the generation is incremented. Put together this means that any access of a
/// deallocated slot should deterministically provide an error.
#[derive(Default)]
pub struct HostResourceData {
    cur_generation: u32,
    table_slot_metadata: Vec<TableSlot>,
}

#[derive(Copy, Clone)]
pub struct TableSlot {
    generation: u32,
    pub(super) instance: Option<RuntimeInstance>,
    pub(super) dtor: Option<SendSyncPtr<VMFuncRef>>,
}

/// Host representation of an index into a table slot.
///
/// This is morally (u32, u32) but is encoded as a 64-bit integer. The low
/// 32-bits are the table index and the upper 32-bits are the generation
/// counter.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
#[repr(transparent)]
pub struct HostResourceIndex(u64);

impl HostResourceIndex {
    pub(super) fn new(idx: u32, generation: u32) -> HostResourceIndex {
        HostResourceIndex(u64::from(idx) | (u64::from(generation) << 32))
    }

    pub(super) const fn index(&self) -> u32 {
        (self.0 & 0xffffffff) as u32
    }

    pub(super) const fn generation(&self) -> u32 {
        (self.0 >> 32) as u32
    }
}

impl<'a> HostResourceTables<'a> {
    pub fn new_host(store: &'a mut StoreOpaque) -> HostResourceTables<'a> {
        let (tables, data) = store.component_resource_tables_and_host_resource_data(None);
        HostResourceTables::from_parts(tables, data)
    }

    pub fn from_parts(
        tables: ResourceTables<'a>,
        host_resource_data: &'a mut HostResourceData,
    ) -> Self {
        HostResourceTables {
            tables,
            host_resource_data,
        }
    }

    /// Lifts an `own` resource that resides in the host's tables at the `idx`
    /// specified into its `rep`.
    ///
    /// # Errors
    ///
    /// Returns an error if `idx` doesn't point to a valid owned resource, or
    /// if `idx` can't be lifted as an `own` (e.g. it has active borrows).
    pub fn host_resource_lift_own(&mut self, idx: HostResourceIndex) -> Result<u32> {
        let (idx, _) = self.validate_host_index(idx, true)?;
        self.tables.resource_lift_own(TypedResourceIndex::Host(idx))
    }

    /// See [`HostResourceTables::host_resource_lift_own`].
    pub fn host_resource_lift_borrow(&mut self, idx: HostResourceIndex) -> Result<u32> {
        let (idx, _) = self.validate_host_index(idx, false)?;
        self.tables
            .resource_lift_borrow(TypedResourceIndex::Host(idx))
    }

    /// Lowers an `own` resource to be owned by the host.
    ///
    /// This returns a new index into the host's set of resource tables which
    /// will point to the `rep` specified. The returned index is suitable for
    /// conversion into either [`Resource`] or [`ResourceAny`].
    ///
    /// The `dtor` and instance `instance` are specified as well to know what
    /// destructor to run when this resource is destroyed.
    pub fn host_resource_lower_own(
        &mut self,
        rep: u32,
        dtor: Option<NonNull<VMFuncRef>>,
        instance: Option<RuntimeInstance>,
    ) -> Result<HostResourceIndex> {
        let idx = self.tables.resource_lower_own(TypedResource::Host(rep))?;
        Ok(self.new_host_index(idx, dtor, instance))
    }

    /// See [`HostResourceTables::host_resource_lower_own`].
    pub fn host_resource_lower_borrow(&mut self, rep: u32) -> Result<HostResourceIndex> {
        let idx = self
            .tables
            .resource_lower_borrow(TypedResource::Host(rep))?;
        Ok(self.new_host_index(idx, None, None))
    }

    /// Validates that `idx` is still valid for the host tables, notably
    /// ensuring that the generation listed in `idx` is the same as the
    /// last recorded generation of the slot itself.
    ///
    /// The `is_removal` option indicates whether or not this table access will
    /// end up removing the element from the host table. In such a situation the
    /// current generation number is incremented.
    fn validate_host_index(
        &mut self,
        idx: HostResourceIndex,
        is_removal: bool,
    ) -> Result<(u32, Option<TableSlot>)> {
        let actual = usize::try_from(idx.index())
            .ok()
            .and_then(|i| self.host_resource_data.table_slot_metadata.get(i).copied());

        // If `idx` is out-of-bounds then skip returning an error. In such a
        // situation the operation that this is guarding will return a more
        // precise error, such as a lift operation.
        if let Some(actual) = actual {
            if actual.generation != idx.generation() {
                bail!("host-owned resource is being used with the wrong type");
            }
        }

        // Bump the current generation of this is a removal to ensure any
        // future item placed in this slot can't be pointed to by the `idx`
        // provided above.
        if is_removal {
            self.host_resource_data.cur_generation += 1;
        }

        Ok((idx.index(), actual))
    }

    /// Creates a new `HostResourceIndex` which will point to the raw table
    /// slot provided by `idx`.
    ///
    /// This will register metadata necessary to track the current generation
    /// in the returned `HostResourceIndex` as well.
    fn new_host_index(
        &mut self,
        idx: u32,
        dtor: Option<NonNull<VMFuncRef>>,
        instance: Option<RuntimeInstance>,
    ) -> HostResourceIndex {
        let list = &mut self.host_resource_data.table_slot_metadata;
        let info = TableSlot {
            generation: self.host_resource_data.cur_generation,
            instance,
            dtor: dtor.map(SendSyncPtr::new),
        };
        match list.get_mut(idx as usize) {
            Some(slot) => *slot = info,
            None => {
                // Resource handles start at 1, not zero, so push two elements
                // for the first resource handle.
                if list.is_empty() {
                    assert_eq!(idx, 1);
                    list.push(TableSlot {
                        generation: 0,
                        instance: None,
                        dtor: None,
                    });
                }
                assert_eq!(idx as usize, list.len());
                list.push(info);
            }
        }

        HostResourceIndex::new(idx, info.generation)
    }

    /// Drops a host-owned resource from host tables.
    ///
    /// This method will attempt to interpret `idx` as pointing to either a
    /// `borrow` or `own` resource with the `expected` type specified. This
    /// method will then return the underlying `rep` if it points to an `own`
    /// resource which can then be further processed for destruction.
    ///
    /// # Errors
    ///
    /// Returns an error if `idx` doesn't point to a valid resource, points to
    /// an `own` with active borrows, or if it doesn't have the type `expected`
    /// in the host tables.
    pub(super) fn host_resource_drop(
        &mut self,
        idx: HostResourceIndex,
    ) -> Result<Option<(u32, TableSlot)>> {
        let (idx, slot) = self.validate_host_index(idx, true)?;
        match self.tables.resource_drop(TypedResourceIndex::Host(idx))? {
            Some(rep) => Ok(Some((rep, slot.unwrap()))),
            None => Ok(None),
        }
    }

    /// Lowers an `own` resource into the guest, converting the `rep` specified
    /// into a guest-local index.
    ///
    /// The `ty` provided is which table to put this into.
    pub fn guest_resource_lower_own(
        &mut self,
        rep: u32,
        ty: TypeResourceTableIndex,
    ) -> Result<u32> {
        self.tables
            .resource_lower_own(TypedResource::Component { ty, rep })
    }

    /// Lowers a `borrow` resource into the guest, converting the `rep`
    /// specified into a guest-local index.
    ///
    /// The `ty` provided is which table to put this into.
    ///
    /// Note that this cannot be used in isolation because lowering a borrow
    /// into a guest has a special case where `rep` is returned directly if `ty`
    /// belongs to the component being lowered into. That property must be
    /// handled by the caller of this function.
    pub fn guest_resource_lower_borrow(
        &mut self,
        rep: u32,
        ty: TypeResourceTableIndex,
    ) -> Result<u32> {
        self.tables
            .resource_lower_borrow(TypedResource::Component { ty, rep })
    }

    /// Lifts an `own` resource from the `idx` specified from the table `ty`.
    ///
    /// This will lookup the appropriate table in the guest and return the `rep`
    /// corresponding to `idx` if it's valid.
    pub fn guest_resource_lift_own(
        &mut self,
        index: u32,
        ty: TypeResourceTableIndex,
    ) -> Result<u32> {
        self.tables
            .resource_lift_own(TypedResourceIndex::Component { ty, index })
    }

    /// Lifts a `borrow` resource from the `idx` specified from the table `ty`.
    ///
    /// This will lookup the appropriate table in the guest and return the `rep`
    /// corresponding to `idx` if it's valid.
    pub fn guest_resource_lift_borrow(
        &mut self,
        index: u32,
        ty: TypeResourceTableIndex,
    ) -> Result<u32> {
        self.tables
            .resource_lift_borrow(TypedResourceIndex::Component { ty, index })
    }

    /// Completes a call into the component instance, validating that it's ok to
    /// complete by ensuring the are no remaining active borrows.
    #[inline]
    pub fn validate_scope_exit(&mut self) -> Result<()> {
        self.tables.validate_scope_exit()
    }
}
