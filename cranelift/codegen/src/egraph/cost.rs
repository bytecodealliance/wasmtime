//! Cost functions for egraph representation.

use crate::ir::{DataFlowGraph, Inst, Opcode};
use cranelift_entity::ImmutableEntitySet;

/// The compound cost of an expression.
///
/// Tracks the set instructions that make up this expression and sums their
/// costs, avoiding "double counting" the costs of values that were defined by
/// the same instruction and values that appear multiple times within the
/// expression (i.e. the expression is a DAG and not a tree).
#[derive(Clone, Debug)]
pub(crate) struct ExprCost {
    // The total cost of this expression.
    total: ScalarCost,
    // The set of instructions that must be evaluated to produce the associated
    // expression.
    insts: ImmutableEntitySet<Inst>,
}

impl Ord for ExprCost {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.total.cmp(&other.total)
    }
}

impl PartialOrd for ExprCost {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.total.partial_cmp(&other.total)
    }
}

impl PartialEq for ExprCost {
    fn eq(&self, other: &Self) -> bool {
        self.total == other.total
    }
}

impl Eq for ExprCost {}

impl ExprCost {
    /// Create an `ExprCost` with zero total cost and an empty set of
    /// instructions.
    pub fn zero() -> Self {
        Self {
            total: ScalarCost::zero(),
            insts: ImmutableEntitySet::default(),
        }
    }

    /// Create the cost for just the given instruction.
    pub fn for_inst(dfg: &DataFlowGraph, inst: Inst) -> Self {
        Self {
            total: ScalarCost::of_opcode(dfg.insts[inst].opcode()),
            insts: ImmutableEntitySet::unit(inst),
        }
    }

    /// Add the other cost into this cost, unioning its set of instructions into
    /// this cost's set, and only incrementing the total cost for new
    /// instructions.
    pub fn add(&mut self, dfg: &DataFlowGraph, other: &Self) {
        match (self.insts.len(), other.insts.len()) {
            // Nothing to do in this case.
            (_, 0) => {}

            // Clone `other` into `self` so that we reuse its set allocations.
            (0, _) => {
                *self = other.clone();
            }

            // Commute the addition so that we are (a) iterating over the
            // smaller of the two sets, and (b) maximizing reuse of existing set
            // allocations.
            (a, b) if a < b => {
                let mut other = other.clone();
                for inst in self.insts.iter() {
                    if other.insts.insert(inst) {
                        other.total = other.total + ScalarCost::of_opcode(dfg.insts[inst].opcode());
                    }
                }
                *self = other;
            }

            _ => {
                for inst in other.insts.iter() {
                    if self.insts.insert(inst) {
                        self.total = self.total + ScalarCost::of_opcode(dfg.insts[inst].opcode());
                    }
                }
            }
        }
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
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct ScalarCost(u32);

impl core::fmt::Debug for ScalarCost {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if *self == ScalarCost::infinity() {
            write!(f, "Cost::Infinite")
        } else {
            f.debug_struct("Cost::Finite")
                .field("op_cost", &self.op_cost())
                .field("depth", &self.depth())
                .finish()
        }
    }
}

impl Ord for ScalarCost {
    #[inline]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        // We make sure that the high bits are the op cost and the low bits are
        // the depth. This means that we can use normal integer comparison to
        // order by op cost and then depth.
        //
        // We want to break op cost ties with depth (rather than the other way
        // around). When the op cost is the same, we prefer shallow and wide
        // expressions to narrow and deep expressions and breaking ties with
        // `depth` gives us that. For example, `(a + b) + (c + d)` is preferred
        // to `((a + b) + c) + d`. This is beneficial because it exposes more
        // instruction-level parallelism and shortens live ranges.
        self.0.cmp(&other.0)
    }
}

impl PartialOrd for ScalarCost {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl ScalarCost {
    const DEPTH_BITS: u8 = 8;
    const DEPTH_MASK: u32 = (1 << Self::DEPTH_BITS) - 1;
    const OP_COST_MASK: u32 = !Self::DEPTH_MASK;
    const MAX_OP_COST: u32 = Self::OP_COST_MASK >> Self::DEPTH_BITS;

    pub(crate) fn infinity() -> ScalarCost {
        // 2^32 - 1 is, uh, pretty close to infinite... (we use `Cost`
        // only for heuristics and always saturate so this suffices!)
        ScalarCost(u32::MAX)
    }

    pub(crate) fn zero() -> ScalarCost {
        ScalarCost(0)
    }

