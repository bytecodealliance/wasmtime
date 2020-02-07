//! Data structure representing the live range of an SSA value.
//!
//! Live ranges are tracked per SSA value, not per variable or virtual register. The live range of
//! an SSA value begins where it is defined and extends to all program points where the value is
//! still needed.
//!
//! # Local Live Ranges
//!
//! Inside a single basic block, the live range of a value is always an interval between
//! two program points (if the value is live in the block at all). The starting point is either:
//!
//! 1. The instruction that defines the value, or
//! 2. The block header, because the value is an argument to the block, or
//! 3. The block header, because the value is defined in another block and live-in to this one.
//!
//! The ending point of the local live range is the last of the following program points in the
//! block:
//!
//! 1. The last use in the block, where a *use* is an instruction that has the value as an argument.
//! 2. The last branch or jump instruction in the block that can reach a use.
//! 3. If the value has no uses anywhere (a *dead value*), the program point that defines it.
//!
//! Note that 2. includes loop back-edges to the same block. In general, if a value is defined
//! outside a loop and used inside the loop, it will be live in the entire loop.
//!
//! # Global Live Ranges
//!
//! Values that appear in more than one block have a *global live range* which can be seen as the
//! disjoint union of the per-block local intervals for all of the blocks where the value is live.
//! Together with a `ProgramOrder` which provides a linear ordering of the blocks, the global live
//! range becomes a linear sequence of disjoint intervals, at most one per block.
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
//! A few notes about the implementation of the live intervals field `liveins`. This should not
//! concern someone only looking to use the public interface.
//!
//! ## Current representation
//!
//! Our current implementation uses a sorted array of compressed intervals, represented by their
//! boundaries (Block, Inst), sorted by Block. This is a simple data structure, enables coalescing of
//! intervals easily, and shows some nice performance behavior. See
//! https://github.com/bytecodealliance/cranelift/issues/1084 for benchmarks against using a
//! bforest::Map<Block, Inst>.
//!
//! ## block ordering
//!
//! The relative order of blocks is used to maintain a sorted list of live-in intervals and to
//! coalesce adjacent live-in intervals when the prior interval covers the whole block. This doesn't
//! depend on any property of the program order, so alternative orderings are possible:
//!
//! 1. The block layout order. This is what we currently use.
//! 2. A topological order of the dominator tree. All the live-in intervals would come after the
//!    def interval.
//! 3. A numerical order by block number. Performant because it doesn't need to indirect through the
//!    `ProgramOrder` for comparisons.
//!
//! These orderings will cause small differences in coalescing opportunities, but all of them would
//! do a decent job of compressing a long live range. The numerical order might be preferable
//! because:
//!
//! - It has better performance because block numbers can be compared directly without any table
//!   lookups.
//! - If block numbers are not reused, it is safe to allocate new blocks without getting spurious
//!   live-in intervals from any coalesced representations that happen to cross a new block.
//!
//! For comparing instructions, the layout order is always what we want.
//!
//! ## Alternative representation
//!
//! Since a local live-in interval always begins at its block header, it is uniquely described by its
//! end point instruction alone. We can use the layout to look up the block containing the end point.
//! This means that a sorted `Vec<Inst>` would be enough to represent the set of live-in intervals.
//!
//! Coalescing is an important compression technique because some live ranges can span thousands of
//! blocks. We can represent that by switching to a sorted `Vec<ProgramPoint>` representation where
//! an `[Block, Inst]` pair represents a coalesced range, while an `Inst` entry without a preceding
//! `Block` entry represents a single live-in interval.
//!
//! This representation is more compact for a live range with many uncoalesced live-in intervals.
//! It is more complicated to work with, though, so it is probably not worth it. The performance
//! benefits of switching to a numerical block order only appears if the binary search is doing
//! block-block comparisons.
//!
//! A `BTreeMap<Block, Inst>` could have been used for the live-in intervals, but it doesn't provide
//! the necessary API to make coalescing easy, nor does it optimize for our types' sizes.
//!
//! Even the specialized `bforest::Map<Block, Inst>` implementation is slower than a plain sorted
//! array, see https://github.com/bytecodealliance/cranelift/issues/1084 for details.

