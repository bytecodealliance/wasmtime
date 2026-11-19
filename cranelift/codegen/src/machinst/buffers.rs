//! The buffers that `MachInstEmit::emit` writes to.
//!
//! [`MachBuffers`] derefs to the [`MachBuffer`] for the function's code.

use crate::binemit::CodeOffset;
use crate::ir;
use crate::machinst::{MachBuffer, MachBufferFinalized, MachLabel, VCodeConstants, VCodeInst};
use core::ops::{Deref, DerefMut};
use cranelift_control::ControlPlane;

/// The buffer for a function's code.
pub struct MachBuffers<'a, I: VCodeInst> {
    text: &'a mut MachBuffer<I>,
}

impl<'a, I: VCodeInst> MachBuffers<'a, I> {
    /// Wraps the buffer for a function's code.
    pub fn text_only(text: &'a mut MachBuffer<I>) -> Self {
        MachBuffers { text }
    }

    // These forward to the code's buffer. Called through `Deref` they cannot
    // take `cur_offset()` of the same buffer as an argument.

    /// See [`MachBuffer::use_label_at_offset`].
    pub fn use_label_at_offset(&mut self, offset: CodeOffset, label: MachLabel, kind: I::LabelUse) {
        self.text.use_label_at_offset(offset, label, kind);
    }

    /// See [`MachBuffer::add_patchable_call_site`].
    pub fn add_patchable_call_site(&mut self, len: u32) {
        self.text.add_patchable_call_site(len);
    }

    /// See [`MachBuffer::push_user_stack_map`].
    pub fn push_user_stack_map(
        &mut self,
        emit_state: &I::State,
        pc: CodeOffset,
        stack_map: ir::UserStackMap,
    ) {
        self.text.push_user_stack_map(emit_state, pc, stack_map);
    }

    /// Finishes the buffer.
    pub(crate) fn finish(
        self,
        constants: &VCodeConstants,
        ctrl_plane: &mut ControlPlane,
    ) -> MachBufferFinalized {
        let text = core::mem::replace(self.text, MachBuffer::new());
        text.finish(constants, ctrl_plane)
    }
}

impl<I: VCodeInst> Deref for MachBuffers<'_, I> {
    type Target = MachBuffer<I>;

    fn deref(&self) -> &Self::Target {
        self.text
    }
}

impl<I: VCodeInst> DerefMut for MachBuffers<'_, I> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.text
    }
}

impl<I: VCodeInst> Extend<u8> for MachBuffers<'_, I> {
    fn extend<T: IntoIterator<Item = u8>>(&mut self, iter: T) {
        self.text.extend(iter);
    }
}
