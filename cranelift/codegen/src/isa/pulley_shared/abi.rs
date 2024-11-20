//! Implementation of a standard Pulley ABI.

use super::{inst::*, PulleyFlags, PulleyTargetKind};
use crate::isa::pulley_shared::PulleyBackend;
use crate::{
    ir::{self, types::*, MemFlags, Signature},
    isa::{self, unwind::UnwindInst},
    machinst::*,
    settings, CodegenResult,
};
use alloc::{boxed::Box, vec::Vec};
use core::marker::PhantomData;
use regalloc2::{MachineEnv, PReg, PRegSet};
use smallvec::{smallvec, SmallVec};
use std::borrow::ToOwned;
use std::sync::OnceLock;

/// Support for the Pulley ABI from the callee side (within a function body).
pub(crate) type PulleyCallee<P> = Callee<PulleyMachineDeps<P>>;

/// Support for the Pulley ABI from the caller side (at a callsite).
pub(crate) type PulleyABICallSite<P> = CallSite<PulleyMachineDeps<P>>;

/// Pulley-specific ABI behavior. This struct just serves as an implementation
/// point for the trait; it is never actually instantiated.
pub struct PulleyMachineDeps<P>
where
    P: PulleyTargetKind,
{
    _phantom: PhantomData<P>,
}

