//! In-memory representation of compiled machine code, with labels and fixups to
//! refer to those labels. Handles constant-pool island insertion and also
//! veneer insertion for out-of-range jumps.
//!
//! This code exists to solve three problems:
//!
//! - Branch targets for forward branches are not known until later, when we
//!   emit code in a single pass through the instruction structs.
//!
//! - On many architectures, address references or offsets have limited range.
//!   For example, on AArch64, conditional branches can only target code +/- 1MB
//!   from the branch itself.
//!
//! - The lowering of control flow from the CFG-with-edges produced by
//!   [BlockLoweringOrder], combined with many empty edge blocks when the register
//!   allocator does not need to insert any spills/reloads/moves in edge blocks,
//!   results in many suboptimal branch patterns. The lowering also pays no
//!   attention to block order, and so two-target conditional forms (cond-br
//!   followed by uncond-br) can often by avoided because one of the targets is
//!   the fallthrough. There are several cases here where we can simplify to use
//!   fewer branches.
//!
//! This "buffer" implements a single-pass code emission strategy (with a later
//! "fixup" pass, but only through recorded fixups, not all instructions). The
//! basic idea is:
//!
//! - Emit branches as they are, including two-target (cond/uncond) compound
//!   forms, but with zero offsets and optimistically assuming the target will be
//!   in range. Record the "fixup" for later. Targets are denoted instead by
//!   symbolic "labels" that are then bound to certain offsets in the buffer as
//!   we emit code. (Nominally, there is a label at the start of every basic
//!   block.)
//!
//! - As we do this, track the offset in the buffer at which the first label
//!   reference "goes out of range". We call this the "deadline". If we reach the
//!   deadline and we still have not bound the label to which an unresolved branch
//!   refers, we have a problem!
//!
//! - To solve this problem, we emit "islands" full of "veneers". An island is
//!   simply a chunk of code inserted in the middle of the code actually produced
//!   by the emitter (e.g., vcode iterating over instruction structs). The emitter
//!   has some awareness of this: it either asks for an island between blocks, so
//!   it is not accidentally executed, or else it emits a branch around the island
//!   when all other options fail (see [Inst::EmitIsland] meta-instruction).
//!
//! - A "veneer" is an instruction (or sequence of instructions) in an "island"
//!   that implements a longer-range reference to a label. The idea is that, for
//!   example, a branch with a limited range can branch to a "veneer" instead,
//!   which is simply a branch in a form that can use a longer-range reference. On
//!   AArch64, for example, conditionals have a +/- 1 MB range, but a conditional
//!   can branch to an unconditional branch which has a +/- 128 MB range. Hence, a
//!   conditional branch's label reference can be fixed up with a "veneer" to
//!   achieve a longer range.
//!
//! - To implement all of this, we require the backend to provide a `LabelUse`
//!   type that implements a trait. This is nominally an enum that records one of
//!   several kinds of references to an offset in code -- basically, a relocation
//!   type -- and will usually correspond to different instruction formats. The
//!   `LabelUse` implementation specifies the maximum range, how to patch in the
//!   actual label location when known, and how to generate a veneer to extend the
//!   range.
//!
//! That satisfies label references, but we still may have suboptimal branch
//! patterns. To clean up the branches, we do a simple "peephole"-style
//! optimization on the fly. To do so, the emitter (e.g., `Inst::emit()`)
//! informs the buffer of branches in the code and, in the case of conditionals,
//! the code that would have been emitted to invert this branch's condition. We
//! track the "latest branches": these are branches that are contiguous up to
//! the current offset. (If any code is emitted after a branch, that branch or
//! run of contiguous branches is no longer "latest".) The latest branches are
//! those that we can edit by simply truncating the buffer and doing something
//! else instead.
//!
//! To optimize branches, we implement several simple rules, and try to apply
//! them to the "latest branches" when possible:
//!
//! - A branch with a label target, when that label is bound to the ending
//!   offset of the branch (the fallthrough location), can be removed altogether,
//!   because the branch would have no effect).
//!
//! - An unconditional branch that starts at a label location, and branches to
//!   another label, results in a "label alias": all references to the label bound
//!   *to* this branch instruction are instead resolved to the *target* of the
//!   branch instruction. This effectively removes empty blocks that just
//!   unconditionally branch to the next block. We call this "branch threading".
//!
//! - A conditional followed by an unconditional, when the conditional branches
//!   to the unconditional's fallthrough, results in (i) the truncation of the
//!   unconditional, (ii) the inversion of the condition's condition, and (iii)
//!   replacement of the conditional's target (using the original target of the
//!   unconditional). This is a fancy way of saying "we can flip a two-target
//!   conditional branch's taken/not-taken targets if it works better with our
//!   fallthrough". To make this work, the emitter actually gives the buffer
//!   *both* forms of every conditional branch: the true form is emitted into the
//!   buffer, and the "inverted" machine-code bytes are provided as part of the
//!   branch-fixup metadata.
//!
//! - An unconditional B preceded by another unconditional P, when B's label(s) have
//!   been redirected to target(B), can be removed entirely. This is an extension
//!   of the branch-threading optimization, and is valid because if we know there
//!   will be no fallthrough into this branch instruction (the prior instruction
//!   is an unconditional jump), and if we know we have successfully redirected
//!   all labels, then this branch instruction is unreachable. Note that this
//!   works because the redirection happens before the label is ever resolved
//!   (fixups happen at island emission time, at which point latest-branches are
//!   cleared, or at the end of emission), so we are sure to catch and redirect
//!   all possible paths to this instruction.

