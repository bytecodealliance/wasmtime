//! Implementation of a standard Arm32 ABI.

use crate::CodegenResult;
use crate::ir;
use crate::ir::types::*;
use crate::ir::{Signature, Type};
use crate::isa;
use crate::isa::arm32::inst::*;
use crate::isa::arm32::settings::Flags as Arm32Flags;
use crate::machinst::*;
use crate::settings;
use alloc::vec::Vec;
use regalloc2::{MachineEnv, PRegSet};
use smallvec::{SmallVec, smallvec};

/// Support for the arm32 ABI from the callee side (within a function body).
pub(crate) type Arm32Callee = Callee<Arm32MachineDeps>;

/// arm32-specific ABI behavior. This struct just serves as an implementation
/// point for the trait; it is never actually instantiated.
pub struct Arm32MachineDeps;

impl IsaFlags for Arm32Flags {}

impl ABIMachineSpec for Arm32MachineDeps {
    type I = Inst;
    type F = Arm32Flags;

    /// 128 MB, as with the other backends, to avoid overflow with 32-bit
    /// arithmetic on stack offsets.
    const STACK_ARG_RET_SIZE_LIMIT: u32 = 128 * 1024 * 1024;

    fn word_bits() -> u32 {
        32
    }

    fn stack_align(_call_conv: isa::CallConv) -> u32 {
        // AAPCS requires 8-byte stack alignment at public interfaces.
        8
    }

