//! Cost functions for egraph representation.

use crate::ir::Opcode;

/// A cost of computing some value in the program.
///
/// Costs are measured in an arbitrary union that we represent in a
/// `u32`. The ordering is meant to be meaningful, but the value of a
/// single unit is arbitrary (and "not to scale"). We use a collection
/// of heuristics to try to make this approximation at least usable.
///
/// We start by defining costs for each opcode (see `pure_op_cost`
/// below). The cost of computing some value, initially, is the cost
/// of its opcode, plus the cost of computing its inputs.
///
/// We then adjust the cost according to loop nests: for each
/// loop-nest level, we multiply by 1024. Because we only have 32
/// bits, we limit this scaling to a loop-level of two (i.e., multiply
/// by 2^20 ~= 1M).
///
/// Arithmetic on costs is always saturating: we don't want to wrap
/// around and return to a tiny cost when adding the costs of two very
/// expensive operations. It is better to approximate and lose some
/// precision than to lose the ordering by wrapping.
///
/// Finally, we reserve the highest value, `u32::MAX`, as a sentinel
/// that means "infinite". This is separate from the finite costs and
/// not reachable by doing arithmetic on them (even when overflowing)
/// -- we saturate just *below* infinity. (This is done by the
/// `finite()` method.) An infinite cost is used to represent a value
/// that cannot be computed, or otherwise serve as a sentinel when
/// performing search for the lowest-cost representation of a value.
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
        | Opcode::Bor
        | Opcode::Bxor
        | Opcode::Bnot
        | Opcode::Ishl
        | Opcode::Ushr
        | Opcode::Sshr => Cost(2),
        // Everything else (pure.)
        _ => Cost(3),
    }
}