use crate::binemit::{Addend, CodeOffset, CodeSink, Reloc};
use crate::ir::{ExternalName, Opcode, SourceLoc, TrapCode};
use crate::machinst::{BlockIndex, MachInstLabelUse, VCodeInst};

use log::trace;
use smallvec::SmallVec;
use std::mem;

/// A buffer of output to be produced, fixed up, and then emitted to a CodeSink
/// in bulk.
///
/// This struct uses `SmallVec`s to support small-ish function bodies without
/// any heap allocation. As such, it will be several kilobytes large. This is
/// likely fine as long as it is stack-allocated for function emission then
/// thrown away; but beware if many buffer objects are retained persistently.
pub struct MachBuffer<I: VCodeInst> {
    /// The buffer contents, as raw bytes.
    data: SmallVec<[u8; 1024]>,
    /// Any relocations referring to this code. Note that only *external*
    /// relocations are tracked here; references to labels within the buffer are
    /// resolved before emission.
    relocs: SmallVec<[MachReloc; 16]>,
    /// Any trap records referring to this code.
    traps: SmallVec<[MachTrap; 16]>,
    /// Any call site records referring to this code.
    call_sites: SmallVec<[MachCallSite; 16]>,
    /// Any source location mappings referring to this code.
    srclocs: SmallVec<[MachSrcLoc; 64]>,
    /// The current source location in progress (after `start_srcloc()` and
    /// before `end_srcloc()`).  This is a (start_offset, src_loc) tuple.
    cur_srcloc: Option<(CodeOffset, SourceLoc)>,
    /// Known label offsets; `UNKNOWN_LABEL_OFFSET` if unknown.
    label_offsets: SmallVec<[CodeOffset; 16]>,
    /// Label aliases: when one label points to an unconditional jump, and that
    /// jump points to another label, we can redirect references to the first
    /// label immediately to the second. (We don't chase arbitrarily deep to
    /// avoid problems with cycles, but rather only one level, i.e.  through one
    /// jump.)
    label_aliases: SmallVec<[MachLabel; 16]>,
    /// Constants that must be emitted at some point.
    pending_constants: SmallVec<[MachLabelConstant; 16]>,
    /// Fixups that must be performed after all code is emitted.
    fixup_records: SmallVec<[MachLabelFixup<I>; 16]>,
    /// Current deadline at which all constants are flushed and all code labels
    /// are extended by emitting long-range jumps in an island. This flush
    /// should be rare (e.g., on AArch64, the shortest-range PC-rel references
    /// are +/- 1MB for conditional jumps and load-literal instructions), so
    /// it's acceptable to track a minimum and flush-all rather than doing more
    /// detailed "current minimum" / sort-by-deadline trickery.
    island_deadline: CodeOffset,
    /// How many bytes are needed in the worst case for an island, given all
    /// pending constants and fixups.
    island_worst_case_size: CodeOffset,
    /// Latest branches, to facilitate in-place editing for better fallthrough
    /// behavior and empty-block removal.
    latest_branches: SmallVec<[MachBranch; 4]>,
    /// All labels, in offset order.
    labels_by_offset: SmallVec<[(MachLabel, CodeOffset); 16]>,
}

/// A `MachBuffer` once emission is completed: holds generated code and records,
/// without fixups. This allows the type to be independent of the backend.
pub struct MachBufferFinalized {
    /// The buffer contents, as raw bytes.
    pub data: SmallVec<[u8; 1024]>,
    /// Any relocations referring to this code. Note that only *external*
    /// relocations are tracked here; references to labels within the buffer are
    /// resolved before emission.
    relocs: SmallVec<[MachReloc; 16]>,
    /// Any trap records referring to this code.
    traps: SmallVec<[MachTrap; 16]>,
    /// Any call site records referring to this code.
    call_sites: SmallVec<[MachCallSite; 16]>,
    /// Any source location mappings referring to this code.
    srclocs: SmallVec<[MachSrcLoc; 64]>,
}

static UNKNOWN_LABEL_OFFSET: CodeOffset = 0xffff_ffff;
static UNKNOWN_LABEL: MachLabel = MachLabel(0xffff_ffff);

/// A label refers to some offset in a `MachBuffer`. It may not be resolved at
/// the point at which it is used by emitted code; the buffer records "fixups"
/// for references to the label, and will come back and patch the code
/// appropriately when the label's location is eventually known.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MachLabel(u32);

impl MachLabel {
    /// Get a label for a block. (The first N MachLabels are always reseved for
    /// the N blocks in the vcode.)
    pub fn from_block(bindex: BlockIndex) -> MachLabel {
        MachLabel(bindex)
    }

    /// Get the numeric label index.
    pub fn get(self) -> u32 {
        self.0
    }
}

