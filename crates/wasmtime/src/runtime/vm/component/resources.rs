//! Implementation of the canonical-ABI related intrinsics for resources in the
//! component model.
//!
//! This module contains all the relevant gory details of the
//! component model related to lifting and lowering resources. For example
//! intrinsics like `resource.new` will bottom out in calling this file, and
//! this is where resource tables are actually defined and modified.
//!
//! The main types in this file are:
//!
//! * `ResourceTables` - the "here's everything" context which is required to
//!   perform canonical ABI operations.
//!
//! * `ResourceTable` - an individual instance of a table of resources,
//!   basically "just a slab" though.
//!
//! * `CallContexts` - store-local information about active calls and borrows
//!   and runtime state tracking that to ensure that everything is handled
//!   correctly.
//!
//! Individual operations are exposed through methods on `ResourceTables` for
//! lifting/lowering/etc. This does mean though that some other fiddly bits
//! about ABI details can be found in lifting/lowering throughout Wasmtime,
//! namely in the `Resource<T>` and `ResourceAny` types.

use crate::prelude::*;
use core::mem;
use wasmtime_environ::PrimaryMap;
use wasmtime_environ::component::TypeResourceTableIndex;

/// The maximum handle value is specified in
/// <https://github.com/WebAssembly/component-model/blob/main/design/mvp/CanonicalABI.md>
/// currently and keeps the upper bit free for use in the component.
const MAX_RESOURCE_HANDLE: u32 = 1 << 30;

/// Contextual state necessary to perform resource-related operations.
///
/// This state a bit odd since it has a few optional bits, but the idea is that
/// whenever this is constructed the bits required to perform operations are
/// always `Some`. For example:
///
/// * During lifting and lowering both `tables` and `host_table` are `Some`.
/// * During wasm's own intrinsics only `tables` is `Some`.
/// * During embedder-invoked resource destruction calls only `host_table` is
///   `Some`.
///
/// This is all packaged up into one state though to make it easier to operate
/// on and to centralize handling of the state related to resources due to how
/// critical it is for correctness.
pub struct ResourceTables<'a> {
    /// Runtime state for all resources defined in a component.
    ///
    /// This is required whenever a `TypeResourceTableIndex` is provided as it's
    /// the lookup where that happens. Not present during embedder-originating
    /// operations though such as `ResourceAny::resource_drop` which won't
    /// consult this table as it's only operating over the host table.
    pub tables: Option<&'a mut PrimaryMap<TypeResourceTableIndex, ResourceTable>>,

    /// Runtime state for resources currently owned by the host.
    ///
    /// This is the single table used by the host stored within `Store<T>`. Host
    /// resources will point into this and effectively have the same semantics
    /// as-if they're in-component resources. The major distinction though is
    /// that this is a heterogeneous table instead of only containing a single
    /// type.
    pub host_table: Option<&'a mut ResourceTable>,

    /// Scope information about calls actively in use to track information such
    /// as borrow counts.
    pub calls: &'a mut CallContexts,
}

/// An individual slab of resources used for a single table within a component.
/// Not much fancier than a general slab data structure.
#[derive(Default)]
pub struct ResourceTable {
    /// Next slot to allocate, or `self.slots.len()` if they're all full.
    next: u32,
    /// Runtime state of all slots.
    slots: Vec<Slot>,
}

enum Slot {
    /// This slot is free and points to the next free slot, forming a linked
    /// list of free slots.
    Free { next: u32 },

    /// This slot contains an owned resource with the listed representation.
    ///
    /// The `lend_count` tracks how many times this has been lent out as a
    /// `borrow` and if nonzero this can't be removed.
    Own { rep: u32, lend_count: u32 },

    /// This slot contains a `borrow` resource that's connected to the `scope`
    /// provided. The `rep` is listed and dropping this borrow will decrement
    /// the borrow count of the `scope`.
    Borrow { rep: u32, scope: usize },
}

/// State related to borrows and calls within a component.
///
/// This is created once per `Store` and updated and modified throughout the
/// lifetime of the store. This primarily tracks borrow counts and what slots
/// should be updated when calls go out of scope.
#[derive(Default)]
pub struct CallContexts {
    scopes: Vec<CallContext>,
}

