use super::{TypedResource, TypedResourceIndex};
use crate::{Result, bail};
use alloc::vec::Vec;
use core::mem;
use wasmtime_environ::component::{TypeFutureTableIndex, TypeStreamTableIndex};

/// The maximum handle value is specified in
/// <https://github.com/WebAssembly/component-model/blob/main/design/mvp/CanonicalABI.md>
/// currently and keeps the upper bits free for use in the component and ABI.
const MAX_HANDLE: u32 = 1 << 28;

/// Represents the state of a stream or future handle from the perspective of a
/// given component instance.
#[derive(Debug, Eq, PartialEq)]
pub enum TransmitLocalState {
    /// The write end of the stream or future.
    Write {
        /// Whether the component instance has been notified that the stream or
        /// future is "done" (i.e. the other end has dropped, or, in the case of
        /// a future, a value has been transmitted).
        done: bool,
    },
    /// The read end of the stream or future.
    Read {
        /// Whether the component instance has been notified that the stream or
        /// future is "done" (i.e. the other end has dropped, or, in the case of
        /// a future, a value has been transmitted).
        done: bool,
    },
    /// A read or write is in progress.
    Busy,
}

/// Return value from [`HandleTable::remove_resource`].
pub enum RemovedResource {
    /// An `own` resource was removed with the specified `rep`
    Own { rep: u32 },
    /// A `borrow` resource was removed originally created within `scope`.
    Borrow { scope: u32 },
}

/// Different kinds of waitables returned by [`HandleTable::waitable_rep`].
pub enum Waitable {
    Subtask { is_host: bool },
    Future,
    Stream,
}

#[derive(Debug)]
enum Slot {
    Free {
        next: u32,
    },

    /// Represents an owned resource handle with the listed representation.
    ///
    /// The `lend_count` tracks how many times this has been lent out as a
    /// `borrow` and if nonzero this can't be removed.
    ResourceOwn {
        resource: TypedResource,
        lend_count: u32,
    },

    /// Represents a borrowed resource handle connected to the `scope`
    /// provided.
    ///
    /// The `rep` is listed and dropping this borrow will decrement the borrow
    /// count of the `scope`.
    ResourceBorrow {
        resource: TypedResource,
        scope: u32,
    },

    /// Represents a host task handle.
    HostTask {
        rep: u32,
    },

    /// Represents a guest task handle.
    GuestTask {
        rep: u32,
    },

    /// Represents a guest thread handle.
    #[cfg(feature = "component-model-async")]
    GuestThread {
        rep: u32,
    },

    /// Represents a stream handle.
    Stream {
        ty: TypeStreamTableIndex,
        rep: u32,
        state: TransmitLocalState,
    },

    /// Represents a future handle.
    Future {
        ty: TypeFutureTableIndex,
        rep: u32,
        state: TransmitLocalState,
    },

    /// Represents a waitable-set handle.
    WaitableSet {
        rep: u32,
    },

    /// Represents an error-context handle.
    ErrorContext {
        rep: u32,
    },
}

pub struct HandleTable {
    next: u32,
    slots: Vec<Slot>,
}

impl Default for HandleTable {
    fn default() -> Self {
        Self {
            next: 0,
            slots: Vec::new(),
        }
    }
}

impl HandleTable {
    /// Returns whether or not this table is empty.
    pub fn is_empty(&self) -> bool {
        self.slots
            .iter()
            .all(|slot| matches!(slot, Slot::Free { .. }))
    }

    fn insert(&mut self, slot: Slot) -> Result<u32> {
        let next = self.next as usize;
        if next == self.slots.len() {
            self.slots.push(Slot::Free {
                next: self.next.checked_add(1).unwrap(),
            });
        }
        let ret = self.next;
        self.next = match mem::replace(&mut self.slots[next], slot) {
            Slot::Free { next } => next,
            _ => unreachable!(),
        };
        // The component model reserves index 0 as never allocatable so add one
        // to the table index to start the numbering at 1 instead. Also note
        // that the component model places an upper-limit per-table on the
        // maximum allowed index.
        let ret = ret + 1;
        if ret >= MAX_HANDLE {
            bail!("cannot allocate another handle: index overflow");
        }

        Ok(ret)
    }

