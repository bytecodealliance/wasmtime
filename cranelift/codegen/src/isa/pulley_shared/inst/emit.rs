//! Pulley binary code emission.

use super::*;
use crate::binemit::StackMap;
use crate::ir;
use crate::isa::pulley_shared::abi::PulleyMachineDeps;
use crate::isa::pulley_shared::PointerWidth;
use crate::trace;
use core::marker::PhantomData;
use cranelift_control::ControlPlane;
use pulley_interpreter::encode as enc;
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
    stack_map: Option<StackMap>,
    user_stack_map: Option<ir::UserStackMap>,
    pub virtual_sp_offset: i64,
    frame_layout: FrameLayout,
}

impl<P> EmitState<P>
where
    P: PulleyTargetKind,
{
    fn take_stack_map(&mut self) -> (Option<StackMap>, Option<ir::UserStackMap>) {
        (self.stack_map.take(), self.user_stack_map.take())
    }

    pub(crate) fn adjust_virtual_sp_offset(&mut self, amount: i64) {
        let old = self.virtual_sp_offset;
        let new = self.virtual_sp_offset + amount;
        trace!("adjust virtual sp offset by {amount:#x}: {old:#x} -> {new:#x}",);
        self.virtual_sp_offset = new;
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
            stack_map: None,
            user_stack_map: None,
            virtual_sp_offset: 0,
            frame_layout: abi.frame_layout().clone(),
        }
    }

    fn pre_safepoint(
        &mut self,
        stack_map: Option<StackMap>,
        user_stack_map: Option<ir::UserStackMap>,
    ) {
        self.stack_map = stack_map;
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
        let start = sink.cur_offset();
        pulley_emit(self, sink, emit_info, state, start);

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
    _emit_info: &EmitInfo,
    state: &mut EmitState<P>,
    start_offset: u32,
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

        Inst::Nop => todo!(),

        Inst::GetSp { dst } => enc::get_sp(sink, dst),

        Inst::Ret => enc::ret(sink),

        Inst::LoadExtName { .. } => todo!(),

        Inst::Call { callee, info } => {
            let (stack_map, user_stack_map) = state.take_stack_map();
            if let Some(s) = stack_map {
                sink.add_stack_map(StackMapExtent::UpcomingBytes(5), s);
            }
            sink.put1(pulley_interpreter::Opcode::Call as u8);
            sink.add_reloc(
                // TODO: is it actually okay to reuse this reloc here?
                Reloc::X86CallPCRel4,
                &**callee,
                // This addend adjusts for the difference between the start of
                // the instruction and the beginning of the immediate field.
                -1,
            );
            sink.put4(0);
            if let Some(s) = user_stack_map {
                let offset = sink.cur_offset();
                sink.push_user_stack_map(state, offset, s);
            }
            sink.add_call_site();

            let callee_pop_size = i64::from(info.callee_pop_size);
            state.adjust_virtual_sp_offset(-callee_pop_size);
        }

        Inst::IndirectCall { .. } => todo!(),

        Inst::Jump { label } => {
            sink.use_label_at_offset(start_offset + 1, *label, LabelUse::Jump(1));
            sink.add_uncond_branch(start_offset, start_offset + 5, *label);
            enc::jump(sink, 0x00000000);
        }

        Inst::BrIf {
            c,
            taken,
            not_taken,
        } => {
            // If taken.
            let taken_start = start_offset + 2;
            let taken_end = taken_start + 4;

            sink.use_label_at_offset(taken_start, *taken, LabelUse::Jump(2));
            let mut inverted = SmallVec::<[u8; 16]>::new();
            enc::br_if_not(&mut inverted, c, 0x00000000);
            debug_assert_eq!(
                inverted.len(),
                usize::try_from(taken_end - start_offset).unwrap()
            );

            sink.add_cond_branch(start_offset, taken_end, *taken, &inverted);
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
                start_offset,
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
                start_offset,
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
                start_offset,
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
                start_offset,
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
                start_offset,
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
                start_offset,
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

        Inst::Xadd32 { dst, src1, src2 } => {
            enc::xadd32(sink, dst, src1, src2);
        }
        Inst::Xadd64 { dst, src1, src2 } => {
            enc::xadd64(sink, dst, src1, src2);
        }

        Inst::Xeq64 { dst, src1, src2 } => {
            enc::xeq64(sink, dst, src1, src2);
        }
        Inst::Xneq64 { dst, src1, src2 } => {
            enc::xneq64(sink, dst, src1, src2);
        }
        Inst::Xslt64 { dst, src1, src2 } => {
            enc::xslt64(sink, dst, src1, src2);
        }
        Inst::Xslteq64 { dst, src1, src2 } => {
            enc::xslteq64(sink, dst, src1, src2);
        }
        Inst::Xult64 { dst, src1, src2 } => {
            enc::xult64(sink, dst, src1, src2);
        }
        Inst::Xulteq64 { dst, src1, src2 } => {
            enc::xulteq64(sink, dst, src1, src2);
        }
        Inst::Xeq32 { dst, src1, src2 } => {
            enc::xeq32(sink, dst, src1, src2);
        }
        Inst::Xneq32 { dst, src1, src2 } => {
            enc::xneq32(sink, dst, src1, src2);
        }
        Inst::Xslt32 { dst, src1, src2 } => {
            enc::xslt32(sink, dst, src1, src2);
        }
        Inst::Xslteq32 { dst, src1, src2 } => {
            enc::xslteq32(sink, dst, src1, src2);
        }
        Inst::Xult32 { dst, src1, src2 } => {
            enc::xult32(sink, dst, src1, src2);
        }
        Inst::Xulteq32 { dst, src1, src2 } => {
            enc::xulteq32(sink, dst, src1, src2);
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
                        PointerWidth::PointerWidth32 => enc::xadd32(sink, dst, base, dst),
                        PointerWidth::PointerWidth64 => enc::xadd64(sink, dst, base, dst),
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
