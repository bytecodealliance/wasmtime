//! Mutators for the `gc` operations.

use crate::generators::gc_ops::ops::{GcOp, GcOps};
use crate::generators::gc_ops::types::{RecGroupId, TypeId};
use mutatis::{Candidates, Context, DefaultMutate, Generate, Mutate, Result as MutResult};
use smallvec::SmallVec;

/// A mutator for the gc ops.
#[derive(Debug)]
pub struct GcOpsMutator;

impl GcOpsMutator {
    // Define a mutation that adds an operation to the ops list.
    fn add_operation(&mut self, c: &mut Candidates<'_>, ops: &mut GcOps) -> mutatis::Result<()> {
        if !c.shrink() {
            c.mutation(|ctx| {
                if let Some(idx) = ctx.rng().gen_index(ops.ops.len() + 1) {
                    let op = GcOp::generate(ctx)?;
                    ops.ops.insert(idx, op);
                    log::debug!("Added operation {op:?} to ops list");
                }
                Ok(())
            })?;
        }
        Ok(())
    }

    // Define a mutation that removes an operation from the ops list.
    fn remove_operation(&mut self, c: &mut Candidates<'_>, ops: &mut GcOps) -> mutatis::Result<()> {
        if !ops.ops.is_empty() {
            c.mutation(|ctx| {
                let idx = ctx.rng().gen_index(ops.ops.len()).expect("ops not empty");
                let removed = ops.ops.remove(idx);
                log::debug!("Removed operation {removed:?} from ops list");
                Ok(())
            })?;
        }
        Ok(())
    }

    // Define a mutation that adds an empty struct type to an existing (rec ...) group.
    fn add_new_struct_type_to_rec_group(
        &mut self,
        c: &mut Candidates<'_>,
        ops: &mut GcOps,
    ) -> mutatis::Result<()> {
        if !c.shrink()
            && !ops.types.rec_groups.is_empty()
            && ops.types.type_defs.len() < ops.limits.max_types as usize
        {
            c.mutation(|ctx| {
                let group_id = ctx
                    .rng()
                    .choose(&ops.types.rec_groups)
                    .copied()
                    .expect("rec_groups not empty");
                let new_tid = ops.types.fresh_type_id(ctx.rng());
                ops.types.insert_empty_struct(new_tid, group_id);
                log::debug!("Added empty struct type {new_tid:?} to rec group {group_id:?}");
                Ok(())
            })?;
        }
        Ok(())
    }

    // Define a mutation that removes a struct type from an existing (rec ...).
    // It may result in empty rec groups. Empty rec groups are allowed.
    fn remove_struct_type_from_rec_group(
        &mut self,
        c: &mut Candidates<'_>,
        ops: &mut GcOps,
    ) -> mutatis::Result<()> {
        if !ops.types.type_defs.is_empty() {
            c.mutation(|ctx| {
                let tid = ctx
                    .rng()
                    .choose(ops.types.type_defs.keys())
                    .copied()
                    .expect("type_defs not empty");
                ops.types.type_defs.remove(&tid);
                log::debug!("Removed struct type {tid:?}");
                Ok(())
            })?;
        }
        Ok(())
    }

    // Define a mutation that moves a struct type within an existing rec group.
    fn move_struct_type_within_rec_group(
        &mut self,
        c: &mut Candidates<'_>,
        ops: &mut GcOps,
    ) -> mutatis::Result<()> {
        if !ops.types.rec_groups.is_empty() && ops.types.type_defs.len() >= 2 {
            c.mutation(|ctx| {
                let mut chosen: Option<(RecGroupId, TypeId, TypeId)> = None;

                // Randomly choose a rec group.
                for _ in 0..ops.limits.max_rec_groups {
                    let gid = ctx
                        .rng()
                        .choose(&ops.types.rec_groups)
                        .copied()
                        .expect("rec_groups not empty");

                    // Collect member TypeIds of that rec group.
                    let mut members: SmallVec<[TypeId; 32]> = SmallVec::new();
                    for (tid, def) in ops.types.type_defs.iter() {
                        if def.rec_group == gid {
                            members.push(*tid);
                        }
                    }

                    // If this is a singleton/empty group, try another rec group.
                    if members.len() < 2 {
                        continue;
                    }

                    // Pick two distinct members randomly.
                    let tid_a = *ctx.rng().choose(&members).expect("len >= 2");
                    let mut tid_b = *ctx.rng().choose(&members).expect("len >= 2");
                    for _ in 0..members.len() {
                        if tid_a != tid_b {
                            break;
                        }
                        tid_b = *ctx.rng().choose(&members).unwrap();
                    }
                    if tid_a == tid_b {
                        continue;
                    }
                    chosen = Some((gid, tid_a, tid_b));
                    break;
                }

                // Move within group - reorder for encoding by swapping map values.
                if let Some((gid, tid_a, tid_b)) = chosen {
                    let a_def = ops.types.type_defs.remove(&tid_a).expect("tid_a present");
                    let b_def = ops.types.type_defs.remove(&tid_b).expect("tid_b present");
                    debug_assert!(a_def.rec_group == gid);
                    debug_assert!(b_def.rec_group == gid);
                    ops.types.type_defs.insert(tid_a, b_def);
                    ops.types.type_defs.insert(tid_b, a_def);
                    log::debug!("Reordered types {tid_a:?} and {tid_b:?} in rec group {gid:?}");
                }

                Ok(())
            })?;
        }
        Ok(())
    }

