//! Mutators for the `gc` operations.
use crate::generators::gc_ops::limits::GcOpsLimits;
use crate::generators::gc_ops::ops::{GcOp, GcOps};
use crate::generators::gc_ops::types::{TypeId, Types};
use mutatis::{
    Candidates, Context, DefaultMutate, Generate, Mutate, Result as MutResult, mutators as m,
};
use smallvec::SmallVec;
use std::collections::BTreeMap;

/// Mutator for [`Types`]: handles adding/removing types and all rec-group
/// structural mutations (duplicate, remove, merge, split, move).
#[derive(Debug, Default)]
pub struct TypesMutator;

impl TypesMutator {
    /// Add an empty struct type to a random existing rec group.
    fn add_struct(
        &mut self,
        c: &mut Candidates<'_>,
        types: &mut Types,
        limits: &GcOpsLimits,
    ) -> mutatis::Result<()> {
        if c.shrink()
            || types.rec_groups.is_empty()
            || types.type_defs.len()
                >= usize::try_from(limits.max_types).expect("max_types is too large")
        {
            return Ok(());
        }
        c.mutation(|ctx| {
            let Some(gid) = ctx.rng().choose(types.rec_groups.keys()).copied() else {
                return Ok(());
            };
            let tid = types.fresh_type_id(ctx.rng());
            let is_final = (ctx.rng().gen_u32() % 4) == 0;
            let supertype = if types.type_defs.is_empty() {
                None
            } else {
                ctx.rng().choose(types.type_defs.keys()).copied()
            };
            types.insert_empty_struct(tid, gid, is_final, supertype);
            log::debug!("Added empty struct {tid:?} to rec group {gid:?}");
            Ok(())
        })?;
        Ok(())
    }

    /// Remove a random type from its rec group.
    fn remove_struct(&mut self, c: &mut Candidates<'_>, types: &mut Types) -> mutatis::Result<()> {
        if types.type_defs.is_empty() {
            return Ok(());
        }
        c.mutation(|ctx| {
            let Some(tid) = ctx.rng().choose(types.type_defs.keys()).copied() else {
                return Ok(());
            };
            types.remove_type(tid);
            log::debug!("Removed struct type {tid:?}");
            Ok(())
        })?;
        Ok(())
    }

    /// Swap two random types within a rec group.
    fn swap_within_group(
        &mut self,
        c: &mut Candidates<'_>,
        types: &mut Types,
    ) -> mutatis::Result<()> {
        if types.rec_groups.is_empty() || types.type_defs.len() <= 2 {
            return Ok(());
        }
        c.mutation(|ctx| {
            for _ in 0..16 {
                let Some(gid) = ctx.rng().choose(types.rec_groups.keys()).copied() else {
                    return Ok(());
                };

                let Some(member_set) = types.rec_groups.get(&gid) else {
                    continue;
                };
                let members: SmallVec<[TypeId; 32]> = member_set.iter().copied().collect();
                if members.len() < 2 {
                    continue;
                }

                let Some(tid_a) = ctx.rng().choose(&members).copied() else {
                    return Ok(());
                };
                let Some(mut tid_b) = ctx.rng().choose(&members).copied() else {
                    return Ok(());
                };
                for _ in 0..members.len() {
                    if tid_a != tid_b {
                        break;
                    }
                    let Some(next_tid) = ctx.rng().choose(&members).copied() else {
                        return Ok(());
                    };
                    tid_b = next_tid;
                }
                if tid_a == tid_b {
                    continue;
                }

                let Some(a_def) = types.type_defs.remove(&tid_a) else {
                    return Ok(());
                };
                let Some(b_def) = types.type_defs.remove(&tid_b) else {
                    types.type_defs.insert(tid_a, a_def);
                    return Ok(());
                };
                types.type_defs.insert(tid_a, b_def);
                types.type_defs.insert(tid_b, a_def);
                log::debug!("Swapped types {tid_a:?} and {tid_b:?} in rec group {gid:?}");
                return Ok(());
            }
            Ok(())
        })?;
        Ok(())
    }

