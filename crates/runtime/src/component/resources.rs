use anyhow::{bail, Result};
use std::mem;
use wasmtime_environ::component::TypeResourceTableIndex;
use wasmtime_environ::PrimaryMap;

pub struct ResourceTables {
    tables: PrimaryMap<TypeResourceTableIndex, ResourceTable>,
}

#[derive(Default)]
struct ResourceTable {
    next: usize,
    slots: Vec<Slot<u32>>,
}

enum Slot<T> {
    Free { next: usize },
    Taken(T),
}

impl ResourceTables {
    pub fn new(amt: usize) -> ResourceTables {
        let mut tables = PrimaryMap::with_capacity(amt);
        for _ in 0..amt {
            tables.push(ResourceTable::default());
        }
        ResourceTables { tables }
    }

    pub fn resource_new(&mut self, ty: TypeResourceTableIndex, rep: u32) -> u32 {
        self.tables[ty].insert(rep)
    }

    pub fn resource_rep(&mut self, ty: TypeResourceTableIndex, idx: u32) -> Result<u32> {
        self.tables[ty].get(idx)
    }

    pub fn resource_drop(&mut self, ty: TypeResourceTableIndex, idx: u32) -> Result<()> {
        let rep = self.tables[ty].remove(idx)?;
        Ok(())
        // TODO: how to run the dtor for `rep`?
    }

    pub fn resource_lower_own(&mut self, ty: TypeResourceTableIndex, rep: u32) -> u32 {
        // TODO: this impl should probably not be literally the same as `resource_new`
        self.tables[ty].insert(rep)
    }

    pub fn resource_lift_own(&mut self, ty: TypeResourceTableIndex, idx: u32) -> Result<u32> {
        self.tables[ty].remove(idx)
    }
}

impl ResourceTable {
    pub fn insert(&mut self, rep: u32) -> u32 {
        if self.next == self.slots.len() {
            self.slots.push(Slot::Free {
                next: self.next + 1,
            });
        }
        let ret = self.next;
        self.next = match mem::replace(&mut self.slots[self.next], Slot::Taken(rep)) {
            Slot::Free { next } => next,
            Slot::Taken(_) => unreachable!(),
        };
        u32::try_from(ret).unwrap()
    }

    pub fn get(&mut self, idx: u32) -> Result<u32> {
        match self.slots.get(idx as usize) {
            Some(Slot::Taken(rep)) => Ok(*rep),
            _ => bail!("unknown handle index {idx}"),
        }
    }

    pub fn remove(&mut self, idx: u32) -> Result<u32> {
        let rep = match self.slots.get(idx as usize) {
            Some(Slot::Taken(rep)) => *rep,
            _ => bail!("unknown handle index {idx}"),
        };
        // TODO: dtor called here
        self.slots[idx as usize] = Slot::Free { next: self.next };
        self.next = idx as usize;
        Ok(rep)
    }
}
