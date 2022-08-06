//! Node definition for EGraph representation.

use super::MemoryState;
use crate::ir::{Block, Inst, InstructionImms, Opcode, SourceLoc, Type};
use cranelift_egraph::{BumpArena, BumpSlice, CtxEq, CtxHash, Id, Language};
use cranelift_entity::{EntityList, ListPool};
use std::cell::Cell;
use std::hash::{Hash, Hasher};

#[derive(Debug)]
pub enum Node {
    /// A blockparam. Effectively an input/root; does not refer to
    /// predecessors' branch arguments, because this would create
    /// cycles.
    Param {
        /// Cached hash.
        hash: Cell<u32>,

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
        /// Cached hash.
        hash: Cell<u32>,

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
        /// Cached hash.
        hash: Cell<u32>,

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
        /// Cached hash.
        hash: Cell<u32>,

        /// `Inst` or `Pure` node.
        value: Id,
        /// Index of the result we want.
        result: usize,
        /// Type of the value.
        ty: Type,
    },

    /// A load instruction. Nominally a side-effecting `Inst` (and
    /// included in the list of side-effecting roots so it will always
    /// be elaborated), but represented as a distinct kind of node so
    /// that we can leverage deduplication to do
    /// redundant-load-elimination for free (and make store-to-load
    /// forwarding much easier).
    Load {
        /// Cached hash.
        hash: Cell<u32>,

        // -- identity depends on:
        /// The original load operation. Must have one argument, the
        /// address.
        op: InstructionImms,
        /// The type of the load result.
        ty: Type,
        /// Canonicalized address. Used for identity, but not for
        /// computing the value.
        addr_canonical: Id,
        /// The abstract memory state that this load accesses.
        mem_state: MemoryState,

        // -- not included in dedup key:
        /// Address argument. Actual address has an offset, which is
        /// included in `op` (and thus already considered as part of
        /// the key).
        addr: Id,
        /// The `Inst` we will use for a trap location for this
        /// load. Excluded from Eq/Hash so that loads that are
        /// identical except for the specific instance will dedup on
        /// top of each other.
        inst: Inst,
        /// Source location, for traps. Not included in Eq/Hash.
        srcloc: SourceLoc,
    },
}

