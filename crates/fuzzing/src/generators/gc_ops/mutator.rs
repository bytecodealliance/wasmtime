//! Mutators for the `gc` operations.

use crate::generators::gc_ops::ops::{GcOp, GcOps};
use crate::generators::gc_ops::types::{RecGroupId, TypeId};
use mutatis::{Candidates, Context, DefaultMutate, Generate, Mutate, Result as MutResult};
use smallvec::SmallVec;

/// A mutator for the gc ops
#[derive(Debug)]
pub struct GcOpsMutator;

impl Mutate<GcOps> for GcOpsMutator {
    fn mutate(&mut self, c: &mut Candidates<'_>, ops: &mut GcOps) -> mutatis::Result<()> {
        // Define a mutation that adds an operation to the ops list
        if !c.shrink() {
            c.mutation(|ctx| {
                if let Some(idx) = ctx.rng().gen_index(ops.ops.len() + 1) {
                    let op = GcOp::generate(ctx)?;
                    ops.ops.insert(idx, op);
                    log::debug!("Added operation {:?} to ops list", op);
                }
                Ok(())
            })?;
        }

        // Define a mutation that removes an operation from the ops list.
        if !ops.ops.is_empty() {
            c.mutation(|ctx| {
                let idx = ctx.rng().gen_index(ops.ops.len()).expect("ops not empty");
                let removed = ops.ops.remove(idx);
                log::debug!("Removed operation {:?} from ops list", removed);
                Ok(())
            })?;
        }

        // Define a mutation that adds an empty struct type to an existing (rec ...) group.
        if !c.shrink()
            && !ops.types.rec_groups.is_empty()
            && ops.types.type_defs.len() < ops.limits.max_types as usize
        {
            c.mutation(|ctx| {
                // Pick a random rec group.
                if let Some(group_id) = ctx.rng().choose(&ops.types.rec_groups).copied() {
                    // Get the next available type id.
                    let new_raw_type_id = ops
                        .types
                        .type_defs
                        .keys()
                        .next_back()
                        .map(|id| id.0)
                        .unwrap_or(0)
                        .saturating_add(1);

                    // Insert new struct with new type id to the chosen rec group.
                    let new_tid = TypeId(new_raw_type_id);
                    ops.types.insert_empty_struct(new_tid, group_id);
                    log::debug!(
                        "Added empty struct type {:?} to rec group {:?}",
                        new_tid,
                        group_id
                    );
                }
                Ok(())
            })?;
        }

        // Define a mutation that removes a struct type from an existing (rec ...).
        // It may result in empty rec groups. Empty rec groups are allowed.
        if !ops.types.type_defs.is_empty() {
            c.mutation(|ctx| {
                // Pick a random struct type
                if let Some(tid) = ctx.rng().choose(ops.types.type_defs.keys()).copied() {
                    // Remove the chosen struct type.
                    ops.types.type_defs.remove(&tid);
                    log::debug!("Removed struct type {:?}", tid);
                }
                Ok(())
            })?;
        }

        // Define a mutation that moves a struct type from one (rec ...) group to another.
        // It will be a different rec group with high probability but it may try
        // to move it to the same rec group.
        if !ops.types.type_defs.is_empty() && ops.types.rec_groups.len() >= 2 {
            c.mutation(|ctx| {
                // Pick a random type
                if let Some(tid) = ctx.rng().choose(ops.types.type_defs.keys()).copied() {
                    // Pick a random recursive group.
                    if let Some(new_gid) = ctx.rng().choose(&ops.types.rec_groups).copied() {
                        // Get the old rec group for debug purposes.
                        let old_gid = ops.types.type_defs.get(&tid).unwrap().rec_group;
                        // Move the struct type to the new rec group.
                        ops.types.type_defs.get_mut(&tid).unwrap().rec_group = new_gid;
                        log::debug!(
                            "Moved type {:?} from rec group {:?} to {:?}",
                            tid,
                            old_gid,
                            new_gid
                        );
                    }
                }
                Ok(())
            })?;
        }

        // Define a mutation that moves a struct type within an existing rec group.
        if !ops.types.rec_groups.is_empty() && ops.types.type_defs.len() >= 2 {
            c.mutation(|ctx| {
                let mut chosen: Option<(RecGroupId, TypeId, TypeId)> = None;

                // Randomly choose a rec group.
                for _ in 0..ops.limits.max_rec_groups {
                    let gid = match ctx.rng().choose(&ops.types.rec_groups).copied() {
                        Some(g) => g,
                        None => return Ok(()),
                    };

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
                    log::debug!(
                        "Reordered types {:?} and {:?} in rec group {:?}",
                        tid_a,
                        tid_b,
                        gid
                    );
                }

                Ok(())
            })?;
        }

        // Define a mutation that duplicates a (rec ...) group.
        if !c.shrink()
            && !ops.types.rec_groups.is_empty()
            && ops.types.rec_groups.len() < ops.limits.max_rec_groups as usize
            && ops.types.type_defs.len() < ops.limits.max_types as usize
        {
            c.mutation(|ctx| {
                // Pick a random rec group to duplicate.
                let Some(source_gid) = ctx.rng().choose(&ops.types.rec_groups).copied() else {
                    return Ok(());
                };

                // Create a new rec group.
                let new_gid_raw = ops
                    .types
                    .rec_groups
                    .iter()
                    .next_back()
                    .map(|id| id.0)
                    .unwrap_or(0)
                    .saturating_add(1);
                let new_gid = RecGroupId(new_gid_raw);
                ops.types.insert_rec_group(new_gid);

                let mut next_type_id_raw = ops
                    .types
                    .type_defs
                    .keys()
                    .next_back()
                    .map(|id| id.0)
                    .unwrap_or(0)
                    .saturating_add(1);

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
                        .insert_empty_struct(TypeId(next_type_id_raw), new_gid);
                    next_type_id_raw = next_type_id_raw.saturating_add(1);
                }

                log::debug!(
                    "Duplicated rec group {:?} as new group {:?} ({} types)",
                    source_gid,
                    new_gid,
                    count
                );
                Ok(())
            })?;
        }