impl<P> ABIMachineSpec for PulleyMachineDeps<P>
where
    P: PulleyTargetKind,
{
    type I = InstAndKind<P>;
    type F = PulleyFlags;

    /// This is the limit for the size of argument and return-value areas on the
    /// stack. We place a reasonable limit here to avoid integer overflow issues
    /// with 32-bit arithmetic: for now, 128 MB.
    const STACK_ARG_RET_SIZE_LIMIT: u32 = 128 * 1024 * 1024;

    fn word_bits() -> u32 {
        P::pointer_width().bits().into()
    }

    /// Return required stack alignment in bytes.
    fn stack_align(_call_conv: isa::CallConv) -> u32 {
        16
    }

    fn compute_arg_locs(
        call_conv: isa::CallConv,
        flags: &settings::Flags,
        params: &[ir::AbiParam],
        args_or_rets: ArgsOrRets,
        add_ret_area_ptr: bool,
        mut args: ArgsAccumulator,
    ) -> CodegenResult<(u32, Option<usize>)> {
        // NB: make sure this method stays in sync with
        // `cranelift_pulley::interp::Vm::call`.

        let x_end = 15;
        let f_end = 15;
        let v_end = 15;

        let mut next_x_reg = 0;
        let mut next_f_reg = 0;
        let mut next_v_reg = 0;
        let mut next_stack: u32 = 0;

        let ret_area_ptr = if add_ret_area_ptr {
            debug_assert_eq!(args_or_rets, ArgsOrRets::Args);
            next_x_reg += 1;
            Some(ABIArg::reg(
                x_reg(next_x_reg - 1).to_real_reg().unwrap(),
                I64,
                ir::ArgumentExtension::None,
                ir::ArgumentPurpose::Normal,
            ))
        } else {
            None
        };

        for param in params {
            // Find the regclass(es) of the register(s) used to store a value of
            // this type.
            let (rcs, reg_tys) = Self::I::rc_for_type(param.value_type)?;

            let mut slots = ABIArgSlotVec::new();
            for (rc, reg_ty) in rcs.iter().zip(reg_tys.iter()) {
                let next_reg = if (next_x_reg <= x_end) && *rc == RegClass::Int {
                    let x = Some(x_reg(next_x_reg));
                    next_x_reg += 1;
                    x
                } else if (next_f_reg <= f_end) && *rc == RegClass::Float {
                    let f = Some(f_reg(next_f_reg));
                    next_f_reg += 1;
                    f
                } else if (next_v_reg <= v_end) && *rc == RegClass::Vector {
                    let v = Some(v_reg(next_v_reg));
                    next_v_reg += 1;
                    v
                } else {
                    None
                };

                if let Some(reg) = next_reg {
                    slots.push(ABIArgSlot::Reg {
                        reg: reg.to_real_reg().unwrap(),
                        ty: *reg_ty,
                        extension: param.extension,
                    });
                } else {
                    if args_or_rets == ArgsOrRets::Rets && !flags.enable_multi_ret_implicit_sret() {
                        return Err(crate::CodegenError::Unsupported(
                            "Too many return values to fit in registers. \
                            Use a StructReturn argument instead. (#9510)"
                                .to_owned(),
                        ));
                    }

                    // Compute size and 16-byte stack alignment happens
                    // separately after all args.
                    let size = reg_ty.bits() / 8;
                    let size = std::cmp::max(size, 8);

                    // Align.
                    debug_assert!(size.is_power_of_two());
                    next_stack = align_to(next_stack, size);

                    slots.push(ABIArgSlot::Stack {
                        offset: i64::from(next_stack),
                        ty: *reg_ty,
                        extension: param.extension,
                    });

                    next_stack += size;
                }
            }

            args.push(ABIArg::Slots {
                slots,
                purpose: param.purpose,
            });
        }

        let pos = if let Some(ret_area_ptr) = ret_area_ptr {
            args.push_non_formal(ret_area_ptr);
            Some(args.args().len() - 1)
        } else {
            None
        };

        next_stack = align_to(next_stack, Self::stack_align(call_conv));

        Ok((next_stack, pos))
    }

    fn gen_load_stack(mem: StackAMode, into_reg: Writable<Reg>, ty: Type) -> Self::I {
        Inst::gen_load(into_reg, mem.into(), ty, MemFlags::trusted()).into()
    }

    fn gen_store_stack(mem: StackAMode, from_reg: Reg, ty: Type) -> Self::I {
        Inst::gen_store(mem.into(), from_reg, ty, MemFlags::trusted()).into()
    }

    fn gen_move(to_reg: Writable<Reg>, from_reg: Reg, ty: Type) -> Self::I {
        Self::I::gen_move(to_reg, from_reg, ty)
    }

    fn gen_extend(
        dst: Writable<Reg>,
        src: Reg,
        signed: bool,
        from_bits: u8,
        to_bits: u8,
    ) -> Self::I {
        let src = XReg::new(src).unwrap();
        let dst = dst.try_into().unwrap();
        match (signed, from_bits) {
            (true, 8) => Inst::Sext8 { dst, src }.into(),
            (true, 16) => Inst::Sext16 { dst, src }.into(),
            (true, 32) => Inst::Sext32 { dst, src }.into(),
            (false, 8) => Inst::Zext8 { dst, src }.into(),
            (false, 16) => Inst::Zext16 { dst, src }.into(),
            (false, 32) => Inst::Zext32 { dst, src }.into(),
            _ => unimplemented!("extend {from_bits} to {to_bits} as signed? {signed}"),
        }
    }

    fn get_ext_mode(
        _call_conv: isa::CallConv,
        specified: ir::ArgumentExtension,
    ) -> ir::ArgumentExtension {
        specified
    }

    fn gen_args(args: Vec<ArgPair>) -> Self::I {
        Inst::Args { args }.into()
    }

    fn gen_rets(rets: Vec<RetPair>) -> Self::I {
        Inst::Rets { rets }.into()
    }

    fn get_stacklimit_reg(_call_conv: isa::CallConv) -> Reg {
        spilltmp_reg()
    }

    fn gen_add_imm(
        _call_conv: isa::CallConv,
        into_reg: Writable<Reg>,
        from_reg: Reg,
        imm: u32,
    ) -> SmallInstVec<Self::I> {
        let dst = into_reg.try_into().unwrap();
        let imm = imm as i32;
        smallvec![
            Inst::Xconst32 { dst, imm }.into(),
            Inst::Xadd32 {
                dst,
                src1: from_reg.try_into().unwrap(),
                src2: dst.to_reg(),
            }
            .into()
        ]
    }

    fn gen_stack_lower_bound_trap(_limit_reg: Reg) -> SmallInstVec<Self::I> {
        unimplemented!("pulley shouldn't need stack bound checks")
    }

    fn gen_get_stack_addr(mem: StackAMode, dst: Writable<Reg>) -> Self::I {
        let dst = dst.to_reg();
        let dst = XReg::new(dst).unwrap();
        let dst = WritableXReg::from_reg(dst);
        let mem = mem.into();
        Inst::LoadAddr { dst, mem }.into()
    }

    fn gen_load_base_offset(into_reg: Writable<Reg>, base: Reg, offset: i32, ty: Type) -> Self::I {
        let offset = i64::from(offset);
        let base = XReg::try_from(base).unwrap();
        let mem = Amode::RegOffset { base, offset };
        Inst::gen_load(into_reg, mem, ty, MemFlags::trusted()).into()
    }

    fn gen_store_base_offset(_base: Reg, _offset: i32, _from_reg: Reg, _ty: Type) -> Self::I {
        todo!()
    }

    fn gen_sp_reg_adjust(amount: i32) -> SmallInstVec<Self::I> {
        if amount == 0 {
            return smallvec![];
        }

        let inst = if amount < 0 {
            let amount = amount.checked_neg().unwrap();
            if let Ok(amt) = u32::try_from(amount) {
                Inst::StackAlloc32 { amt }
            } else {
                unreachable!()
            }
        } else {
            if let Ok(amt) = u32::try_from(amount) {
                Inst::StackFree32 { amt }
            } else {
                unreachable!()
            }
        };
        smallvec![inst.into()]
    }

    fn gen_prologue_frame_setup(
        _call_conv: isa::CallConv,
        flags: &settings::Flags,
        _isa_flags: &PulleyFlags,
        frame_layout: &FrameLayout,
    ) -> SmallInstVec<Self::I> {
        let mut insts = SmallVec::new();

        if frame_layout.setup_area_size > 0 {
            insts.push(Inst::PushFrame.into());
            if flags.unwind_info() {
                insts.push(
                    Inst::Unwind {
                        inst: UnwindInst::PushFrameRegs {
                            offset_upward_to_caller_sp: frame_layout.setup_area_size,
                        },
                    }
                    .into(),
                );
            }
        }

        insts
    }

    /// Reverse of `gen_prologue_frame_setup`.
    fn gen_epilogue_frame_restore(
        _call_conv: isa::CallConv,
        _flags: &settings::Flags,
        _isa_flags: &PulleyFlags,
        frame_layout: &FrameLayout,
    ) -> SmallInstVec<Self::I> {
        let mut insts = SmallVec::new();

        if frame_layout.setup_area_size > 0 {
            insts.push(Inst::PopFrame.into());
        }

        if frame_layout.tail_args_size > 0 {
            insts.extend(Self::gen_sp_reg_adjust(
                frame_layout.tail_args_size.try_into().unwrap(),
            ));
        }

        insts
    }

    fn gen_return(
        _call_conv: isa::CallConv,
        _isa_flags: &PulleyFlags,
        _frame_layout: &FrameLayout,
    ) -> SmallInstVec<Self::I> {
        smallvec![Inst::Ret {}.into()]
    }

    fn gen_probestack(_insts: &mut SmallInstVec<Self::I>, _frame_size: u32) {
        // Pulley doesn't implement stack probes since all stack pointer
        // decrements are checked already.
    }

    fn gen_clobber_save(
        _call_conv: isa::CallConv,
        flags: &settings::Flags,
        frame_layout: &FrameLayout,
    ) -> SmallVec<[Self::I; 16]> {
        let mut insts = SmallVec::new();
        let setup_frame = frame_layout.setup_area_size > 0;

        let incoming_args_diff = frame_layout.tail_args_size - frame_layout.incoming_args_size;
        if incoming_args_diff > 0 {
            // Decrement SP by the amount of additional incoming argument space
            // we need
            insts.extend(Self::gen_sp_reg_adjust(-(incoming_args_diff as i32)));

            if setup_frame {
                // Write the lr position on the stack again, as it hasn't
                // changed since it was pushed in `gen_prologue_frame_setup`
                insts.push(
                    Inst::gen_store(
                        Amode::SpOffset { offset: 8 },
                        lr_reg(),
                        I64,
                        MemFlags::trusted(),
                    )
                    .into(),
                );
                insts.push(
                    Inst::gen_load(
                        writable_fp_reg(),
                        Amode::SpOffset {
                            offset: i64::from(incoming_args_diff),
                        },
                        I64,
                        MemFlags::trusted(),
                    )
                    .into(),
                );
                insts.push(
                    Inst::gen_store(
                        Amode::SpOffset { offset: 0 },
                        fp_reg(),
                        I64,
                        MemFlags::trusted(),
                    )
                    .into(),
                );

                // Finally, sync the frame pointer with SP.
                insts.push(Self::I::gen_move(writable_fp_reg(), stack_reg(), I64));
            }
        }

        if flags.unwind_info() && setup_frame {
            // The *unwind* frame (but not the actual frame) starts at the
            // clobbers, just below the saved FP/LR pair.
            insts.push(
                Inst::Unwind {
                    inst: UnwindInst::DefineNewFrame {
                        offset_downward_to_clobbers: frame_layout.clobber_size,
                        offset_upward_to_caller_sp: frame_layout.setup_area_size,
                    },
                }
                .into(),
            );
        }

        // Adjust the stack pointer downward for clobbers, the function fixed
        // frame (spillslots and storage slots), and outgoing arguments.
        let stack_size = frame_layout.clobber_size
            + frame_layout.fixed_frame_storage_size
            + frame_layout.outgoing_args_size;

        // Store each clobbered register in order at offsets from SP, placing
        // them above the fixed frame slots.
        if stack_size > 0 {
            insts.extend(Self::gen_sp_reg_adjust(-i32::try_from(stack_size).unwrap()));

            let mut cur_offset = 8;
            for reg in &frame_layout.clobbered_callee_saves {
                let r_reg = reg.to_reg();
                let ty = match r_reg.class() {
                    RegClass::Int => I64,
                    RegClass::Float => F64,
                    RegClass::Vector => unreachable!("no vector registers are callee-save"),
                };
                insts.push(
                    Inst::gen_store(
                        Amode::SpOffset {
                            offset: i64::from(stack_size - cur_offset),
                        },
                        Reg::from(reg.to_reg()),
                        ty,
                        MemFlags::trusted(),
                    )
                    .into(),
                );

                if flags.unwind_info() {
                    insts.push(
                        Inst::Unwind {
                            inst: UnwindInst::SaveReg {
                                clobber_offset: frame_layout.clobber_size - cur_offset,
                                reg: r_reg,
                            },
                        }
                        .into(),
                    );
                }

                cur_offset += 8
            }
        }

        insts
    }

    fn gen_clobber_restore(
        _call_conv: isa::CallConv,
        _flags: &settings::Flags,
        frame_layout: &FrameLayout,
    ) -> SmallVec<[Self::I; 16]> {
        let mut insts = SmallVec::new();

        let stack_size = frame_layout.clobber_size
            + frame_layout.fixed_frame_storage_size
            + frame_layout.outgoing_args_size;

        let mut cur_offset = 8;
        for reg in &frame_layout.clobbered_callee_saves {
            let rreg = reg.to_reg();
            let ty = match rreg.class() {
                RegClass::Int => I64,
                RegClass::Float => F64,
                RegClass::Vector => unreachable!("vector registers are never callee-saved"),
            };
            insts.push(
                Inst::gen_load(
                    reg.map(Reg::from),
                    Amode::SpOffset {
                        offset: i64::from(stack_size - cur_offset),
                    },
                    ty,
                    MemFlags::trusted(),
                )
                .into(),
            );
            cur_offset += 8
        }

        if stack_size > 0 {
            insts.extend(Self::gen_sp_reg_adjust(stack_size as i32));
        }

        insts
    }

    fn gen_call(
        dest: &CallDest,
        _tmp: Writable<Reg>,
        info: CallInfo<()>,
    ) -> SmallVec<[Self::I; 2]> {
        match dest {
            // "near" calls are pulley->pulley calls so they use a normal "call"
            // opcode
            CallDest::ExtName(name, RelocDistance::Near) => smallvec![Inst::Call {
                info: Box::new(info.map(|()| name.clone()))
            }
            .into()],
            // "far" calls are pulley->host calls so they use a different opcode
            // which is lowered with a special relocation in the backend.
            CallDest::ExtName(name, RelocDistance::Far) => smallvec![Inst::IndirectCallHost {
                info: Box::new(info.map(|()| name.clone()))
            }
            .into()],
            // Indirect calls are all assumed to be pulley->pulley calls
            CallDest::Reg(reg) => smallvec![Inst::IndirectCall {
                info: Box::new(info.map(|()| XReg::new(*reg).unwrap()))
            }
            .into()],
        }
    }

    fn gen_memcpy<F: FnMut(Type) -> Writable<Reg>>(
        _call_conv: isa::CallConv,
        _dst: Reg,
        _src: Reg,
        _size: usize,
        _alloc_tmp: F,
    ) -> SmallVec<[Self::I; 8]> {
        todo!()
    }

    fn get_number_of_spillslots_for_value(
        rc: RegClass,
        _target_vector_bytes: u32,
        _isa_flags: &PulleyFlags,
    ) -> u32 {
        match rc {
            RegClass::Int => 1,
            RegClass::Float => todo!(),
            RegClass::Vector => unreachable!(),
        }
    }

    fn get_machine_env(_flags: &settings::Flags, _call_conv: isa::CallConv) -> &MachineEnv {
        static MACHINE_ENV: OnceLock<MachineEnv> = OnceLock::new();
        MACHINE_ENV.get_or_init(create_reg_enviroment)
    }

    fn get_regs_clobbered_by_call(_call_conv_of_callee: isa::CallConv) -> PRegSet {
        DEFAULT_CLOBBERS
    }

    fn compute_frame_layout(
        _call_conv: isa::CallConv,
        flags: &settings::Flags,
        _sig: &Signature,
        regs: &[Writable<RealReg>],
        is_leaf: bool,
        incoming_args_size: u32,
        tail_args_size: u32,
        fixed_frame_storage_size: u32,
        outgoing_args_size: u32,
    ) -> FrameLayout {
        let mut regs: Vec<Writable<RealReg>> = regs
            .iter()
            .cloned()
            .filter(|r| DEFAULT_CALLEE_SAVES.contains(r.to_reg().into()))
            .collect();

        regs.sort_unstable();

        // Compute clobber size.
        let clobber_size = compute_clobber_size(&regs);

        // Compute linkage frame size.
        let setup_area_size = if flags.preserve_frame_pointers()
            || !is_leaf
            // The function arguments that are passed on the stack are addressed
            // relative to the Frame Pointer.
            || incoming_args_size > 0
            || clobber_size > 0
            || fixed_frame_storage_size > 0
        {
            16 // FP, LR
        } else {
            0
        };

        FrameLayout {
            incoming_args_size,
            tail_args_size,
            setup_area_size,
            clobber_size,
            fixed_frame_storage_size,
            outgoing_args_size,
            clobbered_callee_saves: regs,
        }
    }

    fn gen_inline_probestack(
        _insts: &mut SmallInstVec<Self::I>,
        _call_conv: isa::CallConv,
        _frame_size: u32,
        _guard_size: u32,
    ) {
        // Pulley doesn't need inline probestacks because it always checks stack
        // decrements.
    }
}

