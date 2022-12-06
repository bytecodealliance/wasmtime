//! Cost functions for egraph representation.

use crate::ir::Opcode;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Cost(u32);
impl Cost {
    pub(crate) fn at_level(&self, loop_level: usize) -> Cost {
        let loop_level = std::cmp::min(2, loop_level);
        let multiplier = 1u32 << ((10 * loop_level) as u32);
        Cost(self.0.saturating_mul(multiplier)).finite()
    }

    pub(crate) fn infinity() -> Cost {
        // 2^32 - 1 is, uh, pretty close to infinite... (we use `Cost`
        // only for heuristics and always saturate so this suffices!)
        Cost(u32::MAX)
    }

    pub(crate) fn zero() -> Cost {
        Cost(0)
    }

    /// Clamp this cost at a "finite" value. Can be used in
    /// conjunction with saturating ops to avoid saturating into
    /// `infinity()`.
    fn finite(self) -> Cost {
        Cost(std::cmp::min(u32::MAX - 1, self.0))
    }
}

impl std::default::Default for Cost {
    fn default() -> Cost {
        Cost::zero()
    }
}

impl std::ops::Add<Cost> for Cost {
    type Output = Cost;
    fn add(self, other: Cost) -> Cost {
        Cost(self.0.saturating_add(other.0)).finite()
    }
}

/// Return the cost of a *pure* opcode. Caller is responsible for
/// checking that the opcode came from an instruction that satisfies
/// `inst_predicates::is_pure_for_egraph()`.
pub(crate) fn pure_op_cost(op: Opcode) -> Cost {
    match op {
        // Constants.
        Opcode::Iconst | Opcode::F32const | Opcode::F64const => Cost(0),
        // Extends/reduces.
        Opcode::Uextend | Opcode::Sextend | Opcode::Ireduce | Opcode::Iconcat | Opcode::Isplit => {
            Cost(1)
        }
        // "Simple" arithmetic.
        Opcode::Iadd
        | Opcode::Isub
        | Opcode::Band
        | Opcode::BandNot
        | Opcode::Bor
        | Opcode::BorNot
        | Opcode::Bxor
        | Opcode::BxorNot
        | Opcode::Bnot => Cost(2),
        // Everything else (pure.)
        _ => Cost(3),
    }
}
