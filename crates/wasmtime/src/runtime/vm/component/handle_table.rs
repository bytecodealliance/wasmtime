use super::{TransmitLocalStateKind, TypedResourceIndex};
use crate::prelude::TryVec;
use crate::vm::{VMHandleTableSlot, VMScope, VMTransmitLocalState, VMTypedResource, VmPtr, VmSafe};
use crate::{Result, bail};
use core::mem;
use core::ptr::NonNull;
use wasmtime_environ::component::{TypeFutureTableIndex, TypeStreamTableIndex};

// SAFETY: `VMHandleTableSlot` is generated with `#[repr(C, u32)]` and every
// variant holds only `u32`s and `VmSafe` structs of them.
unsafe impl VmSafe for VMHandleTableSlot {}

#[cfg(test)]
mod test_vm_handle_table_slot {
    use wasmtime_environ::PtrSize;

    /// A lent resource's `lend_count` must sit at the same offset whether the
    /// handle is an `own` or a `borrow`, because compiled code adjusts it
    /// without first branching on the slot's tag.
    ///
    /// A field inserted ahead of `lend_count` in only one of the two variants
    /// would silently break that, which is what this guards against.
    ///
    /// Every field offset here is itself checked against the real Rust layout
    /// by the generated `test_vm_type_layouts::vm_handle_table_slot`.
    #[test]
    fn lend_count_offset_is_variant_agnostic() {
        for ptr in [4u8, 8] {
            let slot = ptr.vm_handle_table_slot();
            assert_eq!(
                slot.resource_own().lend_count(),
                slot.resource_borrow().lend_count(),
            );
        }
    }
}

/// The maximum handle value is specified in
/// <https://github.com/WebAssembly/component-model/blob/main/design/mvp/CanonicalABI.md>
/// currently and keeps the upper bits free for use in the component and ABI.
const MAX_HANDLE: u32 = 1 << 28;

/// Count of allocated slots across all `HandleTable`s in a store.
///
/// This count and limit is intended to roughly correspond with the amount of
/// host memory being used to track handles within a store. This includes
/// operations such as:
///
/// * Creating a handle (via `insert`) retains space for this. This is never
///   decremented since the tables are never shrunk.
/// * Tracking lends to create a `borrow` (via `resource_lend` and in
///   `resource_lift_borrow` elsewhere allocates memory). This is later
///   decremented in `resource_undo_lend` because the backing vector is indeed
///   dropped when the frame goes out of scope.
///
/// This means that sometimes a handle is double-counted (e.g. borrows in some
/// situations), but this is intended to still reflect live active memory use.
pub struct HandleCounter {
    used: usize,
    limit: usize,
}

impl HandleCounter {
    /// Creates a counter with no allocated slots and the given `limit`.
    pub fn new(limit: usize) -> HandleCounter {
        HandleCounter { used: 0, limit }
    }

    /// Returns the configured limit on allocated slots.
    pub fn limit(&self) -> usize {
        self.limit
    }

    /// Updates the limit. Does not retroactively trap tables already over it.
    pub fn set_limit(&mut self, limit: usize) {
        self.limit = limit;
    }

    /// Returns whether `n` more slots can be allocated without exceeding the
    /// limit.
    pub fn has_room(&self, n: usize) -> bool {
        self.used
            .checked_add(n)
            .is_some_and(|used| used <= self.limit)
    }
}

/// Return value from [`HandleTable::remove_resource`].
pub enum RemovedResource {
    /// An `own` resource was removed with the specified `rep`
    Own { rep: u32 },
    /// A `borrow` resource was removed originally created within `scope`.
    Borrow { scope: VMScope },
}

/// Different kinds of waitables returned by [`HandleTable::waitable_rep`].
pub enum Waitable {
    Subtask { is_host: bool },
    Future,
    Stream,
}

pub struct HandleTable {
    next: u32,
    slots: TryVec<VMHandleTableSlot>,
}

impl Default for HandleTable {
    fn default() -> Self {
        Self {
            next: 0,
            slots: TryVec::new(),
        }
    }
}

impl HandleTable {
    /// This table's slots, as compiled code reads them.
    pub fn slots_ptr(&mut self) -> VmPtr<VMHandleTableSlot> {
        // An empty `TryVec`'s pointer is dangling but well-aligned; a table
        // with no slots has none for compiled code to reach.
        VmPtr::from(NonNull::new(self.slots.as_mut_ptr()).unwrap())
    }