    // Define a mutation that moves a struct type from one (rec ...) group to another.
    // It will be a different rec group with high probability but it may try
    // to move it to the same rec group.
    fn move_struct_type_between_rec_groups(
        &mut self,
        c: &mut Candidates<'_>,
        ops: &mut GcOps,
    ) -> mutatis::Result<()> {
        if !ops.types.type_defs.is_empty() && ops.types.rec_groups.len() >= 2 {
            c.mutation(|ctx| {
                let tid = ctx
                    .rng()
                    .choose(ops.types.type_defs.keys())
                    .copied()
                    .expect("type_defs not empty");
                let new_gid = ctx
                    .rng()
                    .choose(&ops.types.rec_groups)
                    .copied()
                    .expect("rec_groups not empty");
                let old_gid = ops.types.type_defs.get(&tid).unwrap().rec_group;
                ops.types.type_defs.get_mut(&tid).unwrap().rec_group = new_gid;
                log::debug!("Moved type {tid:?} from rec group {old_gid:?} to {new_gid:?}");
                Ok(())
            })?;
        }
        Ok(())
    }

    // Define a mutation that duplicates a (rec ...) group.
    fn duplicate_rec_group(
        &mut self,
        c: &mut Candidates<'_>,
        ops: &mut GcOps,
    ) -> mutatis::Result<()> {
        if !c.shrink()
            && !ops.types.rec_groups.is_empty()
            && ops.types.rec_groups.len() < ops.limits.max_rec_groups as usize
            && ops.types.type_defs.len() < ops.limits.max_types as usize
        {
            c.mutation(|ctx| {
                let source_gid = ctx
                    .rng()
                    .choose(&ops.types.rec_groups)
                    .copied()
                    .expect("rec_groups not empty");

                // Create a new rec group.
                let new_gid = ops.types.fresh_rec_group_id(ctx.rng());
                ops.types.insert_rec_group(new_gid);

                let count = ops
                    .types
                    .type_defs
                    .values()
                    .filter(|def| def.rec_group == source_gid)
                    .count();

                // Skip empty rec groups.
                if count == 0 {
                    return Ok(());
                }

                // Since our structs are empty, we can just insert them into the new rec group.
                // We will update mutators while adding new features to the fuzzer.
                for _ in 0..count {
                    ops.types
                        .insert_empty_struct(ops.types.fresh_type_id(ctx.rng()), new_gid);
                }

                log::debug!(
                    "Duplicated rec group {source_gid:?} as new group {new_gid:?} ({count} types)"
                );
                Ok(())
            })?;
        }
        Ok(())
    }

    // Define a mutation that removes a whole (rec ...) group.
    fn remove_rec_group(&mut self, c: &mut Candidates<'_>, ops: &mut GcOps) -> mutatis::Result<()> {
        if ops.types.rec_groups.len() > 2 {
            c.mutation(|ctx| {
                let gid = ctx
                    .rng()
                    .choose(&ops.types.rec_groups)
                    .copied()
                    .expect("rec_groups not empty");

                ops.types.type_defs.retain(|_, def| def.rec_group != gid);
                ops.types.rec_groups.remove(&gid);

                log::debug!("Removed rec group {gid:?} and its member types");
                Ok(())
            })?;
        }
        Ok(())
    }

