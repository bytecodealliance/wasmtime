use anyhow::{bail, Result};
use std::mem;
use wasmtime_environ::component::TypeResourceTableIndex;
use wasmtime_environ::PrimaryMap;

pub struct ResourceTables {
    tables: PrimaryMap<TypeResourceTableIndex, ResourceTable>,
    calls: Vec<CallContext>,
}

#[derive(Default)]
struct ResourceTable {
    next: u32,
    slots: Vec<Slot>,
}

enum Slot {
    Free { next: u32 },
    Own { rep: u32, lend_count: u32 },
    Borrow { rep: u32, scope: usize },
}

#[derive(Default)]
struct CallContext {
    lenders: Vec<Lender>,
    borrow_count: u32,
}

#[derive(Copy, Clone)]
pub struct Lender {
    ty: TypeResourceTableIndex,
    idx: u32,
}

impl ResourceTables {
    pub fn new(amt: usize) -> ResourceTables {
        let mut tables = PrimaryMap::with_capacity(amt);
        for _ in 0..amt {
            tables.push(ResourceTable::default());
        }
        ResourceTables {
            tables,
            calls: Vec::new(),
        }
    }

    /// Implementation of the `resource.new` canonical intrinsic.
    ///
    /// Note that this is the same as `resource_lower_own`.
    pub fn resource_new(&mut self, ty: TypeResourceTableIndex, rep: u32) -> u32 {
        self.tables[ty].insert(Slot::Own { rep, lend_count: 0 })
    }

    /// Implementation of the `resource.rep` canonical intrinsic.
    ///
    /// This one's one of the simpler ones: "just get the rep please"
    pub fn resource_rep(&mut self, ty: TypeResourceTableIndex, idx: u32) -> Result<u32> {
        match self.tables[ty].get_mut(idx)? {
            Slot::Own { rep, .. } | Slot::Borrow { rep, .. } => Ok(*rep),
            Slot::Free { .. } => unreachable!(),
        }
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
    pub fn resource_drop(&mut self, ty: TypeResourceTableIndex, idx: u32) -> Result<Option<u32>> {
        match self.tables[ty].remove(idx)? {
            Slot::Own { rep, lend_count: 0 } => Ok(Some(rep)),
            Slot::Own { .. } => bail!("cannot remove owned resource while borrowed"),
            Slot::Borrow { scope, .. } => {
                self.calls[scope].borrow_count -= 1;
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
    pub fn resource_lower_own(&mut self, ty: TypeResourceTableIndex, rep: u32) -> u32 {
        self.tables[ty].insert(Slot::Own { rep, lend_count: 0 })
    }

    /// Attempts to remove an "own" handle from the specified table and its
    /// index.
    ///
    /// This operation will fail if `idx` is invalid, if it's a `borrow` handle,
    /// or if the own handle has currently been "lent" as a borrow.
    ///
    /// This is an implementation of the canonical ABI `lift_own` function.
    pub fn resource_lift_own(&mut self, ty: TypeResourceTableIndex, idx: u32) -> Result<u32> {
        match self.tables[ty].remove(idx)? {
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
    pub fn resource_lift_borrow(&mut self, ty: TypeResourceTableIndex, idx: u32) -> Result<u32> {
        match self.tables[ty].get_mut(idx)? {
            Slot::Own { rep, lend_count } => {
                // The decrement to this count happens in `exit_call`.
                *lend_count = lend_count.checked_add(1).unwrap();
                self.calls
                    .last_mut()
                    .unwrap()
                    .lenders
                    .push(Lender { ty, idx });
                Ok(*rep)
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
    pub fn resource_lower_borrow(&mut self, ty: TypeResourceTableIndex, rep: u32) -> u32 {
        let scope = self.calls.len() - 1;
        let borrow_count = &mut self.calls.last_mut().unwrap().borrow_count;
        *borrow_count = borrow_count.checked_add(1).unwrap();
        self.tables[ty].insert(Slot::Borrow { rep, scope })
    }

    pub fn enter_call(&mut self) {
        self.calls.push(CallContext::default());
    }

    pub fn exit_call(&mut self) -> Result<()> {
        let cx = self.calls.pop().unwrap();
        if cx.borrow_count > 0 {
            bail!("borrow handles still remain at the end of the call")
        }
        for lender in cx.lenders.iter() {
            // Note the panics here which should never get triggered in theory
            // due to the dynamic tracking of borrows and such employed for
            // resources.
            match self.tables[lender.ty].get_mut(lender.idx).unwrap() {
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
    fn next(&self) -> usize {
        self.next as usize
    }

    fn insert(&mut self, new: Slot) -> u32 {
        let next = self.next();
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
        u32::try_from(ret).unwrap()
    }

    fn get_mut(&mut self, idx: u32) -> Result<&mut Slot> {
        match usize::try_from(idx)
            .ok()
            .and_then(|i| self.slots.get_mut(i))
        {
            None | Some(Slot::Free { .. }) => bail!("unknown handle index {idx}"),
            Some(other) => Ok(other),
        }
    }

    fn remove(&mut self, idx: u32) -> Result<Slot> {
        match usize::try_from(idx).ok().and_then(|i| self.slots.get(i)) {
            Some(Slot::Own { .. }) | Some(Slot::Borrow { .. }) => {}
            _ => bail!("unknown handle index {idx}"),
        };
        let ret = mem::replace(
            &mut self.slots[idx as usize],
            Slot::Free { next: self.next },
        );
        self.next = idx;
        Ok(ret)
    }
}
