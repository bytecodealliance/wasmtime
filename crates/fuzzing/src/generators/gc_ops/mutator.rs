//! Mutators for the `gc` operations.

use crate::generators::gc_ops::{
    limits::{
        GcOpsLimits, MAX_OPS, MAX_REC_GROUPS_RANGE, MAX_TYPES_RANGE, NUM_GLOBALS_RANGE,
        NUM_PARAMS_RANGE, TABLE_SIZE_RANGE,
    },
    ops::{GcOp, GcOps},
    types::{RecGroupId, TypeId, Types},
};

use mutatis::mutators as m;
use mutatis::{Candidates, Context, DefaultMutate, Generate, Mutate, Result as MutResult};

/// A mutator for the gc ops
#[derive(Debug)]
pub struct GcOpsMutator;

impl Mutate<GcOps> for GcOpsMutator {
    fn mutate(&mut self, c: &mut Candidates<'_>, ops: &mut GcOps) -> mutatis::Result<()> {
        if !c.shrink() {
            c.mutation(|ctx| {
                if let Some(idx) = ctx.rng().gen_index(ops.ops.len() + 1) {
                    let stack = ops.abstract_stack_depth(idx);
                    let (op, _new_stack_size) = GcOp::generate(ctx, &ops, stack)?;
                    ops.ops.insert(idx, op);
                }
                Ok(())
            })?;
        }

        if !ops.ops.is_empty() {
            c.mutation(|ctx| {
                let idx = ctx
                    .rng()
                    .gen_index(ops.ops.len())
                    .expect("ops is not empty");
                ops.ops.remove(idx);
                Ok(())
            })?;
        }
        Ok(())
    }
}

impl DefaultMutate for GcOps {
    type DefaultMutate = GcOpsMutator;
}

impl Default for GcOpsMutator {
    fn default() -> Self {
        GcOpsMutator
    }
}

impl<'a> arbitrary::Arbitrary<'a> for GcOps {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let mut session = mutatis::Session::new().seed(u.arbitrary()?);
        session
            .generate()
            .map_err(|_| arbitrary::Error::IncorrectFormat)
    }
}

impl Generate<GcOps> for GcOpsMutator {
    fn generate(&mut self, ctx: &mut Context) -> MutResult<GcOps> {
        let num_params = m::range(NUM_PARAMS_RANGE).generate(ctx)?;
        let num_globals = m::range(NUM_GLOBALS_RANGE).generate(ctx)?;
        let table_size = m::range(TABLE_SIZE_RANGE).generate(ctx)?;

        let max_rec_groups = m::range(MAX_REC_GROUPS_RANGE).generate(ctx)?;
        let max_types = m::range(MAX_TYPES_RANGE).generate(ctx)?;

        let mut ops = GcOps {
            limits: GcOpsLimits {
                num_params,
                num_globals,
                table_size,
                max_rec_groups,
                max_types,
            },
            ops: {
                let mut v = vec![GcOp::Null(), GcOp::Drop(), GcOp::Gc()];
                if num_params > 0 {
                    v.push(GcOp::LocalSet(0));
                    v.push(GcOp::LocalGet(0));
                }
                if num_globals > 0 {
                    v.push(GcOp::GlobalSet(0));
                    v.push(GcOp::GlobalGet(0));
                }
                if max_types > 0 {
                    v.push(GcOp::StructNew(0));
                }
                v
            },
            types: Types::new(),
        };

        for i in 0..ops.limits.max_rec_groups {
            ops.types.insert_rec_group(RecGroupId(i));
        }

        if ops.limits.max_rec_groups > 0 {
            for i in 0..ops.limits.max_types {
                let tid = TypeId(i);
                let gid = RecGroupId(m::range(0..=ops.limits.max_rec_groups - 1).generate(ctx)?);

                ops.types.insert_empty_struct(tid, gid);
            }
        }

        let mut stack: usize = 0;

        while ops.ops.len() < MAX_OPS {
            let (op, new_stack_len) = GcOp::generate(ctx, &ops, stack)?;
            ops.ops.push(op);
            stack = new_stack_len;
        }

        // Drop any leftover refs on the stack.
        for _ in 0..stack {
            ops.ops.push(GcOp::Drop());
        }

        Ok(ops)
    }
}
