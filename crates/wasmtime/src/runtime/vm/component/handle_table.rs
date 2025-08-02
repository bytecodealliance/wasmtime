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

pub enum ResourceKind {
    /// Represents an owned resource handle with the listed representation.
    ///
    /// The `lend_count` tracks how many times this has been lent out as a
    /// `borrow` and if nonzero this can't be removed.
    Own {
        resource: TypedResource,
        lend_count: u32,
    },
    /// Represents a borrowed resource handle connected to the `scope`
    /// provided.
    ///
    /// The `rep` is listed and dropping this borrow will decrement the borrow
    /// count of the `scope`.
    Borrow {
        resource: TypedResource,
        scope: usize,
    },
}

enum Slot {
    Free { next: u32 },
    Handle { rep: u32, kind: HandleKind },
    Resource { kind: ResourceKind },
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

    pub fn insert_resource(&mut self, kind: ResourceKind) -> Result<u32> {
        self.insert(Slot::Resource { kind })
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

    fn handle_index_to_table_index(&self, idx: u32) -> Option<usize> {
        // NB: `idx` is decremented by one to account for the `+1` above during
        // allocation.
        let idx = idx.checked_sub(1)?;
        usize::try_from(idx).ok()
    }

    pub(super) fn resource_rep(&self, idx: TypedResourceIndex) -> Result<u32> {
        let slot = self
            .handle_index_to_table_index(idx.raw_index())
            .and_then(|i| self.slots.get(i));
        match slot {
            None | Some(Slot::Free { .. }) => bail!("unknown handle index {}", idx.raw_index()),
            Some(Slot::Handle { .. }) => {
                bail!("index {} is a handle, not a resource", idx.raw_index())
            }
            Some(Slot::Resource {
                kind: ResourceKind::Own { resource, .. } | ResourceKind::Borrow { resource, .. },
            }) => resource.rep(&idx),
        }
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

    pub(super) fn get_mut_resource(
        &mut self,
        idx: TypedResourceIndex,
    ) -> Result<&mut ResourceKind> {
        let slot = self
            .handle_index_to_table_index(idx.raw_index())
            .and_then(|i| self.slots.get_mut(i));
        match slot {
            None | Some(Slot::Free { .. }) => bail!("unknown handle index {}", idx.raw_index()),
            Some(Slot::Handle { .. }) => {
                bail!("index {} is a handle, not a resource", idx.raw_index())
            }
            Some(Slot::Resource { kind }) => Ok(kind),
        }
    }

    pub fn get_mut_handle_by_index(&mut self, idx: u32) -> Result<(u32, &mut HandleKind)> {
        let slot = self
            .handle_index_to_table_index(idx)
            .and_then(|i| self.slots.get_mut(i));
        match slot {
            None | Some(Slot::Free { .. }) => bail!("unknown handle index {idx}"),
            Some(Slot::Resource { .. }) => bail!("index {idx} is a resource, not a handle"),
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

    pub(super) fn remove_resource(&mut self, idx: TypedResourceIndex) -> Result<ResourceKind> {
        _ = self.resource_rep(idx)?;

        let to_fill = Slot::Free { next: self.next };
        let Slot::Resource { kind } = mem::replace(self.get_mut(idx.raw_index())?, to_fill) else {
            unreachable!()
        };
        self.next = idx.raw_index() - 1;
        Ok(kind)
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
