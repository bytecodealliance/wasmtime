//! Data structure representing the live range of an SSA value.
//!
//! Live ranges are tracked per SSA value, not per variable or virtual register. The live range of
//! an SSA value begins where it is defined and extends to all program points where the value is
//! still needed.
//!
//! # Local Live Ranges
//!
//! Inside a single extended basic block, the live range of a value is always an interval between
//! two program points (if the value is live in the EBB at all). The starting point is either:
//!
//! 1. The instruction that defines the value, or
//! 2. The EBB header, because the value is an argument to the EBB, or
//! 3. The EBB header, because the value is defined in another EBB and live-in to this one.
//!
//! The ending point of the local live range is the last of the following program points in the
//! EBB:
//!
//! 1. The last use in the EBB, where a *use* is an instruction that has the value as an argument.
//! 2. The last branch or jump instruction in the EBB that can reach a use.
//! 3. If the value has no uses anywhere (a *dead value*), the program point that defines it.
//!
//! Note that 2. includes loop back-edges to the same EBB. In general, if a value is defined
//! outside a loop and used inside the loop, it will be live in the entire loop.
//!
//! # Global Live Ranges
//!
//! Values that appear in more than one EBB have a *global live range* which can be seen as the
//! disjoint union of the per-EBB local intervals for all of the EBBs where the value is live.
//! Together with a `ProgramOrder` which provides a linear ordering of the EBBs, the global live
//! range becomes a linear sequence of disjoint intervals, at most one per EBB.
//!
//! In the special case of a dead value, the global live range is a single interval where the start
//! and end points are the same. The global live range of a value is never completely empty.
//!
//! # Register interference
//!
//! The register allocator uses live ranges to determine if values *interfere*, which means that
//! they can't be stored in the same register. Two live ranges interfere if and only if any of
//! their intervals overlap.
//!
//! If one live range ends at an instruction that defines another live range, those two live ranges
//! are not considered to interfere. This is because most ISAs allow instructions to reuse an input
//! register for an output value. If Cranelift gets support for inline assembly, we will need to
//! handle *early clobbers* which are output registers that are not allowed to alias any input
//! registers.
//!
//! If `i1 < i2 < i3` are program points, we have:
//!
//! - `i1-i2` and `i1-i3` interfere because the intervals overlap.
//! - `i1-i2` and `i2-i3` don't interfere.
//! - `i1-i3` and `i2-i2` do interfere because the dead def would clobber the register.
//! - `i1-i2` and `i2-i2` don't interfere.
//! - `i2-i3` and `i2-i2` do interfere.
//!
//! Because of this behavior around interval end points, live range interference is not completely
//! equivalent to mathematical intersection of open or half-open intervals.
//!
//! # Implementation notes
//!
//! A few notes about the implementation of this data structure. This should not concern someone
//! only looking to use the public interface.
//!
//! ## EBB ordering
//!
//! The relative order of EBBs is used to maintain a sorted list of live-in intervals and to
//! coalesce adjacent live-in intervals when the prior interval covers the whole EBB. This doesn't
//! depend on any property of the program order, so alternative orderings are possible:
//!
//! 1. The EBB layout order. This is what we currently use.
//! 2. A topological order of the dominator tree. All the live-in intervals would come after the
//!    def interval.
//! 3. A numerical order by EBB number. Performant because it doesn't need to indirect through the
//!    `ProgramOrder` for comparisons.
//!
//! These orderings will cause small differences in coalescing opportunities, but all of them would
//! do a decent job of compressing a long live range. The numerical order might be preferable
//! because:
//!
//! - It has better performance because EBB numbers can be compared directly without any table
//!   lookups.
//! - If EBB numbers are not reused, it is safe to allocate new EBBs without getting spurious
//!   live-in intervals from any coalesced representations that happen to cross a new EBB.
//!
//! For comparing instructions, the layout order is always what we want.
//!
//! ## Alternative representation
//!
//! Since a local live-in interval always begins at its EBB header, it is uniquely described by its
//! end point instruction alone. We can use the layout to look up the EBB containing the end point.
//! This means that a sorted `Vec<Inst>` would be enough to represent the set of live-in intervals.
//!
//! Coalescing is an important compression technique because some live ranges can span thousands of
//! EBBs. We can represent that by switching to a sorted `Vec<ProgramPoint>` representation where
//! an `[Ebb, Inst]` pair represents a coalesced range, while an `Inst` entry without a preceding
//! `Ebb` entry represents a single live-in interval.
//!
//! This representation is more compact for a live range with many uncoalesced live-in intervals.
//! It is more complicated to work with, though, so it is probably not worth it. The performance
//! benefits of switching to a numerical EBB order only appears if the binary search is doing
//! EBB-EBB comparisons.
//!
//! ## B-tree representation
//!
//! A `BTreeMap<Ebb, Inst>` could also be used for the live-in intervals. It looks like the
//! standard library B-tree doesn't provide the necessary interface for an efficient implementation
//! of coalescing, so we would need to roll our own.
//!