        // Define a mutation that removes a whole (rec ...) group.
        if ops.types.rec_groups.len() > 2 {
            c.mutation(|ctx| {
                // Pick a random rec group to remove.
                let Some(gid) = ctx.rng().choose(&ops.types.rec_groups).copied() else {
                    return Ok(());
                };

                ops.types.type_defs.retain(|_, def| def.rec_group != gid);
                ops.types.rec_groups.remove(&gid);

                log::debug!("Removed rec group {:?} and its member types", gid);
                Ok(())
            })?;
        }

        // Define a mutation that merges two (rec ...) groups.
        if !ops.types.rec_groups.is_empty() && ops.types.rec_groups.len() > 2 {
            c.mutation(|ctx| {
                // Pick two distinct rec groups.
                let Some(dst_gid) = ctx.rng().choose(&ops.types.rec_groups).copied() else {
                    return Ok(());
                };

                // Try a few times to pick a different source group.
                let mut src_gid = None;
                for _ in 0..16 {
                    let Some(g) = ctx.rng().choose(&ops.types.rec_groups).copied() else {
                        return Ok(());
                    };
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
                log::debug!("Merged rec group {:?} into {:?}", src_gid, dst_gid);

                Ok(())
            })?;
        }

        // Define a mutation that splits a (rec ...) group in two, if possible.
        if !c.shrink()
            && ops.types.rec_groups.len() < ops.limits.max_rec_groups as usize
            && ops.types.type_defs.len() >= 2
        {
            c.mutation(|ctx| {
                // Pick a rec group with at least 2 members.
                let mut old_gid = None;
                let mut members: SmallVec<[TypeId; 32]> = SmallVec::new();

                for _ in 0..16 {
                    let Some(gid) = ctx.rng().choose(&ops.types.rec_groups).copied() else {
                        return Ok(());
                    };

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
                let new_gid_raw = ops
                    .types
                    .rec_groups
                    .iter()
                    .next_back()
                    .map(|g| g.0)
                    .unwrap_or(0)
                    .saturating_add(1);
                let new_gid = RecGroupId(new_gid_raw);
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

                log::debug!(
                    "Split rec group {:?}: moved {} of {} members into new group {:?}",
                    old_gid,
                    k,
                    len,
                    new_gid
                );
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
