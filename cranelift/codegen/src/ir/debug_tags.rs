//! Debug tag storage.
//!
//! Cranelift permits the embedder to place "debug tags" on
//! instructions in CLIF. These tags are sequences of items of various
//! kinds, with no other meaning imposed by Cranelift. They are passed
//! through to metadata provided alongside the compilation result.
//!
//! When Cranelift inlines a function, it will prepend any tags from
//! the call instruction at the inlining callsite to tags on all
//! inlined instructions.
//!
//! These tags can be used, for example, to identify stackslots that
//! store user state, or to denote positions in user source. In
//! general, the intent is to allow perfect reconstruction of original
//! (source-level) program state in an instrumentation-based
//! debug-info scheme, as long as the instruction(s) on which these
//! tags are attached are preserved. This will be the case for any
//! instructions with side-effects.
//!
//! A few answers to design questions that lead to this design:
//!
//! - Why not use the SourceLoc mechanism? Debug tags are richer than
//!   that infrastructure because they preserve inlining location and
//!   are interleaved properly with any other tags describing the
//!   frame.
//! - Why not attach debug tags only to special sequence-point
//!   instructions? This is driven by inlining: we should have the
//!   semantic information about a callsite attached directly to the
//!   call and observe it there, not have a magic "look backward to
//!   find a sequence point" behavior in the inliner.
//!
//! In other words, the needs of preserving "virtual" frames across an
//! inlining transform drive this design.

use crate::ir::{Inst, StackSlot};
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::ops::Range;

/// Debug tags for instructions.
#[derive(Clone, PartialEq, Hash, Default)]
#[cfg_attr(
    feature = "enable-serde",
    derive(serde_derive::Serialize, serde_derive::Deserialize)
)]
pub struct DebugTags {
    /// Pool of tags, referred to by `insts` below.
    tags: Vec<DebugTag>,

    /// Per-instruction range for its list of tags in the tag pool (if
    /// any).
    ///
    /// Note: we don't use `PackedOption` and `EntityList` here
    /// because the values that we are storing are not entities.
    insts: BTreeMap<Inst, Range<u32>>,
}

/// One debug tag.
#[derive(Clone, Debug, PartialEq, Hash)]
#[cfg_attr(
    feature = "enable-serde",
    derive(serde_derive::Serialize, serde_derive::Deserialize)
)]
pub enum DebugTag {
    /// User-specified `u32` value, opaque to Cranelift.
    User(u32),

    /// A stack slot reference.
    StackSlot(StackSlot),
}

impl DebugTags {
    /// Set the tags on an instruction, overwriting existing tag list.
    ///
    /// Tags can only be set on call instructions (those for which
    /// [`crate::ir::Opcode::is_call()`] returns `true`) and on
    /// `sequence_point` instructions. This property is checked by the
    /// CLIF verifier.
    pub fn set(&mut self, inst: Inst, tags: impl IntoIterator<Item = DebugTag>) {
        let start = u32::try_from(self.tags.len()).unwrap();
        self.tags.extend(tags);
        let end = u32::try_from(self.tags.len()).unwrap();
        if end > start {
            self.insts.insert(inst, start..end);
        } else {
            self.insts.remove(&inst);
        }
    }

    /// Get the tags associated with an instruction.
    pub fn get(&self, inst: Inst) -> &[DebugTag] {
        if let Some(range) = self.insts.get(&inst) {
            let start = usize::try_from(range.start).unwrap();
            let end = usize::try_from(range.end).unwrap();
            &self.tags[start..end]
        } else {
            &[]
        }
    }

    /// Does the given instruction have any tags?
    pub fn has(&self, inst: Inst) -> bool {
        // We rely on the invariant that an entry in the map is
        // present only if the list range is non-empty.
        self.insts.contains_key(&inst)
    }

    /// Clone the tags from one instruction to another.
    ///
    /// This clone is cheap (references the same underlying storage)
    /// because the tag lists are immutable.
    pub fn clone_tags(&mut self, from: Inst, to: Inst) {
        if let Some(range) = self.insts.get(&from).cloned() {
            self.insts.insert(to, range);
        } else {
            self.insts.remove(&to);
        }
    }

    /// Are any debug tags present?
    ///
    /// This is used for adjusting margins when pretty-printing CLIF.
    pub fn is_empty(&self) -> bool {
        self.insts.is_empty()
    }

    /// Clear all tags.
    pub fn clear(&mut self) {
        self.insts.clear();
        self.tags.clear();
    }
}

impl core::fmt::Display for DebugTag {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            DebugTag::User(value) => write!(f, "{value}"),
            DebugTag::StackSlot(slot) => write!(f, "{slot}"),
        }
    }
}
