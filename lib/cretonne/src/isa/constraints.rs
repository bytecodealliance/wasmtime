//! Register constraints for instruction operands.
//!
//! An encoding recipe specifies how an instruction is encoded as binary machine code, but it only
//! works if the operands and results satisfy certain constraints. Constraints on immediate
//! operands are checked by instruction predicates when the recipe is chosen.
//!
//! It is the register allocator's job to make sure that the register constraints on value operands
//! are satisfied.

use isa::{RegClass, RegUnit};

/// Register constraint for a single value operand or instruction result.
pub struct OperandConstraint {
    /// The kind of constraint.
    pub kind: ConstraintKind,

    /// The register class of the operand.
    ///
    /// This applies to all kinds of constraints, but with slightly different meaning.
    pub regclass: RegClass,
}

/// The different kinds of operand constraints.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ConstraintKind {
    /// This operand or result must be a register from the given register class.
    Reg,

    /// This operand or result must be a fixed register.
    ///
    /// The constraint's `regclass` field is the top-level register class containing the fixed
    /// register.
    FixedReg(RegUnit),

    /// This result value must use the same register as an input value operand. Input operands
    /// can't be tied.
    ///
    /// The associated number is the index of the input value operand this result is tied to.
    ///
    /// The constraint's `regclass` field is the top-level register class containing the tied
    /// operand's register class.
    Tied(u8),

    /// This operand must be a value in a stack slot.
    ///
    /// The constraint's `regclass` field is the register class that would normally be used to load
    /// and store values of this type.
    Stack,
}

/// Constraints for an encoding recipe.
pub struct RecipeConstraints {
    /// Constraints for the instruction's fixed value operands.
    ///
    /// If the instruction takes a variable number of operands, the register constraints for those
    /// operands must be computed dynamically.
    ///
    /// - For branches and jumps, EBB arguments must match the expectations of the destination EBB.
    /// - For calls and returns, the calling convention ABI specifies constraints.
    pub ins: &'static [OperandConstraint],

    /// Constraints for the instruction's fixed results.
    ///
    /// If the instruction produces a variable number of results, it's probably a call and the
    /// constraints must be derived from the calling convention ABI.
    pub outs: &'static [OperandConstraint],
}
