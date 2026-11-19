//! A pair of buffers for the code of a function and its constants.
//!
//! With the `separate_constant_pool` setting, a function's constants are not
//! emitted into its code. They go to a second [`MachBuffer`], which the
//! embedder places in a section of its own, and the code refers to them with
//! relocations. [`MachBuffers`] is what `MachInstEmit::emit` writes to. It
//! derefs to the buffer for code, and it links a place in the code to a
//! constant in the second buffer.

use crate::binemit::{Addend, CodeOffset, Reloc};
use crate::ir;
use crate::machinst::{
    MachBuffer, MachBufferFinalized, MachInstLabelUse, MachLabel, VCodeConstant, VCodeConstants,
    VCodeInst,
};
use alloc::vec::Vec;
use core::ops::{Deref, DerefMut};
use cranelift_control::ControlPlane;

/// A reference from code to a constant in the buffer of a
/// [`MachConstantsFinalized`].
///
/// It is resolved like a [`MachReloc`](crate::MachReloc) whose target is the
/// address of the constant, except that, as for a label use, the result is
/// added to the bytes already at `offset`. On x86-64 they hold minus the size
/// of any immediate after the displacement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "enable-serde",
    derive(serde_derive::Serialize, serde_derive::Deserialize)
)]
pub struct MachConstantRef {
    /// Offset of the reference in the code.
    pub offset: CodeOffset,
    /// The kind of relocation to apply.
    pub kind: Reloc,
    /// The addend, as for `MachReloc::addend`.
    pub addend: Addend,
    /// Offset of the referenced constant in the constants buffer.
    pub constant_offset: CodeOffset,
}

/// The constants of a function, in a buffer of their own, and the references
/// to them from the function's code.
#[derive(PartialEq, Debug, Clone)]
#[cfg_attr(
    feature = "enable-serde",
    derive(serde_derive::Serialize, serde_derive::Deserialize)
)]
pub struct MachConstantsFinalized {
    /// The constants. Its `min_alignment` covers the alignment of each of
    /// them.
    pub buffer: MachBufferFinalized,
    /// The references from the code, in order of emission.
    pub refs: Vec<MachConstantRef>,
}

/// A reference from code to a label of the constants buffer.
struct ConstantUse {
    text_offset: CodeOffset,
    label: MachLabel,
    kind: Reloc,
    addend: Addend,
}

/// The buffer for a function's code and, with the `separate_constant_pool`
/// setting, the one for its constants.
pub struct MachBuffers<'a, I: VCodeInst> {
    text: &'a mut MachBuffer<I>,
    rodata: Option<&'a mut MachBuffer<I>>,
    uses: Vec<ConstantUse>,
}

impl<'a, I: VCodeInst> MachBuffers<'a, I> {
    /// Creates a pair from the buffers for code and constants.
    pub(crate) fn new(text: &'a mut MachBuffer<I>, rodata: Option<&'a mut MachBuffer<I>>) -> Self {
        MachBuffers {
            text,
            rodata,
            uses: Vec::new(),
        }
    }

    /// Creates a pair that has no buffer for constants, so that they are
    /// emitted into the code.
    pub fn text_only(text: &'a mut MachBuffer<I>) -> Self {
        Self::new(text, None)
    }

    /// Registers the function's constants with the buffer that holds them.
    ///
    /// With a buffer for constants the code's buffer has none registered, so
    /// asking it for a constant through `Deref` panics.
    pub(crate) fn register_constants(&mut self, constants: &VCodeConstants) {
        match self.rodata.as_deref_mut() {
            Some(rodata) => rodata.register_constants(constants),
            None => self.text.register_constants(constants),
        }
    }