impl<I: VCodeInst> MachBuffer<I> {
    /// Create a new section, known to start at `start_offset` and with a size limited to `length_limit`.
    pub fn new() -> MachBuffer<I> {
        MachBuffer {
            data: SmallVec::new(),
            relocs: SmallVec::new(),
            traps: SmallVec::new(),
            call_sites: SmallVec::new(),
            srclocs: SmallVec::new(),
            cur_srcloc: None,
            label_offsets: SmallVec::new(),
            label_aliases: SmallVec::new(),
            pending_constants: SmallVec::new(),
            fixup_records: SmallVec::new(),
            island_deadline: UNKNOWN_LABEL_OFFSET,
            island_worst_case_size: 0,
            latest_branches: SmallVec::new(),
            labels_by_offset: SmallVec::new(),
        }
    }

    /// Current offset from start of buffer.
    pub fn cur_offset(&self) -> CodeOffset {
        self.data.len() as CodeOffset
    }

    /// Add a byte.
    pub fn put1(&mut self, value: u8) {
        trace!("MachBuffer: put byte @ {}: {:x}", self.cur_offset(), value);
        self.data.push(value);
    }

    /// Add 2 bytes.
    pub fn put2(&mut self, value: u16) {
        trace!(
            "MachBuffer: put 16-bit word @ {}: {:x}",
            self.cur_offset(),
            value
        );
        let bytes = value.to_le_bytes();
        self.data.extend_from_slice(&bytes[..]);
    }

    /// Add 4 bytes.
    pub fn put4(&mut self, value: u32) {
        trace!(
            "MachBuffer: put 32-bit word @ {}: {:x}",
            self.cur_offset(),
            value
        );
        let bytes = value.to_le_bytes();
        self.data.extend_from_slice(&bytes[..]);
    }

    /// Add 8 bytes.
    pub fn put8(&mut self, value: u64) {
        trace!(
            "MachBuffer: put 64-bit word @ {}: {:x}",
            self.cur_offset(),
            value
        );
        let bytes = value.to_le_bytes();
        self.data.extend_from_slice(&bytes[..]);
    }

    /// Add a slice of bytes.
    pub fn put_data(&mut self, data: &[u8]) {
        trace!(
            "MachBuffer: put data @ {}: len {}",
            self.cur_offset(),
            data.len()
        );
        self.data.extend_from_slice(data);
    }

    /// Reserve appended space and return a mutable slice referring to it.
    pub fn get_appended_space(&mut self, len: usize) -> &mut [u8] {
        trace!("MachBuffer: put data @ {}: len {}", self.cur_offset(), len);
        let off = self.data.len();
        let new_len = self.data.len() + len;
        self.data.resize(new_len, 0);
        &mut self.data[off..]
    }

    /// Align up to the given alignment.
    pub fn align_to(&mut self, align_to: CodeOffset) {
        trace!("MachBuffer: align to {}", align_to);
        assert!(align_to.is_power_of_two());
        while self.cur_offset() & (align_to - 1) != 0 {
            self.put1(0);
        }
    }

    /// Allocate a `Label` to refer to some offset. May not be bound to a fixed
    /// offset yet.
    pub fn get_label(&mut self) -> MachLabel {
        let l = self.label_offsets.len() as u32;
        self.label_offsets.push(UNKNOWN_LABEL_OFFSET);
        self.label_aliases.push(UNKNOWN_LABEL);
        trace!("MachBuffer: new label -> {:?}", MachLabel(l));
        MachLabel(l)
    }

    /// Reserve the first N MachLabels for blocks.
    pub fn reserve_labels_for_blocks(&mut self, blocks: BlockIndex) {
        trace!("MachBuffer: first {} labels are for blocks", blocks);
        debug_assert!(self.label_offsets.is_empty());
        self.label_offsets
            .resize(blocks as usize, UNKNOWN_LABEL_OFFSET);
        self.label_aliases.resize(blocks as usize, UNKNOWN_LABEL);
    }

    /// Bind a label to the current offset.
    pub fn bind_label(&mut self, label: MachLabel) {
        trace!(
            "MachBuffer: bind label {:?} at offset {}",
            label,
            self.cur_offset()
        );
        let offset = self.cur_offset();
        self.label_offsets[label.0 as usize] = offset;
        self.labels_by_offset.push((label, offset));
        self.optimize_branches();
    }

    /// Resolve a label to an offset, if known. May return `UNKNOWN_LABEL_OFFSET`.
    fn resolve_label_offset(&self, label: MachLabel) -> CodeOffset {
        let alias = self.label_aliases[label.0 as usize];
        if alias != UNKNOWN_LABEL {
            self.label_offsets[alias.0 as usize]
        } else {
            self.label_offsets[label.0 as usize]
        }
    }

    /// Emit a reference to the given label with the given reference type (i.e.,
    /// branch-instruction format) at the current offset.  This is like a
    /// relocation, but handled internally.
    ///
    /// Because the offset of the label may already be known and the patch may
    /// happen immediately, the buffer must already contain bytes at `offset` up
    /// to `offset + kind.patch_size()`.
    pub fn use_label_at_offset(&mut self, offset: CodeOffset, label: MachLabel, kind: I::LabelUse) {
        trace!(
            "MachBuffer: use_label_at_offset: offset {} label {:?} kind {:?}",
            offset,
            label,
            kind
        );
        debug_assert!(offset + kind.patch_size() <= self.cur_offset());

        // Add the fixup, and update the worst-case island size based on a
        // veneer for this label use.
        self.fixup_records.push(MachLabelFixup {
            label,
            offset,
            kind,
        });
        if kind.supports_veneer() {
            self.island_worst_case_size += kind.veneer_size();
            self.island_worst_case_size &= !(I::LabelUse::ALIGN - 1);
        }
        let deadline = offset + kind.max_pos_range();
        if deadline < self.island_deadline {
            self.island_deadline = deadline;
        }
    }

