//! Mutators for the `gc` operations.

use crate::generators::gc_ops::ops::{GcOp, GcOps};
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
    fn generate(&mut self, _ctx: &mut Context) -> MutResult<GcOps> {
        let mut ops = GcOps::default();
        let mut session = mutatis::Session::new();

        for _ in 0..64 {
            session.mutate(&mut ops)?;
        }

        Ok(ops)
    }
}