    // Define a mutation that merges two (rec ...) groups.
    fn merge_rec_groups(&mut self, c: &mut Candidates<'_>, ops: &mut GcOps) -> mutatis::Result<()> {
        if !ops.types.rec_groups.is_empty() && ops.types.rec_groups.len() > 2 {
            c.mutation(|ctx| {
                let dst_gid = ctx
                    .rng()
                    .choose(&ops.types.rec_groups)
                    .copied()
                    .expect("rec_groups not empty");

                let mut src_gid = None;
                for _ in 0..16 {
                    let g = ctx
                        .rng()
                        .choose(&ops.types.rec_groups)
                        .copied()
                        .expect("rec_groups not empty");

                    if g != dst_gid {
                        src_gid = Some(g);
                        break;
                    }
                }

                let Some(src_gid) = src_gid else {
                    // Could not find a distinct group (should be very unlikely with len>2).
                    return Ok(());
                };

                // Collect all members of src_gid.
                let mut members: SmallVec<[TypeId; 32]> = SmallVec::new();
                for (tid, def) in ops.types.type_defs.iter() {
                    if def.rec_group == src_gid {
                        members.push(*tid);
                    }
                }

                // Move all types from src_gid into dst_gid.
                for tid in members {
                    if let Some(def) = ops.types.type_defs.get_mut(&tid) {
                        def.rec_group = dst_gid;
                    }
                }

                // Remove the now-merged-away group id.
                ops.types.rec_groups.remove(&src_gid);
                log::debug!("Merged rec group {src_gid:?} into {dst_gid:?}");

                Ok(())
            })?;
        }
        Ok(())
    }

    // Define a mutation that splits a (rec ...) group in two, if possible.
    fn split_rec_group(&mut self, c: &mut Candidates<'_>, ops: &mut GcOps) -> mutatis::Result<()> {
        if !c.shrink()
            && ops.types.rec_groups.len() < ops.limits.max_rec_groups as usize
            && ops.types.type_defs.len() >= 2
        {
            c.mutation(|ctx| {
                // Pick a rec group with at least 2 members.
                let mut old_gid = None;
                let mut members: SmallVec<[TypeId; 32]> = SmallVec::new();

                for _ in 0..16 {
                    let gid = ctx
                        .rng()
                        .choose(&ops.types.rec_groups)
                        .copied()
                        .expect("rec_groups not empty");

                    members.clear();
                    for (tid, def) in ops.types.type_defs.iter() {
                        if def.rec_group == gid {
                            members.push(*tid);
                        }
                    }

                    if members.len() >= 2 {
                        old_gid = Some(gid);
                        break;
                    }
                }

                let Some(old_gid) = old_gid else {
                    return Ok(());
                };

                // Create a new rec group.
                let new_gid = ops.types.fresh_rec_group_id(ctx.rng());
                ops.types.insert_rec_group(new_gid);

                // Choose k in [1, len-1] (so both groups remain non-empty).
                let len = members.len();
                let Some(k_minus_1) = ctx.rng().gen_index(len - 1) else {
                    return Ok(());
                };
                let k = k_minus_1 + 1;

                // Move k distinct members by removing them from `members` as we pick.
                for _ in 0..k {
                    let Some(i) = ctx.rng().gen_index(members.len()) else {
                        break;
                    };
                    let tid = members.remove(i);
                    if let Some(def) = ops.types.type_defs.get_mut(&tid) {
                        def.rec_group = new_gid;
                    }
                }

                log::debug!("Split rec group {old_gid:?}: moved {k} of {len} members into new group {new_gid:?}");
                Ok(())
            })?;
        }
        Ok(())
    }
}

impl Mutate<GcOps> for GcOpsMutator {
    fn mutate(&mut self, c: &mut Candidates<'_>, ops: &mut GcOps) -> mutatis::Result<()> {
        self.add_operation(c, ops)?;
        self.remove_operation(c, ops)?;
        self.add_new_struct_type_to_rec_group(c, ops)?;
        self.remove_struct_type_from_rec_group(c, ops)?;
        self.move_struct_type_within_rec_group(c, ops)?;
        self.move_struct_type_between_rec_groups(c, ops)?;
        self.duplicate_rec_group(c, ops)?;
        self.remove_rec_group(c, ops)?;
        self.merge_rec_groups(c, ops)?;
        self.split_rec_group(c, ops)?;

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
