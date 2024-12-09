//! Pulley binary code emission.

use super::*;
use crate::ir::{self, Endianness};
use crate::isa::pulley_shared::abi::PulleyMachineDeps;
use crate::isa::pulley_shared::PointerWidth;
use core::marker::PhantomData;
use cranelift_control::ControlPlane;
use pulley_interpreter::encode as enc;
use pulley_interpreter::regs::BinaryOperands;

pub struct EmitInfo {
    #[allow(dead_code)] // Will get used as we fill out this backend.
    shared_flags: settings::Flags,

    #[allow(dead_code)] // Will get used as we fill out this backend.
    isa_flags: crate::isa::pulley_shared::settings::Flags,
}

impl EmitInfo {
    pub(crate) fn new(
        shared_flags: settings::Flags,
        isa_flags: crate::isa::pulley_shared::settings::Flags,
    ) -> Self {
        Self {
            shared_flags,
            isa_flags,
        }
    }

    fn endianness(&self, flags: MemFlags) -> Endianness {
        let target_endianness = if self.isa_flags.big_endian() {
            Endianness::Big
        } else {
            Endianness::Little
        };
        flags.endianness(target_endianness)
    }
}

/// State carried between emissions of a sequence of instructions.
#[derive(Default, Clone, Debug)]
pub struct EmitState<P>
where
    P: PulleyTargetKind,
{
    _phantom: PhantomData<P>,
    ctrl_plane: ControlPlane,
    user_stack_map: Option<ir::UserStackMap>,
    frame_layout: FrameLayout,
}

impl<P> EmitState<P>
where
    P: PulleyTargetKind,
{
    fn take_stack_map(&mut self) -> Option<ir::UserStackMap> {
        self.user_stack_map.take()
    }
}

impl<P> MachInstEmitState<InstAndKind<P>> for EmitState<P>
where
    P: PulleyTargetKind,
{
    fn new(abi: &Callee<PulleyMachineDeps<P>>, ctrl_plane: ControlPlane) -> Self {
        EmitState {
            _phantom: PhantomData,
            ctrl_plane,
            user_stack_map: None,
            frame_layout: abi.frame_layout().clone(),
        }
    }

    fn pre_safepoint(&mut self, user_stack_map: Option<ir::UserStackMap>) {
        self.user_stack_map = user_stack_map;
    }

    fn ctrl_plane_mut(&mut self) -> &mut ControlPlane {
        &mut self.ctrl_plane
    }

    fn take_ctrl_plane(self) -> ControlPlane {
        self.ctrl_plane
    }

    fn frame_layout(&self) -> &FrameLayout {
        &self.frame_layout
    }
}

impl<P> MachInstEmit for InstAndKind<P>
where
    P: PulleyTargetKind,
{
    type State = EmitState<P>;
    type Info = EmitInfo;

    fn emit(&self, sink: &mut MachBuffer<Self>, emit_info: &Self::Info, state: &mut Self::State) {
        // N.B.: we *must* not exceed the "worst-case size" used to compute
        // where to insert islands, except when islands are explicitly triggered
        // (with an `EmitIsland`). We check this in debug builds. This is `mut`
        // to allow disabling the check for `JTSequence`, which is always
        // emitted following an `EmitIsland`.
        let mut start = sink.cur_offset();
        pulley_emit(self, sink, emit_info, state, &mut start);

        let end = sink.cur_offset();
        assert!(
            (end - start) <= InstAndKind::<P>::worst_case_size(),
            "encoded inst {self:?} longer than worst-case size: length: {}, Inst::worst_case_size() = {}",
            end - start,
            InstAndKind::<P>::worst_case_size()
        );
    }

    fn pretty_print_inst(&self, state: &mut Self::State) -> String {
        self.print_with_state(state)
    }
}