    fn remove(&mut self, idx: u32) -> Result<()> {
        let to_fill = Slot::Free { next: self.next };
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

    fn get_mut(&mut self, idx: u32) -> Result<&mut Slot> {
        let slot = self
            .handle_index_to_table_index(idx)
            .and_then(|i| self.slots.get_mut(i));
        match slot {
            None | Some(Slot::Free { .. }) => bail!("unknown handle index {idx}"),
            Some(slot) => Ok(slot),
        }
    }

    /// Inserts a new `own` resource into this table whose type/rep are
    /// specified by `resource`.
    pub fn resource_own_insert(&mut self, resource: TypedResource) -> Result<u32> {
        self.insert(Slot::ResourceOwn {
            resource,
            lend_count: 0,
        })
    }

    /// Inserts a new `borrow` resource into this table whose type/rep are
    /// specified by `resource`. The `scope` specified is used by
    /// `CallContexts` to manage lending information.
    pub fn resource_borrow_insert(&mut self, resource: TypedResource, scope: u32) -> Result<u32> {
        self.insert(Slot::ResourceBorrow { resource, scope })
    }

    /// Returns the internal "rep" of the resource specified by `idx`.
    ///
    /// Returns an error if `idx` is out-of-bounds or doesn't point to a
    /// resource of the appropriate type.
    pub fn resource_rep(&mut self, idx: TypedResourceIndex) -> Result<u32> {
        match self.get_mut(idx.raw_index())? {
            Slot::ResourceOwn { resource, .. } | Slot::ResourceBorrow { resource, .. } => {
                resource.rep(&idx)
            }
            _ => bail!("index is not a resource"),
        }
    }

    /// Accesses the "rep" of the resource pointed to by `idx` as part of a
    /// lending operation.
    ///
    /// This will increase `lend_count` for owned resources and must be paired
    /// with a `resource_undo_lend` below later on (managed by `CallContexts`).
    ///
    /// Upon success returns the "rep" plus whether the borrow came from an
    /// `own` handle.
    pub fn resource_lend(&mut self, idx: TypedResourceIndex) -> Result<(u32, bool)> {
        match self.get_mut(idx.raw_index())? {
            Slot::ResourceOwn {
                resource,
                lend_count,
            } => {
                let rep = resource.rep(&idx)?;
                *lend_count = lend_count.checked_add(1).unwrap();
                Ok((rep, true))
            }
            Slot::ResourceBorrow { resource, .. } => Ok((resource.rep(&idx)?, false)),
            _ => bail!("index {} is not a resource", idx.raw_index()),
        }
    }

    /// For `own` resources that were borrowed in `resource_lend`, undoes the
    /// lending operation.
    pub fn resource_undo_lend(&mut self, idx: TypedResourceIndex) -> Result<()> {
        match self.get_mut(idx.raw_index())? {
            Slot::ResourceOwn { lend_count, .. } => {
                *lend_count -= 1;
                Ok(())
            }
            _ => bail!("index {} is not an own resource", idx.raw_index()),
        }
    }

    /// Removes the resource specified by `idx` from the table.
    ///
    /// This can fail if `idx` doesn't point to a resource, points to a
    /// borrowed resource, or points to a resource of the wrong type.
    pub fn remove_resource(&mut self, idx: TypedResourceIndex) -> Result<RemovedResource> {
        let ret = match self.get_mut(idx.raw_index())? {
            Slot::ResourceOwn {
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
            Slot::ResourceBorrow { resource, scope } => {
                // Ensure the drop is done with the right type
                resource.rep(&idx)?;
                RemovedResource::Borrow { scope: *scope }
            }
            _ => bail!("index {} is not a resource", idx.raw_index()),
        };
        self.remove(idx.raw_index())?;
        Ok(ret)
    }

    /// Inserts a readable-end stream of type `ty` and with the specified `rep`
    /// into this table.
    ///
    /// Returns the table-local index of the stream.
    pub fn stream_insert_read(&mut self, ty: TypeStreamTableIndex, rep: u32) -> Result<u32> {
        self.insert(Slot::Stream {
            rep,
            ty,
            state: TransmitLocalState::Read { done: false },
        })
    }

    /// Inserts a writable-end stream of type `ty` and with the specified `rep`
    /// into this table.
    ///
    /// Returns the table-local index of the stream.
    pub fn stream_insert_write(&mut self, ty: TypeStreamTableIndex, rep: u32) -> Result<u32> {
        self.insert(Slot::Stream {
            rep,
            ty,
            state: TransmitLocalState::Write { done: false },
        })
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
    ) -> Result<(u32, &mut TransmitLocalState)> {
        match self.get_mut(idx)? {
            Slot::Stream { rep, ty, state } => {
                if *ty != expected_ty {
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
            Slot::Stream { rep, ty, state } => {
                if *ty != expected_ty {
                    bail!("handle is a stream of a different type");
                }
                let is_done = match state {
                    TransmitLocalState::Read { done } => *done,
                    TransmitLocalState::Write { .. } => {
                        bail!("handle is not a readable end of a stream")
                    }
                    TransmitLocalState::Busy => bail!("cannot remove busy stream"),
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
            Slot::Stream { rep, ty, state } => {
                if *ty != expected_ty {
                    bail!("handle is a stream of a different type");
                }
                match state {
                    TransmitLocalState::Write { .. } => {}
                    TransmitLocalState::Read { .. } => {
                        bail!("passed read end to `stream.drop-writable`")
                    }
                    TransmitLocalState::Busy => bail!("cannot drop busy stream"),
                }
                *rep
            }
            _ => bail!("handle is not a stream"),
        };
        self.remove(idx)?;
        Ok(ret)
    }

    /// Inserts a readable-end future of type `ty` and with the specified `rep`
    /// into this table.
    ///
    /// Returns the table-local index of the future.
    pub fn future_insert_read(&mut self, ty: TypeFutureTableIndex, rep: u32) -> Result<u32> {
        self.insert(Slot::Future {
            rep,
            ty,
            state: TransmitLocalState::Read { done: false },
        })
    }

    /// Inserts a writable-end future of type `ty` and with the specified `rep`
    /// into this table.
    ///
    /// Returns the table-local index of the future.
    pub fn future_insert_write(&mut self, ty: TypeFutureTableIndex, rep: u32) -> Result<u32> {
        self.insert(Slot::Future {
            rep,
            ty,
            state: TransmitLocalState::Write { done: false },
        })
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
    ) -> Result<(u32, &mut TransmitLocalState)> {
        match self.get_mut(idx)? {
            Slot::Future { rep, ty, state } => {
                if *ty != expected_ty {
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
            Slot::Future { rep, ty, state } => {
                if *ty != expected_ty {
                    bail!("handle is a future of a different type");
                }
                let is_done = match state {
                    TransmitLocalState::Read { done } => *done,
                    TransmitLocalState::Write { .. } => {
                        bail!("handle is not a readable end of a future")
                    }
                    TransmitLocalState::Busy => bail!("cannot remove busy future"),
                };
                (*rep, is_done)
            }
            _ => bail!("handle is not a future"),
        };
        self.remove(idx)?;
        Ok(ret)
    }

    /// Removes the writable future handle from `idx`, returning its `rep`.
    pub fn future_remove_writable(
        &mut self,
        expected_ty: TypeFutureTableIndex,
        idx: u32,
    ) -> Result<u32> {
        let ret = match self.get_mut(idx)? {
            Slot::Future { rep, ty, state } => {
                if *ty != expected_ty {
                    bail!("handle is a future of a different type");
                }
                match state {
                    TransmitLocalState::Write { .. } => {}
                    TransmitLocalState::Read { .. } => {
                        bail!("passed read end to `future.drop-writable`")
                    }
                    TransmitLocalState::Busy => bail!("cannot drop busy future"),
                }
                *rep
            }
            _ => bail!("handle is not a future"),
        };
        self.remove(idx)?;
        Ok(ret)
    }

    /// Inserts the error-context `rep` into this table, returning the index it
    /// now resides at.
    pub fn error_context_insert(&mut self, rep: u32) -> Result<u32> {
        self.insert(Slot::ErrorContext { rep })
    }

    /// Returns the `rep` of an error-context pointed to by `idx`.
    pub fn error_context_rep(&mut self, idx: u32) -> Result<u32> {
        match self.get_mut(idx)? {
            Slot::ErrorContext { rep } => Ok(*rep),
            _ => bail!("handle is not an error-context"),
        }
    }

    /// Drops the error-context pointed to by `idx`.
    ///
    /// Returns the internal `rep`.
    pub fn error_context_drop(&mut self, idx: u32) -> Result<u32> {
        let rep = match self.get_mut(idx)? {
            Slot::ErrorContext { rep } => *rep,
            _ => bail!("handle is not an error-context"),
        };
        self.remove(idx)?;
        Ok(rep)
    }

    /// Inserts `rep` as a guest subtask into this table.
    pub fn subtask_insert_guest(&mut self, rep: u32) -> Result<u32> {
        self.insert(Slot::GuestTask { rep })
    }

    /// Inserts `rep` as a host subtask into this table.
    pub fn subtask_insert_host(&mut self, rep: u32) -> Result<u32> {
        self.insert(Slot::HostTask { rep })
    }

    /// Returns the `rep` of the subtask at `idx` as well as if it's a host
    /// task or not.
    pub fn subtask_rep(&mut self, idx: u32) -> Result<(u32, bool)> {
        match self.get_mut(idx)? {
            Slot::GuestTask { rep } => Ok((*rep, false)),
            Slot::HostTask { rep } => Ok((*rep, true)),
            _ => bail!("handle is not a subtask"),
        }
    }

    /// Removes the subtask set at `idx`, returning its `rep`.
    pub fn subtask_remove(&mut self, idx: u32) -> Result<(u32, bool)> {
        let ret = match self.get_mut(idx)? {
            Slot::GuestTask { rep } => (*rep, false),
            Slot::HostTask { rep } => (*rep, true),
            _ => bail!("handle is not a subtask"),
        };
        self.remove(idx)?;
        Ok(ret)
    }

    /// Inserts `rep` as a waitable set into this table.
    pub fn waitable_set_insert(&mut self, rep: u32) -> Result<u32> {
        self.insert(Slot::WaitableSet { rep })
    }

    /// Returns the `rep` of an waitable-set pointed to by `idx`.
    pub fn waitable_set_rep(&mut self, idx: u32) -> Result<u32> {
        match self.get_mut(idx)? {
            Slot::WaitableSet { rep, .. } => Ok(*rep),
            _ => bail!("handle is not an waitable-set"),
        }
    }

    /// Removes the waitable set at `idx`, returning its `rep`.
    pub fn waitable_set_remove(&mut self, idx: u32) -> Result<u32> {
        let ret = match self.get_mut(idx)? {
            Slot::WaitableSet { rep } => *rep,
            _ => bail!("handle is not a waitable-set"),
        };
        self.remove(idx)?;
        Ok(ret)
    }

    /// Returns the `rep` for the waitable specified by `idx` along with what
    /// kind of waitable it is.
    pub fn waitable_rep(&mut self, idx: u32) -> Result<(u32, Waitable)> {
        match self.get_mut(idx)? {
            Slot::GuestTask { rep } => Ok((*rep, Waitable::Subtask { is_host: false })),
            Slot::HostTask { rep } => Ok((*rep, Waitable::Subtask { is_host: true })),
            Slot::Future { rep, .. } => Ok((*rep, Waitable::Future)),
            Slot::Stream { rep, .. } => Ok((*rep, Waitable::Stream)),
            _ => bail!("handle is not a waitable"),
        }
    }
}

#[derive(Default)]
#[cfg(feature = "component-model-async")]
pub struct ThreadHandleTable(HandleTable);

#[cfg(feature = "component-model-async")]
impl ThreadHandleTable {
    /// Inserts the guest thread `rep` into this table, returning the index it
    /// now resides at.
    pub fn guest_thread_insert(&mut self, rep: u32) -> Result<u32> {
        self.0.insert(Slot::GuestThread { rep })
    }

    /// Returns the `rep` of a guest thread pointed to by `idx`.
    pub fn guest_thread_rep(&mut self, idx: u32) -> Result<u32> {
        match self.0.get_mut(idx)? {
            Slot::GuestThread { rep } => Ok(*rep),
            _ => bail!("handle is not a guest thread"),
        }
    }

    /// Removes the guest thread pointed to by `idx`.
    ///
    /// Returns the internal `rep`.
    pub fn guest_thread_remove(&mut self, idx: u32) -> Result<u32> {
        let rep = match self.0.get_mut(idx)? {
            Slot::GuestThread { rep } => *rep,
            _ => bail!("handle is not a guest thread"),
        };
        self.0.remove(idx)?;
        Ok(rep)
    }
}