use bforest;
use entity::SparseMapValue;
use ir::{Ebb, ExpandedProgramPoint, Inst, Layout, ProgramOrder, ProgramPoint, Value};
use regalloc::affinity::Affinity;
use std::cmp::Ordering;
use std::marker::PhantomData;

/// Global live range of a single SSA value.
///
/// As [explained in the module documentation](index.html#local-live-ranges), the live range of an
/// SSA value is the disjoint union of a set of intervals, each local to a single EBB, and with at
/// most one interval per EBB. We further distinguish between:
///
/// 1. The *def interval* is the local interval in the EBB where the value is defined, and
/// 2. The *live-in intervals* are the local intervals in the remaining EBBs.
///
/// A live-in interval always begins at the EBB header, while the def interval can begin at the
/// defining instruction, or at the EBB header for an EBB argument value.
///
/// All values have a def interval, but a large proportion of values don't have any live-in
/// intervals. These are called *local live ranges*.
///
/// # Program order requirements
///
/// The internal representation of a `LiveRange` depends on a consistent `ProgramOrder` both for
/// ordering instructions inside an EBB *and* for ordering EBBs. The methods that depend on the
/// ordering take an explicit `ProgramOrder` object, and it is the caller's responsibility to
/// ensure that the provided ordering is consistent between calls.
///
/// In particular, changing the order of EBBs or inserting new EBBs will invalidate live ranges.
///
/// Inserting new instructions in the layout is safe, but removing instructions is not. Besides the
/// instructions using or defining their value, `LiveRange` structs can contain references to
/// branch and jump instructions.
pub type LiveRange = GenLiveRange<Layout>;

/// Generic live range implementation.
///
/// The intended generic parameter is `PO=Layout`, but tests are simpler with a mock order.
/// Use `LiveRange` instead of using this generic directly.
pub struct GenLiveRange<PO: ProgramOrder> {
    /// The value described by this live range.
    /// This member can't be modified in case the live range is stored in a `SparseMap`.
    value: Value,

    /// The preferred register allocation for this value.
    pub affinity: Affinity,

    /// The instruction or EBB header where this value is defined.
    def_begin: ProgramPoint,

    /// The end point of the def interval. This must always belong to the same EBB as `def_begin`.
    ///
    /// We always have `def_begin <= def_end` with equality implying a dead def live range with no
    /// uses.
    def_end: ProgramPoint,

    /// Additional live-in intervals sorted in program order.
    ///
    /// This map is empty for most values which are only used in one EBB.
    ///
    /// A map entry `ebb -> inst` means that the live range is live-in to `ebb`, continuing up to
    /// `inst` which may belong to a later EBB in the program order.
    ///
    /// The entries are non-overlapping, and none of them overlap the EBB where the value is
    /// defined.
    liveins: bforest::Map<Ebb, Inst>,

    po: PhantomData<*const PO>,
}

/// Context information needed to query a `LiveRange`.
pub struct LiveRangeContext<'a, PO: 'a + ProgramOrder> {
    /// Ordering of EBBs.
    pub order: &'a PO,
    /// Memory pool.
    pub forest: &'a bforest::MapForest<Ebb, Inst>,
}