fn pulley_emit<P>(
    inst: &Inst,
    sink: &mut MachBuffer<InstAndKind<P>>,
    emit_info: &EmitInfo,
    state: &mut EmitState<P>,
    start_offset: &mut u32,
) where
    P: PulleyTargetKind,
{
    match inst {
        // Pseduo-instructions that don't actually encode to anything.
        Inst::Args { .. } | Inst::Rets { .. } | Inst::Unwind { .. } => {}

        Inst::TrapIf {
            cond,
            size,
            src1,
            src2,
            code,
        } => {
            let label = sink.defer_trap(*code);

            let cur_off = sink.cur_offset();
            sink.use_label_at_offset(cur_off + 3, label, LabelUse::Jump(3));

            use ir::condcodes::IntCC::*;
            use OperandSize::*;
            match (cond, size) {
                (Equal, Size32) => enc::br_if_xeq32(sink, src1, src2, 0),
                (Equal, Size64) => enc::br_if_xeq64(sink, src1, src2, 0),

                (NotEqual, Size32) => enc::br_if_xneq32(sink, src1, src2, 0),
                (NotEqual, Size64) => enc::br_if_xneq64(sink, src1, src2, 0),

                (SignedLessThan, Size32) => enc::br_if_xslt32(sink, src1, src2, 0),
                (SignedLessThan, Size64) => enc::br_if_xslt64(sink, src1, src2, 0),

                (SignedLessThanOrEqual, Size32) => enc::br_if_xslteq32(sink, src1, src2, 0),
                (SignedLessThanOrEqual, Size64) => enc::br_if_xslteq64(sink, src1, src2, 0),

                (UnsignedLessThan, Size32) => enc::br_if_xult32(sink, src1, src2, 0),
                (UnsignedLessThan, Size64) => enc::br_if_xult64(sink, src1, src2, 0),

                (UnsignedLessThanOrEqual, Size32) => enc::br_if_xulteq32(sink, src1, src2, 0),
                (UnsignedLessThanOrEqual, Size64) => enc::br_if_xulteq64(sink, src1, src2, 0),

                (SignedGreaterThan, Size32) => enc::br_if_xslt32(sink, src2, src1, 0),
                (SignedGreaterThan, Size64) => enc::br_if_xslt64(sink, src2, src1, 0),

                (SignedGreaterThanOrEqual, Size32) => enc::br_if_xslteq32(sink, src2, src1, 0),
                (SignedGreaterThanOrEqual, Size64) => enc::br_if_xslteq64(sink, src2, src1, 0),

                (UnsignedGreaterThan, Size32) => enc::br_if_xult32(sink, src2, src1, 0),
                (UnsignedGreaterThan, Size64) => enc::br_if_xult64(sink, src2, src1, 0),

                (UnsignedGreaterThanOrEqual, Size32) => enc::br_if_xulteq32(sink, src2, src1, 0),
                (UnsignedGreaterThanOrEqual, Size64) => enc::br_if_xulteq64(sink, src2, src1, 0),
            }
        }

        Inst::Nop => todo!(),

        Inst::GetSpecial { dst, reg } => enc::xmov(sink, dst, reg),

        Inst::LoadExtName { .. } => todo!(),

        Inst::Call { info } => {
            sink.put1(pulley_interpreter::Opcode::Call as u8);
            sink.add_reloc(
                // TODO: is it actually okay to reuse this reloc here?
                Reloc::X86CallPCRel4,
                &info.dest,
                // This addend adjusts for the difference between the start of
                // the instruction and the beginning of the immediate field.
                -1,
            );
            sink.put4(0);
            if let Some(s) = state.take_stack_map() {
                let offset = sink.cur_offset();
                sink.push_user_stack_map(state, offset, s);
            }
            sink.add_call_site();

            let adjust = -i32::try_from(info.callee_pop_size).unwrap();
            for i in PulleyMachineDeps::<P>::gen_sp_reg_adjust(adjust) {
                <InstAndKind<P>>::from(i).emit(sink, emit_info, state);
            }
        }

        Inst::IndirectCall { info } => {
            enc::call_indirect(sink, info.dest);

            if let Some(s) = state.take_stack_map() {
                let offset = sink.cur_offset();
                sink.push_user_stack_map(state, offset, s);
            }

            sink.add_call_site();

            let adjust = -i32::try_from(info.callee_pop_size).unwrap();
            for i in PulleyMachineDeps::<P>::gen_sp_reg_adjust(adjust) {
                <InstAndKind<P>>::from(i).emit(sink, emit_info, state);
            }
        }

        Inst::IndirectCallHost { info } => {
            // Emit a relocation to fill in the actual immediate argument here
            // in `call_indirect_host`.
            sink.add_reloc(Reloc::PulleyCallIndirectHost, &info.dest, 0);
            enc::call_indirect_host(sink, 0_u8);

            if let Some(s) = state.take_stack_map() {
                let offset = sink.cur_offset();
                sink.push_user_stack_map(state, offset, s);
            }
            sink.add_call_site();

            // If a callee pop is happening here that means that something has
            // messed up, these are expected to be "very simple" signatures.
            assert!(info.callee_pop_size == 0);
        }

        Inst::Jump { label } => {
            sink.use_label_at_offset(*start_offset + 1, *label, LabelUse::Jump(1));
            sink.add_uncond_branch(*start_offset, *start_offset + 5, *label);
            enc::jump(sink, 0x00000000);
        }

        Inst::BrIf {
            c,
            taken,
            not_taken,
        } => {
            // If taken.
            let taken_start = *start_offset + 2;
            let taken_end = taken_start + 4;

            sink.use_label_at_offset(taken_start, *taken, LabelUse::Jump(2));
            let mut inverted = SmallVec::<[u8; 16]>::new();
            enc::br_if_not(&mut inverted, c, 0x00000000);
            debug_assert_eq!(
                inverted.len(),
                usize::try_from(taken_end - *start_offset).unwrap()
            );

            sink.add_cond_branch(*start_offset, taken_end, *taken, &inverted);
            enc::br_if(sink, c, 0x00000000);
            debug_assert_eq!(sink.cur_offset(), taken_end);

            // If not taken.
            let not_taken_start = taken_end + 1;
            let not_taken_end = not_taken_start + 4;

            sink.use_label_at_offset(not_taken_start, *not_taken, LabelUse::Jump(1));
            sink.add_uncond_branch(taken_end, not_taken_end, *not_taken);
            enc::jump(sink, 0x00000000);
        }

        Inst::BrIfXeq32 {
            src1,
            src2,
            taken,
            not_taken,
        } => {
            br_if_cond_helper(
                sink,
                *start_offset,
                *src1,
                *src2,
                taken,
                not_taken,
                enc::br_if_xeq32,
                enc::br_if_xneq32,
            );
        }

        Inst::BrIfXneq32 {
            src1,
            src2,
            taken,
            not_taken,
        } => {
            br_if_cond_helper(
                sink,
                *start_offset,
                *src1,
                *src2,
                taken,
                not_taken,
                enc::br_if_xneq32,
                enc::br_if_xeq32,
            );
        }

        Inst::BrIfXslt32 {
            src1,
            src2,
            taken,
            not_taken,
        } => {
            br_if_cond_helper(
                sink,
                *start_offset,
                *src1,
                *src2,
                taken,
                not_taken,
                enc::br_if_xslt32,
                |s, src1, src2, x| enc::br_if_xslteq32(s, src2, src1, x),
            );
        }

        Inst::BrIfXslteq32 {
            src1,
            src2,
            taken,
            not_taken,
        } => {
            br_if_cond_helper(
                sink,
                *start_offset,
                *src1,
                *src2,
                taken,
                not_taken,
                enc::br_if_xslteq32,
                |s, src1, src2, x| enc::br_if_xslt32(s, src2, src1, x),
            );
        }

        Inst::BrIfXult32 {
            src1,
            src2,
            taken,
            not_taken,
        } => {
            br_if_cond_helper(
                sink,
                *start_offset,
                *src1,
                *src2,
                taken,
                not_taken,
                enc::br_if_xult32,
                |s, src1, src2, x| enc::br_if_xulteq32(s, src2, src1, x),
            );
        }

        Inst::BrIfXulteq32 {
            src1,
            src2,
            taken,
            not_taken,
        } => {
            br_if_cond_helper(
                sink,
                *start_offset,
                *src1,
                *src2,
                taken,
                not_taken,
                enc::br_if_xulteq32,
                |s, src1, src2, x| enc::br_if_xult32(s, src2, src1, x),
            );
        }

        Inst::LoadAddr { dst, mem } => {
            let base = mem.get_base_register();
            let offset = mem.get_offset_with_state(state);

            if let Some(base) = base {
                if offset == 0 {
                    enc::xmov(sink, dst, base);
                } else {
                    if let Ok(offset) = i8::try_from(offset) {
                        enc::xconst8(sink, dst, offset);
                    } else if let Ok(offset) = i16::try_from(offset) {
                        enc::xconst16(sink, dst, offset);
                    } else {
                        enc::xconst32(sink, dst, offset);
                    }

                    match P::pointer_width() {
                        PointerWidth::PointerWidth32 => {
                            enc::xadd32(sink, BinaryOperands::new(dst, base, dst))
                        }
                        PointerWidth::PointerWidth64 => {
                            enc::xadd64(sink, BinaryOperands::new(dst, base, dst))
                        }
                    }
                }
            } else {
                unreachable!("all pulley amodes have a base register right now")
            }
        }

        Inst::XLoad {
            dst,
            mem,
            ty,
            flags,
            ext,
        } => {
            use Endianness as E;
            use ExtKind as X;
            let r = mem.get_base_register().unwrap();
            let x = mem.get_offset_with_state(state);
            let endian = emit_info.endianness(*flags);
            match *ty {
                I8 => match ext {
                    X::None | X::Zero32 => enc::xload8_u32_offset32(sink, dst, r, x),
                    X::Zero64 => enc::xload8_u64_offset32(sink, dst, r, x),
                    X::Sign32 => enc::xload8_s32_offset32(sink, dst, r, x),
                    X::Sign64 => enc::xload8_s64_offset32(sink, dst, r, x),
                },
                I16 => match (ext, endian) {
                    (X::None | X::Zero32, E::Little) => {
                        enc::xload16le_u32_offset32(sink, dst, r, x);
                    }
                    (X::Sign32, E::Little) => {
                        enc::xload16le_s32_offset32(sink, dst, r, x);
                    }
                    (X::Zero64, E::Little) => {
                        enc::xload16le_u64_offset32(sink, dst, r, x);
                    }
                    (X::Sign64, E::Little) => {
                        enc::xload16le_s64_offset32(sink, dst, r, x);
                    }
                    (X::None | X::Zero32 | X::Zero64, E::Big) => {
                        enc::xload16be_u64_offset32(sink, dst, r, x);
                    }
                    (X::Sign32 | X::Sign64, E::Big) => {
                        enc::xload16be_s64_offset32(sink, dst, r, x);
                    }
                },
                I32 => match (ext, endian) {
                    (X::None | X::Zero32 | X::Sign32, E::Little) => {
                        enc::xload32le_offset32(sink, dst, r, x);
                    }
                    (X::Zero64, E::Little) => {
                        enc::xload32le_u64_offset32(sink, dst, r, x);
                    }
                    (X::Sign64, E::Little) => {
                        enc::xload32le_s64_offset32(sink, dst, r, x);
                    }
                    (X::None | X::Zero32 | X::Zero64, E::Big) => {
                        enc::xload32be_u64_offset32(sink, dst, r, x);
                    }
                    (X::Sign32 | X::Sign64, E::Big) => {
                        enc::xload32be_s64_offset32(sink, dst, r, x);
                    }
                },
                I64 => match endian {
                    E::Little => enc::xload64le_offset32(sink, dst, r, x),
                    E::Big => enc::xload64be_offset32(sink, dst, r, x),
                },
                _ => unimplemented!("xload ty={ty:?}"),
            }
        }

        Inst::FLoad {
            dst,
            mem,
            ty,
            flags,
        } => {
            use Endianness as E;
            let r = mem.get_base_register().unwrap();
            let x = mem.get_offset_with_state(state);
            let endian = emit_info.endianness(*flags);
            match *ty {
                F32 => match endian {
                    E::Little => enc::fload32le_offset32(sink, dst, r, x),
                    E::Big => enc::fload32be_offset32(sink, dst, r, x),
                },
                F64 => match endian {
                    E::Little => enc::fload64le_offset32(sink, dst, r, x),
                    E::Big => enc::fload64be_offset32(sink, dst, r, x),
                },
                _ => unimplemented!("fload ty={ty:?}"),
            }
        }

        Inst::VLoad {
            dst,
            mem,
            ty,
            flags,
        } => {
            let r = mem.get_base_register().unwrap();
            let x = mem.get_offset_with_state(state);
            let endian = emit_info.endianness(*flags);
            assert_eq!(endian, Endianness::Little);
            assert_eq!(ty.bytes(), 16);
            enc::vload128le_offset32(sink, dst, r, x);
        }

        Inst::XStore {
            mem,
            src,
            ty,
            flags,
        } => {
            use Endianness as E;
            let r = mem.get_base_register().unwrap();
            let x = mem.get_offset_with_state(state);
            let endian = emit_info.endianness(*flags);
            match *ty {
                I8 => enc::xstore8_offset32(sink, r, x, src),
                I16 => match endian {
                    E::Little => enc::xstore16le_offset32(sink, r, x, src),
                    E::Big => enc::xstore16be_offset32(sink, r, x, src),
                },
                I32 => match endian {
                    E::Little => enc::xstore32le_offset32(sink, r, x, src),
                    E::Big => enc::xstore32be_offset32(sink, r, x, src),
                },
                I64 => match endian {
                    E::Little => enc::xstore64le_offset32(sink, r, x, src),
                    E::Big => enc::xstore64be_offset32(sink, r, x, src),
                },
                _ => unimplemented!("xstore ty={ty:?}"),
            }
        }

        Inst::FStore {
            mem,
            src,
            ty,
            flags,
        } => {
            use Endianness as E;
            let r = mem.get_base_register().unwrap();
            let x = mem.get_offset_with_state(state);
            let endian = emit_info.endianness(*flags);
            match *ty {
                F32 => match endian {
                    E::Little => enc::fstore32le_offset32(sink, r, x, src),
                    E::Big => enc::fstore32be_offset32(sink, r, x, src),
                },
                F64 => match endian {
                    E::Little => enc::fstore64le_offset32(sink, r, x, src),
                    E::Big => enc::fstore64be_offset32(sink, r, x, src),
                },
                _ => unimplemented!("fstore ty={ty:?}"),
            }
        }

        Inst::VStore {
            mem,
            src,
            ty,
            flags,
        } => {
            let r = mem.get_base_register().unwrap();
            let x = mem.get_offset_with_state(state);
            let endian = emit_info.endianness(*flags);
            assert_eq!(endian, Endianness::Little);
            assert_eq!(ty.bytes(), 16);
            enc::vstore128le_offset32(sink, r, x, src);
        }

        Inst::BrTable {
            idx,
            default,
            targets,
        } => {
            // Encode the `br_table32` instruction directly which expects the
            // next `amt` 4-byte integers to all be relative offsets. Each
            // offset is the pc-relative offset of the branch destination.
            //
            // Pulley clamps the branch targets to the `amt` specified so the
            // final branch target is the default jump target.
            //
            // Note that this instruction may have many branch targets so it
            // manually checks to see if an island is needed. If so we emit a
            // jump around the island before the `br_table32` itself gets
            // emitted.
            let amt = u32::try_from(targets.len() + 1).expect("too many branch targets");
            let br_table_size = amt * 4 + 6;
            if sink.island_needed(br_table_size) {
                let label = sink.get_label();
                <InstAndKind<P>>::from(Inst::Jump { label }).emit(sink, emit_info, state);
                sink.emit_island(br_table_size, &mut state.ctrl_plane);
                sink.bind_label(label, &mut state.ctrl_plane);
            }
            enc::br_table32(sink, *idx, amt);
            for target in targets.iter() {
                let offset = sink.cur_offset();
                sink.use_label_at_offset(offset, *target, LabelUse::Jump(0));
                sink.put4(0);
            }
            let offset = sink.cur_offset();
            sink.use_label_at_offset(offset, *default, LabelUse::Jump(0));
            sink.put4(0);

            // We manually handled `emit_island` above when dealing with
            // `island_needed` so update the starting offset to the current
            // offset so this instruction doesn't accidentally trigger
            // the assertion that we're always under worst-case-size.
            *start_offset = sink.cur_offset();
        }

        Inst::Raw { raw } => {
            match raw {
                RawInst::PushFrame | RawInst::StackAlloc32 { .. } => {
                    sink.add_trap(ir::TrapCode::STACK_OVERFLOW);
                }
                _ => {}
            }
            super::generated::emit(raw, sink)
        }
    }
}

