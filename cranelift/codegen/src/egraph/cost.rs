//! Cost functions for egraph representation.

use crate::ir::{Inst, Opcode};
use cranelift_entity::EntityRef;

/// Approximate cost of an expression as a DAG of instructions.
///
/// In addition to the saturating total cost, this tracks an approximate
/// footprint of the instructions that contribute to the expression. When an
/// operand's whole footprint is already covered, we don't charge its total
/// again. This catches common shared-DAG shapes like `iadd x, x` without
/// allocating precise instruction sets in the egraph extraction hot path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ExprCost {
    total: Cost,
    inst_buckets: u32,
}

impl ExprCost {
    pub(crate) fn zero() -> Self {
        Self {
            total: Cost::zero(),
            inst_buckets: 0,
        }
    }

    pub(crate) fn infinity() -> Self {
        Self {
            total: Cost::infinity(),
            inst_buckets: 0,
        }
    }

    pub(crate) fn for_inst(inst: Inst, op: Opcode) -> Self {
        Self {
            total: Cost::of_opcode(op),
            inst_buckets: Self::inst_bucket(inst),
        }
    }

    /// Compute the cost of the operation and its given operands.
    ///
    /// Caller is responsible for checking that the opcode came from an instruction
    /// that satisfies `inst_predicates::is_pure_for_egraph()`.
    pub(crate) fn of_pure_op(
        inst: Inst,
        op: Opcode,
        operand_costs: impl IntoIterator<Item = Self>,
    ) -> Self {
        let mut cost = Self::for_inst(inst, op);
        for operand_cost in operand_costs {
            cost.add_operand(operand_cost);
        }
        cost
    }
}

impl ExprCost {
    fn inst_bucket(inst: Inst) -> u32 {
        let index = u32::try_from(inst.index()).unwrap();
        let hash = index.wrapping_mul(0x9e37_79b9);
        1u32 << (hash >> 27)
    }

    fn add_operand(&mut self, other: Self) {
        let new_buckets = other.inst_buckets & !self.inst_buckets;
        if new_buckets != 0 {
            self.total = self.total + other.total;
        }
        self.inst_buckets |= other.inst_buckets;
    }
}

impl PartialOrd for ExprCost {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ExprCost {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.total.cmp(&other.total)
    }
}

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
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Cost(u32);

impl core::fmt::Debug for Cost {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if *self == Cost::infinity() {
            write!(f, "Cost::Infinite")
        } else {
            f.debug_tuple("Cost::Finite").field(&self.cost()).finish()
        }
    }
}

impl Cost {
    pub(crate) fn infinity() -> Cost {
        // 2^32 - 1 is, uh, pretty close to infinite... (we use `Cost`
        // only for heuristics and always saturate so this suffices!)
        Cost(u32::MAX)
    }

    pub(crate) fn zero() -> Cost {
        Cost(0)
    }

    /// Construct a new `Cost`.
    fn new(cost: u32) -> Cost {
        Cost(cost)
    }

    fn cost(&self) -> u32 {
        self.0
    }

    /// Return the cost of an opcode.
    fn of_opcode(op: Opcode) -> Cost {
        match op {
            // Constants.
            Opcode::Iconst | Opcode::F32const | Opcode::F64const => Cost::new(1),

            // Extends/reduces.
            Opcode::Uextend
            | Opcode::Sextend
            | Opcode::Ireduce
            | Opcode::Iconcat
            | Opcode::Isplit => Cost::new(1),

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

            // "Expensive" arithmetic.
            Opcode::Imul => Cost::new(10),

            // Everything else.
            _ => {
                // By default, be slightly more expensive than "simple"
                // arithmetic.
                let mut c = Cost::new(4);

                // And then get more expensive as the opcode does more side
                // effects.
                if op.can_trap() || op.other_side_effects() {
                    c = c + Cost::new(10);
                }
                if op.can_load() {
                    c = c + Cost::new(20);
                }
                if op.can_store() {
                    c = c + Cost::new(50);
                }

                c
            }
        }
    }

    /// Compute the cost of the operation and its given operands.
    ///
    /// Caller is responsible for checking that the opcode came from an instruction
    /// that satisfies `inst_predicates::is_pure_for_egraph()`.
    pub(crate) fn of_pure_op(op: Opcode, operand_costs: impl IntoIterator<Item = Self>) -> Self {
        let c = Self::of_opcode(op) + operand_costs.into_iter().sum();
        Cost::new(c.cost())
    }

    /// Compute the cost of an operation in the side-effectful skeleton.
    pub(crate) fn of_skeleton_op(op: Opcode, arity: usize) -> Self {
        Cost::of_opcode(op) + Cost::new(u32::try_from(arity).unwrap())
    }
}

impl core::iter::Sum<Cost> for Cost {
    fn sum<I: Iterator<Item = Cost>>(iter: I) -> Self {
        iter.fold(Self::zero(), |a, b| a + b)
    }
}

impl core::default::Default for Cost {
    fn default() -> Cost {
        Cost::zero()
    }
}

impl core::ops::Add<Cost> for Cost {
    type Output = Cost;

    fn add(self, other: Cost) -> Cost {
        Cost::new(self.cost().saturating_add(other.cost()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_cost() {
        let a = Cost::new(5);
        let b = Cost::new(37);
        assert_eq!(a + b, Cost::new(42));
        assert_eq!(b + a, Cost::new(42));
    }

    #[test]
    fn add_infinity() {
        let a = Cost::new(5);
        let b = Cost::infinity();
        assert_eq!(a + b, Cost::infinity());
        assert_eq!(b + a, Cost::infinity());
    }

    #[test]
    fn op_cost_saturates_to_infinity() {
        let a = Cost::new(u32::MAX - 10);
        let b = Cost::new(11);
        assert_eq!(a + b, Cost::infinity());
        assert_eq!(b + a, Cost::infinity());
    }

    #[test]
    fn expr_cost_skips_fully_covered_operand() {
        let x = ExprCost::for_inst(Inst::new(0), Opcode::Iconst);
        let add = ExprCost::of_pure_op(Inst::new(1), Opcode::Iadd, [x, x]);

        assert_eq!(add.total, Cost::new(4));
    }

    #[test]
    fn expr_cost_grows_linearly_for_repeated_self_adds() {
        let mut cost = ExprCost::for_inst(Inst::new(0), Opcode::Iconst);
        for index in 1..4 {
            cost = ExprCost::of_pure_op(Inst::new(index), Opcode::Iadd, [cost, cost]);
        }

        assert_eq!(cost.total, Cost::new(10));
    }
}
