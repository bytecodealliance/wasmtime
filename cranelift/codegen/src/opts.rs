//! Optimization driver using ISLE rewrite rules on an egraph.

use crate::egg::FuncEGraph;
pub use crate::egg::Node;
pub use crate::ir::condcodes::{FloatCC, IntCC};
pub use crate::ir::immediates::{Ieee32, Ieee64, Imm64, Offset32, Uimm32, Uimm64, Uimm8};
pub use crate::ir::types::*;
pub use crate::ir::{
    AtomicRmwOp, Block, Constant, FuncRef, GlobalValue, Heap, Immediate, InstructionImms,
    JumpTable, MemFlags, Opcode, StackSlot, Table, TrapCode, Type, Value,
};
use crate::isle_common_prelude_methods;
pub use cranelift_egraph::Id;
use cranelift_entity::{EntityList, EntityRef};
use smallvec::{smallvec, SmallVec};

pub type IdArray = EntityList<Id>;
pub type Unit = ();

pub type ConstructorVec<T> = SmallVec<[T; 8]>;

mod generated_code;

struct IsleContext<'a, 'b> {
    egraph: &'a mut FuncEGraph<'b>,
}

pub fn optimize<'a>(egraph: &mut FuncEGraph<'a>) {
    let mut ctx = IsleContext { egraph };
    ctx.do_rewrites();
}

impl<'a, 'b> IsleContext<'a, 'b> {
    pub fn do_rewrites(&mut self) {
        const MAX_ITERS: usize = 10;
        let mut iters = 0;
        while !self.egraph.egraph.dirty_classes_workset().is_empty() && iters < MAX_ITERS {
            let mut dirty_batch = self.egraph.egraph.dirty_classes_workset().take_batch();
            for dirty_eclass_id in dirty_batch.batch() {
                self.do_rewrites_on_eclass(dirty_eclass_id);
            }
            self.egraph
                .egraph
                .dirty_classes_workset()
                .reuse(dirty_batch);

            self.egraph.egraph.rebuild(&mut self.egraph.node_ctx);
            iters += 1;
        }
    }

    fn do_rewrites_on_eclass(&mut self, id: Id) {
        log::trace!("running rules on eclass {}", id.index());
        let optimized_ids = generated_code::constructor_simplify(self, id);
        if let Some(ids) = optimized_ids {
            for new_id in ids {
                log::trace!(" -> merging in new eclass {}", new_id);
                self.egraph
                    .egraph
                    .union(id, new_id, &mut self.egraph.node_ctx);
            }
        }
    }
}

impl<'a, 'b> generated_code::Context for IsleContext<'a, 'b> {
    isle_common_prelude_methods!();

    fn eclass_type(&mut self, eclass: Id) -> Option<Type> {
        for node in self.egraph.egraph.enodes(eclass) {
            match node {
                &Node::Pure { types, .. } if types.len() == 1 => {
                    return Some(types.as_slice(&self.egraph.node_ctx.types)[0]);
                }
                _ => {}
            }
        }
        None
    }

    fn pure_enodes_etor(
        &mut self,
        eclass: Id,
        multi_index: &mut usize,
    ) -> Option<(Type, InstructionImms, IdArray)> {
        let nodes = self.egraph.egraph.enodes(eclass);
        while *multi_index < nodes.len() {
            let i = *multi_index;
            *multi_index += 1;
            match nodes[i] {
                Node::Pure { op, args, types } if types.len() == 1 => {
                    let ty = types.as_slice(&self.egraph.node_ctx.types)[0];
                    return Some((ty, op.clone(), args.clone()));
                }
                _ => {}
            }
        }
        None
    }

    fn pure_enode_ctor(&mut self, ty: Type, op: &InstructionImms, args: IdArray) -> Id {
        let types = self.egraph.node_ctx.types.single(ty);
        let types = types.freeze(&mut self.egraph.node_ctx.types);
        let op = op.clone();
        self.egraph
            .egraph
            .add(Node::Pure { op, args, types }, &mut self.egraph.node_ctx)
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

    fn eclass_union(&mut self, arg0: Id, arg1: Id) -> Unit {
        self.egraph
            .egraph
            .union(arg0, arg1, &mut self.egraph.node_ctx);

        ()
    }

    fn commutative_ctor(&mut self, a: Id, b: Id) -> Option<ConstructorVec<generated_code::Pair>> {
        Some(smallvec![
            generated_code::Pair::Id { a, b },
            generated_code::Pair::Id { b, a }
        ])
    }
}
