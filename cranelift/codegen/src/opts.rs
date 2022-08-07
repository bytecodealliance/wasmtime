//! Optimization driver using ISLE rewrite rules on an egraph.

use crate::egg::FuncEGraph;
use crate::egg::MemoryState;
pub use crate::egg::{Node, NodeCtx};
pub use crate::ir::condcodes::{FloatCC, IntCC};
pub use crate::ir::immediates::{Ieee32, Ieee64, Imm64, Offset32, Uimm32, Uimm64, Uimm8};
pub use crate::ir::types::*;
pub use crate::ir::{
    AtomicRmwOp, Block, Constant, FuncRef, GlobalValue, Heap, Immediate, InstructionImms,
    JumpTable, MemFlags, Opcode, StackSlot, Table, TrapCode, Type, Value,
};
use crate::isle_common_prelude_methods;
pub use cranelift_egraph::{Id, NewOrExisting, NodeIter};
use cranelift_entity::{EntityList, EntityRef};
use smallvec::{smallvec, SmallVec};
use std::marker::PhantomData;

pub type IdArray = EntityList<Id>;
#[allow(dead_code)]
pub type Unit = ();

pub type ConstructorVec<T> = SmallVec<[T; 8]>;

mod generated_code;

struct IsleContext<'a, 'b> {
    egraph: &'a mut FuncEGraph<'b>,
}

pub fn optimize_eclass<'a>(id: Id, egraph: &mut FuncEGraph<'a>) -> Id {
    log::trace!("running rules on eclass {}", id.index());
    egraph.stats.rewrite_rule_invoked += 1;

    // Store-to-load forwarding rewrites the ID without unioning with
    // the original: we want to eliminate the load entirely.
    let id = store_to_load(id, egraph);
    // Find all possible rewrites and union them in, returning the
    // union.
    let mut ctx = IsleContext { egraph };
    let optimized_ids = generated_code::constructor_simplify(&mut ctx, id);
    let mut union_id = id;
    if let Some(ids) = optimized_ids {
        egraph.stats.rewrite_rule_return_count_sum += ids.len() as u64;
        for new_id in ids {
            egraph.stats.node_union += 1;
            union_id = egraph.egraph.union(union_id, new_id);
        }
    }
    union_id
}