fn br_if_cond_helper<P>(
    sink: &mut MachBuffer<InstAndKind<P>>,
    start_offset: u32,
    src1: XReg,
    src2: XReg,
    taken: &MachLabel,
    not_taken: &MachLabel,
    mut enc: impl FnMut(&mut MachBuffer<InstAndKind<P>>, XReg, XReg, i32),
    mut enc_inverted: impl FnMut(&mut SmallVec<[u8; 16]>, XReg, XReg, i32),
) where
    P: PulleyTargetKind,
{
    // If taken.
    let taken_start = start_offset + 3;
    let taken_end = taken_start + 4;

    sink.use_label_at_offset(taken_start, *taken, LabelUse::Jump(3));
    let mut inverted = SmallVec::<[u8; 16]>::new();
    enc_inverted(&mut inverted, src1, src2, 0x00000000);
    debug_assert_eq!(
        inverted.len(),
        usize::try_from(taken_end - start_offset).unwrap()
    );

    sink.add_cond_branch(start_offset, taken_end, *taken, &inverted);
    enc(sink, src1, src2, 0x00000000);
    debug_assert_eq!(sink.cur_offset(), taken_end);

    // If not taken.
    let not_taken_start = taken_end + 1;
    let not_taken_end = not_taken_start + 4;

    sink.use_label_at_offset(not_taken_start, *not_taken, LabelUse::Jump(1));
    sink.add_uncond_branch(taken_end, not_taken_end, *not_taken);
    enc::jump(sink, 0x00000000);
}