    /// Move a random type from one rec group to another.
    fn move_between_groups(
        &mut self,
        c: &mut Candidates<'_>,
        types: &mut Types,
    ) -> mutatis::Result<()> {
        if types.rec_groups.len() < 2 {
            return Ok(());
        }
        c.mutation(|ctx| {
            let mut old_gid = None;
            for _ in 0..16 {
                let Some(gid) = ctx.rng().choose(types.rec_groups.keys()).copied() else {
                    return Ok(());
                };
                if types
                    .rec_groups
                    .get(&gid)
                    .map(|s| !s.is_empty())
                    .unwrap_or(false)
                {
                    old_gid = Some(gid);
                    break;
                }
            }
            let Some(old_gid) = old_gid else {
                return Ok(());
            };

            let Some(tid) = types
                .rec_groups
                .get(&old_gid)
                .and_then(|members| ctx.rng().choose(members.iter()).copied())
            else {
                return Ok(());
            };

            let Some(new_gid) = ctx.rng().choose(types.rec_groups.keys()).copied() else {
                return Ok(());
            };

            let Some(old_members) = types.rec_groups.get_mut(&old_gid) else {
                return Ok(());
            };
            old_members.remove(&tid);
            let Some(new_members) = types.rec_groups.get_mut(&new_gid) else {
                return Ok(());
            };
            new_members.insert(tid);
            log::debug!("Moved type {tid:?} from {old_gid:?} to {new_gid:?}");
            Ok(())
        })?;
        Ok(())
    }

    /// Duplicate a rec group (copy its structure with fresh type ids).
    fn duplicate_group(
        &mut self,
        c: &mut Candidates<'_>,
        types: &mut Types,
        limits: &GcOpsLimits,
    ) -> mutatis::Result<()> {
        if c.shrink()
            || types.rec_groups.is_empty()
            || types.rec_groups.len()
                >= usize::try_from(limits.max_rec_groups).expect("max_rec_groups is too large")
            || types.type_defs.len()
                >= usize::try_from(limits.max_types).expect("max_types is too large")
        {
            return Ok(());
        }
        c.mutation(|ctx| {
            let Some(src_gid) = ctx.rng().choose(types.rec_groups.keys()).copied() else {
                return Ok(());
            };
            let Some(src_members) = types.rec_groups.get(&src_gid) else {
                return Ok(());
            };
            if src_members.is_empty() {
                return Ok(());
            }

            // Collect (TypeId, is_final, supertype) for members of the source group.
            let members: SmallVec<[(TypeId, bool, Option<TypeId>); 32]> = src_members
                .iter()
                .filter_map(|tid| {
                    types
                        .type_defs
                        .get(tid)
                        .map(|def| (*tid, def.is_final, def.supertype))
                })
                .collect();

            if members.is_empty() {
                return Ok(());
            }

            // Create a new rec group.
            let new_gid = types.fresh_rec_group_id(ctx.rng());
            types.insert_rec_group(new_gid);

            // Allocate fresh type ids for each member and build old-to-new map.
            let mut old_to_new: BTreeMap<TypeId, TypeId> = BTreeMap::new();
            for (old_tid, _, _) in &members {
                old_to_new.insert(*old_tid, types.fresh_type_id(ctx.rng()));
            }

            // Insert duplicated defs, rewriting intra-group supertype edges to cloned ids.
            for (old_tid, is_final, supertype) in &members {
                let new_tid = old_to_new[old_tid];
                let mapped_super = supertype.map(|st| *old_to_new.get(&st).unwrap_or(&st));
                types.insert_empty_struct(new_tid, new_gid, *is_final, mapped_super);
            }

            log::debug!(
                "Duplicated rec group {src_gid:?} as {new_gid:?} ({count} types)",
                count = members.len()
            );
            Ok(())
        })?;
        Ok(())
    }

    /// Remove a whole rec group and all its member types.
    fn remove_group(&mut self, c: &mut Candidates<'_>, types: &mut Types) -> mutatis::Result<()> {
        if types.rec_groups.len() <= 2 {
            return Ok(());
        }
        c.mutation(|ctx| {
            let Some(gid) = ctx.rng().choose(types.rec_groups.keys()).copied() else {
                return Ok(());
            };
            let Some(members) = types.rec_groups.remove(&gid) else {
                return Ok(());
            };
            for tid in &members {
                types.type_defs.remove(tid);
            }
            log::debug!(
                "Removed rec group {gid:?} and its {len} member types",
                len = members.len()
            );
            Ok(())
        })?;
        Ok(())
    }

    /// Merge two rec groups into one.
    fn merge_groups(&mut self, c: &mut Candidates<'_>, types: &mut Types) -> mutatis::Result<()> {
        if types.rec_groups.len() <= 2 {
            return Ok(());
        }
        c.mutation(|ctx| {
            let Some(dst_gid) = ctx.rng().choose(types.rec_groups.keys()).copied() else {
                return Ok(());
            };

            // Find a distinct source group.
            let mut src_gid = None;
            for _ in 0..16 {
                let Some(g) = ctx.rng().choose(types.rec_groups.keys()).copied() else {
                    return Ok(());
                };
                if g != dst_gid {
                    src_gid = Some(g);
                    break;
                }
            }
            let Some(src_gid) = src_gid else {
                return Ok(());
            };

            // Move all members from src into dst.
            let Some(src_members) = types.rec_groups.remove(&src_gid) else {
                return Ok(());
            };
            let Some(dst_members) = types.rec_groups.get_mut(&dst_gid) else {
                return Ok(());
            };
            dst_members.extend(src_members.iter());
            log::debug!("Merged rec group {src_gid:?} into {dst_gid:?}");
            Ok(())
        })?;
        Ok(())
    }