fn store_to_load<'a>(id: Id, egraph: &mut FuncEGraph<'a>) -> Id {
    if let Some(load_key) = egraph.egraph.classes[id].get_node() {
        if let Node::Load {
            op: load_op,
            ty: load_ty,
            addr: load_addr,
            mem_state: MemoryState::Store(store_inst),
            ..
        } = load_key.node::<NodeCtx>(&egraph.egraph.nodes)
        {
            log::trace!(" -> got load op for id {}: {:?}", id, load_op);
            if let Some((store_ty, store_id)) = egraph.store_nodes.get(&store_inst) {
                log::trace!(" -> got store id: {} ty: {}", store_id, store_ty);
                if *store_ty == *load_ty {
                    if let Some(store_key) = egraph.egraph.classes[*store_id].get_node() {
                        if let Node::Inst {
                            op: store_op,
                            args: store_args,
                            ..
                        } = store_key.node::<NodeCtx>(&egraph.egraph.nodes)
                        {
                            log::trace!(
                                "load id {} from store id {}: {:?}, {:?}",
                                id,
                                store_id,
                                load_op,
                                store_op
                            );
                            match (load_op, store_op) {
                                (
                                    InstructionImms::Load {
                                        opcode: Opcode::Load,
                                        offset: load_offset,
                                        ..
                                    },
                                    InstructionImms::Store {
                                        opcode: Opcode::Store,
                                        offset: store_offset,
                                        ..
                                    },
                                ) if *load_offset == *store_offset => {
                                    log::trace!(" -> same offset");
                                    let store_args = store_args.as_slice(&egraph.node_ctx.args);
                                    let store_data = store_args[0];
                                    let store_addr = store_args[1];
                                    let store_addr = egraph.egraph.canonical_id(store_addr);
                                    let load_addr = egraph.egraph.canonical_id(*load_addr);
                                    if store_addr == load_addr {
                                        log::trace!(" -> same address; forwarding");
                                        egraph.stats.store_to_load_forward += 1;
                                        return store_data;
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }

    id
}

struct PureNodesEtorIter<'a, 'b>
where
    'b: 'a,
{
    root: Id,
    iter: NodeIter<NodeCtx>,
    _phantom1: PhantomData<&'a ()>,
    _phantom2: PhantomData<&'b ()>,
}

impl<'a, 'b> generated_code::ContextIter for PureNodesEtorIter<'a, 'b>
where
    'b: 'a,
{
    type Context = IsleContext<'a, 'b>;
    type Output = (Type, InstructionImms, IdArray);

    fn next(&mut self, ctx: &mut IsleContext<'a, 'b>) -> Option<Self::Output> {
        while let Some(node) = self.iter.next(&ctx.egraph.egraph) {
            log::trace!("iter from root {}: node {:?}", self.root, node);
            match node {
                Node::Pure { op, args, types } if types.len() == 1 => {
                    let ty = types.as_slice(&ctx.egraph.node_ctx.types)[0];
                    return Some((ty, op.clone(), args.clone()));
                }
                _ => {}
            }
        }
        None
    }
}

impl<'a, 'b> generated_code::Context for IsleContext<'a, 'b> {
    isle_common_prelude_methods!();

    fn eclass_type(&mut self, eclass: Id) -> Option<Type> {
        let mut iter = self.egraph.egraph.enodes(eclass);
        while let Some(node) = iter.next(&self.egraph.egraph) {
            match node {
                &Node::Pure { types, .. } if types.len() == 1 => {
                    return Some(types.as_slice(&self.egraph.node_ctx.types)[0]);
                }
                _ => {}
            }
        }
        None
    }

    type pure_enodes_etor_iter = PureNodesEtorIter<'a, 'b>;

    fn pure_enodes_etor(&mut self, eclass: Id) -> Option<PureNodesEtorIter<'a, 'b>> {
        Some(PureNodesEtorIter {
            root: eclass,
            iter: self.egraph.egraph.enodes(eclass),
            _phantom1: PhantomData,
            _phantom2: PhantomData,
        })
    }

    fn pure_enode_ctor(&mut self, ty: Type, op: &InstructionImms, args: IdArray) -> Id {
        let types = self.egraph.node_ctx.types.single(ty);
        let types = types.freeze(&mut self.egraph.node_ctx.types);
        let op = op.clone();
        match self
            .egraph
            .egraph
            .add(Node::Pure { op, args, types }, &mut self.egraph.node_ctx)
        {
            NewOrExisting::New(id) => optimize_eclass(id, self.egraph),
            NewOrExisting::Existing(id) => id,
        }
    }

    fn id_array_0_etor(&mut self, arg0: IdArray) -> Option<()> {
        let values = arg0.as_slice(&self.egraph.node_ctx.args);
        if values.len() == 0 {
            Some(())
        } else {
            None
        }
    }

    fn id_array_0_ctor(&mut self) -> IdArray {
        EntityList::default()
    }

    fn id_array_1_etor(&mut self, arg0: IdArray) -> Option<Id> {
        let values = arg0.as_slice(&self.egraph.node_ctx.args);
        if values.len() == 1 {
            Some(values[0])
        } else {
            None
        }
    }

    fn id_array_1_ctor(&mut self, arg0: Id) -> IdArray {
        EntityList::from_iter([arg0].into_iter(), &mut self.egraph.node_ctx.args)
    }

    fn id_array_2_etor(&mut self, arg0: IdArray) -> Option<(Id, Id)> {
        let values = arg0.as_slice(&self.egraph.node_ctx.args);
        if values.len() == 2 {
            Some((values[0], values[1]))
        } else {
            None
        }
    }

    fn id_array_2_ctor(&mut self, arg0: Id, arg1: Id) -> IdArray {
        EntityList::from_iter([arg0, arg1].into_iter(), &mut self.egraph.node_ctx.args)
    }

    fn id_array_3_etor(&mut self, arg0: IdArray) -> Option<(Id, Id, Id)> {
        let values = arg0.as_slice(&self.egraph.node_ctx.args);
        if values.len() == 3 {
            Some((values[0], values[1], values[2]))
        } else {
            None
        }
    }

    fn id_array_3_ctor(&mut self, arg0: Id, arg1: Id, arg2: Id) -> IdArray {
        EntityList::from_iter(
            [arg0, arg1, arg2].into_iter(),
            &mut self.egraph.node_ctx.args,
        )
    }

    fn commutative_ctor(&mut self, a: Id, b: Id) -> Option<ConstructorVec<generated_code::Pair>> {
        Some(smallvec![
            generated_code::Pair::Id { a, b },
            generated_code::Pair::Id { b, a }
        ])
    }
}
