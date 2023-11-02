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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Cost {
    opcode_cost: u32,
    depth: u32,
}

impl Ord for Cost {
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Break `opcode_cost` ties with `depth`. This means that, when the
        // opcode cost is the same, we prefer shallow and wide expressions to
        // narrow and deep. For example, `(a + b) + (c + d)` is preferred to
        // `((a + b) + c) + d`. This is beneficial because it exposes more
        // instruction-level parallelism and shortens live ranges.
        self.opcode_cost
            .cmp(&other.opcode_cost)
            .then_with(|| self.depth.cmp(&other.depth))
    }
}

impl PartialOrd for Cost {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Cost {
    pub(crate) fn infinity() -> Cost {
        // 2^32 - 1 is, uh, pretty close to infinite... (we use `Cost`
        // only for heuristics and always saturate so this suffices!)
        Cost {
            opcode_cost: u32::MAX,
            depth: u32::MAX,
        }
    }

    pub(crate) fn zero() -> Cost {
        Cost {
            opcode_cost: 0,
            depth: 0,
        }
    }

    pub(crate) fn new(opcode_cost: u32) -> Cost {
        let cost = Cost {
            opcode_cost,
            depth: 0,
        };
        cost.finite()
    }

    /// Clamp this cost at a "finite" value. Can be used in
    /// conjunction with saturating ops to avoid saturating into
    /// `infinity()`.
    fn finite(self) -> Cost {
        Cost {
            opcode_cost: std::cmp::min(u32::MAX - 1, self.opcode_cost),
            depth: std::cmp::min(u32::MAX - 1, self.depth),
        }
    }

    /// Compute the cost of the operation and its given operands.
    ///
    /// Caller is responsible for checking that the opcode came from an instruction
    /// that satisfies `inst_predicates::is_pure_for_egraph()`.
    pub(crate) fn of_pure_op(op: Opcode, operand_costs: impl IntoIterator<Item = Self>) -> Self {
        let mut c: Self = pure_op_cost(op) + operand_costs.into_iter().sum();
        c.depth = c.depth.saturating_add(1);
        c
    }
}

impl std::iter::Sum<Cost> for Cost {
    fn sum<I: Iterator<Item = Cost>>(iter: I) -> Self {
        let mut c = Self::zero();
        for x in iter {
            c = c + x;
        }
        c
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
        let cost = Cost {
            opcode_cost: self.opcode_cost.saturating_add(other.opcode_cost),
            depth: std::cmp::max(self.depth, other.depth),
        };
        cost.finite()
    }
}

/// Return the cost of a *pure* opcode.
///
/// Caller is responsible for checking that the opcode came from an instruction
/// that satisfies `inst_predicates::is_pure_for_egraph()`.
fn pure_op_cost(op: Opcode) -> Cost {
    match op {
        // Constants.
        Opcode::Iconst | Opcode::F32const | Opcode::F64const => Cost::new(1),

        // Extends/reduces.
        Opcode::Uextend | Opcode::Sextend | Opcode::Ireduce | Opcode::Iconcat | Opcode::Isplit => {
            Cost::new(2)
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
        | Opcode::Sshr => Cost::new(3),

        // Everything else (pure.)
        _ => Cost::new(4),
    }
}