#[derive(Default)]
struct CallContext {
    lenders: Vec<Lender>,
    borrow_count: u32,
}

#[derive(Copy, Clone)]
struct Lender {
    ty: Option<TypeResourceTableIndex>,
    idx: u32,
}

impl ResourceTables<'_> {
    fn table(&mut self, ty: Option<TypeResourceTableIndex>) -> &mut ResourceTable {
        match ty {
            None => self.host_table.as_mut().unwrap(),
            Some(idx) => &mut self.tables.as_mut().unwrap()[idx],
        }
    }

    /// Implementation of the `resource.new` canonical intrinsic.
    ///
    /// Note that this is the same as `resource_lower_own`.
    pub fn resource_new(&mut self, ty: Option<TypeResourceTableIndex>, rep: u32) -> Result<u32> {
        self.table(ty).insert(Slot::Own { rep, lend_count: 0 })
    }

    /// Implementation of the `resource.rep` canonical intrinsic.
    ///
    /// This one's one of the simpler ones: "just get the rep please"
    pub fn resource_rep(&mut self, ty: Option<TypeResourceTableIndex>, idx: u32) -> Result<u32> {
        self.table(ty).rep(idx)
    }

    /// Implementation of the `resource.drop` canonical intrinsic minus the
    /// actual invocation of the destructor.
    ///
    /// This will drop the handle at the `idx` specified, removing it from the
    /// specified table. This operation can fail if:
    ///
    /// * The index is invalid.
    /// * The index points to an `own` resource which has active borrows.
    ///
    /// Otherwise this will return `Some(rep)` if the destructor for `rep` needs
    /// to run. If `None` is returned then that means a `borrow` handle was
    /// removed and no destructor is necessary.
    pub fn resource_drop(
        &mut self,
        ty: Option<TypeResourceTableIndex>,
        idx: u32,
    ) -> Result<Option<u32>> {
        match self.table(ty).remove(idx)? {
            Slot::Own { rep, lend_count: 0 } => Ok(Some(rep)),
            Slot::Own { .. } => bail!("cannot remove owned resource while borrowed"),
            Slot::Borrow { scope, .. } => {
                self.calls.scopes[scope].borrow_count -= 1;
                Ok(None)
            }
            Slot::Free { .. } => unreachable!(),
        }
    }

    /// Inserts a new "own" handle into the specified table.
    ///
    /// This will insert the specified representation into the specified type
    /// table.
    ///
    /// Note that this operation is infallible, and additionally that this is
    /// the same as `resource_new` implementation-wise.
    ///
    /// This is an implementation of the canonical ABI `lower_own` function.
    pub fn resource_lower_own(
        &mut self,
        ty: Option<TypeResourceTableIndex>,
        rep: u32,
    ) -> Result<u32> {
        self.table(ty).insert(Slot::Own { rep, lend_count: 0 })
    }

    /// Attempts to remove an "own" handle from the specified table and its
    /// index.
    ///
    /// This operation will fail if `idx` is invalid, if it's a `borrow` handle,
    /// or if the own handle has currently been "lent" as a borrow.
    ///
    /// This is an implementation of the canonical ABI `lift_own` function.
    pub fn resource_lift_own(
        &mut self,
        ty: Option<TypeResourceTableIndex>,
        idx: u32,
    ) -> Result<u32> {
        match self.table(ty).remove(idx)? {
            Slot::Own { rep, lend_count: 0 } => Ok(rep),
            Slot::Own { .. } => bail!("cannot remove owned resource while borrowed"),
            Slot::Borrow { .. } => bail!("cannot lift own resource from a borrow"),
            Slot::Free { .. } => unreachable!(),
        }
    }

    /// Extracts the underlying resource representation by lifting a "borrow"
    /// from the tables.
    ///
    /// This primarily employs dynamic tracking when a borrow is created from an
    /// "own" handle to ensure that the "own" handle isn't dropped while the
    /// borrow is active and additionally that when the current call scope
    /// returns the lend operation is undone.
    ///
    /// This is an implementation of the canonical ABI `lift_borrow` function.
    pub fn resource_lift_borrow(
        &mut self,
        ty: Option<TypeResourceTableIndex>,
        idx: u32,
    ) -> Result<u32> {
        match self.table(ty).get_mut(idx)? {
            Slot::Own { rep, lend_count } => {
                // The decrement to this count happens in `exit_call`.
                *lend_count = lend_count.checked_add(1).unwrap();
                let rep = *rep;
                let scope = self.calls.scopes.last_mut().unwrap();
                scope.lenders.push(Lender { ty, idx });
                Ok(rep)
            }
            Slot::Borrow { rep, .. } => Ok(*rep),
            Slot::Free { .. } => unreachable!(),
        }
    }

    /// Records a new `borrow` resource with the given representation within the
    /// current call scope.
    ///
    /// This requires that a call scope is active. Additionally the number of
    /// active borrows in the latest scope will be increased and must be
    /// decreased through a future use of `resource_drop` before the current
    /// call scope exits.
    ///
    /// This some of the implementation of the canonical ABI `lower_borrow`
    /// function. The other half of this implementation is located on
    /// `VMComponentContext` which handles the special case of avoiding borrow
    /// tracking entirely.
    pub fn resource_lower_borrow(
        &mut self,
        ty: Option<TypeResourceTableIndex>,
        rep: u32,
    ) -> Result<u32> {
        let scope = self.calls.scopes.len() - 1;
        let borrow_count = &mut self.calls.scopes.last_mut().unwrap().borrow_count;
        *borrow_count = borrow_count.checked_add(1).unwrap();
        self.table(ty).insert(Slot::Borrow { rep, scope })
    }

    /// Enters a new calling context, starting a fresh count of borrows and
    /// such.
    #[inline]
    pub fn enter_call(&mut self) {
        self.calls.scopes.push(CallContext::default());
    }

    /// Exits the previously pushed calling context.
    ///
    /// This requires all information to be available within this
    /// `ResourceTables` and is only called during lowering/lifting operations
    /// at this time.
    #[inline]
    pub fn exit_call(&mut self) -> Result<()> {
        let cx = self.calls.scopes.pop().unwrap();
        if cx.borrow_count > 0 {
            bail!("borrow handles still remain at the end of the call")
        }
        for lender in cx.lenders.iter() {
            // Note the panics here which should never get triggered in theory
            // due to the dynamic tracking of borrows and such employed for
            // resources.
            match self.table(lender.ty).get_mut(lender.idx).unwrap() {
                Slot::Own { lend_count, .. } => {
                    *lend_count -= 1;
                }
                _ => unreachable!(),
            }
        }
        Ok(())
    }
}

