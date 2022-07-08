//! Node definition for EGraph representation.

use crate::ir::{Block, Inst, InstructionImms, Opcode, SourceLoc, Type};
use cranelift_egraph::{Id, Language};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Node<'a> {
    /// A blockparam. Effectively an input/root; refers to the
    /// CLIF-level Value.
    Param {
        /// CLIF block this param comes from.
        block: Block,
        /// Index of blockparam within block.
        index: u32,
        /// Type of the value.
        ty: Type,
    },
    /// A CLIF instruction that is pure (has no side-effects). Not
    /// tied to any location; we will compute a set of locations at
    /// which to compute this node during lowering back out of the
    /// egraph.
    Pure {
        /// The instruction data, without SSA values.
        op: InstructionImms,
        /// eclass arguments to the operator.
        args: &'a mut [Id],
        /// Types of results.
        types: &'a [Type],
    },
    /// A CLIF instruction that has side-effects or is otherwise not
    /// representable by `Pure`.
    Inst {
        /// The instruction data, without SSA values.
        op: InstructionImms,
        /// eclass arguments to the operator.
        args: &'a mut [Id],
        /// Types of results.
        types: &'a [Type],
        /// The original instruction. We include this so that the
        /// `Inst`s are not deduplicated: every instance is a
        /// logically separate and unique side-effect. However,
        /// because we clear the DataFlowGraph before elaboration,
        /// this `Inst` is *not* valid to fetch any details from the
        /// original instruction.
        inst: Inst,
        /// The source location to preserve.
        srcloc: SourceLoc,
    },
    /// A projection of one result of an `Inst` or `Pure`.
    Result {
        /// `Inst` or `Pure` node.
        value: Id,
        /// Index of the result we want.
        result: usize,
        /// Type of the value.
        ty: Type,
    },
}

fn op_cost(op: &InstructionImms) -> usize {
    match op.opcode() {
        // Constants.
        Opcode::Iconst | Opcode::F32const | Opcode::F64const | Opcode::Bconst => 0,
        // Extends/reduces.
        Opcode::Bextend
        | Opcode::Breduce
        | Opcode::Uextend
        | Opcode::Sextend
        | Opcode::Ireduce
        | Opcode::Iconcat
        | Opcode::Isplit => 1,
        // "Simple" arithmetic.
        Opcode::Iadd
        | Opcode::Isub
        | Opcode::Band
        | Opcode::BandNot
        | Opcode::Bor
        | Opcode::BorNot
        | Opcode::Bxor
        | Opcode::BxorNot
        | Opcode::Bnot => 2,
        // Everything else.
        _ => 3,
    }
}

impl<'a> Node<'a> {
    pub(crate) fn cost(&self) -> usize {
        match self {
            // Projections and parameters have no cost: they just
            // alias values.
            Node::Result { .. } | Node::Param { .. } => 0,
            Node::Pure { op, .. } => op_cost(op),
            // Side-effecting ops have the highest cost, but they're
            // special-cased below while scheduling because we must
            // perform them.
            Node::Inst { .. } => 10,
        }
    }
}

impl<'a> Language for Node<'a> {
    fn children(&self) -> &[Id] {
        match self {
            Node::Param { .. } => &[],
            Node::Pure { args, .. } | Node::Inst { args, .. } => args,
            Node::Result { value, .. } => std::slice::from_ref(value),
        }
    }

    fn children_mut(&mut self) -> &mut [Id] {
        match self {
            Node::Param { .. } => &mut [],
            Node::Pure { args, .. } | Node::Inst { args, .. } => args,
            Node::Result { value, .. } => std::slice::from_mut(value),
        }
    }
}