impl Node {
    pub(crate) fn is_non_pure(&self) -> bool {
        match self {
            Node::Inst { .. } | Node::Load { .. } => true,
            _ => false,
        }
    }
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

impl NodeCtx {
    pub(crate) fn with_capacity(types: usize, args: usize) -> Self {
        Self {
            types: BumpArena::arena_with_capacity(types),
            args: ListPool::with_capacity(args),
        }
    }
}

impl CtxEq<Node, Node> for NodeCtx {
    fn ctx_eq(&self, a: &Node, b: &Node) -> bool {
        let a_hash = a.cached_hash().get();
        let b_hash = b.cached_hash().get();
        if a_hash != b_hash && a_hash != 0 && b_hash != 0 {
            return false;
        }

        match (a, b) {
            (
                &Node::Param {
                    hash: _,
                    block,
                    index,
                    ty,
                },
                &Node::Param {
                    hash: _,
                    block: other_block,
                    index: other_index,
                    ty: other_ty,
                },
            ) => block == other_block && index == other_index && ty == other_ty,
            (
                &Node::Result {
                    hash: _,
                    value,
                    result,
                    ty,
                },
                &Node::Result {
                    hash: _,
                    value: other_value,
                    result: other_result,
                    ty: other_ty,
                },
            ) => value == other_value && result == other_result && ty == other_ty,
            (
                &Node::Pure {
                    hash: _,
                    ref op,
                    ref args,
                    ref types,
                },
                &Node::Pure {
                    hash: _,
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
                &Node::Inst {
                    hash: _,
                    inst,
                    ref args,
                    ..
                },
                &Node::Inst {
                    hash: _,
                    inst: other_inst,
                    args: ref other_args,
                    ..
                },
            ) => inst == other_inst && args.as_slice(&self.args) == other_args.as_slice(&self.args),
            (
                &Node::Load {
                    hash: _,
                    ref op,
                    ty,
                    addr_canonical,
                    mem_state,
                    ..
                },
                &Node::Load {
                    hash: _,
                    op: ref other_op,
                    ty: other_ty,
                    addr_canonical: other_addr_canonical,
                    mem_state: other_mem_state,
                    // Explicitly exclude: `inst` and `srcloc`. We
                    // want loads to merge if identical in
                    // opcode/offset, address expression, and last
                    // store (this does implicit
                    // redundant-load-elimination.)
                    ..
                },
            ) => {
                op == other_op
                    && ty == other_ty
                    && addr_canonical == other_addr_canonical
                    && mem_state == other_mem_state
            }
            _ => false,
        }
    }
}

impl Node {
    fn cached_hash(&self) -> &Cell<u32> {
        match self {
            Node::Param { hash, .. }
            | Node::Result { hash, .. }
            | Node::Pure { hash, .. }
            | Node::Inst { hash, .. }
            | Node::Load { hash, .. } => hash,
        }
    }
}

impl CtxHash<Node> for NodeCtx {
    fn ctx_hash(&self, value: &Node) -> u64 {
        let hash = value.cached_hash();
        if hash.get() != 0 {
            return hash.get() as u64;
        }

        let mut state = crate::fx::FxHasher::default();
        std::mem::discriminant(value).hash(&mut state);
        match value {
            &Node::Param {
                hash: _,
                block,
                index,
                ty,
            } => {
                block.hash(&mut state);
                index.hash(&mut state);
                ty.hash(&mut state);
            }
            &Node::Result {
                hash: _,
                value,
                result,
                ty,
            } => {
                value.hash(&mut state);
                result.hash(&mut state);
                ty.hash(&mut state);
            }
            &Node::Pure {
                hash: _,
                ref op,
                ref args,
                ref types,
            } => {
                op.hash(&mut state);
                args.as_slice(&self.args).hash(&mut state);
                types.as_slice(&self.types).hash(&mut state);
            }
            &Node::Inst {
                hash: _,
                inst,
                ref args,
                ..
            } => {
                inst.hash(&mut state);
                args.as_slice(&self.args).hash(&mut state);
            }
            &Node::Load {
                hash: _,
                ref op,
                ty,
                addr_canonical,
                mem_state,
                ..
            } => {
                op.hash(&mut state);
                ty.hash(&mut state);
                addr_canonical.hash(&mut state);
                mem_state.hash(&mut state);
            }
        }

        let h = (state.finish() & 0xffff_ffff) as u32;
        hash.set(h);
        h as u64
    }
}

pub(crate) fn op_cost(op: &InstructionImms) -> usize {
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

impl Language for NodeCtx {
    type Node = Node;

    fn children<'a>(&'a self, node: &'a Node) -> &'a [Id] {
        match node {
            Node::Param { .. } => &[],
            Node::Pure { args, .. } | Node::Inst { args, .. } => args.as_slice(&self.args),
            Node::Load { addr, .. } => std::slice::from_ref(addr),
            Node::Result { value, .. } => std::slice::from_ref(value),
        }
    }

    fn children_mut<'a>(&'a mut self, node: &'a mut Node) -> &'a mut [Id] {
        match node {
            Node::Param { .. } => &mut [],
            Node::Pure { args, .. } | Node::Inst { args, .. } => args.as_mut_slice(&mut self.args),
            Node::Load { addr, .. } => std::slice::from_mut(addr),
            Node::Result { value, .. } => std::slice::from_mut(value),
        }
    }

    fn needs_dedup(&self, node: &Node) -> bool {
        match node {
            Node::Pure { .. } | Node::Load { .. } => true,
            _ => false,
        }
    }
}