impl ResourceTable {
    fn insert(&mut self, new: Slot) -> Result<u32> {
        let next = self.next as usize;
        if next == self.slots.len() {
            self.slots.push(Slot::Free {
                next: self.next.checked_add(1).unwrap(),
            });
        }
        let ret = self.next;
        self.next = match mem::replace(&mut self.slots[next], new) {
            Slot::Free { next } => next,
            _ => unreachable!(),
        };

        // The component model reserves index 0 as never allocatable so add one
        // to the table index to start the numbering at 1 instead. Also note
        // that the component model places an upper-limit per-table on the
        // maximum allowed index.
        let ret = ret + 1;
        if ret >= MAX_RESOURCE_HANDLE {
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

    fn rep(&self, idx: u32) -> Result<u32> {
        let slot = self
            .handle_index_to_table_index(idx)
            .and_then(|i| self.slots.get(i));
        match slot {
            None | Some(Slot::Free { .. }) => bail!("unknown handle index {idx}"),
            Some(Slot::Own { rep, .. } | Slot::Borrow { rep, .. }) => Ok(*rep),
        }
    }

    fn get_mut(&mut self, idx: u32) -> Result<&mut Slot> {
        let slot = self
            .handle_index_to_table_index(idx)
            .and_then(|i| self.slots.get_mut(i));
        match slot {
            None | Some(Slot::Free { .. }) => bail!("unknown handle index {idx}"),
            Some(other) => Ok(other),
        }
    }

    fn remove(&mut self, idx: u32) -> Result<Slot> {
        let to_fill = Slot::Free { next: self.next };
        let ret = mem::replace(self.get_mut(idx)?, to_fill);
        self.next = idx - 1;
        Ok(ret)
    }
}