    /// Inform the buffer of an unconditional branch at the given offset,
    /// targetting the given label. May be used to optimize branches.
    /// The last added label-use must correspond to this branch.
    pub fn add_uncond_branch(&mut self, start: CodeOffset, end: CodeOffset, target: MachLabel) {
        assert!(!self.fixup_records.is_empty());
        let fixup = self.fixup_records.len() - 1;
        self.latest_branches.push(MachBranch {
            start,
            end,
            target,
            fixup,
            inverted: None,
        });
    }

    /// Inform the buffer of a conditional branch at the given offset,
    /// targetting the given label. May be used to optimize branches.
    /// The last added label-use must correspond to this branch.
    pub fn add_cond_branch(
        &mut self,
        start: CodeOffset,
        end: CodeOffset,
        target: MachLabel,
        inverted: &[u8],
    ) {
        assert!(!self.fixup_records.is_empty());
        let fixup = self.fixup_records.len() - 1;
        let inverted = Some(SmallVec::from(inverted));
        self.latest_branches.push(MachBranch {
            start,
            end,
            target,
            fixup,
            inverted,
        });
    }

    fn truncate_last_branch(&mut self) {
        let b = self.latest_branches.pop().unwrap();
        assert!(b.end == self.cur_offset());
        self.data.truncate(b.start as usize);
        self.fixup_records.truncate(b.fixup);
        let cur_off = self.cur_offset();
        trace!(
            "truncate_last_branch: truncated {:?}; off now {}",
            b,
            cur_off
        );
        for &mut (l, ref mut off) in self.labels_by_offset.iter_mut().rev() {
            if *off > cur_off {
                *off = cur_off;
                trace!(" -> label {:?} reassigned to {}", l, cur_off);
                self.label_offsets[l.0 as usize] = cur_off;
            } else {
                break;
            }
        }
    }

    fn optimize_branches(&mut self) {
        trace!(
            "enter optimize_branches:\n b = {:?}\n l = {:?}\n f = {:?}",
            self.latest_branches,
            self.labels_by_offset,
            self.fixup_records
        );
        while let Some(b) = self.latest_branches.last() {
            let cur_off = self.cur_offset();
            trace!("optimize_branches: last branch {:?} at off {}", b, cur_off);
            // If there has been any code emission since the end of the last branch or
            // label definition, then there's nothing we can edit (because we
            // don't move code once placed, only back up and overwrite), so
            // clear the records and finish.
            if b.end < cur_off {
                break;
            }

            // If latest is an unconditional branch:
            // - For each label at this point, make the label an alias of
            //   the branch target. We can now assume below that the
            //   unconditional branch is reachable only via fallthrough, and we
            //   are free to remove it in an optimization.
            // - If there is a prior unconditional branch that ends just before
            //   this one begins, then we can truncate this branch, because it is
            //   entirely unreachable (due to above). Trim the end of the
            //   `labels_by_offset` array and continue around the loop.
            // - If there is a prior conditional branch whose target label
            //   resolves to the current offset (branches around the
            //   unconditional branch), then remove the unconditional branch,
            //   and make the target of the unconditional the target of the
            //   conditional instead.
            if b.is_uncond() {
                // Set any label equal to current branch's start as an alias of
                // the branch's target.
                for &(l, off) in self.labels_by_offset.iter().rev() {
                    trace!(" -> uncond: latest label {:?} at off {}", l, off);
                    if off > b.start {
                        continue;
                    } else if off == b.start {
                        trace!(" -> setting alias to {:?}", b.target);
                        self.label_aliases[l.0 as usize] = b.target;
                    } else {
                        break;
                    }
                }

                // If the branch target is the next offset,

                // Examine any immediately preceding branch.
                if self.latest_branches.len() > 1 {
                    let prev_b = &self.latest_branches[self.latest_branches.len() - 2];
                    trace!(" -> more than one branch; prev_b = {:?}", prev_b);
                    // This uncond is immediately after another uncond; we've
                    // already redirected labels to this uncond away; so we can
                    // truncate this uncond.
                    if prev_b.is_uncond() && prev_b.end == b.start {
                        trace!(" -> uncond follows another uncond; truncating");
                        self.truncate_last_branch();
                        continue;
                    }

                    // This uncond is immediately after a conditional, and the
                    // conditional's target is the end of this uncond, and we've
                    // already redirected labels to this uncond away; so we can
                    // truncate this uncond, flip the sense of the conditional, and
                    // set the conditional's target (in `latest_branches` and in
                    // `fixup_records`) to the uncond's target.
                    if prev_b.is_cond()
                        && prev_b.end == b.start
                        && self.resolve_label_offset(prev_b.target) == cur_off
                    {
                        trace!(" -> uncond follows a conditional, and conditional's target resolves to current offset");
                        let target = b.target;
                        let data = prev_b.inverted.clone().unwrap();
                        self.truncate_last_branch();
                        let prev_b = self.latest_branches.last_mut().unwrap();
                        let not_inverted = SmallVec::from(
                            &self.data[(prev_b.start as usize)..(prev_b.end as usize)],
                        );
                        self.data.truncate(prev_b.start as usize);
                        self.data.extend_from_slice(&data[..]);
                        prev_b.inverted = Some(not_inverted);
                        self.fixup_records[prev_b.fixup].label = target;
                        trace!(" -> reassigning target of condbr to {:?}", target);
                        prev_b.target = target;
                        continue;
                    }
                }
            }

            // For any branch, conditional or unconditional:
            // - If the target is a label at the current offset, then remove
            //   the conditional branch, and reset all labels that targetted
            //   the current offset (end of branch) to the truncated
            //   end-of-code.
            if self.resolve_label_offset(b.target) == cur_off {
                trace!("branch with target == cur off; truncating");
                self.truncate_last_branch();
            }

            // If we couldn't do anything with the last branch, then break.
            break;
        }

        self.purge_latest_branches();

        trace!(
            "leave optimize_branches:\n b = {:?}\n l = {:?}\n f = {:?}",
            self.latest_branches,
            self.labels_by_offset,
            self.fixup_records
        );
    }