    /// Makes the code at `text_offset` refer to `constant` with a relocation
    /// of `kind` and `addend`.
    ///
    /// With a buffer for constants, the constant is placed there the first
    /// time it is used and the relocation is recorded. Without one, the
    /// constant goes to an island in the code, and the reference is the label
    /// use that `MachInstLabelUse::from_reloc` gives for the relocation.
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
                let label = rodata.get_label_for_constant(constant);
                self.uses.push(ConstantUse {
                    text_offset,
                    label,
                    kind,
                    addend,
                });
            }
        }
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

    /// Finishes both buffers. The second result is `None` without a buffer for
    /// constants or if the code refers to none.
    pub(crate) fn finish(
        self,
        constants: &VCodeConstants,
        ctrl_plane: &mut ControlPlane,
    ) -> (MachBufferFinalized, Option<MachConstantsFinalized>) {
        let rodata = match self.rodata {
            Some(rodata) if !self.uses.is_empty() => {
                // The buffer holds no code, so this is a place for an island
                // and it writes out the constants that were used.
                rodata.emit_island(0, ctrl_plane);
                let refs = self
                    .uses
                    .iter()
                    .map(|u| MachConstantRef {
                        offset: u.text_offset,
                        kind: u.kind,
                        addend: u.addend,
                        constant_offset: rodata.resolve_label_offset(u.label),
                    })
                    .collect();
                let buffer = core::mem::replace(rodata, MachBuffer::new());
                Some(MachConstantsFinalized {
                    buffer: buffer.finish(constants, ctrl_plane),
                    refs,
                })
            }
            _ => None,
        };
        let text = core::mem::replace(self.text, MachBuffer::new());
        (text.finish(constants, ctrl_plane), rodata)
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

#[cfg(all(test, feature = "arm64"))]
mod test {
    use super::*;
    use crate::ir::MemFlagsData;
    use crate::isa::aarch64;
    use crate::isa::aarch64::inst::{AMode, EmitInfo, Inst, writable_vreg, writable_xreg, xreg};
    use crate::machinst::{
        MachInstEmit, MachInstEmitState, MachTextSectionBuilder, TextSectionBuilder,
        VCodeConstantData,
    };
    use crate::settings;

    fn emit_info() -> EmitInfo {
        let flags = settings::Flags::new(settings::builder());
        let isa_flags = aarch64::settings::Flags::new(&flags, &aarch64::settings::builder());
        EmitInfo::new(flags, isa_flags)
    }

    #[test]
    fn constants_are_placed_at_first_use() {
        let mut constants = VCodeConstants::with_capacity(3);
        let a = constants.insert(VCodeConstantData::U64([1, 2, 3, 4, 5, 6, 7, 8]));
        let b = constants.insert(VCodeConstantData::Generated(
            (0..16).collect::<Vec<u8>>().into(),
        ));
        let _unused = constants.insert(VCodeConstantData::U64([9; 8]));

        let mut text = MachBuffer::<Inst>::new();
        let mut rodata = MachBuffer::<Inst>::new();
        let mut buffers = MachBuffers::new(&mut text, Some(&mut rodata));
        buffers.register_constants(&constants);
        buffers.use_constant(4, b, Reloc::Aarch64AdrPrelPgHi21, 0);
        buffers.use_constant(12, a, Reloc::Aarch64AdrPrelPgHi21, 0);
        buffers.use_constant(20, b, Reloc::Aarch64AdrPrelPgHi21, 0);
        let mut ctrl_plane = ControlPlane::default();
        let (_, finished) = buffers.finish(&constants, &mut ctrl_plane);
        let finished = finished.unwrap();

        // The first constant used comes first. The second needs 8 bytes of
        // alignment, and the buffer is as aligned as the largest of them.
        let data = finished.buffer.data();
        assert_eq!(data.len(), 24);
        assert_eq!(&data[..16], &(0..16).collect::<Vec<u8>>());
        assert_eq!(&data[16..], &[1, 2, 3, 4, 5, 6, 7, 8]);
        assert!(finished.buffer.min_alignment >= 16);
        let offsets: Vec<_> = finished.refs.iter().map(|r| r.constant_offset).collect();
        assert_eq!(offsets, [0, 16, 0]);
    }

    #[test]
    fn aarch64_constant_load_resolves_through_the_text_builder() {
        let info = emit_info();
        let mut state = <Inst as MachInstEmit>::State::default();
        let mut text = MachBuffer::new();
        let mut rodata = MachBuffer::new();
        let mut constants = VCodeConstants::with_capacity(1);
        let constant = constants.insert(VCodeConstantData::U64([1, 2, 3, 4, 5, 6, 7, 8]));
        let mut buffers = MachBuffers::new(&mut text, Some(&mut rodata));
        buffers.register_constants(&constants);

        // `adrp x0, <page of constant>` and `ldr d0, [x0, <offset in page>]`.
        Inst::AdrpConst {
            rd: writable_xreg(0),
            constant,
        }
        .emit(&mut buffers, &info, &mut state);
        Inst::FpuLoad64 {
            rd: writable_vreg(0),
            mem: AMode::ConstPageOff {
                rn: xreg(0),
                addr: constant,
            },
            flags: MemFlagsData::trusted(),
        }
        .emit(&mut buffers, &info, &mut state);
        let (code, constants) = buffers.finish(&constants, state.ctrl_plane_mut());
        let constants = constants.unwrap();

        // The constant is not in the code. The code refers to it with a
        // relocation for each of the two instructions.
        assert_eq!(code.data().len(), 8);
        assert_eq!(constants.buffer.data(), &[1, 2, 3, 4, 5, 6, 7, 8]);
        let reloc = |offset, kind| MachConstantRef {
            offset,
            kind,
            addend: 0,
            constant_offset: 0,
        };
        assert_eq!(
            constants.refs,
            [
                reloc(0, Reloc::Aarch64AdrPrelPgHi21),
                reloc(4, Reloc::Aarch64Ldst64AbsLo12Nc),
            ]
        );

        // Place the code, then a page of padding, then the constant, which
        // lands at 0x14028: 0x14 pages and 0x28 bytes on from the code.
        let mut ctrl_plane = ControlPlane::default();
        let mut text = MachTextSectionBuilder::<Inst>::new(2);
        text.append(true, code.data(), 4, &mut ctrl_plane);
        text.append(false, &[0; 0x14020], 1, &mut ctrl_plane);
        text.append(true, constants.buffer.data(), 8, &mut ctrl_plane);
        for r in &constants.refs {
            assert!(text.resolve_reloc(r.offset.into(), r.kind, r.addend, 1));
        }
        let code = text.finish(&mut ctrl_plane);
        let words: Vec<u32> = code[..8]
            .chunks(4)
            .map(|w| u32::from_le_bytes(w.try_into().unwrap()))
            .collect();
        // adrp x0, #0x14000
        assert_eq!(words[0], 0x9000_00a0);
        // ldr d0, [x0, #0x28]
        assert_eq!(words[1], 0xfd40_1400);
    }
}
