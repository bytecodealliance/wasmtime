//! User-defined stack maps.
//!
//! This module provides types allowing users to define stack maps and associate
//! them with safepoints.
//!
//! A **safepoint** is a program point (i.e. CLIF instruction) where it must be
//! safe to run GC. Currently all non-tail call instructions are considered
//! safepoints. (This does *not* allow, for example, skipping safepoints for
//! calls that are statically known not to trigger collections, or to have a
//! safepoint on a volatile load to a page that gets protected when it is time
//! to GC, triggering a fault that pauses the mutator and lets the collector do
//! its work before resuming the mutator. We can lift this restriction in the
//! future, if necessary.)
//!
//! A **stack map** is a description of where to find all the GC-managed values
//! that are live at a particular safepoint. Stack maps let the collector find
//! on-stack roots. Each stack map is logically a set of offsets into the stack
//! frame and the type of value at that associated offset. However, because the
//! stack layout isn't defined until much later in the compiler's pipeline, each
//! stack map entry instead includes both an `ir::StackSlot` and an offset
//! within that slot.
//!
//! These stack maps are **user-defined** in that it is the CLIF producer's
//! responsibility to identify and spill the live GC-managed values and attach
//! the associated stack map entries to each safepoint themselves (see
//! `cranelift_frontend::Function::declare_needs_stack_map` and
//! `cranelift_codegen::ir::DataFlowGraph::append_user_stack_map_entry`). Cranelift
//! will not insert spills and record these stack map entries automatically (in
//! contrast to the old system and its `r64` values).

use crate::ir;
use smallvec::SmallVec;

pub(crate) type UserStackMapEntryVec = SmallVec<[UserStackMapEntry; 4]>;

/// A stack map entry describes a GC-managed value and its location at a
/// particular instruction.
#[derive(Clone, PartialEq, Hash)]
#[cfg_attr(
    feature = "enable-serde",
    derive(serde_derive::Serialize, serde_derive::Deserialize)
)]
pub struct UserStackMapEntry {
    /// The type of the value stored in this stack map entry.
    pub ty: ir::Type,

    /// The stack slot that this stack map entry is within.
    pub slot: ir::StackSlot,

    /// The offset within the stack slot where this entry's value can be found.
    pub offset: u32,
}