use crate::entity::SparseMapValue;
use crate::ir::{Block, ExpandedProgramPoint, Inst, Layout, ProgramOrder, ProgramPoint, Value};
use crate::regalloc::affinity::Affinity;
use core::cmp::Ordering;
use core::marker::PhantomData;
use smallvec::SmallVec;

/// Global live range of a single SSA value.
///
/// As [explained in the module documentation](index.html#local-live-ranges), the live range of an
/// SSA value is the disjoint union of a set of intervals, each local to a single block, and with at
/// most one interval per block. We further distinguish between:
///
/// 1. The *def interval* is the local interval in the block where the value is defined, and
/// 2. The *live-in intervals* are the local intervals in the remaining blocks.
///
/// A live-in interval always begins at the block header, while the def interval can begin at the
/// defining instruction, or at the block header for an block argument value.
///
/// All values have a def interval, but a large proportion of values don't have any live-in
/// intervals. These are called *local live ranges*.
///
/// # Program order requirements
///
/// The internal representation of a `LiveRange` depends on a consistent `ProgramOrder` both for
/// ordering instructions inside an block *and* for ordering blocks. The methods that depend on the
/// ordering take an explicit `ProgramOrder` object, and it is the caller's responsibility to
/// ensure that the provided ordering is consistent between calls.
///
/// In particular, changing the order of blocks or inserting new blocks will invalidate live ranges.
///
/// Inserting new instructions in the layout is safe, but removing instructions is not. Besides the
/// instructions using or defining their value, `LiveRange` structs can contain references to
/// branch and jump instructions.
pub type LiveRange = GenericLiveRange<Layout>;

// See comment of liveins below.
pub struct Interval {
    begin: Block,
    end: Inst,
}

/// Generic live range implementation.
///
/// The intended generic parameter is `PO=Layout`, but tests are simpler with a mock order.
/// Use `LiveRange` instead of using this generic directly.
pub struct GenericLiveRange<PO: ProgramOrder> {
    /// The value described by this live range.
    /// This member can't be modified in case the live range is stored in a `SparseMap`.
    value: Value,

    /// The preferred register allocation for this value.
    pub affinity: Affinity,

    /// The instruction or block header where this value is defined.
    def_begin: ProgramPoint,

    /// The end point of the def interval. This must always belong to the same block as `def_begin`.
    ///
    /// We always have `def_begin <= def_end` with equality implying a dead def live range with no
    /// uses.
    def_end: ProgramPoint,

    /// Additional live-in intervals sorted in program order.
    ///
    /// This vector is empty for most values which are only used in one block.
    ///
    /// An entry `block -> inst` means that the live range is live-in to `block`, continuing up to
    /// `inst` which may belong to a later block in the program order.
    ///
    /// The entries are non-overlapping, and none of them overlap the block where the value is
    /// defined.
    liveins: SmallVec<[Interval; 2]>,

    po: PhantomData<*const PO>,
}

/// A simple helper macro to make comparisons more natural to read.
macro_rules! cmp {
    ($order:ident, $a:ident > $b:expr) => {
        $order.cmp($a, $b) == Ordering::Greater
    };
    ($order:ident, $a:ident >= $b:expr) => {
        $order.cmp($a, $b) != Ordering::Less
    };
    ($order:ident, $a:ident < $b:expr) => {
        $order.cmp($a, $b) == Ordering::Less
    };
    ($order:ident, $a:ident <= $b:expr) => {
        $order.cmp($a, $b) != Ordering::Greater
    };
}

impl<PO: ProgramOrder> GenericLiveRange<PO> {
    /// Create a new live range for `value` defined at `def`.
    ///
    /// The live range will be created as dead, but it can be extended with `extend_in_block()`.
    pub fn new(value: Value, def: ProgramPoint, affinity: Affinity) -> Self {
        Self {
            value,
            affinity,
            def_begin: def,
            def_end: def,
            liveins: SmallVec::new(),
            po: PhantomData,
        }
    }

