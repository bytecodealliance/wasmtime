use regalloc::{Reg, RegUsageMapper, Writable};
use smallvec::SmallVec;

// Define our own register-mapping trait so we can do arbitrary register
// renaming that are more free form than what `regalloc` constrains us to with
// its `RegUsageMapper` trait definition.
pub trait RegMapper {
    fn get_use(&self, reg: Reg) -> Option<Reg>;
    fn get_def(&self, reg: Reg) -> Option<Reg>;
    fn get_mod(&self, reg: Reg) -> Option<Reg>;

    fn map_use(&self, r: &mut Reg) {
        if let Some(new) = self.get_use(*r) {
            *r = new;
        }
    }

    fn map_def(&self, r: &mut Writable<Reg>) {
        if let Some(new) = self.get_def(r.to_reg()) {
            *r = Writable::from_reg(new);
        }
    }

    fn map_mod(&self, r: &mut Writable<Reg>) {
        if let Some(new) = self.get_mod(r.to_reg()) {
            *r = Writable::from_reg(new);
        }
    }
}

impl<T> RegMapper for T
where
    T: RegUsageMapper,
{
    fn get_use(&self, reg: Reg) -> Option<Reg> {
        let v = reg.as_virtual_reg()?;
        self.get_use(v).map(|r| r.to_reg())
    }

    fn get_def(&self, reg: Reg) -> Option<Reg> {
        let v = reg.as_virtual_reg()?;
        self.get_def(v).map(|r| r.to_reg())
    }

    fn get_mod(&self, reg: Reg) -> Option<Reg> {
        let v = reg.as_virtual_reg()?;
        self.get_mod(v).map(|r| r.to_reg())
    }
}

#[derive(Default)]
pub struct RegRenamer {
    // Map of `(old, new)` register names. Use a `SmallVec` because we typically
    // only have one or two renamings.
    renames: SmallVec<[(Reg, Reg); 2]>,
}

impl RegRenamer {
    pub fn add_rename(&mut self, old: Reg, new: Reg) {
        self.renames.push((old, new));
    }

    fn get_rename(&self, reg: Reg) -> Option<Reg> {
        self.renames
            .iter()
            .find(|(old, _)| reg == *old)
            .map(|(_, new)| *new)
    }
}

impl RegMapper for RegRenamer {
    fn get_use(&self, reg: Reg) -> Option<Reg> {
        self.get_rename(reg)
    }

    fn get_def(&self, reg: Reg) -> Option<Reg> {
        self.get_rename(reg)
    }

    fn get_mod(&self, reg: Reg) -> Option<Reg> {
        self.get_rename(reg)
    }
}