    /// Construct a new `Cost` from the given parts.
    ///
    /// If the opcode cost is greater than or equal to the maximum representable
    /// opcode cost, then the resulting `Cost` saturates to infinity.
    fn new(opcode_cost: u32, depth: u8) -> ScalarCost {
        if opcode_cost >= Self::MAX_OP_COST {
            Self::infinity()
        } else {
            ScalarCost(opcode_cost << Self::DEPTH_BITS | u32::from(depth))
        }
    }

    fn depth(&self) -> u8 {
        let depth = self.0 & Self::DEPTH_MASK;
        u8::try_from(depth).unwrap()
    }

    fn op_cost(&self) -> u32 {
        (self.0 & Self::OP_COST_MASK) >> Self::DEPTH_BITS
    }

    /// Return the cost of an opcode.
    pub(crate) fn of_opcode(op: Opcode) -> ScalarCost {
        match op {
            // Constants.
            Opcode::Iconst | Opcode::F32const | Opcode::F64const => ScalarCost::new(1, 0),

            // Extends/reduces.
            Opcode::Uextend
            | Opcode::Sextend
            | Opcode::Ireduce
            | Opcode::Iconcat
            | Opcode::Isplit => ScalarCost::new(1, 0),

            // "Simple" arithmetic.
            Opcode::Iadd
            | Opcode::Isub
            | Opcode::Band
            | Opcode::Bor
            | Opcode::Bxor
            | Opcode::Bnot
            | Opcode::Ishl
            | Opcode::Ushr
            | Opcode::Sshr => ScalarCost::new(3, 0),

            // "Expensive" arithmetic.
            Opcode::Imul => ScalarCost::new(10, 0),

            // Everything else.
            _ => {
                // By default, be slightly more expensive than "simple"
                // arithmetic.
                let mut c = ScalarCost::new(4, 0);

                // And then get more expensive as the opcode does more side
                // effects.
                if op.can_trap() || op.other_side_effects() {
                    c = c + ScalarCost::new(10, 0);
                }
                if op.can_load() {
                    c = c + ScalarCost::new(20, 0);
                }
                if op.can_store() {
                    c = c + ScalarCost::new(50, 0);
                }

                c
            }
        }
    }

    /// Compute the cost of an operation in the side-effectful skeleton.
    pub(crate) fn of_skeleton_op(op: Opcode, arity: usize) -> Self {
        ScalarCost::of_opcode(op)
            + ScalarCost::new(u32::try_from(arity).unwrap(), (arity != 0) as _)
    }
}

impl core::iter::Sum<ScalarCost> for ScalarCost {
    fn sum<I: Iterator<Item = ScalarCost>>(iter: I) -> Self {
        iter.fold(Self::zero(), |a, b| a + b)
    }
}

impl core::default::Default for ScalarCost {
    fn default() -> ScalarCost {
        ScalarCost::zero()
    }
}

impl core::ops::Add<ScalarCost> for ScalarCost {
    type Output = ScalarCost;

    fn add(self, other: ScalarCost) -> ScalarCost {
        let op_cost = self.op_cost().saturating_add(other.op_cost());
        let depth = core::cmp::max(self.depth(), other.depth());
        ScalarCost::new(op_cost, depth)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl ScalarCost {
        fn of_opcode_and_operands(
            op: Opcode,
            operand_costs: impl IntoIterator<Item = Self>,
        ) -> Self {
            let c = Self::of_opcode(op) + operand_costs.into_iter().sum();
            ScalarCost::new(c.op_cost(), c.depth().saturating_add(1))
        }
    }

    #[test]
    fn add_cost() {
        let a = ScalarCost::new(5, 2);
        let b = ScalarCost::new(37, 3);
        assert_eq!(a + b, ScalarCost::new(42, 3));
        assert_eq!(b + a, ScalarCost::new(42, 3));
    }

    #[test]
    fn add_infinity() {
        let a = ScalarCost::new(5, 2);
        let b = ScalarCost::infinity();
        assert_eq!(a + b, ScalarCost::infinity());
        assert_eq!(b + a, ScalarCost::infinity());
    }

    #[test]
    fn op_cost_saturates_to_infinity() {
        let a = ScalarCost::new(ScalarCost::MAX_OP_COST - 10, 2);
        let b = ScalarCost::new(11, 2);
        assert_eq!(a + b, ScalarCost::infinity());
        assert_eq!(b + a, ScalarCost::infinity());
    }

    #[test]
    fn depth_saturates_to_max_depth() {
        let a = ScalarCost::new(10, u8::MAX);
        let b = ScalarCost::new(10, 1);
        assert_eq!(
            ScalarCost::of_opcode_and_operands(Opcode::Iconst, [a, b]),
            ScalarCost::new(21, u8::MAX)
        );
        assert_eq!(
            ScalarCost::of_opcode_and_operands(Opcode::Iconst, [b, a]),
            ScalarCost::new(21, u8::MAX)
        );
    }
}