impl<'a, PO: ProgramOrder> LiveRangeContext<'a, PO> {
    /// Make a new context.
    pub fn new(
        order: &'a PO,
        forest: &'a bforest::MapForest<Ebb, Inst>,
    ) -> LiveRangeContext<'a, PO> {
        LiveRangeContext { order, forest }
    }
}

impl<'a, PO: ProgramOrder> Clone for LiveRangeContext<'a, PO> {
    fn clone(&self) -> Self {
        LiveRangeContext {
            order: self.order,
            forest: self.forest,
        }
    }
}

impl<'a, PO: ProgramOrder> Copy for LiveRangeContext<'a, PO> {}

/// Forest of B-trees used for storing live ranges.
pub type LiveRangeForest = bforest::MapForest<Ebb, Inst>;

struct Cmp<'a, PO: ProgramOrder + 'a>(&'a PO);

impl<'a, PO: ProgramOrder> bforest::Comparator<Ebb> for Cmp<'a, PO> {
    fn cmp(&self, a: Ebb, b: Ebb) -> Ordering {
        self.0.cmp(a, b)
    }
}

impl<PO: ProgramOrder> GenLiveRange<PO> {
    /// Create a new live range for `value` defined at `def`.
    ///
    /// The live range will be created as dead, but it can be extended with `extend_in_ebb()`.
    pub fn new(value: Value, def: ProgramPoint, affinity: Affinity) -> Self {
        Self {
            value,
            affinity,
            def_begin: def,
            def_end: def,
            liveins: bforest::Map::new(),
            po: PhantomData,
        }
    }

    /// Extend the local interval for `ebb` so it reaches `to` which must belong to `ebb`.
    /// Create a live-in interval if necessary.
    ///
    /// If the live range already has a local interval in `ebb`, extend its end point so it
    /// includes `to`, and return false.
    ///
    /// If the live range did not previously have a local interval in `ebb`, add one so the value
    /// is live-in to `ebb`, extending to `to`. Return true.
    ///
    /// The return value can be used to detect if we just learned that the value is live-in to
    /// `ebb`. This can trigger recursive extensions in `ebb`'s CFG predecessor blocks.
    pub fn extend_in_ebb(
        &mut self,
        ebb: Ebb,
        to: Inst,
        order: &PO,
        forest: &mut bforest::MapForest<Ebb, Inst>,
    ) -> bool {
        // First check if we're extending the def interval.
        //
        // We're assuming here that `to` never precedes `def_begin` in the same EBB, but we can't
        // check it without a method for getting `to`'s EBB.
        if order.cmp(ebb, self.def_end) != Ordering::Greater
            && order.cmp(to, self.def_begin) != Ordering::Less
        {
            let to_pp = to.into();
            debug_assert_ne!(
                to_pp, self.def_begin,
                "Can't use value in the defining instruction."
            );
            if order.cmp(to, self.def_end) == Ordering::Greater {
                self.def_end = to_pp;
            }
            return false;
        }

        // Now check if we're extending any of the existing live-in intervals.
        let cmp = Cmp(order);
        let mut c = self.liveins.cursor(forest, &cmp);
        let first_time_livein;

        if let Some(end) = c.goto(ebb) {
            // There's an interval beginning at `ebb`. See if it extends.
            first_time_livein = false;
            if order.cmp(end, to) == Ordering::Less {
                *c.value_mut().unwrap() = to;
            } else {
                return first_time_livein;
            }
        } else if let Some((_, end)) = c.prev() {
            // There's no interval beginning at `ebb`, but we could still be live-in at `ebb` with
            // a coalesced interval that begins before and ends after.
            if order.cmp(end, ebb) == Ordering::Greater {
                // Yep, the previous interval overlaps `ebb`.
                first_time_livein = false;
                if order.cmp(end, to) == Ordering::Less {
                    *c.value_mut().unwrap() = to;
                } else {
                    return first_time_livein;
                }
            } else {
                first_time_livein = true;
                // The current interval does not overlap `ebb`, but it may still be possible to
                // coalesce with it.
                if order.is_ebb_gap(end, ebb) {
                    *c.value_mut().unwrap() = to;
                } else {
                    c.insert(ebb, to);
                }
            }
        } else {
            // There is no existing interval before `ebb`.
            first_time_livein = true;
            c.insert(ebb, to);
        }

        // Now `c` to left pointing at an interval that ends in `to`.
        debug_assert_eq!(c.value(), Some(to));

        // See if it can be coalesced with the following interval.
        if let Some((next_ebb, next_end)) = c.next() {
            if order.is_ebb_gap(to, next_ebb) {
                // Remove this interval and extend the previous end point to `next_end`.
                c.remove();
                c.prev();
                *c.value_mut().unwrap() = next_end;
            }
        }

        first_time_livein
    }

    /// Is this the live range of a dead value?
    ///
    /// A dead value has no uses, and its live range ends at the same program point where it is
    /// defined.
    pub fn is_dead(&self) -> bool {
        self.def_begin == self.def_end
    }

    /// Is this a local live range?
    ///
    /// A local live range is only used in the same EBB where it was defined. It is allowed to span
    /// multiple basic blocks within that EBB.
    pub fn is_local(&self) -> bool {
        self.liveins.is_empty()
    }

    /// Get the program point where this live range is defined.
    ///
    /// This will be an EBB header when the value is an EBB argument, otherwise it is the defining
    /// instruction.
    pub fn def(&self) -> ProgramPoint {
        self.def_begin
    }

    /// Move the definition of this value to a new program point.
    ///
    /// It is only valid to move the definition within the same EBB, and it can't be moved beyond
    /// `def_local_end()`.
    pub fn move_def_locally(&mut self, def: ProgramPoint) {
        self.def_begin = def;
    }

    /// Get the local end-point of this live range in the EBB where it is defined.
    ///
    /// This can be the EBB header itself in the case of a dead EBB argument.
    /// Otherwise, it will be the last local use or branch/jump that can reach a use.
    pub fn def_local_end(&self) -> ProgramPoint {
        self.def_end
    }

    /// Get the local end-point of this live range in an EBB where it is live-in.
    ///
    /// If this live range is not live-in to `ebb`, return `None`. Otherwise, return the end-point
    /// of this live range's local interval in `ebb`.
    ///
    /// If the live range is live through all of `ebb`, the terminator of `ebb` is a correct
    /// answer, but it is also possible that an even later program point is returned. So don't
    /// depend on the returned `Inst` to belong to `ebb`.
    pub fn livein_local_end(&self, ebb: Ebb, ctx: LiveRangeContext<PO>) -> Option<Inst> {
        let cmp = Cmp(ctx.order);
        self.liveins
            .get_or_less(ebb, ctx.forest, &cmp)
            .and_then(|(_, inst)| {
                // We have an entry that ends at `inst`.
                if ctx.order.cmp(inst, ebb) == Ordering::Greater {
                    Some(inst)
                } else {
                    None
                }
            })
    }

    /// Is this value live-in to `ebb`?
    ///
    /// An EBB argument is not considered to be live in.
    pub fn is_livein(&self, ebb: Ebb, ctx: LiveRangeContext<PO>) -> bool {
        self.livein_local_end(ebb, ctx).is_some()
    }

    /// Get all the live-in intervals.
    ///
    /// Note that the intervals are stored in a compressed form so each entry may span multiple
    /// EBBs where the value is live in.
    pub fn liveins<'a>(&'a self, ctx: LiveRangeContext<'a, PO>) -> bforest::MapIter<'a, Ebb, Inst> {
        self.liveins.iter(ctx.forest)
    }

    /// Check if this live range overlaps a definition in `ebb`.
    pub fn overlaps_def(
        &self,
        def: ExpandedProgramPoint,
        ebb: Ebb,
        ctx: LiveRangeContext<PO>,
    ) -> bool {
        // Two defs at the same program point always overlap, even if one is dead.
        if def == self.def_begin.into() {
            return true;
        }

        // Check for an overlap with the local range.
        if ctx.order.cmp(def, self.def_begin) != Ordering::Less
            && ctx.order.cmp(def, self.def_end) == Ordering::Less
        {
            return true;
        }

        // Check for an overlap with a live-in range.
        match self.livein_local_end(ebb, ctx) {
            Some(inst) => ctx.order.cmp(def, inst) == Ordering::Less,
            None => false,
        }
    }

    /// Check if this live range reaches a use at `user` in `ebb`.
    pub fn reaches_use(&self, user: Inst, ebb: Ebb, ctx: LiveRangeContext<PO>) -> bool {
        // Check for an overlap with the local range.
        if ctx.order.cmp(user, self.def_begin) == Ordering::Greater
            && ctx.order.cmp(user, self.def_end) != Ordering::Greater
        {
            return true;
        }

        // Check for an overlap with a live-in range.
        match self.livein_local_end(ebb, ctx) {
            Some(inst) => ctx.order.cmp(user, inst) != Ordering::Greater,
            None => false,
        }
    }

    /// Check if this live range is killed at `user` in `ebb`.
    pub fn killed_at(&self, user: Inst, ebb: Ebb, ctx: LiveRangeContext<PO>) -> bool {
        self.def_local_end() == user.into() || self.livein_local_end(ebb, ctx) == Some(user)
    }
}

/// Allow a `LiveRange` to be stored in a `SparseMap` indexed by values.
impl<PO: ProgramOrder> SparseMapValue<Value> for GenLiveRange<PO> {
    fn key(&self) -> Value {
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::{GenLiveRange, LiveRangeContext};
    use bforest;
    use entity::EntityRef;
    use ir::{Ebb, Inst, Value};
    use ir::{ExpandedProgramPoint, ProgramOrder};
    use std::cmp::Ordering;
    use std::vec::Vec;

    // Dummy program order which simply compares indexes.
    // It is assumed that EBBs have indexes that are multiples of 10, and instructions have indexes
    // in between. `is_ebb_gap` assumes that terminator instructions have indexes of the form
    // ebb * 10 + 1. This is used in the coalesce test.
    struct ProgOrder {}

    impl ProgramOrder for ProgOrder {
        fn cmp<A, B>(&self, a: A, b: B) -> Ordering
        where
            A: Into<ExpandedProgramPoint>,
            B: Into<ExpandedProgramPoint>,
        {
            fn idx(pp: ExpandedProgramPoint) -> usize {
                match pp {
                    ExpandedProgramPoint::Inst(i) => i.index(),
                    ExpandedProgramPoint::Ebb(e) => e.index(),
                }
            }

            let ia = idx(a.into());
            let ib = idx(b.into());
            ia.cmp(&ib)
        }

        fn is_ebb_gap(&self, inst: Inst, ebb: Ebb) -> bool {
            inst.index() % 10 == 1 && ebb.index() / 10 == inst.index() / 10 + 1
        }
    }

    impl ProgOrder {
        // Get the EBB corresponding to `inst`.
        fn inst_ebb(&self, inst: Inst) -> Ebb {
            let i = inst.index();
            Ebb::new(i - i % 10)
        }

        // Get the EBB of a program point.
        fn pp_ebb<PP: Into<ExpandedProgramPoint>>(&self, pp: PP) -> Ebb {
            match pp.into() {
                ExpandedProgramPoint::Inst(i) => self.inst_ebb(i),
                ExpandedProgramPoint::Ebb(e) => e,
            }
        }

        // Validate the live range invariants.
        fn validate(&self, lr: &GenLiveRange<ProgOrder>, forest: &bforest::MapForest<Ebb, Inst>) {
            // The def interval must cover a single EBB.
            let def_ebb = self.pp_ebb(lr.def_begin);
            assert_eq!(def_ebb, self.pp_ebb(lr.def_end));

            // Check that the def interval isn't backwards.
            match self.cmp(lr.def_begin, lr.def_end) {
                Ordering::Equal => assert!(lr.liveins.is_empty()),
                Ordering::Greater => {
                    panic!("Backwards def interval: {}-{}", lr.def_begin, lr.def_end)
                }
                Ordering::Less => {}
            }

            // Check the live-in intervals.
            let mut prev_end = None;
            for (begin, end) in lr.liveins.iter(forest) {
                assert_eq!(self.cmp(begin, end), Ordering::Less);
                if let Some(e) = prev_end {
                    assert_eq!(self.cmp(e, begin), Ordering::Less);
                }

                assert!(
                    self.cmp(lr.def_end, begin) == Ordering::Less
                        || self.cmp(lr.def_begin, end) == Ordering::Greater,
                    "Interval can't overlap the def EBB"
                );

                // Save for next round.
                prev_end = Some(end);
            }
        }
    }

    // Singleton `ProgramOrder` for tests below.
    const PO: &'static ProgOrder = &ProgOrder {};

    #[test]
    fn dead_def_range() {
        let v0 = Value::new(0);
        let e0 = Ebb::new(0);
        let i1 = Inst::new(1);
        let i2 = Inst::new(2);
        let e2 = Ebb::new(2);
        let lr = GenLiveRange::new(v0, i1.into(), Default::default());
        let forest = &bforest::MapForest::new();
        let ctx = LiveRangeContext::new(PO, forest);
        assert!(lr.is_dead());
        assert!(lr.is_local());
        assert_eq!(lr.def(), i1.into());
        assert_eq!(lr.def_local_end(), i1.into());
        assert_eq!(lr.livein_local_end(e2, ctx), None);
        PO.validate(&lr, ctx.forest);

        // A dead live range overlaps its own def program point.
        assert!(lr.overlaps_def(i1.into(), e0, ctx));
        assert!(!lr.overlaps_def(i2.into(), e0, ctx));
        assert!(!lr.overlaps_def(e0.into(), e0, ctx));
    }

    #[test]
    fn dead_arg_range() {
        let v0 = Value::new(0);
        let e2 = Ebb::new(2);
        let lr = GenLiveRange::new(v0, e2.into(), Default::default());
        let forest = &bforest::MapForest::new();
        let ctx = LiveRangeContext::new(PO, forest);
        assert!(lr.is_dead());
        assert!(lr.is_local());
        assert_eq!(lr.def(), e2.into());
        assert_eq!(lr.def_local_end(), e2.into());
        // The def interval of an EBB argument does not count as live-in.
        assert_eq!(lr.livein_local_end(e2, ctx), None);
        PO.validate(&lr, ctx.forest);
    }

    #[test]
    fn local_def() {
        let v0 = Value::new(0);
        let e10 = Ebb::new(10);
        let i11 = Inst::new(11);
        let i12 = Inst::new(12);
        let i13 = Inst::new(13);
        let mut lr = GenLiveRange::new(v0, i11.into(), Default::default());
        let forest = &mut bforest::MapForest::new();

        assert_eq!(lr.extend_in_ebb(e10, i13, PO, forest), false);
        PO.validate(&lr, forest);
        assert!(!lr.is_dead());
        assert!(lr.is_local());
        assert_eq!(lr.def(), i11.into());
        assert_eq!(lr.def_local_end(), i13.into());

        // Extending to an already covered inst should not change anything.
        assert_eq!(lr.extend_in_ebb(e10, i12, PO, forest), false);
        PO.validate(&lr, forest);
        assert_eq!(lr.def(), i11.into());
        assert_eq!(lr.def_local_end(), i13.into());
    }

    #[test]
    fn local_arg() {
        let v0 = Value::new(0);
        let e10 = Ebb::new(10);
        let i11 = Inst::new(11);
        let i12 = Inst::new(12);
        let i13 = Inst::new(13);
        let mut lr = GenLiveRange::new(v0, e10.into(), Default::default());
        let forest = &mut bforest::MapForest::new();

        // Extending a dead EBB argument in its own block should not indicate that a live-in
        // interval was created.
        assert_eq!(lr.extend_in_ebb(e10, i12, PO, forest), false);
        PO.validate(&lr, forest);
        assert!(!lr.is_dead());
        assert!(lr.is_local());
        assert_eq!(lr.def(), e10.into());
        assert_eq!(lr.def_local_end(), i12.into());

        // Extending to an already covered inst should not change anything.
        assert_eq!(lr.extend_in_ebb(e10, i11, PO, forest), false);
        PO.validate(&lr, forest);
        assert_eq!(lr.def(), e10.into());
        assert_eq!(lr.def_local_end(), i12.into());

        // Extending further.
        assert_eq!(lr.extend_in_ebb(e10, i13, PO, forest), false);
        PO.validate(&lr, forest);
        assert_eq!(lr.def(), e10.into());
        assert_eq!(lr.def_local_end(), i13.into());
    }

    #[test]
    fn global_def() {
        let v0 = Value::new(0);
        let e10 = Ebb::new(10);
        let i11 = Inst::new(11);
        let i12 = Inst::new(12);
        let e20 = Ebb::new(20);
        let i21 = Inst::new(21);
        let i22 = Inst::new(22);
        let i23 = Inst::new(23);
        let mut lr = GenLiveRange::new(v0, i11.into(), Default::default());
        let forest = &mut bforest::MapForest::new();

        assert_eq!(lr.extend_in_ebb(e10, i12, PO, forest), false);

        // Adding a live-in interval.
        assert_eq!(lr.extend_in_ebb(e20, i22, PO, forest), true);
        PO.validate(&lr, forest);
        assert_eq!(
            lr.livein_local_end(e20, LiveRangeContext::new(PO, forest)),
            Some(i22)
        );

        // Non-extending the live-in.
        assert_eq!(lr.extend_in_ebb(e20, i21, PO, forest), false);
        assert_eq!(
            lr.livein_local_end(e20, LiveRangeContext::new(PO, forest)),
            Some(i22)
        );

        // Extending the existing live-in.
        assert_eq!(lr.extend_in_ebb(e20, i23, PO, forest), false);
        PO.validate(&lr, forest);
        assert_eq!(
            lr.livein_local_end(e20, LiveRangeContext::new(PO, forest)),
            Some(i23)
        );
    }

    #[test]
    fn coalesce() {
        let v0 = Value::new(0);
        let i11 = Inst::new(11);
        let e20 = Ebb::new(20);
        let i21 = Inst::new(21);
        let e30 = Ebb::new(30);
        let i31 = Inst::new(31);
        let e40 = Ebb::new(40);
        let i41 = Inst::new(41);
        let mut lr = GenLiveRange::new(v0, i11.into(), Default::default());
        let forest = &mut bforest::MapForest::new();

        assert_eq!(lr.extend_in_ebb(e30, i31, PO, forest), true);
        assert_eq!(
            lr.liveins(LiveRangeContext::new(PO, forest))
                .collect::<Vec<_>>(),
            [(e30, i31)]
        );

        // Coalesce to previous
        assert_eq!(lr.extend_in_ebb(e40, i41, PO, forest), true);
        assert_eq!(
            lr.liveins(LiveRangeContext::new(PO, forest))
                .collect::<Vec<_>>(),
            [(e30, i41)]
        );

        // Coalesce to next
        assert_eq!(lr.extend_in_ebb(e20, i21, PO, forest), true);
        assert_eq!(
            lr.liveins(LiveRangeContext::new(PO, forest))
                .collect::<Vec<_>>(),
            [(e20, i41)]
        );

        let mut lr = GenLiveRange::new(v0, i11.into(), Default::default());

        assert_eq!(lr.extend_in_ebb(e40, i41, PO, forest), true);
        assert_eq!(
            lr.liveins(LiveRangeContext::new(PO, forest))
                .collect::<Vec<_>>(),
            [(e40, i41)]
        );

        assert_eq!(lr.extend_in_ebb(e20, i21, PO, forest), true);
        assert_eq!(
            lr.liveins(LiveRangeContext::new(PO, forest))
                .collect::<Vec<_>>(),
            [(e20, i21), (e40, i41)]
        );

        // Coalesce to previous and next
        assert_eq!(lr.extend_in_ebb(e30, i31, PO, forest), true);
        assert_eq!(
            lr.liveins(LiveRangeContext::new(PO, forest))
                .collect::<Vec<_>>(),
            [(e20, i41)]
        );
    }

    // TODO: Add more tests that exercise the binary search algorithm.
}