impl<P> PulleyABICallSite<P>
where
    P: PulleyTargetKind,
{
    pub fn emit_return_call(
        self,
        _ctx: &mut Lower<InstAndKind<P>>,
        _args: isle::ValueSlice,
        _backend: &PulleyBackend<P>,
    ) {
        todo!()
    }
}

const DEFAULT_CALLEE_SAVES: PRegSet = PRegSet::empty()
    // Integer registers.
    .with(px_reg(16))
    .with(px_reg(17))
    .with(px_reg(18))
    .with(px_reg(19))
    .with(px_reg(20))
    .with(px_reg(21))
    .with(px_reg(22))
    .with(px_reg(23))
    .with(px_reg(24))
    .with(px_reg(25))
    .with(px_reg(26))
    .with(px_reg(27))
    .with(px_reg(28))
    .with(px_reg(29))
    .with(px_reg(30))
    .with(px_reg(31))
    // Float registers.
    .with(px_reg(16))
    .with(px_reg(17))
    .with(px_reg(18))
    .with(px_reg(19))
    .with(px_reg(20))
    .with(px_reg(21))
    .with(px_reg(22))
    .with(px_reg(23))
    .with(px_reg(24))
    .with(px_reg(25))
    .with(px_reg(26))
    .with(px_reg(27))
    .with(px_reg(28))
    .with(px_reg(29))
    .with(px_reg(30))
    .with(px_reg(31))
    // Note: no vector registers are callee-saved.
