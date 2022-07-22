//! Node definition for EGraph representation.

use crate::ir::{Block, Inst, InstructionImms, Opcode, SourceLoc, Type};
use cranelift_egraph::{BumpArena, BumpSlice, CtxEq, CtxHash, Id, Language};
use cranelift_entity::{EntityList, ListPool};
use std::hash::{Hash, Hasher};

#[derive(Debug)]
pub enum Node {
    /// A blockparam. Effectively an input/root; does not refer to
    /// predecessors' branch arguments, because this would create
    /// cycles.
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
        args: EntityList<Id>,
        /// Types of results.
        types: BumpSlice<Type>,
    },
    /// A CLIF instruction that has side-effects or is otherwise not
    /// representable by `Pure`.
    Inst {
        /// The instruction data, without SSA values.
        op: InstructionImms,
        /// eclass arguments to the operator.
        args: EntityList<Id>,
        /// Types of results.
        types: BumpSlice<Type>,
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

/// Context for comparing and hashing Nodes.
pub struct NodeCtx {
    /// Arena for result-type arrays.
    pub types: BumpArena<Type>,
    /// Arena for arg eclass-ID lists.
    pub args: ListPool<Id>,
}

impl std::default::Default for NodeCtx {
    fn default() -> Self {
        Self {
            types: BumpArena::default(),
            args: ListPool::new(),
        }
    }
}

impl CtxEq<Node, Node> for NodeCtx {
    fn ctx_eq(&self, a: &Node, b: &Node) -> bool {
        match (a, b) {
            (
                &Node::Param { block, index, ty },
                &Node::Param {
                    block: other_block,
                    index: other_index,
                    ty: other_ty,
                },
            ) => block == other_block && index == other_index && ty == other_ty,
            (
                &Node::Result { value, result, ty },
                &Node::Result {
                    value: other_value,
                    result: other_result,
                    ty: other_ty,
                },
            ) => value == other_value && result == other_result && ty == other_ty,
            (
                &Node::Pure {
                    ref op,
                    ref args,
                    ref types,
                },
                &Node::Pure {
                    op: ref other_op,
                    args: ref other_args,
                    types: ref other_types,
                },
            ) => {
                *op == *other_op
                    && args.as_slice(&self.args) == other_args.as_slice(&self.args)
                    && types.as_slice(&self.types) == other_types.as_slice(&self.types)
            }
            (
                &Node::Inst { inst, ref args, .. },
                &Node::Inst {
                    inst: other_inst,
                    args: ref other_args,
                    ..
                },
            ) => inst == other_inst && args.as_slice(&self.args) == other_args.as_slice(&self.args),
            _ => false,
        }
    }
}

impl CtxHash<Node> for NodeCtx {
    fn ctx_hash<H: Hasher>(&self, value: &Node, state: &mut H) {
        std::mem::discriminant(value).hash(state);
        match value {
            &Node::Param { block, index, ty } => {
                block.hash(state);
                index.hash(state);
                ty.hash(state);
            }
            &Node::Result { value, result, ty } => {
                value.hash(state);
                result.hash(state);
                ty.hash(state);
            }
            &Node::Pure {
                ref op,
                ref args,
                ref types,
            } => {
                op.hash(state);
                args.as_slice(&self.args).hash(state);
                types.as_slice(&self.types).hash(state);
            }
            &Node::Inst { inst, ref args, .. } => {
                inst.hash(state);
                args.as_slice(&self.args).hash(state);
            }
        }
    }
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

impl Node {
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

impl Language for NodeCtx {
    type Node = Node;

    fn children<'a>(&'a self, node: &'a Node) -> &'a [Id] {
        match node {
            Node::Param { .. } => &[],
            Node::Pure { args, .. } | Node::Inst { args, .. } => args.as_slice(&self.args),
            Node::Result { value, .. } => std::slice::from_ref(value),
        }
    }

    fn children_mut<'a>(&'a mut self, node: &'a mut Node) -> &'a mut [Id] {
        match node {
            Node::Param { .. } => &mut [],
            Node::Pure { args, .. } | Node::Inst { args, .. } => args.as_mut_slice(&mut self.args),
            Node::Result { value, .. } => std::slice::from_mut(value),
        }
    }
}