    /// Split a rec group into two.
    fn split_group(
        &mut self,
        c: &mut Candidates<'_>,
        types: &mut Types,
        limits: &GcOpsLimits,
    ) -> mutatis::Result<()> {
        if c.shrink()
            || types.rec_groups.is_empty()
            || types.type_defs.len() < 2
            || types.rec_groups.len()
                >= usize::try_from(limits.max_rec_groups).expect("max_rec_groups is too large")
        {
            return Ok(());
        }
        c.mutation(|ctx| {
            // Find a group with >= 2 members.
            let mut old_gid = None;
            for _ in 0..16 {
                let Some(gid) = ctx.rng().choose(types.rec_groups.keys()).copied() else {
                    return Ok(());
                };
                if types.rec_groups.get(&gid).map(|s| s.len()).unwrap_or(0) >= 2 {
                    old_gid = Some(gid);
                    break;
                }
            }
            let Some(old_gid) = old_gid else {
                return Ok(());
            };

            let new_gid = types.fresh_rec_group_id(ctx.rng());
            types.insert_rec_group(new_gid);

            // Collect members so we can pick from them.
            let Some(old_members) = types.rec_groups.get(&old_gid) else {
                return Ok(());
            };
            let mut members: SmallVec<[TypeId; 32]> = old_members.iter().copied().collect();
            let len = members.len();

            // Choose k in [1, len-1] so both groups stay non-empty.
            let Some(k_minus_1) = ctx.rng().gen_index(len - 1) else {
                return Ok(());
            };
            let k = k_minus_1 + 1;

            for _ in 0..k {
                let Some(i) = ctx.rng().gen_index(members.len()) else {
                    break;
                };
                let tid = members.remove(i);
                let Some(old_members) = types.rec_groups.get_mut(&old_gid) else {
                    return Ok(());
                };
                old_members.remove(&tid);
                let Some(new_members) = types.rec_groups.get_mut(&new_gid) else {
                    return Ok(());
                };
                new_members.insert(tid);
            }

            log::debug!("Split rec group {old_gid:?}: moved {k} of {len} members into {new_gid:?}");
            Ok(())
        })?;
        Ok(())
    }

    /// Run all type / rec-group mutations. [`GcOpsLimits`] come from [`GcOps`].
    fn mutate_with_limits(
        &mut self,
        c: &mut Candidates<'_>,
        types: &mut Types,
        limits: &GcOpsLimits,
    ) -> mutatis::Result<()> {
        self.add_struct(c, types, limits)?;
        self.remove_struct(c, types)?;
        self.swap_within_group(c, types)?;
        self.move_between_groups(c, types)?;
        self.duplicate_group(c, types, limits)?;
        self.remove_group(c, types)?;
        self.merge_groups(c, types)?;
        self.split_group(c, types, limits)?;
        Ok(())
    }
}

/// Mutator for [`GcOps`].
///
/// Also implements [`Mutate`] / [`Generate`] for [`GcOp`] so `m::vec` can mutate
/// `Vec<GcOp>` without a second struct.
#[derive(Debug, Default)]
pub struct GcOpsMutator {
    types_mutator: TypesMutator,
}

impl Mutate<GcOp> for GcOpsMutator {
    fn mutate(&mut self, c: &mut Candidates<'_>, value: &mut GcOp) -> MutResult<()> {
        c.mutation(|ctx| {
            *value = GcOp::generate(ctx)?;
            Ok(())
        })?;
        Ok(())
    }
}

impl Generate<GcOp> for GcOpsMutator {
    fn generate(&mut self, context: &mut Context) -> MutResult<GcOp> {
        GcOp::generate(context)
    }
}

impl DefaultMutate for GcOp {
    type DefaultMutate = GcOpsMutator;
}

impl Mutate<GcOps> for GcOpsMutator {
    fn mutate(&mut self, c: &mut Candidates<'_>, ops: &mut GcOps) -> mutatis::Result<()> {
        m::vec(GcOpsMutator::default()).mutate(c, &mut ops.ops)?;
        self.types_mutator
            .mutate_with_limits(c, &mut ops.types, &ops.limits)?;
        Ok(())
    }
}

impl DefaultMutate for GcOps {
    type DefaultMutate = GcOpsMutator;
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
