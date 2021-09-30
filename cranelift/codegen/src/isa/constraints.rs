//! Register constraints for instruction operands.
//!
//! An encoding recipe specifies how an instruction is encoded as binary machine code, but it only
//! works if the operands and results satisfy certain constraints. Constraints on immediate
//! operands are checked by instruction predicates when the recipe is chosen.
//!
//! It is the register allocator's job to make sure that the register constraints on value operands
//! are satisfied.

use crate::binemit::CodeOffset;

/// Constraints on the range of a branch instruction.
///
/// A branch instruction usually encodes its destination as a signed n-bit offset from an origin.
/// The origin depends on the ISA and the specific instruction:
///
/// - RISC-V and ARM Aarch64 use the address of the branch instruction, `origin = 0`.
/// - x86 uses the address of the instruction following the branch, `origin = 2` for a 2-byte
///   branch instruction.
/// - ARM's A32 encoding uses the address of the branch instruction + 8 bytes, `origin = 8`.
#[derive(Clone, Copy, Debug)]
pub struct BranchRange {
    /// Offset in bytes from the address of the branch instruction to the origin used for computing
    /// the branch displacement. This is the destination of a branch that encodes a 0 displacement.
    pub origin: u8,

    /// Number of bits in the signed byte displacement encoded in the instruction. This does not
    /// account for branches that can only target aligned addresses.
    pub bits: u8,
}

impl BranchRange {
    /// Determine if this branch range can represent the range from `branch` to `dest`, where
    /// `branch` is the code offset of the branch instruction itself and `dest` is the code offset
    /// of the destination block header.
    ///
    /// This method does not detect if the range is larger than 2 GB.
    pub fn contains(self, branch: CodeOffset, dest: CodeOffset) -> bool {
        let d = dest.wrapping_sub(branch + CodeOffset::from(self.origin)) as i32;
        let s = 32 - self.bits;
        d == d << s >> s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn branch_range() {
        // ARM T1 branch.
        let t1 = BranchRange { origin: 4, bits: 9 };
        assert!(t1.contains(0, 0));
        assert!(t1.contains(0, 2));
        assert!(t1.contains(2, 0));
        assert!(t1.contains(1000, 1000));

        // Forward limit.
        assert!(t1.contains(1000, 1258));
        assert!(!t1.contains(1000, 1260));

        // Backward limit
        assert!(t1.contains(1000, 748));
        assert!(!t1.contains(1000, 746));
    }
}