    /// Returns whether or not this table is empty.
    pub fn is_empty(&self) -> bool {
        self.slots
            .iter()
            .all(|slot| matches!(slot, VMHandleTableSlot::Free { .. }))
    }

    /// Begins inserting into this table, which compiled code reads through
    /// `vm_ptr`, re-publishing the table's slots there whenever they move.
    ///
    /// # Safety
    ///
    /// `vm_ptr` must be the `VMComponentContext` entry through which compiled
    /// code reads this table's slots, or else compiled code may later read
    /// through a stale pointer.
    pub unsafe fn insert<'a>(
        &'a mut self,
        counter: &'a mut HandleCounter,
        vm_ptr: &'a mut VmPtr<VMHandleTableSlot>,
    ) -> HandleTableInsertion<'a> {
        HandleTableInsertion {
            table: self,
            counter,
            vm_ptr: Some(vm_ptr),
        }
    }

    /// Begins inserting into this table, which compiled code never reads.
    ///
    /// # Safety
    ///
    /// This table's slots must never be published to compiled code, or else
    /// compiled code may later read through a stale pointer.
    pub unsafe fn insert_unpublished<'a>(
        &'a mut self,
        counter: &'a mut HandleCounter,
    ) -> HandleTableInsertion<'a> {
        HandleTableInsertion {
            table: self,
            counter,
            vm_ptr: None,
        }
    }

    /// Frees the slot at `idx` for reuse by a later `insert` into this table.
    ///
    /// The slot stays charged to the store's `HandleCounter`; see its docs.
    fn remove(&mut self, idx: u32) -> Result<()> {
        let to_fill = VMHandleTableSlot::Free { next: self.next };
        let slot = self.get_mut(idx)?;
        *slot = to_fill;
        self.next = idx - 1;
        Ok(())
    }

    fn handle_index_to_table_index(&self, idx: u32) -> Option<usize> {
        // NB: `idx` is decremented by one to account for the `+1` above during
        // allocation.
        let idx = idx.checked_sub(1)?;
        usize::try_from(idx).ok()
    }

    fn get_mut(&mut self, idx: u32) -> Result<&mut VMHandleTableSlot> {
        let slot = self
            .handle_index_to_table_index(idx)
            .and_then(|i| self.slots.get_mut(i));
        match slot {
            None | Some(VMHandleTableSlot::Free { .. }) => bail!("unknown handle index {idx}"),
            Some(slot) => Ok(slot),
        }
    }

    /// Returns the internal "rep" of the resource specified by `idx`.
    ///
    /// Returns an error if `idx` is out-of-bounds or doesn't point to a
    /// resource of the appropriate type.
    pub fn resource_rep(&mut self, idx: TypedResourceIndex) -> Result<u32> {
        match self.get_mut(idx.raw_index())? {
            VMHandleTableSlot::ResourceOwn { resource, .. }
            | VMHandleTableSlot::ResourceBorrow { resource, .. } => resource.rep(&idx),
            _ => bail!("index is not a resource"),
        }
    }

    /// Accesses the "rep" of the resource pointed to by `idx` as part of a
    /// lending operation.
    ///
    /// This will increase `lend_count` for both `own` and `borrow` handles and
    /// must be paired with a `resource_undo_lend` below later on (managed by
    /// `CallContexts`). Each lend is also charged to `counter` until then.
    ///
    /// Upon success returns the "rep" of the resource.
    pub fn resource_lend(
        &mut self,
        counter: &mut HandleCounter,
        idx: TypedResourceIndex,
    ) -> Result<u32> {
        self.resource_lend_impl(idx, Some(counter))
    }

    /// Like `resource_lend`, but only validates the lend, leaving `lend_count`
    /// and the store's handle count alone.
    ///
    /// For lends recorded in the `lazy_lenders` buffer, whose `lend_count`
    /// increments and handle charges are deferred until the recording thread is
    /// forced.
    pub fn resource_lend_deferred(&mut self, idx: TypedResourceIndex) -> Result<u32> {
        self.resource_lend_impl(idx, None)
    }

    /// Validates a lend of `idx`, additionally counting it when given the
    /// store's `counter`.
    fn resource_lend_impl(
        &mut self,
        idx: TypedResourceIndex,
        counter: Option<&mut HandleCounter>,
    ) -> Result<u32> {
        match self.get_mut(idx.raw_index())? {
            VMHandleTableSlot::ResourceOwn {
                resource,
                lend_count,
            }
            | VMHandleTableSlot::ResourceBorrow {
                resource,
                lend_count,
                ..
            } => {
                let rep = resource.rep(&idx)?;
                if let Some(counter) = counter {
                    let Some(new_lend_count) = lend_count.checked_add(1) else {
                        bail!("cannot lend resource: too many active lends")
                    };
                    if counter.used >= counter.limit {
                        bail!("cannot lend resource: store handle limit exceeded");
                    }
                    counter.used += 1;
                    *lend_count = new_lend_count;
                }
                Ok(rep)
            }
            _ => bail!("index {} is not a resource", idx.raw_index()),
        }
    }

    /// Undoes a lending operation previously performed by `resource_lend`.
    pub fn resource_undo_lend(
        &mut self,
        counter: &mut HandleCounter,
        idx: TypedResourceIndex,
    ) -> Result<()> {
        match self.get_mut(idx.raw_index())? {
            VMHandleTableSlot::ResourceOwn { lend_count, .. }
            | VMHandleTableSlot::ResourceBorrow { lend_count, .. } => {
                *lend_count -= 1;
                counter.used -= 1;
                Ok(())
            }
            _ => bail!("index {} is not a resource", idx.raw_index()),
        }
    }

    /// Removes the resource specified by `idx` from the table.
    ///
    /// This can fail if `idx` doesn't point to a resource, points to a
    /// resource (own or borrow) which is currently lent out, or points to a
    /// resource of the wrong type.
    pub fn remove_resource(&mut self, idx: TypedResourceIndex) -> Result<RemovedResource> {
        let ret = match self.get_mut(idx.raw_index())? {
            VMHandleTableSlot::ResourceOwn {
                resource,
                lend_count,
            } => {
                if *lend_count != 0 {
                    bail!("cannot remove owned resource while borrowed")
                }
                RemovedResource::Own {
                    rep: resource.rep(&idx)?,
                }
            }
            VMHandleTableSlot::ResourceBorrow {
                resource,
                scope,
                lend_count,
            } => {
                // Ensure the drop is done with the right type
                resource.rep(&idx)?;
                if *lend_count != 0 {
                    bail!("cannot remove borrowed resource while it is lent out")
                }
                RemovedResource::Borrow { scope: *scope }
            }
            _ => bail!("index {} is not a resource", idx.raw_index()),
        };
        self.remove(idx.raw_index())?;
        Ok(ret)
    }

    /// Returns the `rep` and `state` associated with the stream pointed to by
    /// `idx`.
    ///
    /// Returns an error if `idx` is out-of-bounds or doesn't point to a stream
    /// of type `ty`.
    pub fn stream_rep(
        &mut self,
        expected_ty: TypeStreamTableIndex,
        idx: u32,
    ) -> Result<(u32, &mut VMTransmitLocalState)> {
        match self.get_mut(idx)? {
            VMHandleTableSlot::Stream { rep, ty, state } => {
                if *ty != expected_ty.as_u32() {
                    bail!("handle is a stream of a different type");
                }
                Ok((*rep, state))
            }
            _ => bail!("handle is not a stream"),
        }
    }

    /// Removes the stream handle from `idx`, returning its `rep`.
    ///
    /// The stream must have the type `ty` and additionally be in a state
    /// suitable for removal.
    ///
    /// Returns the `rep` for the stream along with whether the stream was
    /// "done" or the writable end was witnessed as being done.
    pub fn stream_remove_readable(
        &mut self,
        expected_ty: TypeStreamTableIndex,
        idx: u32,
    ) -> Result<(u32, bool)> {
        let ret = match self.get_mut(idx)? {
            VMHandleTableSlot::Stream { rep, ty, state } => {
                if *ty != expected_ty.as_u32() {
                    bail!("handle is a stream of a different type");
                }
                let is_done = match state.kind() {
                    TransmitLocalStateKind::Read { done } => done,
                    TransmitLocalStateKind::Write { .. } => {
                        bail!("handle is not a readable end of a stream")
                    }
                    TransmitLocalStateKind::Busy | TransmitLocalStateKind::Cancelling => {
                        bail!("cannot remove busy stream")
                    }
                };
                (*rep, is_done)
            }
            _ => bail!("handle is not a stream"),
        };
        self.remove(idx)?;
        Ok(ret)
    }

    /// Removes the writable stream handle from `idx`, returning its `rep`.
    pub fn stream_remove_writable(
        &mut self,
        expected_ty: TypeStreamTableIndex,
        idx: u32,
    ) -> Result<u32> {
        let ret = match self.get_mut(idx)? {
            VMHandleTableSlot::Stream { rep, ty, state } => {
                if *ty != expected_ty.as_u32() {
                    bail!("handle is a stream of a different type");
                }
                match state.kind() {
                    TransmitLocalStateKind::Write { .. } => {}
                    TransmitLocalStateKind::Read { .. } => {
                        bail!("passed read end to `stream.drop-writable`")
                    }
                    TransmitLocalStateKind::Busy | TransmitLocalStateKind::Cancelling => {
                        bail!("cannot drop busy stream")
                    }
                }
                *rep
            }
            _ => bail!("handle is not a stream"),
        };
        self.remove(idx)?;
        Ok(ret)
    }

    /// Returns the `rep` and `state` associated with the future pointed to by
    /// `idx`.
    ///
    /// Returns an error if `idx` is out-of-bounds or doesn't point to a future
    /// of type `ty`.
    pub fn future_rep(
        &mut self,
        expected_ty: TypeFutureTableIndex,
        idx: u32,
    ) -> Result<(u32, &mut VMTransmitLocalState)> {
        match self.get_mut(idx)? {
            VMHandleTableSlot::Future { rep, ty, state } => {
                if *ty != expected_ty.as_u32() {
                    bail!("handle is a future of a different type");
                }
                Ok((*rep, state))
            }
            _ => bail!("handle is not a future"),
        }
    }

    /// Removes the future handle from `idx`, returning its `rep`.
    ///
    /// The future must have the type `ty` and additionally be in a state
    /// suitable for removal.
    ///
    /// Returns the `rep` for the future along with whether the future was
    /// "done" or the writable end was witnessed as being done.
    pub fn future_remove_readable(
        &mut self,
        expected_ty: TypeFutureTableIndex,
        idx: u32,
    ) -> Result<(u32, bool)> {
        let ret = match self.get_mut(idx)? {
            VMHandleTableSlot::Future { rep, ty, state } => {
                if *ty != expected_ty.as_u32() {
                    bail!("handle is a future of a different type");
                }
                let is_done = match state.kind() {
                    TransmitLocalStateKind::Read { done } => done,
                    TransmitLocalStateKind::Write { .. } => {
                        bail!("handle is not a readable end of a future")
                    }
                    TransmitLocalStateKind::Busy | TransmitLocalStateKind::Cancelling => {
                        bail!("cannot remove busy future")
                    }
                };
                (*rep, is_done)
            }
            _ => bail!("handle is not a future"),
        };
        self.remove(idx)?;
        Ok(ret)
    }

    /// Removes the writable future handle from `idx`, returning its `rep` along
    /// with whether the writer is "done" (i.e. it either successfully wrote a
    /// value or was notified that the readable end was dropped).
    pub fn future_remove_writable(
        &mut self,
        expected_ty: TypeFutureTableIndex,
        idx: u32,
    ) -> Result<(u32, bool)> {
        let ret = match self.get_mut(idx)? {
            VMHandleTableSlot::Future { rep, ty, state } => {
                if *ty != expected_ty.as_u32() {
                    bail!("handle is a future of a different type");
                }
                let is_done = match state.kind() {
                    TransmitLocalStateKind::Write { done } => done,
                    TransmitLocalStateKind::Read { .. } => {
                        bail!("passed read end to `future.drop-writable`")
                    }
                    TransmitLocalStateKind::Busy | TransmitLocalStateKind::Cancelling => {
                        bail!("cannot drop busy future")
                    }
                };
                (*rep, is_done)
            }
            _ => bail!("handle is not a future"),
        };
        self.remove(idx)?;
        Ok(ret)
    }

    /// Returns the `rep` of an error-context pointed to by `idx`.
    pub fn error_context_rep(&mut self, idx: u32) -> Result<u32> {
        match self.get_mut(idx)? {
            VMHandleTableSlot::ErrorContext { rep } => Ok(*rep),
            _ => bail!("handle is not an error-context"),
        }
    }

    /// Drops the error-context pointed to by `idx`.
    ///
    /// Returns the internal `rep`.
    pub fn error_context_drop(&mut self, idx: u32) -> Result<u32> {
        let rep = match self.get_mut(idx)? {
            VMHandleTableSlot::ErrorContext { rep } => *rep,
            _ => bail!("handle is not an error-context"),
        };
        self.remove(idx)?;
        Ok(rep)
    }

    /// Returns the `rep` of the subtask at `idx` as well as if it's a host
    /// task or not.
    pub fn subtask_rep(&mut self, idx: u32) -> Result<(u32, bool)> {
        match self.get_mut(idx)? {
            VMHandleTableSlot::GuestTask { rep } => Ok((*rep, false)),
            VMHandleTableSlot::HostTask { rep } => Ok((*rep, true)),
            _ => bail!("handle is not a subtask"),
        }
    }

    /// Removes the subtask set at `idx`, returning its `rep`.
    pub fn subtask_remove(&mut self, idx: u32) -> Result<(u32, bool)> {
        let ret = match self.get_mut(idx)? {
            VMHandleTableSlot::GuestTask { rep } => (*rep, false),
            VMHandleTableSlot::HostTask { rep } => (*rep, true),
            _ => bail!("handle is not a subtask"),
        };
        self.remove(idx)?;
        Ok(ret)
    }

    /// Returns the `rep` of an waitable-set pointed to by `idx`.
    pub fn waitable_set_rep(&mut self, idx: u32) -> Result<u32> {
        match self.get_mut(idx)? {
            VMHandleTableSlot::WaitableSet { rep, .. } => Ok(*rep),
            _ => bail!("handle is not an waitable-set"),
        }
    }

    /// Removes the waitable set at `idx`, returning its `rep`.
    pub fn waitable_set_remove(&mut self, idx: u32) -> Result<u32> {
        let ret = match self.get_mut(idx)? {
            VMHandleTableSlot::WaitableSet { rep } => *rep,
            _ => bail!("handle is not a waitable-set"),
        };
        self.remove(idx)?;
        Ok(ret)
    }

    /// Returns the `rep` for the waitable specified by `idx` along with what
    /// kind of waitable it is.
    pub fn waitable_rep(&mut self, idx: u32) -> Result<(u32, Waitable)> {
        match self.get_mut(idx)? {
            VMHandleTableSlot::GuestTask { rep } => {
                Ok((*rep, Waitable::Subtask { is_host: false }))
            }
            VMHandleTableSlot::HostTask { rep } => Ok((*rep, Waitable::Subtask { is_host: true })),
            VMHandleTableSlot::Future { rep, .. } => Ok((*rep, Waitable::Future)),
            VMHandleTableSlot::Stream { rep, .. } => Ok((*rep, Waitable::Stream)),
            _ => bail!("handle is not a waitable"),
        }
    }
}