    /// Finds an entry in the compressed set of live-in intervals that contains `block`, or return
    /// the position where to insert such a new entry.
    fn lookup_entry_containing_block(&self, block: Block, order: &PO) -> Result<usize, usize> {
        self.liveins
            .binary_search_by(|interval| order.cmp(interval.begin, block))
            .or_else(|n| {
                // The previous interval's end might cover the searched block.
                if n > 0 && cmp!(order, block <= self.liveins[n - 1].end) {
                    Ok(n - 1)
                } else {
                    Err(n)
                }
            })
    }

    /// Extend the local interval for `block` so it reaches `to` which must belong to `block`.
    /// Create a live-in interval if necessary.
    ///
    /// If the live range already has a local interval in `block`, extend its end point so it
    /// includes `to`, and return false.
    ///
    /// If the live range did not previously have a local interval in `block`, add one so the value
    /// is live-in to `block`, extending to `to`. Return true.
    ///
    /// The return value can be used to detect if we just learned that the value is live-in to
    /// `block`. This can trigger recursive extensions in `block`'s CFG predecessor blocks.
    pub fn extend_in_block(&mut self, block: Block, inst: Inst, order: &PO) -> bool {
        // First check if we're extending the def interval.
        //
        // We're assuming here that `inst` never precedes `def_begin` in the same block, but we can't
        // check it without a method for getting `inst`'s block.
        if cmp!(order, block <= self.def_end) && cmp!(order, inst >= self.def_begin) {
            let inst_pp = inst.into();
            debug_assert_ne!(
                inst_pp, self.def_begin,
                "Can't use value in the defining instruction."
            );
            if cmp!(order, inst > self.def_end) {
                self.def_end = inst_pp;
            }
            return false;
        }

        // Now check if we're extending any of the existing live-in intervals.
        match self.lookup_entry_containing_block(block, order) {
            Ok(n) => {
                // We found one interval and might need to extend it.
                if cmp!(order, inst <= self.liveins[n].end) {
                    // Both interval parts are already included in a compressed interval.
                    return false;
                }

                // If the instruction at the end is the last instruction before the next block,
                // coalesce the two intervals:
                // [ival.begin; ival.end] + [next.begin; next.end] = [ival.begin; next.end]
                if let Some(next) = &self.liveins.get(n + 1) {
                    if order.is_block_gap(inst, next.begin) {
                        // At this point we can choose to remove the current interval or the next
                        // one; remove the next one to avoid one memory move.
                        let next_end = next.end;
                        debug_assert!(cmp!(order, next_end > self.liveins[n].end));
                        self.liveins[n].end = next_end;
                        self.liveins.remove(n + 1);
                        return false;
                    }
                }

                // We can't coalesce, just extend the interval.
                self.liveins[n].end = inst;
                false
            }

            Err(n) => {
                // No interval was found containing the current block: we need to insert a new one,
                // unless there's a coalescing opportunity with the previous or next one.
                let coalesce_next = self
                    .liveins
                    .get(n)
                    .filter(|next| order.is_block_gap(inst, next.begin))
                    .is_some();
                let coalesce_prev = self
                    .liveins
                    .get(n.wrapping_sub(1))
                    .filter(|prev| order.is_block_gap(prev.end, block))
                    .is_some();

                match (coalesce_prev, coalesce_next) {
                    // The new interval is the missing hole between prev and next: we can merge
                    // them all together.
                    (true, true) => {
                        let prev_end = self.liveins[n - 1].end;
                        debug_assert!(cmp!(order, prev_end <= self.liveins[n].end));
                        self.liveins[n - 1].end = self.liveins[n].end;
                        self.liveins.remove(n);
                    }

                    // Coalesce only with the previous or next one.
                    (true, false) => {
                        debug_assert!(cmp!(order, inst >= self.liveins[n - 1].end));
                        self.liveins[n - 1].end = inst;
                    }
                    (false, true) => {
                        debug_assert!(cmp!(order, block <= self.liveins[n].begin));
                        self.liveins[n].begin = block;
                    }

                    (false, false) => {
                        // No coalescing opportunity, we have to insert.
                        self.liveins.insert(
                            n,
                            Interval {
                                begin: block,
                                end: inst,
                            },
                        );
                    }
                }

                true
            }
        }
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
    /// A local live range is only used in the same block where it was defined. It is allowed to span
    /// multiple basic blocks within that block.
    pub fn is_local(&self) -> bool {
        self.liveins.is_empty()
    }

    /// Get the program point where this live range is defined.
    ///
    /// This will be an block header when the value is an block argument, otherwise it is the defining
    /// instruction.
    pub fn def(&self) -> ProgramPoint {
        self.def_begin
    }

    /// Move the definition of this value to a new program point.
    ///
    /// It is only valid to move the definition within the same block, and it can't be moved beyond
    /// `def_local_end()`.
    pub fn move_def_locally(&mut self, def: ProgramPoint) {
        self.def_begin = def;
    }

    /// Get the local end-point of this live range in the block where it is defined.
    ///
    /// This can be the block header itself in the case of a dead block argument.
    /// Otherwise, it will be the last local use or branch/jump that can reach a use.
    pub fn def_local_end(&self) -> ProgramPoint {
        self.def_end
    }

    /// Get the local end-point of this live range in an block where it is live-in.
    ///
    /// If this live range is not live-in to `block`, return `None`. Otherwise, return the end-point
    /// of this live range's local interval in `block`.
    ///
    /// If the live range is live through all of `block`, the terminator of `block` is a correct
    /// answer, but it is also possible that an even later program point is returned. So don't
    /// depend on the returned `Inst` to belong to `block`.
    pub fn livein_local_end(&self, block: Block, order: &PO) -> Option<Inst> {
        self.lookup_entry_containing_block(block, order)
            .and_then(|i| {
                let inst = self.liveins[i].end;
                if cmp!(order, block < inst) {
                    Ok(inst)
                } else {
                    // Can be any error type, really, since it's discarded by ok().
                    Err(i)
                }
            })
            .ok()
    }

    /// Is this value live-in to `block`?
    ///
    /// An block argument is not considered to be live in.
    pub fn is_livein(&self, block: Block, order: &PO) -> bool {
        self.livein_local_end(block, order).is_some()
    }

    /// Get all the live-in intervals.
    ///
    /// Note that the intervals are stored in a compressed form so each entry may span multiple
    /// blocks where the value is live in.
    pub fn liveins<'a>(&'a self) -> impl Iterator<Item = (Block, Inst)> + 'a {
        self.liveins
            .iter()
            .map(|interval| (interval.begin, interval.end))
    }

    /// Check if this live range overlaps a definition in `block`.
    pub fn overlaps_def(&self, def: ExpandedProgramPoint, block: Block, order: &PO) -> bool {
        // Two defs at the same program point always overlap, even if one is dead.
        if def == self.def_begin.into() {
            return true;
        }

        // Check for an overlap with the local range.
        if cmp!(order, def >= self.def_begin) && cmp!(order, def < self.def_end) {
            return true;
        }

        // Check for an overlap with a live-in range.
        match self.livein_local_end(block, order) {
            Some(inst) => cmp!(order, def < inst),
            None => false,
        }
    }

    /// Check if this live range reaches a use at `user` in `block`.
    pub fn reaches_use(&self, user: Inst, block: Block, order: &PO) -> bool {
        // Check for an overlap with the local range.
        if cmp!(order, user > self.def_begin) && cmp!(order, user <= self.def_end) {
            return true;
        }

        // Check for an overlap with a live-in range.
        match self.livein_local_end(block, order) {
            Some(inst) => cmp!(order, user <= inst),
            None => false,
        }
    }

    /// Check if this live range is killed at `user` in `block`.
    pub fn killed_at(&self, user: Inst, block: Block, order: &PO) -> bool {
        self.def_local_end() == user.into() || self.livein_local_end(block, order) == Some(user)
    }
}