    fn purge_latest_branches(&mut self) {
        let cur_off = self.cur_offset();
        if let Some(l) = self.latest_branches.last() {
            if l.end < cur_off {
                trace!("purge_latest_branches: removing branch {:?}", l);
                self.latest_branches.clear();
            }
        }
    }

    /// Emit a constant at some point in the future, binding the given label to
    /// its offset. The constant will be placed at most `max_distance` from the
    /// current offset.
    pub fn defer_constant(
        &mut self,
        label: MachLabel,
        align: CodeOffset,
        data: &[u8],
        max_distance: CodeOffset,
    ) {
        let deadline = self.cur_offset() + max_distance;
        self.island_worst_case_size += data.len() as CodeOffset;
        self.island_worst_case_size &= !(I::LabelUse::ALIGN - 1);
        self.pending_constants.push(MachLabelConstant {
            label,
            align,
            data: SmallVec::from(data),
        });
        if deadline < self.island_deadline {
            self.island_deadline = deadline;
        }
    }

    /// Is an island needed within the next N bytes?
    pub fn island_needed(&self, distance: CodeOffset) -> bool {
        let worst_case_end_of_island = self.cur_offset() + distance + self.island_worst_case_size;
        worst_case_end_of_island > self.island_deadline
    }

    /// Emit all pending constants and veneers. Should only be called if
    /// `island_needed()` returns true, i.e., if we actually reach a deadline:
    /// otherwise, unnecessary veneers may be inserted.
    pub fn emit_island(&mut self) {
        // We're going to purge fixups, so no latest-branch editing can happen
        // anymore.
        self.latest_branches.clear();

        let pending_constants = mem::replace(&mut self.pending_constants, SmallVec::new());
        for MachLabelConstant { label, align, data } in pending_constants.into_iter() {
            self.align_to(align);
            self.bind_label(label);
            self.put_data(&data[..]);
        }

        let fixup_records = mem::replace(&mut self.fixup_records, SmallVec::new());
        let mut new_fixups = SmallVec::new();
        for MachLabelFixup {
            label,
            offset,
            kind,
        } in fixup_records.into_iter()
        {
            trace!(
                "emit_island: fixup for label {:?} at offset {} kind {:?}",
                label,
                offset,
                kind
            );
            // We eagerly perform fixups whose label targets are known, if not out
            // of range, to avoid unnecessary veneers.
            let label_offset = self.resolve_label_offset(label);
            let known = label_offset != UNKNOWN_LABEL_OFFSET;
            let in_range = if known {
                if label_offset >= offset {
                    (label_offset - offset) <= kind.max_pos_range()
                } else {
                    (offset - label_offset) <= kind.max_neg_range()
                }
            } else {
                false
            };

            trace!(
                " -> label_offset = {}, known = {}, in_range = {} (pos {} neg {})",
                label_offset,
                known,
                in_range,
                kind.max_pos_range(),
                kind.max_neg_range()
            );

            let start = offset as usize;
            let end = (offset + kind.patch_size()) as usize;
            if in_range {
                debug_assert!(known); // implied by in_range.
                let slice = &mut self.data[start..end];
                trace!("patching in-range!");
                kind.patch(slice, offset, label_offset);
            } else if !known && !kind.supports_veneer() {
                // Nothing for now. Keep it for next round.
                new_fixups.push(MachLabelFixup {
                    label,
                    offset,
                    kind,
                });
            } else if !in_range && kind.supports_veneer() {
                // Allocate space for a veneer in the island.
                self.align_to(I::LabelUse::ALIGN);
                let veneer_offset = self.cur_offset();
                trace!("making a veneer at {}", veneer_offset);
                let slice = &mut self.data[start..end];
                // Patch the original label use to refer to teh veneer.
                trace!(
                    "patching original at offset {} to veneer offset {}",
                    offset,
                    veneer_offset
                );
                kind.patch(slice, offset, veneer_offset);
                // Generate the veneer.
                let veneer_slice = self.get_appended_space(kind.veneer_size() as usize);
                let (veneer_fixup_off, veneer_label_use) =
                    kind.generate_veneer(veneer_slice, veneer_offset);
                trace!(
                    "generated veneer; fixup offset {}, label_use {:?}",
                    veneer_fixup_off,
                    veneer_label_use
                );
                // If the label is known (but was just out of range), do the
                // veneer label-use fixup now too; otherwise, save it for later.
                if known {
                    let start = veneer_fixup_off as usize;
                    let end = (veneer_fixup_off + veneer_label_use.patch_size()) as usize;
                    let veneer_slice = &mut self.data[start..end];
                    trace!("doing veneer fixup right away too");
                    veneer_label_use.patch(veneer_slice, veneer_fixup_off, label_offset);
                } else {
                    new_fixups.push(MachLabelFixup {
                        label,
                        offset: veneer_fixup_off,
                        kind: veneer_label_use,
                    });
                }
            } else {
                panic!(
                    "Cannot support label-use {:?} (known = {}, in-range = {})",
                    kind, known, in_range
                );
            }
        }

        self.fixup_records = new_fixups;
        self.island_deadline = UNKNOWN_LABEL_OFFSET;
    }

