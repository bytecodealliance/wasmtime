//! A linear IR for optimizations.
//!
//! This IR is designed such that it should be easy to combine multiple linear
//! optimizations into a single automata.
//!
//! See also `src/linearize.rs` for the AST to linear IR translation pass.

use crate::cc::ConditionCode;
use crate::integer_interner::{IntegerId, IntegerInterner};
use crate::operator::{Operator, UnquoteOperator};
use crate::paths::{PathId, PathInterner};
use crate::r#type::{BitWidth, Type};
use serde::{Deserialize, Serialize};

/// A set of linear optimizations.
#[derive(Debug)]
pub struct Optimizations {
    /// The linear optimizations.
    pub optimizations: Vec<Optimization>,

    /// The de-duplicated paths referenced by these optimizations.
    pub paths: PathInterner,

    /// The integer literals referenced by these optimizations.
    pub integers: IntegerInterner,
}

/// A linearized optimization.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Optimization {
    /// The chain of increments for this optimization.
    pub increments: Vec<Increment>,
}

/// An increment is a matching operation, the expected result from that
/// operation to continue to the next increment, and the actions to take to
/// build up the LHS scope and RHS instructions given that we got the expected
/// result from this increment's matching operation. Each increment will
/// basically become a state and a transition edge out of that state in the
/// final automata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Increment {
    /// The matching operation to perform.
    pub operation: MatchOp,

    /// The expected result of our matching operation, that enables us to
    /// continue to the next increment. `None` is used for wildcard-style "else"
    /// transitions.
    pub expected: Option<u32>,

    /// Actions to perform, given that the operation resulted in the expected
    /// value.
    pub actions: Vec<Action>,
}

/// A matching operation to be performed on some Cranelift instruction as part
/// of determining whether an optimization is applicable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum MatchOp {
    /// Switch on the opcode of an instruction.
    Opcode {
        /// The path to the instruction whose opcode we're switching on.
        path: PathId,
    },

    /// Does an instruction have a constant value?
    IsConst {
        /// The path to the instruction (or immediate) that we're checking
        /// whether it is constant or not.
        path: PathId,
    },

    /// Is the constant value a power of two?
    IsPowerOfTwo {
        /// The path to the instruction (or immediate) that we are checking
        /// whether it is a constant power of two or not.
        path: PathId,
    },

    /// Switch on the bit width of a value.
    BitWidth {
        /// The path to the instruction (or immediate) whose result's bit width
        /// we are checking.
        path: PathId,
    },

    /// Does the value fit in our target architecture's native word size?
    FitsInNativeWord {
        /// The path to the instruction (or immediate) whose result we are
        /// checking whether it fits in a native word or not.
        path: PathId,
    },

    /// Are the instructions (or immediates) at the given paths the same?
    Eq {
        /// The path to the first instruction (or immediate).
        path_a: PathId,
        /// The path to the second instruction (or immediate).
        path_b: PathId,
    },

    /// Switch on the constant integer value of an instruction.
    IntegerValue {
        /// The path to the instruction.
        path: PathId,
    },

    /// Switch on the constant boolean value of an instruction.
    BooleanValue {
        /// The path to the instruction.
        path: PathId,
    },

    /// Switch on a condition code.
    ConditionCode {
        /// The path to the condition code.
        path: PathId,
    },

    /// No operation. Always evaluates to `None`.
    ///
    /// Exceedingly rare in real optimizations; nonetheless required to support
    /// corner cases of the DSL, such as a LHS pattern that is nothing but a
    /// variable pattern.
    Nop,
}

/// A canonicalized identifier for a left-hand side value that was bound in a
/// pattern.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LhsId(pub u32);

/// A canonicalized identifier for a right-hand side value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RhsId(pub u32);

/// An action to perform when transitioning between states in the automata.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Action {
    /// Implicitly define the n^th built up RHS instruction as something from
    /// the left-hand side.
    GetLhs {
        /// The path to the instruction or value.
        path: PathId,
    },

    /// Implicitly define the n^th RHS instruction as the result of the
    /// compile-time evaluation off this unquote operation.
    UnaryUnquote {
        /// The unquote operator.
        operator: UnquoteOperator,
        /// The constant operand to this unquote.
        operand: RhsId,
    },

    /// Implicitly define the n^th RHS instruction as the result of the
    /// compile-time evaluation off this unquote operation.
    BinaryUnquote {
        /// The unquote operator.
        operator: UnquoteOperator,
        /// The constant operands to this unquote.
        operands: [RhsId; 2],
    },

    /// Implicitly define the n^th RHS as an integer constant.
    MakeIntegerConst {
        /// The constant integer value.
        value: IntegerId,
        /// The bit width of this constant.
        bit_width: BitWidth,
    },

    /// Implicitly define the n^th RHS as a boolean constant.
    MakeBooleanConst {
        /// The constant boolean value.
        value: bool,
        /// The bit width of this constant.
        bit_width: BitWidth,
    },

    /// Implicitly defint the n^th RHS as a condition code.
    MakeConditionCode {
        /// The condition code.
        cc: ConditionCode,
    },

    /// Implicitly define the n^th RHS instruction by making a unary
    /// instruction.
    MakeUnaryInst {
        /// The operand for this instruction.
        operand: RhsId,
        /// The type of this instruction's result.
        r#type: Type,
        /// The operator for this instruction.
        operator: Operator,
    },

    /// Implicitly define the n^th RHS instruction by making a binary
    /// instruction.
    MakeBinaryInst {
        /// The opcode for this instruction.
        operator: Operator,
        /// The type of this instruction's result.
        r#type: Type,
        /// The operands for this instruction.
        operands: [RhsId; 2],
    },

    /// Implicitly define the n^th RHS instruction by making a ternary
    /// instruction.
    MakeTernaryInst {
        /// The opcode for this instruction.
        operator: Operator,
        /// The type of this instruction's result.
        r#type: Type,
        /// The operands for this instruction.
        operands: [RhsId; 3],
    },
}
