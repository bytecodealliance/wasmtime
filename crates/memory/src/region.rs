#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Region {
    pub start: u32,
    pub len: u32,
}

impl Region {
    pub fn overlaps(&self, rhs: Region) -> bool {
        let self_start = self.start as u64;
        let self_end = ((self_start + self.len as u64) as i64 - 1) as u64;

        let rhs_start = rhs.start as u64;
        let rhs_end = ((rhs_start + rhs.len as u64) as i64 - 1) as u64;

        // start of rhs inside self:
        if rhs_start >= self_start && rhs_start < self_end {
            return true;
        }

        // end of rhs inside self:
        if rhs_end >= self_start && rhs_end < self_end {
            return true;
        }

        // start of self inside rhs:
        if self_start >= rhs_start && self_start < rhs_end {
            return true;
        }

        // end of self inside rhs: XXX is this redundant? i suspect it is but im too tired
        if self_end >= rhs_start && self_end < rhs_end {
            return true;
        }

        false
    }
}