;

fn compute_clobber_size(clobbers: &[Writable<RealReg>]) -> u32 {
    let mut clobbered_size = 0;
    for reg in clobbers {
        match reg.to_reg().class() {
            RegClass::Int => {
                clobbered_size += 8;
            }
            RegClass::Float => {
                clobbered_size += 8;
            }
            RegClass::Vector => unimplemented!("Vector Size Clobbered"),
        }
    }
    align_to(clobbered_size, 16)
}

const DEFAULT_CLOBBERS: PRegSet = PRegSet::empty()
    // Integer registers: the first 16 get clobbered.
    .with(px_reg(0))
    .with(px_reg(1))
    .with(px_reg(2))
    .with(px_reg(3))
    .with(px_reg(4))
    .with(px_reg(5))
    .with(px_reg(6))
    .with(px_reg(7))
    .with(px_reg(8))
    .with(px_reg(9))
    .with(px_reg(10))
    .with(px_reg(11))
    .with(px_reg(12))
    .with(px_reg(13))
    .with(px_reg(14))
    .with(px_reg(15))
    // Float registers: the first 16 get clobbered.
    .with(pf_reg(0))
    .with(pf_reg(1))
    .with(pf_reg(2))
    .with(pf_reg(3))
    .with(pf_reg(4))
    .with(pf_reg(5))
    .with(pf_reg(6))
    .with(pf_reg(7))
    .with(pf_reg(9))
    .with(pf_reg(10))
    .with(pf_reg(11))
    .with(pf_reg(12))
    .with(pf_reg(13))
    .with(pf_reg(14))
    .with(pf_reg(15))
    // All vector registers get clobbered.
    .with(pv_reg(0))
    .with(pv_reg(1))
    .with(pv_reg(2))
    .with(pv_reg(3))
    .with(pv_reg(4))
    .with(pv_reg(5))
    .with(pv_reg(6))
    .with(pv_reg(7))
    .with(pv_reg(8))
    .with(pv_reg(9))
    .with(pv_reg(10))
    .with(pv_reg(11))
    .with(pv_reg(12))
    .with(pv_reg(13))
    .with(pv_reg(14))
    .with(pv_reg(15))
    .with(pv_reg(16))
    .with(pv_reg(17))
    .with(pv_reg(18))
    .with(pv_reg(19))
    .with(pv_reg(20))
    .with(pv_reg(21))
    .with(pv_reg(22))
    .with(pv_reg(23))
    .with(pv_reg(24))
    .with(pv_reg(25))
    .with(pv_reg(26))
    .with(pv_reg(27))
    .with(pv_reg(28))
    .with(pv_reg(29))
    .with(pv_reg(30))
    .with(pv_reg(31));

fn create_reg_enviroment() -> MachineEnv {
    // Prefer caller-saved registers over callee-saved registers, because that
    // way we don't need to emit code to save and restore them if we don't
    // mutate them.

    let preferred_regs_by_class: [Vec<PReg>; 3] = {
        let x_registers: Vec<PReg> = (0..16).map(|x| px_reg(x)).collect();
        let f_registers: Vec<PReg> = (0..16).map(|x| pf_reg(x)).collect();
        let v_registers: Vec<PReg> = (0..32).map(|x| pv_reg(x)).collect();
        [x_registers, f_registers, v_registers]
    };

    let non_preferred_regs_by_class: [Vec<PReg>; 3] = {
        let x_registers: Vec<PReg> = (16..XReg::SPECIAL_START)
            .map(|x| px_reg(x.into()))
            .collect();
        let f_registers: Vec<PReg> = (16..32).map(|x| pf_reg(x)).collect();
        let v_registers: Vec<PReg> = vec![];
        [x_registers, f_registers, v_registers]
    };

    MachineEnv {
        preferred_regs_by_class,
        non_preferred_regs_by_class,
        fixed_stack_slots: vec![],
        scratch_by_class: [None, None, None],
    }
}
