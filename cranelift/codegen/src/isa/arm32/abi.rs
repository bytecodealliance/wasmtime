//! Implementation of a standard ARM32 ABI (AAPCS).

use crate::alloc::borrow::ToOwned;
use crate::alloc::vec::Vec;
use crate::ir::{self, types::I32};
use crate::isa::{
    self,
    arm32::{inst::Inst, lower::regs::x_reg, settings::Flags as Arm32Flags},
};
use crate::machinst::{
    ABIArg, ABIArgSlot, ABIArgSlotVec, ABIMachineSpec, ArgPair, ArgsAccumulator, ArgsOrRets,
    Callee, FrameLayout, FunctionCalls, IsaFlags, MachInst, RealReg, RegClass, RetPair,
    SmallInstVec, Writable, align_to,
};
use crate::settings;
use regalloc2::{MachineEnv, PReg, PRegSet};
use smallvec::SmallVec;

pub(crate) type Arm32Callee = Callee<Arm32MachineDeps>;

pub struct Arm32MachineDeps;

impl IsaFlags for Arm32Flags {}

impl ABIMachineSpec for Arm32MachineDeps {
    type I = Inst;
    type F = Arm32Flags;

    const STACK_ARG_RET_SIZE_LIMIT: u32 = 128 * 1024 * 1024;

    fn word_bits() -> u32 {
        32
    }

    fn stack_align(_call_conv: crate::isa::CallConv) -> u32 {
        8
    }

