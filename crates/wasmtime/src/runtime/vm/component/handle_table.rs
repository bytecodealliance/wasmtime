use super::{TypedResource, TypedResourceIndex};
use alloc::vec::Vec;
use anyhow::{Result, bail};
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

/// Represents the kind of handle stored for a given slot.
#[derive(Debug)]
pub enum HandleKind {
    /// Represents a host task handle.
    HostTask,
    /// Represents a guest task handle.
    GuestTask,
    /// Represents a stream handle.
    Stream(TypeStreamTableIndex, TransmitLocalState),
    /// Represents a future handle.
    Future(TypeFutureTableIndex, TransmitLocalState),
    /// Represents a waitable-set handle.
    Set,
    /// Represents an error-context handle.
    ErrorContext {
        /// Number of references held by the (sub-)component.
        ///
        /// This does not include the number of references which might be held
        /// by other (sub-)components.
        local_ref_count: usize,
    },
}

/// Return value from [`HandleTable::remove_resource`].
pub enum RemovedResource {
    /// An `own` resource was removed with the specified `rep`
    Own { rep: u32 },
    /// A `borrow` resource was removed originally created within `scope`.
    Borrow { scope: usize },
}

enum Slot {
    Free {
        next: u32,
    },
    Handle {
        rep: u32,
        kind: HandleKind,
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
        scope: usize,
    },
}

pub struct HandleTable {
    next: u32,
    slots: Vec<Slot>,
    // TODO: This is a sparse table (where zero means "no entry"); it might make
    // more sense to use a `HashMap` here, but we'd need one that's
    // no_std-compatible.  A `BTreeMap` might also be appropriate if we restrict
    // ourselves to `alloc::collections`.
    reps_to_indexes: Vec<u32>,
}

impl Default for HandleTable {
    fn default() -> Self {
        Self {
            next: 0,
            slots: Vec::new(),
            reps_to_indexes: Vec::new(),
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
    pub fn insert_own_resource(&mut self, resource: TypedResource) -> Result<u32> {
        self.insert(Slot::ResourceOwn {
            resource,
            lend_count: 0,
        })
    }

    /// Inserts a new `borrow` resource into this table whose type/rep are
    /// specified by `resource`. The `scope` specified is used by
    /// `CallContexts` to manage lending information.
    pub fn insert_borrow_resource(&mut self, resource: TypedResource, scope: usize) -> Result<u32> {
        self.insert(Slot::ResourceBorrow { resource, scope })
    }

    /// Returns the internal "rep" of the resource specified by `idx`.
    ///
    /// Returns an error if `idx` is out-of-bounds or doesn't point to a
    /// resource of the appropriate type.
    pub fn resource_rep(&self, idx: TypedResourceIndex) -> Result<u32> {
        let slot = self
            .handle_index_to_table_index(idx.raw_index())
            .and_then(|i| self.slots.get(i));
        match slot {
            None | Some(Slot::Free { .. }) => bail!("unknown handle index {}", idx.raw_index()),
            Some(Slot::Handle { .. }) => {
                bail!("index {} is a handle, not a resource", idx.raw_index())
            }
            Some(Slot::ResourceOwn { resource, .. } | Slot::ResourceBorrow { resource, .. }) => {
                resource.rep(&idx)
            }
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
        let to_fill = Slot::Free { next: self.next };
        let slot = self.get_mut(idx.raw_index())?;
        let ret = match slot {
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
        *slot = to_fill;
        Ok(ret)
    }

    pub fn insert_handle(&mut self, rep: u32, kind: HandleKind) -> Result<u32> {
        if matches!(self
            .reps_to_indexes
            .get(usize::try_from(rep).unwrap()), Some(idx) if *idx != 0)
        {
            bail!("rep {rep} already exists in this table");
        }

        let ret = self.insert(Slot::Handle { rep, kind })?;

        let rep = usize::try_from(rep).unwrap();
        if self.reps_to_indexes.len() <= rep {
            self.reps_to_indexes.resize(rep.checked_add(1).unwrap(), 0);
        }

        self.reps_to_indexes[rep] = ret;

        Ok(ret)
    }

    pub fn get_mut_handle_by_index(&mut self, idx: u32) -> Result<(u32, &mut HandleKind)> {
        let slot = self
            .handle_index_to_table_index(idx)
            .and_then(|i| self.slots.get_mut(i));
        match slot {
            None | Some(Slot::Free { .. }) => bail!("unknown handle index {idx}"),
            Some(Slot::ResourceOwn { .. } | Slot::ResourceBorrow { .. }) => {
                bail!("index {idx} is a resource, not a handle")
            }
            Some(Slot::Handle { rep, kind }) => Ok((*rep, kind)),
        }
    }

    pub fn get_mut_handle_by_rep(&mut self, rep: u32) -> Option<(u32, &mut HandleKind)> {
        let index = *self.reps_to_indexes.get(usize::try_from(rep).unwrap())?;
        if index > 0 {
            let (_, kind) = self.get_mut_handle_by_index(index).unwrap();
            Some((index, kind))
        } else {
            None
        }
    }

    pub fn remove_handle_by_index(&mut self, idx: u32) -> Result<(u32, HandleKind)> {
        _ = self.get_mut_handle_by_index(idx)?;

        let to_fill = Slot::Free { next: self.next };
        let Slot::Handle { rep, kind } = mem::replace(self.get_mut(idx)?, to_fill) else {
            unreachable!()
        };
        self.next = idx - 1;
        {
            let rep = usize::try_from(rep).unwrap();
            assert_eq!(idx, self.reps_to_indexes[rep]);
            self.reps_to_indexes[rep] = 0;
        }
        Ok((rep, kind))
    }
}