/// An in-progress insertion into a `HandleTable`, created by
/// `HandleTable::insert` or `HandleTable::insert_unpublished`.
///
/// Keeps the copy of the table's slots pointer that compiled code reads, if
/// any, up to date as the table grows.
#[must_use]
pub struct HandleTableInsertion<'a> {
    table: &'a mut HandleTable,
    counter: &'a mut HandleCounter,
    /// `None` when compiled code never reads `table`.
    vm_ptr: Option<&'a mut VmPtr<VMHandleTableSlot>>,
}

impl HandleTableInsertion<'_> {
    fn insert(&mut self, slot: VMHandleTableSlot) -> Result<u32> {
        let table = &mut *self.table;
        let next = table.next;

        // The component model reserves index 0 as never allocatable so add one
        // to the table index to start the numbering at 1 instead. Also note
        // that the component model places an upper-limit per-table on the
        // maximum allowed index.
        //
        // First check to make sure the returned handle is in-bounds, then do
        // the actual allocation below.
        if next + 1 >= MAX_HANDLE {
            bail!("cannot allocate another handle: index overflow");
        }

        // Growing the table is charged to the store-wide cap; `MAX_HANDLE`
        // alone scales with the guest-controlled instance count. Reusing a
        // free slot was already charged when it was allocated. See
        // `HandleCounter`.
        if next as usize == table.slots.len() {
            if self.counter.used >= self.counter.limit {
                bail!("cannot allocate another handle: store handle limit exceeded");
            }
            table.slots.push(VMHandleTableSlot::Free {
                next: next.checked_add(1).unwrap(),
            })?;
            self.counter.used += 1;
            if let Some(vm_ptr) = &mut self.vm_ptr {
                **vm_ptr = table.slots_ptr();
            }
        }
        table.next = match mem::replace(&mut table.slots[next as usize], slot) {
            VMHandleTableSlot::Free { next } => next,
            _ => unreachable!(),
        };

        Ok(next + 1)
    }

    /// Inserts a new `own` resource whose type/rep are specified by
    /// `resource`.
    pub fn resource_own(&mut self, resource: VMTypedResource) -> Result<u32> {
        self.insert(VMHandleTableSlot::ResourceOwn {
            resource,
            lend_count: 0,
        })
    }

    /// Inserts a new `borrow` resource whose type/rep are specified by
    /// `resource`. The `scope` specified is used by `CallContexts` to manage
    /// lending information.
    pub fn resource_borrow(&mut self, resource: VMTypedResource, scope: VMScope) -> Result<u32> {
        self.insert(VMHandleTableSlot::ResourceBorrow {
            resource,
            scope,
            lend_count: 0,
        })
    }

    /// Inserts a readable-end stream of type `ty` and with the specified `rep`.
    ///
    /// Returns the table-local index of the stream.
    pub fn stream_read(&mut self, ty: TypeStreamTableIndex, rep: u32) -> Result<u32> {
        self.insert(VMHandleTableSlot::Stream {
            rep,
            ty: ty.as_u32(),
            state: VMTransmitLocalState::read(false),
        })
    }

    /// Inserts a writable-end stream of type `ty` and with the specified `rep`.
    ///
    /// Returns the table-local index of the stream.
    pub fn stream_write(&mut self, ty: TypeStreamTableIndex, rep: u32) -> Result<u32> {
        self.insert(VMHandleTableSlot::Stream {
            rep,
            ty: ty.as_u32(),
            state: VMTransmitLocalState::write(false),
        })
    }

    /// Inserts a readable-end future of type `ty` and with the specified `rep`.
    ///
    /// Returns the table-local index of the future.
    pub fn future_read(&mut self, ty: TypeFutureTableIndex, rep: u32) -> Result<u32> {
        self.insert(VMHandleTableSlot::Future {
            rep,
            ty: ty.as_u32(),
            state: VMTransmitLocalState::read(false),
        })
    }

    /// Inserts a writable-end future of type `ty` and with the specified `rep`.
    ///
    /// Returns the table-local index of the future.
    pub fn future_write(&mut self, ty: TypeFutureTableIndex, rep: u32) -> Result<u32> {
        self.insert(VMHandleTableSlot::Future {
            rep,
            ty: ty.as_u32(),
            state: VMTransmitLocalState::write(false),
        })
    }

    /// Inserts the error-context `rep`, returning the index it now resides at.
    pub fn error_context(&mut self, rep: u32) -> Result<u32> {
        self.insert(VMHandleTableSlot::ErrorContext { rep })
    }

    /// Inserts `rep` as a guest subtask.
    pub fn subtask_guest(&mut self, rep: u32) -> Result<u32> {
        self.insert(VMHandleTableSlot::GuestTask { rep })
    }

    /// Inserts `rep` as a host subtask.
    pub fn subtask_host(&mut self, rep: u32) -> Result<u32> {
        self.insert(VMHandleTableSlot::HostTask { rep })
    }

    /// Inserts `rep` as a waitable set.
    pub fn waitable_set(&mut self, rep: u32) -> Result<u32> {
        self.insert(VMHandleTableSlot::WaitableSet { rep })
    }
}