    fn compute_arg_locs(
        call_conv: isa::CallConv,
        flags: &settings::Flags,
        params: &[ir::AbiParam],
        args_or_rets: ArgsOrRets,
        add_ret_area_ptr: bool,
        mut args: ArgsAccumulator,
    ) -> crate::CodegenResult<(u32, Option<usize>)> {
        assert_ne!(
            call_conv,
            isa::CallConv::Winch,
            "arm32 does not support the 'winch' calling convention yet"
        );

        let (r_start, r_end) = match args_or_rets {
            ArgsOrRets::Args => (0, 3),
            ArgsOrRets::Rets => (0, 1),
        };

        // AAPCS requires that multi-register types be allocated in consecutive pairs.
        // On ARM32 with i64 support: r0:r1 is pair 0, r2:r3 is pair 1.
        let mut next_r_reg = r_start;
        let mut next_stack: u32 = 0;

        let ret_area_ptr = if add_ret_area_ptr {
            assert!(ArgsOrRets::Args == args_or_rets);
            // Reserve first available register for return area pointer
            next_r_reg += 1;
            Some(ABIArg::reg(
                x_reg(r_start).to_real_reg().unwrap(),
                I32,
                ir::ArgumentExtension::None,
                ir::ArgumentPurpose::Normal,
            ))
        } else {
            None
        };

        for param in params {
            if let ir::ArgumentPurpose::StructArgument(_) = param.purpose {
                panic!(
                    "StructArgument parameters are not supported on arm32. \
                    Use regular pointer arguments instead."
                );
            }

            let (rcs, reg_tys) = Inst::rc_for_type(param.value_type)?;
            let num_parts = rcs.len();

            // For multi-part types, we need to allocate consecutive register pairs.
            // On ARM32: pair 0 = r0:r1, pair 1 = r2:r3 (AAPCS requirement)
            if num_parts > 1 {
                // Find the first available even register for this multi-part type
                let mut base_reg = next_r_reg;
                while base_reg <= r_end && base_reg % 2 != 0 {
                    base_reg += 1;
                }

                let mut slots = ABIArgSlotVec::new();

                if num_parts == 2 && base_reg + 1 <= r_end {
                    // Allocate both parts of the multi-part type as consecutive registers
                    for (part_idx, (rc, reg_ty)) in rcs.iter().zip(reg_tys.iter()).enumerate() {
                        let reg = x_reg(base_reg + part_idx);
                        if *rc == RegClass::Int {
                            slots.push(ABIArgSlot::Reg {
                                reg: reg.to_real_reg().unwrap(),
                                ty: *reg_ty,
                                extension: param.extension,
                            });
                        } else {
                            // Fallback to stack for non-int parts
                            let size = core::cmp::max(reg_ty.bits() / 8, 4);
                            next_stack = align_to(next_stack, size);
                            slots.push(ABIArgSlot::Stack {
                                offset: next_stack as i64,
                                ty: *reg_ty,
                                extension: param.extension,
                            });
                            next_stack += size;
                        }
                    }
                    // Advance past this pair
                    next_r_reg = base_reg + 2;
                } else {
                    // Not enough registers for multi-part type - use stack
                    for (_rc, reg_ty) in rcs.iter().zip(reg_tys.iter()) {
                        if args_or_rets == ArgsOrRets::Rets
                            && !flags.enable_multi_ret_implicit_sret()
                            && num_parts > 1
                        {
                            return Err(crate::CodegenError::Unsupported(
                                "Multi-part return values not supported on arm32. \
                                Use a StructReturn argument instead. (#9510)"
                                    .to_owned(),
                            ));
                        }

                        let size = core::cmp::max(reg_ty.bits() / 8, 4);
                        debug_assert!(size.is_power_of_two());
                        next_stack = align_to(next_stack, size);
                        slots.push(ABIArgSlot::Stack {
                            offset: next_stack as i64,
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
                continue;
            }

            // Single-register type allocation (simplified path for non-multi-part)
            let mut slots = ABIArgSlotVec::new();
            if next_r_reg <= r_end {
                let reg = x_reg(next_r_reg);
                if *rcs.first().unwrap() == RegClass::Int {
                    slots.push(ABIArgSlot::Reg {
                        reg: reg.to_real_reg().unwrap(),
                        ty: *reg_tys.first().unwrap(),
                        extension: param.extension,
                    });
                    next_r_reg += 1;
                } else {
                    // Stack for non-int single part
                    let size = core::cmp::max(reg_tys.first().unwrap().bits() / 8, 4);
                    next_stack = align_to(next_stack, size);
                    slots.push(ABIArgSlot::Stack {
                        offset: next_stack as i64,
                        ty: *reg_tys.first().unwrap(),
                        extension: param.extension,
                    });
                    next_stack += size;
                }
            } else if args_or_rets == ArgsOrRets::Rets && !flags.enable_multi_ret_implicit_sret() {
                return Err(crate::CodegenError::Unsupported(
                    "Too many return values to fit in registers. \
                    Use a StructReturn argument instead. (#9510)"
                        .to_owned(),
                ));
            } else if !slots.is_empty() || num_parts == 1 {
                // Stack fallback for exhausted regs
                let size = core::cmp::max(reg_tys.first().unwrap().bits() / 8, 4);
                next_stack = align_to(next_stack, size);
                slots.push(ABIArgSlot::Stack {
                    offset: next_stack as i64,
                    ty: *reg_tys.first().unwrap(),
                    extension: param.extension,
                });
                next_stack += size;
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

    fn gen_load_stack(
        _mem: crate::machinst::StackAMode,
        _into_reg: crate::Writable<crate::Reg>,
        _ty: crate::ir::Type,
    ) -> Self::I {
        todo!()
    }

    fn gen_store_stack(
        _mem: crate::machinst::StackAMode,
        _from_reg: crate::Reg,
        _ty: crate::ir::Type,
    ) -> Self::I {
        todo!()
    }

    fn gen_move(
        _to_reg: crate::Writable<crate::Reg>,
        _from_reg: crate::Reg,
        _ty: crate::ir::Type,
    ) -> Self::I {
        todo!()
    }

    fn gen_extend(
        _to_reg: crate::Writable<crate::Reg>,
        _from_reg: crate::Reg,
        _is_signed: bool,
        _from_bits: u8,
        _to_bits: u8,
    ) -> Self::I {
        todo!()
    }

    fn gen_args(args: Vec<ArgPair>) -> Inst {
        Inst::Args { args }
    }

    fn gen_rets(rets: Vec<RetPair>) -> Inst {
        Inst::Rets { rets }
    }

    fn gen_add_imm(
        _call_conv: crate::isa::CallConv,
        _into_reg: crate::Writable<crate::Reg>,
        _from_reg: crate::Reg,
        _imm: u32,
    ) -> crate::machinst::SmallInstVec<Self::I> {
        todo!()
    }

    fn gen_stack_lower_bound_trap(
        _limit_reg: crate::Reg,
    ) -> crate::machinst::SmallInstVec<Self::I> {
        todo!()
    }

    fn gen_get_stack_addr(
        _mem: crate::machinst::StackAMode,
        _into_reg: crate::Writable<crate::Reg>,
    ) -> Self::I {
        todo!()
    }

    fn get_stacklimit_reg(_call_conv: crate::isa::CallConv) -> crate::Reg {
        todo!()
    }

    fn gen_load_base_offset(
        _into_reg: crate::Writable<crate::Reg>,
        _base: crate::Reg,
        _offset: i32,
        _ty: crate::ir::Type,
    ) -> Self::I {
        todo!()
    }

    fn gen_store_base_offset(
        _base: crate::Reg,
        _offset: i32,
        _from_reg: crate::Reg,
        _ty: crate::ir::Type,
    ) -> Self::I {
        todo!()
    }

    fn gen_sp_reg_adjust(_amount: i32) -> crate::machinst::SmallInstVec<Self::I> {
        todo!()
    }

    // Compute frame layout.  Follows the aarch64 pattern: compute clobber_size,
    // then determine setup_area_size based on whether we need FP/LR saved.
    fn compute_frame_layout(
        _call_conv: isa::CallConv,
        _flags: &settings::Flags,
        _sig: &ir::Signature,
        regs: &[Writable<RealReg>],
        function_calls: FunctionCalls,
        incoming_args_size: u32,
        _tail_args_size: u32,
        stackslots_size: u32,
        fixed_frame_storage_size: u32,
        outgoing_args_size: u32,
    ) -> FrameLayout {
        // Filter regs to only callee-saved registers that must be preserved.
        // AAPCS callee-saves: r4-r11 + lr (r14).  Skip them if they're unused in the sig.
        let caller_save = |reg: RealReg| {
            let n = reg.hw_enc();
            n < 4 || n == 13 || n == 15 // sp and pc are never saved; r0-r3, ip (r12) are caller-save
        };

        let mut callee_saved_regs: Vec<Writable<RealReg>> = regs
            .iter()
            .cloned()
            .filter(|r| !caller_save(r.to_reg()))
            .collect();
        callee_saved_regs.sort_unstable();

        // Each saved register is 4 bytes.
        let clobber_size: u32 = callee_saved_regs.len() as u32 * 4;

        // We need a linkage frame (setup area) if there are any clobbers,
        // incoming args on the stack, fixed-frame storage, outgoing args,
        // or if this function makes calls (need LR saved).
        let needs_linkage_frame = !callee_saved_regs.is_empty()
            || incoming_args_size > 0
            || function_calls != FunctionCalls::None;

        // Setup area: FP + LR = 8 bytes when needed.
        let setup_area_size = if needs_linkage_frame { 8 } else { 0 };

        FrameLayout {
            word_bytes: 4,
            incoming_args_size,
            tail_args_size: _tail_args_size,
            setup_area_size,
            clobber_size,
            fixed_frame_storage_size,
            stackslots_size,
            outgoing_args_size,
            clobbered_callee_saves: callee_saved_regs,
            function_calls,
        }
    }

    fn gen_prologue_frame_setup(
        _call_conv: isa::CallConv,
        _flags: &settings::Flags,
        _isa_flags: &Arm32Flags,
        frame_layout: &FrameLayout,
    ) -> SmallInstVec<Inst> {
        let setup_frame = frame_layout.setup_area_size > 0;
        if !setup_frame {
            return SmallVec::new();
        }

        // Save FP and LR onto the stack.
        // push {r11, lr} — equivalent to STMDB.W sp!, {sp-reg..lr-reg}.
        // Register list: r11 = bit 11, lr = bit 14 → mask = (1<<11) | (1<<14).
        let fp_lr_mask: u16 = (1 << 11) | (1 << 14);
        let mut insts = SmallVec::new();
        insts.push(Inst::Push { rs: fp_lr_mask });
        insts
    }

    fn gen_epilogue_frame_restore(
        _call_conv: isa::CallConv,
        _flags: &settings::Flags,
        _isa_flags: &Arm32Flags,
        frame_layout: &FrameLayout,
    ) -> SmallInstVec<Inst> {
        let setup_frame = frame_layout.setup_area_size > 0;
        if !setup_frame {
            return SmallVec::new();
        }

        // Pop FP and LR.
        // pop {r11, lr} — LDMIA.W sp!, {list}.
        // Register list: r11 = bit 11, lr = bit 14 → mask same as push.
        let fp_lr_mask: u16 = (1 << 11) | (1 << 14);
        let mut insts = SmallVec::new();
        insts.push(Inst::Pop { rt: fp_lr_mask });
        insts
    }

    fn gen_return(
        _call_conv: crate::isa::CallConv,
        _isa_flags: &Arm32Flags,
        frame_layout: &crate::FrameLayout,
    ) -> crate::machinst::SmallInstVec<Inst> {
        // If the epilogue already popped PC (direct return), this becomes empty.
        // Otherwise emit BX LR for explicit return.
        if frame_layout.setup_area_size > 0 && frame_layout.clobber_size > 0 {
            // Epilogue uses pop {csave, pc} — nothing to do here.
            SmallVec::new()
        } else {
            let mut insts = SmallVec::new();
            insts.push(Inst::Ret);
            insts
        }
    }

    fn gen_probestack(_insts: &mut crate::machinst::SmallInstVec<Self::I>, _frame_size: u32) {
        todo!()
    }

    fn gen_inline_probestack(
        _insts: &mut crate::machinst::SmallInstVec<Self::I>,
        _call_conv: crate::isa::CallConv,
        _frame_size: u32,
        _guard_size: u32,
    ) {
        todo!()
    }

    fn gen_clobber_save(
        _call_conv: isa::CallConv,
        _flags: &settings::Flags,
        frame_layout: &FrameLayout,
    ) -> SmallVec<[Inst; 16]> {
        if frame_layout.clobber_size == 0 {
            return SmallVec::new();
        }

        let clobbered_int = frame_layout.clobbered_callee_saves_by_class().0;

        // Build a single push mask for all callee-saved registers.
        let mut mask: u16 = 0;
        for r in clobbered_int {
            mask |= 1 << r.to_reg().hw_enc();
        }
        let mut insts = SmallVec::new();
        insts.push(Inst::Push { rs: mask });
        insts
    }

    fn gen_clobber_restore(
        _call_conv: isa::CallConv,
        _flags: &settings::Flags,
        frame_layout: &FrameLayout,
    ) -> SmallVec<[Inst; 16]> {
        if frame_layout.clobber_size == 0 {
            return SmallVec::new();
        }

        // On exit, pop the callee-saved registers plus PC to return directly.
        let clobbered_int = frame_layout.clobbered_callee_saves_by_class().0;

        // Build a single pop mask including PC for direct return.
        let mut mask: u16 = 0;
        for r in clobbered_int {
            mask |= 1 << r.to_reg().hw_enc();
        }
        // Add bit 15 (PC) so that popping into the return address triggers a return.
        mask |= 1 << 15;

        let mut insts = SmallVec::new();
        insts.push(Inst::Pop { rt: mask });
        insts
    }

    fn gen_memcpy<F: FnMut(crate::ir::Type) -> crate::Writable<crate::Reg>>(
        _call_conv: crate::isa::CallConv,
        _dst: crate::Reg,
        _src: crate::Reg,
        _size: usize,
        _alloc_tmp: F,
    ) -> smallvec::SmallVec<[Self::I; 8]> {
        todo!()
    }

    fn get_number_of_spillslots_for_value(
        _rc: crate::RegClass,
        _target_vector_bytes: u32,
        _isa_flags: &Self::F,
    ) -> u32 {
        todo!()
    }

    fn get_machine_env(_flags: &settings::Flags, _call_conv: isa::CallConv) -> &MachineEnv {
        static DEFAULT_ENV: MachineEnv = create_arm32_reg_environment();

        /// Create the ARM32 register environment.
        /// R0-R3 = caller-save (parameter/return/passing registers in AAPCS)
        /// R4-R11 = callee-saved in AAPCS
        /// R12 (IP1) = scratch/callee-save in AAPCS
        /// R13 (SP), R14 (LR), R15 (PC) are excluded.
        const fn create_arm32_reg_environment() -> MachineEnv {
            let preferred: [PRegSet; 3] = [PRegSet::empty(), PRegSet::empty(), PRegSet::empty()];

            // Caller-save (preferred for allocation): R0-R3, R12.
            let caller_save = PRegSet::empty()
                .with(PReg::new(0, RegClass::Int))
                .with(PReg::new(1, RegClass::Int))
                .with(PReg::new(2, RegClass::Int))
                .with(PReg::new(3, RegClass::Int))
                .with(PReg::new(12, RegClass::Int));

            // Callee-saved (non-preferred): R4-R11.
            let callee_save = PRegSet::empty()
                .with(PReg::new(4, RegClass::Int))
                .with(PReg::new(5, RegClass::Int))
                .with(PReg::new(6, RegClass::Int))
                .with(PReg::new(7, RegClass::Int))
                .with(PReg::new(8, RegClass::Int))
                .with(PReg::new(9, RegClass::Int))
                .with(PReg::new(10, RegClass::Int))
                .with(PReg::new(11, RegClass::Int));

            let non_preferred: [PRegSet; 3] = [caller_save, callee_save, PRegSet::empty()];
            MachineEnv {
                preferred_regs_by_class: preferred,
                non_preferred_regs_by_class: non_preferred,
                fixed_stack_slots: vec![],
                scratch_by_class: [None, None, None],
            }
        }

        &DEFAULT_ENV
    }

    fn get_regs_clobbered_by_call(
        _call_conv_of_callee: crate::isa::CallConv,
        _is_exception: bool,
    ) -> regalloc2::PRegSet {
        todo!()
    }

    fn get_ext_mode(
        _call_conv: crate::isa::CallConv,
        specified: crate::ir::ArgumentExtension,
    ) -> crate::ir::ArgumentExtension {
        specified
    }

    fn retval_temp_reg(_call_conv_of_callee: crate::isa::CallConv) -> crate::Writable<crate::Reg> {
        todo!()
    }
}