    /// Finish any deferred emissions and/or fixups.
    pub fn finish(mut self) -> MachBufferFinalized {
        // Ensure that all labels are defined. This is a full (release-mode)
        // assert because we must avoid looping indefinitely below; an
        // unresolved label will prevent the fixup_records vec from emptying.
        assert!(self
            .label_offsets
            .iter()
            .all(|&off| off != UNKNOWN_LABEL_OFFSET));

        while !self.pending_constants.is_empty() || !self.fixup_records.is_empty() {
            // `emit_island()` will emit any pending veneers and constants, and
            // as a side-effect, will also take care of any fixups with resolved
            // labels eagerly.
            self.emit_island();
        }

        MachBufferFinalized {
            data: self.data,
            relocs: self.relocs,
            traps: self.traps,
            call_sites: self.call_sites,
            srclocs: self.srclocs,
        }
    }

    /// Add an external relocation at the current offset.
    pub fn add_reloc(
        &mut self,
        srcloc: SourceLoc,
        kind: Reloc,
        name: &ExternalName,
        addend: Addend,
    ) {
        let name = name.clone();
        self.relocs.push(MachReloc {
            offset: self.data.len() as CodeOffset,
            srcloc,
            kind,
            name,
            addend,
        });
    }

    /// Add a trap record at the current offset.
    pub fn add_trap(&mut self, srcloc: SourceLoc, code: TrapCode) {
        self.traps.push(MachTrap {
            offset: self.data.len() as CodeOffset,
            srcloc,
            code,
        });
    }

    /// Add a call-site record at the current offset.
    pub fn add_call_site(&mut self, srcloc: SourceLoc, opcode: Opcode) {
        self.call_sites.push(MachCallSite {
            ret_addr: self.data.len() as CodeOffset,
            srcloc,
            opcode,
        });
    }

    /// Set the `SourceLoc` for code from this offset until the offset at the
    /// next call to `end_srcloc()`.
    pub fn start_srcloc(&mut self, loc: SourceLoc) {
        self.cur_srcloc = Some((self.cur_offset(), loc));
    }

    /// Mark the end of the `SourceLoc` segment started at the last
    /// `start_srcloc()` call.
    pub fn end_srcloc(&mut self) {
        let (start, loc) = self
            .cur_srcloc
            .take()
            .expect("end_srcloc() called without start_srcloc()");
        let end = self.cur_offset();
        // Skip zero-length extends.
        debug_assert!(end >= start);
        if end > start {
            self.srclocs.push(MachSrcLoc { start, end, loc });
        }
    }
}

impl MachBufferFinalized {
    /// Get a list of source location mapping tuples in sorted-by-start-offset order.
    pub fn get_srclocs_sorted(&self) -> &[MachSrcLoc] {
        &self.srclocs[..]
    }

    /// Get the total required size for the code.
    pub fn total_size(&self) -> CodeOffset {
        self.data.len() as CodeOffset
    }