    fn compute_arg_locs(
        call_conv: isa::CallConv,
        flags: &settings::Flags,
        params: &[ir::AbiParam],
        args_or_rets: ArgsOrRets,
        add_ret_area_ptr: bool,
        mut args: ArgsAccumulator,
    ) -> CodegenResult<(u32, Option<usize>)> {
        // Integer arguments/returns go in r0-r3 (r0-r1 for returns), the rest on
        // the stack. This is a simplified AAPCS.
        let (x_start, x_end, d_end) = match args_or_rets {
            ArgsOrRets::Args => (0u8, 3u8, 7u8),
            ArgsOrRets::Rets => (0u8, 1u8, 1u8),
        };
        let mut next_x_reg = x_start;
        // VFP (hard-float) register counter: each float value takes one D reg
        // (an f32 uses the low S-half). This is a simplified AAPCS-VFP that does
        // not back-fill S registers.
        let mut next_d_reg = 0u8;
        let mut next_stack: u32 = 0;

        let ret_area_ptr = if add_ret_area_ptr {
            assert!(ArgsOrRets::Args == args_or_rets);
            let reg = xreg(next_x_reg);
            next_x_reg += 1;
            Some(ABIArg::reg(
                reg.to_real_reg().unwrap(),
                I32,
                ir::ArgumentExtension::None,
                ir::ArgumentPurpose::Normal,
            ))
        } else {
            None
        };

        for param in params {
            if let ir::ArgumentPurpose::StructArgument(_) = param.purpose {
                panic!("StructArgument parameters are not supported on arm32.");
            }

            let (rcs, reg_tys) = Inst::rc_for_type(param.value_type)?;

            // AAPCS: a 64-bit value passed in registers must start in an
            // even-numbered register (an r0:r1 / r2:r3 pair). If the aligned
            // pair would not fully fit, it goes entirely on the (8-aligned)
            // stack, so bump `next_x_reg` past the argument registers.
            if rcs.len() == 2 {
                next_x_reg = align_to(u32::from(next_x_reg), 2) as u8;
                if next_x_reg + 1 > x_end {
                    next_x_reg = x_end + 1;
                    next_stack = align_to(next_stack, 8);
                }
            }

            let mut slots = ABIArgSlotVec::new();
            for (rc, reg_ty) in rcs.iter().zip(reg_tys.iter()) {
                let reg = match rc {
                    RegClass::Float if next_d_reg <= d_end => {
                        let r = dreg(next_d_reg);
                        next_d_reg += 1;
                        Some(r)
                    }
                    RegClass::Int if next_x_reg <= x_end => {
                        let r = xreg(next_x_reg);
                        next_x_reg += 1;
                        Some(r)
                    }
                    RegClass::Int | RegClass::Float => None,
                    RegClass::Vector => unreachable!("arm32 has no vector registers"),
                };
                if let Some(reg) = reg {
                    slots.push(ABIArgSlot::Reg {
                        reg: reg.to_real_reg().unwrap(),
                        ty: *reg_ty,
                        extension: param.extension,
                    });
                } else {
                    if args_or_rets == ArgsOrRets::Rets && !flags.enable_multi_ret_implicit_sret() {
                        return Err(crate::CodegenError::Unsupported(
                            "Too many return values to fit in registers.".into(),
                        ));
                    }
                    let size = core::cmp::max(reg_ty.bytes(), 4);
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

    fn gen_load_stack(mem: StackAMode, into_reg: Writable<Reg>, _ty: Type) -> Inst {
        if into_reg.to_reg().class() == RegClass::Float {
            Inst::FpuLoad {
                size: FpuSize::F64,
                rd: into_reg,
                mem: amode_from_stack(mem),
            }
        } else {
            Inst::Load {
                rt: into_reg,
                mem: amode_from_stack(mem),
                kind: LoadKind::Word,
            }
        }
    }

    fn gen_store_stack(mem: StackAMode, from_reg: Reg, _ty: Type) -> Inst {
        if from_reg.class() == RegClass::Float {
            Inst::FpuStore {
                size: FpuSize::F64,
                rt: from_reg,
                mem: amode_from_stack(mem),
            }
        } else {
            Inst::Store {
                rt: from_reg,
                mem: amode_from_stack(mem),
                kind: StoreKind::Word,
            }
        }
    }

    fn gen_move(to_reg: Writable<Reg>, from_reg: Reg, ty: Type) -> Inst {
        Inst::gen_move(to_reg, from_reg, ty)
    }

    fn gen_extend(
        _to_reg: Writable<Reg>,
        _from_reg: Reg,
        _signed: bool,
        _from_bits: u8,
        _to_bits: u8,
    ) -> Inst {
        unimplemented!("arm32: integer extension not yet implemented")
    }

    fn get_ext_mode(
        _call_conv: isa::CallConv,
        specified: ir::ArgumentExtension,
    ) -> ir::ArgumentExtension {
        specified
    }

    fn gen_args(args: Vec<ArgPair>) -> Inst {
        Inst::Args { args }
    }

    fn gen_rets(rets: Vec<RetPair>) -> Inst {
        Inst::Rets { rets }
    }

    fn get_stacklimit_reg(_call_conv: isa::CallConv) -> Reg {
        spilltmp_reg()
    }

    fn gen_add_imm(
        _call_conv: isa::CallConv,
        _into_reg: Writable<Reg>,
        _from_reg: Reg,
        _imm: u32,
    ) -> SmallInstVec<Inst> {
        unimplemented!("arm32: gen_add_imm not yet implemented")
    }

    fn gen_stack_lower_bound_trap(_limit_reg: Reg) -> SmallInstVec<Inst> {
        unimplemented!("arm32: stack-limit checks not yet implemented")
    }

    fn gen_get_stack_addr(_mem: StackAMode, _into_reg: Writable<Reg>) -> Inst {
        unimplemented!("arm32: stack-address computation not yet implemented")
    }

    fn gen_load_base_offset(into_reg: Writable<Reg>, base: Reg, offset: i32, _ty: Type) -> Inst {
        let mem = AMode::RegOffset { rn: base, offset };
        if into_reg.to_reg().class() == RegClass::Float {
            Inst::FpuLoad {
                size: FpuSize::F64,
                rd: into_reg,
                mem,
            }
        } else {
            Inst::Load {
                rt: into_reg,
                mem,
                kind: LoadKind::Word,
            }
        }
    }

    fn gen_store_base_offset(base: Reg, offset: i32, from_reg: Reg, _ty: Type) -> Inst {
        let mem = AMode::RegOffset { rn: base, offset };
        if from_reg.class() == RegClass::Float {
            Inst::FpuStore {
                size: FpuSize::F64,
                rt: from_reg,
                mem,
            }
        } else {
            Inst::Store {
                rt: from_reg,
                mem,
                kind: StoreKind::Word,
            }
        }
    }

    fn gen_sp_reg_adjust(amount: i32) -> SmallInstVec<Inst> {
        if amount == 0 {
            smallvec![]
        } else {
            smallvec![Inst::AdjustSp { amount }]
        }
    }

    fn gen_prologue_frame_setup(
        _call_conv: isa::CallConv,
        _flags: &settings::Flags,
        _isa_flags: &Arm32Flags,
        frame_layout: &FrameLayout,
    ) -> SmallInstVec<Inst> {
        let mut insts = smallvec![];
        if frame_layout.setup_area_size > 0 {
            // push {fp, lr}; mov fp, sp
            insts.push(Inst::Push {
                reg_list: FP_LR_REG_LIST,
            });
            insts.push(Inst::MovReg {
                rd: writable_fp_reg(),
                rm: stack_reg(),
            });
        }
        insts
    }

    fn gen_epilogue_frame_restore(
        _call_conv: isa::CallConv,
        _flags: &settings::Flags,
        _isa_flags: &Arm32Flags,
        frame_layout: &FrameLayout,
    ) -> SmallInstVec<Inst> {
        let mut insts = smallvec![];
        if frame_layout.setup_area_size > 0 {
            // pop {fp, lr}
            insts.push(Inst::Pop {
                reg_list: FP_LR_REG_LIST,
            });
        }
        insts
    }

    fn gen_return(
        _call_conv: isa::CallConv,
        _isa_flags: &Arm32Flags,
        _frame_layout: &FrameLayout,
    ) -> SmallInstVec<Inst> {
        smallvec![Inst::Ret]
    }

    fn gen_probestack(_insts: &mut SmallInstVec<Self::I>, _frame_size: u32) {
        unimplemented!("arm32: probestack not yet implemented")
    }

    fn gen_inline_probestack(
        _insts: &mut SmallInstVec<Self::I>,
        _call_conv: isa::CallConv,
        _frame_size: u32,
        _guard_size: u32,
    ) {
        unimplemented!("arm32: inline probestack not yet implemented")
    }

    fn gen_clobber_save(
        _call_conv: isa::CallConv,
        _flags: &settings::Flags,
        frame_layout: &FrameLayout,
    ) -> SmallVec<[Inst; 16]> {
        let mut insts = smallvec![];
        let stack_size = frame_layout.clobber_size
            + frame_layout.fixed_frame_storage_size
            + frame_layout.outgoing_args_size;
        if stack_size > 0 {
            insts.extend(Self::gen_sp_reg_adjust(-(stack_size as i32)));
            // Callee-saved registers are saved at the *top* of the frame, above
            // the fixed stack-slot and outgoing-argument regions (which resolve
            // from sp upward). Placing them from sp=0 would alias those regions
            // and corrupt saved registers.
            let mut cur_offset = 0u32;
            for reg in &frame_layout.clobbered_callee_saves {
                let r = Reg::from(reg.to_reg());
                let bytes = if r.class() == RegClass::Float { 8 } else { 4 };
                cur_offset = align_to(cur_offset, bytes);
                let mem = AMode::SPOffset {
                    offset: i64::from(stack_size - cur_offset - bytes),
                };
                if r.class() == RegClass::Float {
                    insts.push(Inst::FpuStore {
                        size: FpuSize::F64,
                        rt: r,
                        mem,
                    });
                } else {
                    insts.push(Inst::Store {
                        rt: r,
                        mem,
                        kind: StoreKind::Word,
                    });
                }
                cur_offset += bytes;
            }
        }
        insts
    }

    fn gen_clobber_restore(
        _call_conv: isa::CallConv,
        _flags: &settings::Flags,
        frame_layout: &FrameLayout,
    ) -> SmallVec<[Inst; 16]> {
        let mut insts = smallvec![];
        let stack_size = frame_layout.clobber_size
            + frame_layout.fixed_frame_storage_size
            + frame_layout.outgoing_args_size;
        // Mirror the top-of-frame placement used by `gen_clobber_save`.
        let mut cur_offset = 0u32;
        for reg in &frame_layout.clobbered_callee_saves {
            let is_float = reg.to_reg().class() == RegClass::Float;
            let bytes = if is_float { 8 } else { 4 };
            cur_offset = align_to(cur_offset, bytes);
            let mem = AMode::SPOffset {
                offset: i64::from(stack_size - cur_offset - bytes),
            };
            if is_float {
                insts.push(Inst::FpuLoad {
                    size: FpuSize::F64,
                    rd: reg.map(Reg::from),
                    mem,
                });
            } else {
                insts.push(Inst::Load {
                    rt: reg.map(Reg::from),
                    mem,
                    kind: LoadKind::Word,
                });
            }
            cur_offset += bytes;
        }
        if stack_size > 0 {
            insts.extend(Self::gen_sp_reg_adjust(stack_size as i32));
        }
        insts
    }

    fn gen_memcpy<F: FnMut(Type) -> Writable<Reg>>(
        _call_conv: isa::CallConv,
        _dst: Reg,
        _src: Reg,
        _size: usize,
        _alloc_tmp: F,
    ) -> SmallVec<[Self::I; 8]> {
        unimplemented!("arm32: memcpy not yet implemented")
    }

    fn get_number_of_spillslots_for_value(
        rc: RegClass,
        _target_vector_bytes: u32,
        _isa_flags: &Arm32Flags,
    ) -> u32 {
        match rc {
            RegClass::Int => 1,
            // A D register is 8 bytes = two 4-byte spill slots.
            RegClass::Float => 2,
            RegClass::Vector => unimplemented!("arm32 has no vector registers"),
        }
    }

    fn get_machine_env(_flags: &settings::Flags, _call_conv: isa::CallConv) -> &MachineEnv {
        static MACHINE_ENV: MachineEnv = create_reg_environment();
        &MACHINE_ENV
    }

    fn get_regs_clobbered_by_call(
        _call_conv_of_callee: isa::CallConv,
        _is_exception: bool,
    ) -> PRegSet {
        DEFAULT_CLOBBERS
    }

    fn compute_frame_layout(
        call_conv: isa::CallConv,
        flags: &settings::Flags,
        _sig: &Signature,
        regs: &[Writable<RealReg>],
        function_calls: FunctionCalls,
        incoming_args_size: u32,
        tail_args_size: u32,
        stackslots_size: u32,
        fixed_frame_storage_size: u32,
        outgoing_args_size: u32,
    ) -> FrameLayout {
        let is_callee_saved = |reg: &Writable<RealReg>| match call_conv {
            isa::CallConv::PreserveAll => true,
            _ => DEFAULT_CALLEE_SAVES.contains(reg.to_reg().into()),
        };
        let mut regs: Vec<Writable<RealReg>> =
            regs.iter().cloned().filter(is_callee_saved).collect();
        regs.sort_unstable();

        let clobber_size = compute_clobber_size(&regs);

        let setup_area_size = if flags.preserve_frame_pointers()
            || function_calls != FunctionCalls::None
            || incoming_args_size > 0
            || clobber_size > 0
            || fixed_frame_storage_size > 0
        {
            8 // FP, LR
        } else {
            0
        };

        FrameLayout {
            word_bytes: 4,
            incoming_args_size,
            tail_args_size,
            setup_area_size,
            clobber_size,
            fixed_frame_storage_size,
            stackslots_size,
            outgoing_args_size,
            clobbered_callee_saves: regs,
            function_calls,
        }
    }

    fn retval_temp_reg(_call_conv_of_callee: isa::CallConv) -> Writable<Reg> {
        // r12 (ip) is caller-saved and never used to return a value.
        writable_xreg(12)
    }

    fn exception_payload_regs(_call_conv: isa::CallConv) -> &'static [Reg] {
        &[]
    }
}

/// Register-list mask for `{fp, lr}` (r11 and r14), as used by push/pop.
const FP_LR_REG_LIST: u32 = (1 << 11) | (1 << 14);

/// Convert a `StackAMode` (a nominal stack reference) into an `AMode` that is
/// resolved against the frame layout at emit time.
fn amode_from_stack(mem: StackAMode) -> AMode {
    match mem {
        StackAMode::IncomingArg(off, stack_args_size) => AMode::IncomingArg {
            offset: i64::from(stack_args_size) - off,
        },
        StackAMode::Slot(off) => AMode::SlotOffset { offset: off },
        StackAMode::OutgoingArg(off) => AMode::SPOffset { offset: off },
    }
}

/// Callee-saved registers under AAPCS: GPRs r4-r11 and VFP D8-D15.
const DEFAULT_CALLEE_SAVES: PRegSet = PRegSet::empty()
    .with(preg(4))
    .with(preg(5))
    .with(preg(6))
    .with(preg(7))
    .with(preg(8))
    .with(preg(9))
    .with(preg(10))
    .with(preg(11))
    .with(pdreg(8))
    .with(pdreg(9))
    .with(pdreg(10))
    .with(pdreg(11))
    .with(pdreg(12))
    .with(pdreg(13))
    .with(pdreg(14))
    .with(pdreg(15));

/// Caller-saved (clobbered) registers: GPRs r0-r3, r12 and VFP D0-D7.
const DEFAULT_CLOBBERS: PRegSet = PRegSet::empty()
    .with(preg(0))
    .with(preg(1))
    .with(preg(2))
    .with(preg(3))
    .with(preg(12))
    .with(pdreg(0))
    .with(pdreg(1))
    .with(pdreg(2))
    .with(pdreg(3))
    .with(pdreg(4))
    .with(pdreg(5))
    .with(pdreg(6))
    .with(pdreg(7));

fn compute_clobber_size(clobbers: &[Writable<RealReg>]) -> u32 {
    let mut size = 0;
    for reg in clobbers {
        match reg.to_reg().class() {
            RegClass::Int => size += 4,
            RegClass::Float => size += 8,
            RegClass::Vector => unreachable!("arm32 has no vector registers"),
        }
    }
    align_to(size, 8)
}
