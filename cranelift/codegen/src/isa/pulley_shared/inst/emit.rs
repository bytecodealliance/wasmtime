//! Pulley binary code emission.

use super::*;
use crate::ir;
use crate::isa::pulley_shared::abi::PulleyMachineDeps;
use crate::isa::pulley_shared::PointerWidth;
use core::marker::PhantomData;
use cranelift_control::ControlPlane;
use pulley_interpreter::encode as enc;
use pulley_interpreter::regs::BinaryOperands;
use pulley_interpreter::regs::Reg as _;

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

        Inst::Trap { code } => {
            sink.add_trap(*code);
            enc::trap(sink);
        }

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

        Inst::Ret => enc::ret(sink),

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

        Inst::Xmov { dst, src } => enc::xmov(sink, dst, src),
        Inst::Fmov { dst, src } => enc::fmov(sink, dst, src),
        Inst::Vmov { dst, src } => enc::vmov(sink, dst, src),

        Inst::Xconst8 { dst, imm } => enc::xconst8(sink, dst, *imm),
        Inst::Xconst16 { dst, imm } => enc::xconst16(sink, dst, *imm),
        Inst::Xconst32 { dst, imm } => enc::xconst32(sink, dst, *imm),
        Inst::Xconst64 { dst, imm } => enc::xconst64(sink, dst, *imm),

        Inst::Xadd32 { dst, src1, src2 } => enc::xadd32(sink, BinaryOperands::new(dst, src1, src2)),
        Inst::Xadd64 { dst, src1, src2 } => enc::xadd64(sink, BinaryOperands::new(dst, src1, src2)),
        Inst::Xeq64 { dst, src1, src2 } => enc::xeq64(sink, BinaryOperands::new(dst, src1, src2)),
        Inst::Xneq64 { dst, src1, src2 } => enc::xneq64(sink, BinaryOperands::new(dst, src1, src2)),
        Inst::Xslt64 { dst, src1, src2 } => enc::xslt64(sink, BinaryOperands::new(dst, src1, src2)),
        Inst::Xslteq64 { dst, src1, src2 } => {
            enc::xslteq64(sink, BinaryOperands::new(dst, src1, src2))
        }
        Inst::Xult64 { dst, src1, src2 } => enc::xult64(sink, BinaryOperands::new(dst, src1, src2)),
        Inst::Xulteq64 { dst, src1, src2 } => {
            enc::xulteq64(sink, BinaryOperands::new(dst, src1, src2))
        }

        Inst::Xeq32 { dst, src1, src2 } => enc::xeq32(sink, BinaryOperands::new(dst, src1, src2)),
        Inst::Xneq32 { dst, src1, src2 } => enc::xneq32(sink, BinaryOperands::new(dst, src1, src2)),
        Inst::Xslt32 { dst, src1, src2 } => enc::xslt32(sink, BinaryOperands::new(dst, src1, src2)),
        Inst::Xslteq32 { dst, src1, src2 } => {
            enc::xslteq32(sink, BinaryOperands::new(dst, src1, src2))
        }
        Inst::Xult32 { dst, src1, src2 } => enc::xult32(sink, BinaryOperands::new(dst, src1, src2)),
        Inst::Xulteq32 { dst, src1, src2 } => {
            enc::xulteq32(sink, BinaryOperands::new(dst, src1, src2))
        }

        Inst::LoadAddr { dst, mem } => {
            let base = mem.get_base_register();
            let offset = mem.get_offset_with_state(state);

            if let Some(base) = base {
                let base = XReg::new(base).unwrap();

                if offset == 0 {
                    enc::xmov(sink, dst, base);
                } else {
                    if let Ok(offset) = i8::try_from(offset) {
                        enc::xconst8(sink, dst, offset);
                    } else if let Ok(offset) = i16::try_from(offset) {
                        enc::xconst16(sink, dst, offset);
                    } else if let Ok(offset) = i32::try_from(offset) {
                        enc::xconst32(sink, dst, offset);
                    } else {
                        enc::xconst64(sink, dst, offset);
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

        Inst::Load {
            dst,
            mem,
            ty,
            flags: _,
            ext,
        } => {
            use ExtKind as X;
            let r = mem.get_base_register().unwrap();
            let r = reg_to_pulley_xreg(r);
            let dst = reg_to_pulley_xreg(dst.to_reg());
            let x = mem.get_offset_with_state(state);
            match (*ext, *ty, i8::try_from(x)) {
                (X::Sign, types::I32, Ok(0)) => enc::load32_s(sink, dst, r),
                (X::Sign, types::I32, Ok(x)) => enc::load32_s_offset8(sink, dst, r, x),
                (X::Sign, types::I32, Err(_)) => enc::load32_s_offset64(sink, dst, r, x),

                (X::Zero, types::I32, Ok(0)) => enc::load32_u(sink, dst, r),
                (X::Zero, types::I32, Ok(x)) => enc::load32_u_offset8(sink, dst, r, x),
                (X::Zero, types::I32, Err(_)) => enc::load32_u_offset64(sink, dst, r, x),

                (_, types::I64, Ok(0)) => enc::load64(sink, dst, r),
                (_, types::I64, Ok(x)) => enc::load64_offset8(sink, dst, r, x),
                (_, types::I64, Err(_)) => enc::load64_offset64(sink, dst, r, x),

                (..) => unimplemented!("load ext={ext:?} ty={ty}"),
            }
        }

        Inst::Store {
            mem,
            src,
            ty,
            flags: _,
        } => {
            let r = mem.get_base_register().unwrap();
            let r = reg_to_pulley_xreg(r);
            let src = reg_to_pulley_xreg(*src);
            let x = mem.get_offset_with_state(state);
            match (*ty, i8::try_from(x)) {
                (types::I32, Ok(0)) => enc::store32(sink, r, src),
                (types::I32, Ok(x)) => enc::store32_offset8(sink, r, x, src),
                (types::I32, Err(_)) => enc::store32_offset64(sink, r, x, src),

                (types::I64, Ok(0)) => enc::store64(sink, r, src),
                (types::I64, Ok(x)) => enc::store64_offset8(sink, r, x, src),
                (types::I64, Err(_)) => enc::store64_offset64(sink, r, x, src),

                (..) => todo!(),
            }
        }

        Inst::BitcastIntFromFloat32 { dst, src } => enc::bitcast_int_from_float_32(sink, dst, src),
        Inst::BitcastIntFromFloat64 { dst, src } => enc::bitcast_int_from_float_64(sink, dst, src),
        Inst::BitcastFloatFromInt32 { dst, src } => enc::bitcast_float_from_int_32(sink, dst, src),
        Inst::BitcastFloatFromInt64 { dst, src } => enc::bitcast_float_from_int_64(sink, dst, src),

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

fn reg_to_pulley_xreg(r: Reg) -> pulley_interpreter::XReg {
    pulley_interpreter::XReg::new(r.to_real_reg().unwrap().hw_enc()).unwrap()
}