    /// Emit this buffer to the given CodeSink.
    pub fn emit<CS: CodeSink>(&self, sink: &mut CS) {
        // N.B.: we emit every section into the .text section as far as
        // the `CodeSink` is concerned; we do not bother to segregate
        // the contents into the actual program text, the jumptable and the
        // rodata (constant pool). This allows us to generate code assuming
        // that these will not be relocated relative to each other, and avoids
        // having to designate each section as belonging in one of the three
        // fixed categories defined by `CodeSink`. If this becomes a problem
        // later (e.g. because of memory permissions or similar), we can
        // add this designation and segregate the output; take care, however,
        // to add the appropriate relocations in this case.

        let mut next_reloc = 0;
        let mut next_trap = 0;
        let mut next_call_site = 0;
        for (idx, byte) in self.data.iter().enumerate() {
            if next_reloc < self.relocs.len() {
                let reloc = &self.relocs[next_reloc];
                if reloc.offset == idx as CodeOffset {
                    sink.reloc_external(reloc.srcloc, reloc.kind, &reloc.name, reloc.addend);
                    next_reloc += 1;
                }
            }
            if next_trap < self.traps.len() {
                let trap = &self.traps[next_trap];
                if trap.offset == idx as CodeOffset {
                    sink.trap(trap.code, trap.srcloc);
                    next_trap += 1;
                }
            }
            if next_call_site < self.call_sites.len() {
                let call_site = &self.call_sites[next_call_site];
                if call_site.ret_addr == idx as CodeOffset {
                    sink.add_call_site(call_site.opcode, call_site.srcloc);
                    next_call_site += 1;
                }
            }
            sink.put1(*byte);
        }

        sink.begin_jumptables();
        sink.begin_rodata();
        sink.end_codegen();
    }
}

/// A constant that is deferred to the next constant-pool opportunity.
struct MachLabelConstant {
    /// This label will refer to the constant's offset.
    label: MachLabel,
    /// Required alignment.
    align: CodeOffset,
    /// This data will be emitted when able.
    data: SmallVec<[u8; 16]>,
}

/// A fixup to perform on the buffer once code is emitted. Fixups always refer
/// to labels and patch the code based on label offsets. Hence, they are like
/// relocations, but internal to one buffer.
#[derive(Debug)]
struct MachLabelFixup<I: VCodeInst> {
    /// The label whose offset controls this fixup.
    label: MachLabel,
    /// The offset to fix up / patch to refer to this label.
    offset: CodeOffset,
    /// The kind of fixup. This is architecture-specific; each architecture may have,
    /// e.g., several types of branch instructions, each with differently-sized
    /// offset fields and different places within the instruction to place the
    /// bits.
    kind: I::LabelUse,
}

/// A relocation resulting from a compilation.
struct MachReloc {
    /// The offset at which the relocation applies, *relative to the
    /// containing section*.
    offset: CodeOffset,
    /// The original source location.
    srcloc: SourceLoc,
    /// The kind of relocation.
    kind: Reloc,
    /// The external symbol / name to which this relocation refers.
    name: ExternalName,
    /// The addend to add to the symbol value.
    addend: i64,
}

/// A trap record resulting from a compilation.
struct MachTrap {
    /// The offset at which the trap instruction occurs, *relative to the
    /// containing section*.
    offset: CodeOffset,
    /// The original source location.
    srcloc: SourceLoc,
    /// The trap code.
    code: TrapCode,
}

/// A call site record resulting from a compilation.
struct MachCallSite {
    /// The offset of the call's return address, *relative to the containing section*.
    ret_addr: CodeOffset,
    /// The original source location.
    srcloc: SourceLoc,
    /// The call's opcode.
    opcode: Opcode,
}

/// A source-location mapping resulting from a compilation.
#[derive(Clone, Debug)]
pub struct MachSrcLoc {
    /// The start of the region of code corresponding to a source location.
    /// This is relative to the start of the function, not to the start of the
    /// section.
    pub start: CodeOffset,
    /// The end of the region of code corresponding to a source location.
    /// This is relative to the start of the section, not to the start of the
    /// section.
    pub end: CodeOffset,
    /// The source location.
    pub loc: SourceLoc,
}

/// Record of branch instruction in the buffer, to facilitate editing.
#[derive(Clone, Debug)]
struct MachBranch {
    start: CodeOffset,
    end: CodeOffset,
    target: MachLabel,
    fixup: usize,
    inverted: Option<SmallVec<[u8; 8]>>,
}

impl MachBranch {
    fn is_cond(&self) -> bool {
        self.inverted.is_some()
    }
    fn is_uncond(&self) -> bool {
        self.inverted.is_none()
    }
}

// We use an actual instruction definition to do tests, so we depend on the `arm64` feature here.
#[cfg(all(test, feature = "arm64"))]
mod test {
    use super::*;
    use crate::isa::aarch64::inst::xreg;
    use crate::isa::aarch64::inst::{BranchTarget, CondBrKind, Inst};
    use crate::machinst::MachInstEmit;
    use crate::settings;
    use std::default::Default;

    fn label(n: u32) -> MachLabel {
        MachLabel::from_block(n)
    }
    fn target(n: u32) -> BranchTarget {
        BranchTarget::Label(label(n))
    }

    #[test]
    fn test_elide_jump_to_next() {
        let flags = settings::Flags::new(settings::builder());
        let mut buf = MachBuffer::new();
        let mut state = Default::default();

        buf.reserve_labels_for_blocks(2);
        buf.bind_label(label(0));
        let inst = Inst::Jump { dest: target(1) };
        inst.emit(&mut buf, &flags, &mut state);
        buf.bind_label(label(1));
        let buf = buf.finish();
        assert_eq!(0, buf.total_size());
    }