/// Allow a `LiveRange` to be stored in a `SparseMap` indexed by values.
impl<PO: ProgramOrder> SparseMapValue<Value> for GenericLiveRange<PO> {
    fn key(&self) -> Value {
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::{GenericLiveRange, Interval};
    use crate::entity::EntityRef;
    use crate::ir::{Block, Inst, Value};
    use crate::ir::{ExpandedProgramPoint, ProgramOrder};
    use alloc::vec::Vec;
    use core::cmp::Ordering;

    // Dummy program order which simply compares indexes.
    // It is assumed that blocks have indexes that are multiples of 10, and instructions have indexes
    // in between. `is_block_gap` assumes that terminator instructions have indexes of the form
    // block * 10 + 1. This is used in the coalesce test.
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
                    ExpandedProgramPoint::Block(e) => e.index(),
                }
            }

            let ia = idx(a.into());
            let ib = idx(b.into());
            ia.cmp(&ib)
        }

        fn is_block_gap(&self, inst: Inst, block: Block) -> bool {
            inst.index() % 10 == 1 && block.index() / 10 == inst.index() / 10 + 1
        }
    }

    impl ProgOrder {
        // Get the block corresponding to `inst`.
        fn inst_block(&self, inst: Inst) -> Block {
            let i = inst.index();
            Block::new(i - i % 10)
        }

        // Get the block of a program point.
        fn pp_block<PP: Into<ExpandedProgramPoint>>(&self, pp: PP) -> Block {
            match pp.into() {
                ExpandedProgramPoint::Inst(i) => self.inst_block(i),
                ExpandedProgramPoint::Block(e) => e,
            }
        }

        // Validate the live range invariants.
        fn validate(&self, lr: &GenericLiveRange<Self>) {
            // The def interval must cover a single block.
            let def_block = self.pp_block(lr.def_begin);
            assert_eq!(def_block, self.pp_block(lr.def_end));

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
            for Interval { begin, end } in lr.liveins.iter() {
                let begin = *begin;
                let end = *end;

                assert_eq!(self.cmp(begin, end), Ordering::Less);
                if let Some(e) = prev_end {
                    assert_eq!(self.cmp(e, begin), Ordering::Less);
                }

                assert!(
                    self.cmp(lr.def_end, begin) == Ordering::Less
                        || self.cmp(lr.def_begin, end) == Ordering::Greater,
                    "Interval can't overlap the def block"
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
        let e0 = Block::new(0);
        let i1 = Inst::new(1);
        let i2 = Inst::new(2);
        let e2 = Block::new(2);
        let lr = GenericLiveRange::new(v0, i1.into(), Default::default());
        assert!(lr.is_dead());
        assert!(lr.is_local());
        assert_eq!(lr.def(), i1.into());
        assert_eq!(lr.def_local_end(), i1.into());
        assert_eq!(lr.livein_local_end(e2, PO), None);
        PO.validate(&lr);

        // A dead live range overlaps its own def program point.
        assert!(lr.overlaps_def(i1.into(), e0, PO));
        assert!(!lr.overlaps_def(i2.into(), e0, PO));
        assert!(!lr.overlaps_def(e0.into(), e0, PO));
    }

    #[test]
    fn dead_arg_range() {
        let v0 = Value::new(0);
        let e2 = Block::new(2);
        let lr = GenericLiveRange::new(v0, e2.into(), Default::default());
        assert!(lr.is_dead());
        assert!(lr.is_local());
        assert_eq!(lr.def(), e2.into());
        assert_eq!(lr.def_local_end(), e2.into());
        // The def interval of an block argument does not count as live-in.
        assert_eq!(lr.livein_local_end(e2, PO), None);
        PO.validate(&lr);
    }

    #[test]
    fn local_def() {
        let v0 = Value::new(0);
        let e10 = Block::new(10);
        let i11 = Inst::new(11);
        let i12 = Inst::new(12);
        let i13 = Inst::new(13);
        let mut lr = GenericLiveRange::new(v0, i11.into(), Default::default());

        assert_eq!(lr.extend_in_block(e10, i13, PO), false);
        PO.validate(&lr);
        assert!(!lr.is_dead());
        assert!(lr.is_local());
        assert_eq!(lr.def(), i11.into());
        assert_eq!(lr.def_local_end(), i13.into());

        // Extending to an already covered inst should not change anything.
        assert_eq!(lr.extend_in_block(e10, i12, PO), false);
        PO.validate(&lr);
        assert_eq!(lr.def(), i11.into());
        assert_eq!(lr.def_local_end(), i13.into());
    }

    #[test]
    fn local_arg() {
        let v0 = Value::new(0);
        let e10 = Block::new(10);
        let i11 = Inst::new(11);
        let i12 = Inst::new(12);
        let i13 = Inst::new(13);
        let mut lr = GenericLiveRange::new(v0, e10.into(), Default::default());

        // Extending a dead block argument in its own block should not indicate that a live-in
        // interval was created.
        assert_eq!(lr.extend_in_block(e10, i12, PO), false);
        PO.validate(&lr);
        assert!(!lr.is_dead());
        assert!(lr.is_local());
        assert_eq!(lr.def(), e10.into());
        assert_eq!(lr.def_local_end(), i12.into());

        // Extending to an already covered inst should not change anything.
        assert_eq!(lr.extend_in_block(e10, i11, PO), false);
        PO.validate(&lr);
        assert_eq!(lr.def(), e10.into());
        assert_eq!(lr.def_local_end(), i12.into());

        // Extending further.
        assert_eq!(lr.extend_in_block(e10, i13, PO), false);
        PO.validate(&lr);
        assert_eq!(lr.def(), e10.into());
        assert_eq!(lr.def_local_end(), i13.into());
    }

    #[test]
    fn global_def() {
        let v0 = Value::new(0);
        let e10 = Block::new(10);
        let i11 = Inst::new(11);
        let i12 = Inst::new(12);
        let e20 = Block::new(20);
        let i21 = Inst::new(21);
        let i22 = Inst::new(22);
        let i23 = Inst::new(23);
        let mut lr = GenericLiveRange::new(v0, i11.into(), Default::default());

        assert_eq!(lr.extend_in_block(e10, i12, PO), false);

        // Adding a live-in interval.
        assert_eq!(lr.extend_in_block(e20, i22, PO), true);
        PO.validate(&lr);
        assert_eq!(lr.livein_local_end(e20, PO), Some(i22));

        // Non-extending the live-in.
        assert_eq!(lr.extend_in_block(e20, i21, PO), false);
        assert_eq!(lr.livein_local_end(e20, PO), Some(i22));

        // Extending the existing live-in.
        assert_eq!(lr.extend_in_block(e20, i23, PO), false);
        PO.validate(&lr);
        assert_eq!(lr.livein_local_end(e20, PO), Some(i23));
    }

    #[test]
    fn coalesce() {
        let v0 = Value::new(0);
        let i11 = Inst::new(11);
        let e20 = Block::new(20);
        let i21 = Inst::new(21);
        let e30 = Block::new(30);
        let i31 = Inst::new(31);
        let e40 = Block::new(40);
        let i41 = Inst::new(41);
        let mut lr = GenericLiveRange::new(v0, i11.into(), Default::default());

        assert_eq!(lr.extend_in_block(e30, i31, PO,), true);
        assert_eq!(lr.liveins().collect::<Vec<_>>(), [(e30, i31)]);

        // Coalesce to previous
        assert_eq!(lr.extend_in_block(e40, i41, PO,), true);
        assert_eq!(lr.liveins().collect::<Vec<_>>(), [(e30, i41)]);

        // Coalesce to next
        assert_eq!(lr.extend_in_block(e20, i21, PO,), true);
        assert_eq!(lr.liveins().collect::<Vec<_>>(), [(e20, i41)]);

        let mut lr = GenericLiveRange::new(v0, i11.into(), Default::default());

        assert_eq!(lr.extend_in_block(e40, i41, PO,), true);
        assert_eq!(lr.liveins().collect::<Vec<_>>(), [(e40, i41)]);

        assert_eq!(lr.extend_in_block(e20, i21, PO,), true);
        assert_eq!(lr.liveins().collect::<Vec<_>>(), [(e20, i21), (e40, i41)]);

        // Coalesce to previous and next
        assert_eq!(lr.extend_in_block(e30, i31, PO,), true);
        assert_eq!(lr.liveins().collect::<Vec<_>>(), [(e20, i41)]);
    }
}
