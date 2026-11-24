//! When configured to emit constants into a separate section, we use a `MachBuffer` for each
//! section in the output (the equivalent of `.text` and `.rodata`). `MachBuffers` contains borrows
//! to two `MachBuffer`s, one for each section, and abstracts the relocations that let `.text`
//! refer to `.rodata`. Instruction emission takes a `MachBuffers` so it can emit both
//! machine code and data as needed.

use crate::binemit::{Addend, CodeOffset, Reloc};
use crate::machinst::{
    MachBuffer, MachBufferFinalized, MachInstLabelUse, MachLabel, VCodeConstant, VCodeInst,
};
use alloc::vec::Vec;
use core::ops::{Deref, DerefMut};
use cranelift_control::ControlPlane;
use cranelift_entity::SecondaryMap;

/// A reference from the code to a constant.
///
/// The embedder resolves it like a relocation against the constant. The result is added to the
/// bytes already at `offset`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "enable-serde",
    derive(serde_derive::Serialize, serde_derive::Deserialize)
)]
pub struct MachConstantRef {
    /// Where the reference is in the code.
    pub offset: CodeOffset,
    /// The kind of relocation.
    pub kind: Reloc,
    /// The addend, as in `MachReloc::addend`.
    pub addend: Addend,
    /// The constant, as an index into `MachConstantsFinalized::offsets`.
    pub constant: usize,
}

/// The constants of a function and the references to them from its code.
#[derive(PartialEq, Debug, Clone)]
#[cfg_attr(
    feature = "enable-serde",
    derive(serde_derive::Serialize, serde_derive::Deserialize)
)]
pub struct MachConstantsFinalized {
    /// The constants. Its `min_alignment` is at least the alignment of each constant.
    pub buffer: MachBufferFinalized,
    /// Where each constant starts in `buffer`, in the order the code first used them.
    pub offsets: Vec<CodeOffset>,
    /// The references from the code, in the order they were emitted.
    pub refs: Vec<MachConstantRef>,
}

/// A reference from the code to a constant, before the constant has an offset.
struct ConstantUse {
    text_offset: CodeOffset,
    constant: usize,
    kind: Reloc,
    addend: Addend,
}

/// The buffers that a function is emitted into.
///
/// Without the `separate_constant_pool` setting there is only the buffer for the code. With it,
/// there is a second one for the constants.
pub struct MachBuffers<'a, I: VCodeInst> {
    text: &'a mut MachBuffer<I>,
    rodata: Option<&'a mut MachBuffer<I>>,
    /// The index in `labels` of each constant used so far.
    indices: SecondaryMap<VCodeConstant, Option<usize>>,
    /// The label in `rodata` of each constant used so far, in order of first use.
    labels: Vec<MachLabel>,
    uses: Vec<ConstantUse>,
}

impl<'a, I: VCodeInst> MachBuffers<'a, I> {
    /// Creates buffers from a buffer for the code and, optionally, one for the constants.
    ///
    /// The constants must be registered with the buffer that holds them.
    pub(crate) fn new(text: &'a mut MachBuffer<I>, rodata: Option<&'a mut MachBuffer<I>>) -> Self {
        MachBuffers {
            text,
            rodata,
            indices: SecondaryMap::new(),
            labels: Vec::new(),
            uses: Vec::new(),
        }
    }

    /// Creates buffers with no second buffer, so that constants are emitted into the code.
    pub fn text_only(text: &'a mut MachBuffer<I>) -> Self {
        Self::new(text, None)
    }

    /// Makes the code at `text_offset` refer to `constant`, with a relocation of `kind` and
    /// `addend`.
    ///
    /// Without a second buffer, the constant goes in the code buffer and the reference is a label
    /// use. With one, the reference is kept for the embedder to resolve.
    pub fn use_constant(
        &mut self,
        text_offset: CodeOffset,
        constant: VCodeConstant,
        kind: Reloc,
        addend: Addend,
    ) {
        match self.rodata.as_deref_mut() {
            None => {
                let label_use = I::LabelUse::from_reloc(kind, addend)
                    .unwrap_or_else(|| panic!("no label use for a constant reference {kind:?}"));
                let label = self.text.get_label_for_constant(constant);
                self.text.use_label_at_offset(text_offset, label, label_use);
            }
            Some(rodata) => {
                let index = *self.indices[constant].get_or_insert_with(|| {
                    self.labels.push(rodata.get_label_for_constant(constant));
                    self.labels.len() - 1
                });
                self.uses.push(ConstantUse {
                    text_offset,
                    constant: index,
                    kind,
                    addend,
                });
            }
        }
    }

    /// Places the constants that were used in the second buffer.
    ///
    /// Returns where each one starts and the references to them. This consumes `self`, so the
    /// buffers can be finished after it.
    pub(crate) fn finish(
        self,
        ctrl_plane: &mut ControlPlane,
    ) -> (Vec<CodeOffset>, Vec<MachConstantRef>) {
        let Some(rodata) = self.rodata else {
            return (Vec::new(), Vec::new());
        };
        // Nothing else is emitted into this buffer, so the island holds exactly the constants.
        rodata.emit_island(0, ctrl_plane);
        let offsets = self
            .labels
            .iter()
            .map(|&label| rodata.resolve_label_offset(label))
            .collect();
        let refs = self
            .uses
            .iter()
            .map(|u| MachConstantRef {
                offset: u.text_offset,
                kind: u.kind,
                addend: u.addend,
                constant: u.constant,
            })
            .collect();
        (offsets, refs)
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

/// Pulley's encoders take any `Extend<u8>`, which does not resolve through `Deref`.
impl<I: VCodeInst> Extend<u8> for MachBuffers<'_, I> {
    fn extend<T: IntoIterator<Item = u8>>(&mut self, iter: T) {
        self.text.extend(iter);
    }
}