    #[test]
    fn test_elide_trivial_jump_blocks() {
        let flags = settings::Flags::new(settings::builder());
        let mut buf = MachBuffer::new();
        let mut state = Default::default();

        buf.reserve_labels_for_blocks(4);

        buf.bind_label(label(0));
        let inst = Inst::CondBr {
            kind: CondBrKind::NotZero(xreg(0)),
            taken: target(1),
            not_taken: target(2),
        };
        inst.emit(&mut buf, &flags, &mut state);

        buf.bind_label(label(1));
        let inst = Inst::Jump { dest: target(3) };
        inst.emit(&mut buf, &flags, &mut state);

        buf.bind_label(label(2));
        let inst = Inst::Jump { dest: target(3) };
        inst.emit(&mut buf, &flags, &mut state);

        buf.bind_label(label(3));

        let buf = buf.finish();
        assert_eq!(0, buf.total_size());
    }

    #[test]
    fn test_flip_cond() {
        let flags = settings::Flags::new(settings::builder());
        let mut buf = MachBuffer::new();
        let mut state = Default::default();

        buf.reserve_labels_for_blocks(4);

        buf.bind_label(label(0));
        let inst = Inst::CondBr {
            kind: CondBrKind::NotZero(xreg(0)),
            taken: target(1),
            not_taken: target(2),
        };
        inst.emit(&mut buf, &flags, &mut state);

        buf.bind_label(label(1));
        let inst = Inst::Nop4;
        inst.emit(&mut buf, &flags, &mut state);

        buf.bind_label(label(2));
        let inst = Inst::Nop4;
        inst.emit(&mut buf, &flags, &mut state);

        buf.bind_label(label(3));

        let buf = buf.finish();

        let mut buf2 = MachBuffer::new();
        let mut state = Default::default();
        let inst = Inst::OneWayCondBr {
            kind: CondBrKind::Zero(xreg(0)),
            target: BranchTarget::ResolvedOffset(8),
        };
        inst.emit(&mut buf2, &flags, &mut state);
        let inst = Inst::Nop4;
        inst.emit(&mut buf2, &flags, &mut state);
        inst.emit(&mut buf2, &flags, &mut state);

        let buf2 = buf2.finish();

        assert_eq!(buf.data, buf2.data);
    }

    #[test]
    fn test_island() {
        let flags = settings::Flags::new(settings::builder());
        let mut buf = MachBuffer::new();
        let mut state = Default::default();

        buf.reserve_labels_for_blocks(4);

        buf.bind_label(label(0));
        let inst = Inst::CondBr {
            kind: CondBrKind::NotZero(xreg(0)),
            taken: target(2),
            not_taken: target(3),
        };
        inst.emit(&mut buf, &flags, &mut state);

        buf.bind_label(label(1));
        while buf.cur_offset() < 2000000 {
            if buf.island_needed(0) {
                buf.emit_island();
            }
            let inst = Inst::Nop4;
            inst.emit(&mut buf, &flags, &mut state);
        }

        buf.bind_label(label(2));
        let inst = Inst::Nop4;
        inst.emit(&mut buf, &flags, &mut state);

        buf.bind_label(label(3));
        let inst = Inst::Nop4;
        inst.emit(&mut buf, &flags, &mut state);

        let buf = buf.finish();

        assert_eq!(2000000 + 8, buf.total_size());

        let mut buf2 = MachBuffer::new();
        let mut state = Default::default();
        let inst = Inst::CondBr {
            kind: CondBrKind::NotZero(xreg(0)),
            taken: BranchTarget::ResolvedOffset(1048576 - 4),
            not_taken: BranchTarget::ResolvedOffset(2000000 + 4 - 4),
        };
        inst.emit(&mut buf2, &flags, &mut state);

        let buf2 = buf2.finish();

        assert_eq!(&buf.data[0..8], &buf2.data[..]);
    }

    #[test]
    fn test_island_backward() {
        let flags = settings::Flags::new(settings::builder());
        let mut buf = MachBuffer::new();
        let mut state = Default::default();

        buf.reserve_labels_for_blocks(4);

        buf.bind_label(label(0));
        let inst = Inst::Nop4;
        inst.emit(&mut buf, &flags, &mut state);

        buf.bind_label(label(1));
        let inst = Inst::Nop4;
        inst.emit(&mut buf, &flags, &mut state);

        buf.bind_label(label(2));
        while buf.cur_offset() < 2000000 {
            let inst = Inst::Nop4;
            inst.emit(&mut buf, &flags, &mut state);
        }

        buf.bind_label(label(3));
        let inst = Inst::CondBr {
            kind: CondBrKind::NotZero(xreg(0)),
            taken: target(0),
            not_taken: target(1),
        };
        inst.emit(&mut buf, &flags, &mut state);

        let buf = buf.finish();

        assert_eq!(2000000 + 12, buf.total_size());

        let mut buf2 = MachBuffer::new();
        let mut state = Default::default();
        let inst = Inst::CondBr {
            kind: CondBrKind::NotZero(xreg(0)),
            taken: BranchTarget::ResolvedOffset(8),
            not_taken: BranchTarget::ResolvedOffset(4 - (2000000 + 4)),
        };
        inst.emit(&mut buf2, &flags, &mut state);
        let inst = Inst::Jump {
            dest: BranchTarget::ResolvedOffset(-(2000000 + 8)),
        };
        inst.emit(&mut buf2, &flags, &mut state);

        let buf2 = buf2.finish();

        assert_eq!(&buf.data[2000000..], &buf2.data[..]);
    }
}