/// A table of guest threads, which compiled code never reads.
#[derive(Default)]
#[cfg(feature = "component-model-async")]
pub struct ThreadHandleTable(HandleTable);

#[cfg(feature = "component-model-async")]
impl ThreadHandleTable {
    /// Inserts the guest thread `rep` into this table, returning the index it
    /// now resides at.
    pub fn guest_thread_insert(&mut self, counter: &mut HandleCounter, rep: u32) -> Result<u32> {
        // SAFETY: a `ThreadHandleTable` is never published to compiled code.
        unsafe { self.0.insert_unpublished(counter) }.insert(VMHandleTableSlot::GuestThread { rep })
    }

    /// Returns the `rep` of a guest thread pointed to by `idx`.
    pub fn guest_thread_rep(&mut self, idx: u32) -> Result<u32> {
        match self.0.get_mut(idx)? {
            VMHandleTableSlot::GuestThread { rep } => Ok(*rep),
            _ => bail!("handle is not a guest thread"),
        }
    }

    /// Removes the guest thread pointed to by `idx`.
    ///
    /// Returns the internal `rep`.
    pub fn guest_thread_remove(&mut self, idx: u32) -> Result<u32> {
        let rep = match self.0.get_mut(idx)? {
            VMHandleTableSlot::GuestThread { rep } => *rep,
            _ => bail!("handle is not a guest thread"),
        };
        self.0.remove(idx)?;
        Ok(rep)
    }
}
