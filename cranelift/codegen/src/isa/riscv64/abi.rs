//! Implementation of a standard Riscv64 ABI.

use crate::ir;
use crate::ir::types::*;

use crate::ir::ExternalName;
use crate::ir::MemFlags;
use crate::isa;

use crate::isa::riscv64::{inst::EmitState, inst::*};
use crate::isa::CallConv;
use crate::machinst::*;

use crate::ir::types::I8;
use crate::ir::LibCall;
use crate::ir::Signature;
use crate::isa::riscv64::settings::Flags as RiscvFlags;
use crate::isa::unwind::UnwindInst;
use crate::settings;
use crate::CodegenError;
use crate::CodegenResult;
use alloc::boxed::Box;
use alloc::vec::Vec;
use regalloc2::PRegSet;
use regs::x_reg;

use smallvec::{smallvec, SmallVec};

/// Support for the Riscv64 ABI from the callee side (within a function body).
pub(crate) type Riscv64Callee = Callee<Riscv64MachineDeps>;

/// Support for the Riscv64 ABI from the caller side (at a callsite).
pub(crate) type Riscv64ABICaller = Caller<Riscv64MachineDeps>;

/// This is the limit for the size of argument and return-value areas on the
/// stack. We place a reasonable limit here to avoid integer overflow issues
/// with 32-bit arithmetic: for now, 128 MB.
static STACK_ARG_RET_SIZE_LIMIT: u32 = 128 * 1024 * 1024;

/// Riscv64-specific ABI behavior. This struct just serves as an implementation
/// point for the trait; it is never actually instantiated.
pub struct Riscv64MachineDeps;

impl IsaFlags for RiscvFlags {}

impl RiscvFlags {
    pub(crate) fn min_vec_reg_size(&self) -> u64 {
        let entries = [
            (self.has_zvl65536b(), 65536),
            (self.has_zvl32768b(), 32768),
            (self.has_zvl16384b(), 16384),
            (self.has_zvl8192b(), 8192),
            (self.has_zvl4096b(), 4096),
            (self.has_zvl2048b(), 2048),
            (self.has_zvl1024b(), 1024),
            (self.has_zvl512b(), 512),
            (self.has_zvl256b(), 256),
            // In order to claim the Application Profile V extension, a minimum
            // register size of 128 is required. i.e. V implies Zvl128b.
            (self.has_v(), 128),
            (self.has_zvl128b(), 128),
            (self.has_zvl64b(), 64),
            (self.has_zvl32b(), 32),
        ];

        for (has_flag, size) in entries.into_iter() {
            if has_flag {
                return size;
            }
        }

        return 0;
    }
}

impl ABIMachineSpec for Riscv64MachineDeps {
    type I = Inst;
    type F = RiscvFlags;

    fn word_bits() -> u32 {
        64
    }

    /// Return required stack alignment in bytes.
    fn stack_align(_call_conv: isa::CallConv) -> u32 {
        16
    }

    fn compute_arg_locs<'a, I>(
        call_conv: isa::CallConv,
        _flags: &settings::Flags,
        params: I,
        args_or_rets: ArgsOrRets,
        add_ret_area_ptr: bool,
        mut args: ArgsAccumulator<'_>,
    ) -> CodegenResult<(u32, Option<usize>)>
    where
        I: IntoIterator<Item = &'a ir::AbiParam>,
    {
        // All registers that can be used as parameters or rets.
        // both start and end are included.
        let (x_start, x_end, f_start, f_end) = if args_or_rets == ArgsOrRets::Args {
            (10, 17, 10, 17)
        } else {
            let end = if call_conv.extends_wasmtime() { 10 } else { 11 };
            (10, end, 10, end)
        };
        let mut next_x_reg = x_start;
        let mut next_f_reg = f_start;
        // Stack space.
        let mut next_stack: u32 = 0;
        let mut return_one_register_used = false;

        for param in params {
            if let ir::ArgumentPurpose::StructArgument(size) = param.purpose {
                let offset = next_stack;
                assert!(size % 8 == 0, "StructArgument size is not properly aligned");
                next_stack += size;
                args.push(ABIArg::StructArg {
                    pointer: None,
                    offset: offset as i64,
                    size: size as u64,
                    purpose: param.purpose,
                });
                continue;
            }

            // Find regclass(es) of the register(s) used to store a value of this type.
            let (rcs, reg_tys) = Inst::rc_for_type(param.value_type)?;
            let mut slots = ABIArgSlotVec::new();
            for (rc, reg_ty) in rcs.iter().zip(reg_tys.iter()) {
                let next_reg =
                    if (next_x_reg <= x_end) && *rc == RegClass::Int && !return_one_register_used {
                        let x = Some(x_reg(next_x_reg));
                        if args_or_rets == ArgsOrRets::Rets && call_conv.extends_wasmtime() {
                            return_one_register_used = true;
                        }
                        next_x_reg += 1;
                        x
                    } else if (next_f_reg <= f_end)
                        && *rc == RegClass::Float
                        && !return_one_register_used
                    {
                        let x = Some(f_reg(next_f_reg));
                        if args_or_rets == ArgsOrRets::Rets && call_conv.extends_wasmtime() {
                            return_one_register_used = true;
                        }
                        next_f_reg += 1;
                        x
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
                    // Compute size. For the wasmtime ABI it differs from native
                    // ABIs in how multiple values are returned, so we take a
                    // leaf out of arm64's book by not rounding everything up to
                    // 8 bytes. For all ABI arguments, and other ABI returns,
                    // though, each slot takes a minimum of 8 bytes.
                    //
                    // Note that in all cases 16-byte stack alignment happens
                    // separately after all args.
                    let size = reg_ty.bits() / 8;
                    let size = if args_or_rets == ArgsOrRets::Rets && call_conv.extends_wasmtime() {
                        size
                    } else {
                        std::cmp::max(size, 8)
                    };
                    // Align.
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
        }
        let pos: Option<usize> = if add_ret_area_ptr {
            assert!(ArgsOrRets::Args == args_or_rets);
            if next_x_reg <= x_end {
                let arg = ABIArg::reg(
                    x_reg(next_x_reg).to_real_reg().unwrap(),
                    I64,
                    ir::ArgumentExtension::None,
                    ir::ArgumentPurpose::Normal,
                );
                args.push(arg);
            } else {
                let arg = ABIArg::stack(
                    next_stack as i64,
                    I64,
                    ir::ArgumentExtension::None,
                    ir::ArgumentPurpose::Normal,
                );
                args.push(arg);
                next_stack += 8;
            }
            Some(args.args().len() - 1)
        } else {
            None
        };
        next_stack = align_to(next_stack, Self::stack_align(call_conv));
        // To avoid overflow issues, limit the arg/return size to something
        // reasonable -- here, 128 MB.
        if next_stack > STACK_ARG_RET_SIZE_LIMIT {
            return Err(CodegenError::ImplLimitExceeded);
        }
        CodegenResult::Ok((next_stack, pos))
    }

    fn fp_to_arg_offset(_call_conv: isa::CallConv, _flags: &settings::Flags) -> i64 {
        // lr fp.
        16
    }

    fn gen_load_stack(mem: StackAMode, into_reg: Writable<Reg>, ty: Type) -> Inst {
        Inst::gen_load(into_reg, mem.into(), ty, MemFlags::trusted())
    }

    fn gen_store_stack(mem: StackAMode, from_reg: Reg, ty: Type) -> Inst {
        Inst::gen_store(mem.into(), from_reg, ty, MemFlags::trusted())
    }

    fn gen_move(to_reg: Writable<Reg>, from_reg: Reg, ty: Type) -> Inst {
        Inst::gen_move(to_reg, from_reg, ty)
    }

    fn gen_extend(
        to_reg: Writable<Reg>,
        from_reg: Reg,
        signed: bool,
        from_bits: u8,
        to_bits: u8,
    ) -> Inst {
        assert!(from_bits < to_bits);
        Inst::Extend {
            rd: to_reg,
            rn: from_reg,
            signed,
            from_bits,
            to_bits,
        }
    }

    fn get_ext_mode(
        _call_conv: isa::CallConv,
        specified: ir::ArgumentExtension,
    ) -> ir::ArgumentExtension {
        specified
    }

    fn gen_args(_isa_flags: &crate::isa::riscv64::settings::Flags, args: Vec<ArgPair>) -> Inst {
        Inst::Args { args }
    }

    fn gen_ret(_setup_frame: bool, _isa_flags: &Self::F, rets: Vec<RetPair>) -> Inst {
        Inst::Ret { rets }
    }

    fn get_stacklimit_reg() -> Reg {
        spilltmp_reg()
    }

    fn gen_add_imm(into_reg: Writable<Reg>, from_reg: Reg, imm: u32) -> SmallInstVec<Inst> {
        let mut insts = SmallInstVec::new();
        if let Some(imm12) = Imm12::maybe_from_u64(imm as u64) {
            insts.push(Inst::AluRRImm12 {
                alu_op: AluOPRRI::Andi,
                rd: into_reg,
                rs: from_reg,
                imm12,
            });
        } else {
            insts.extend(Inst::load_constant_u32(
                writable_spilltmp_reg2(),
                imm as u64,
                &mut |_| writable_spilltmp_reg2(),
            ));
            insts.push(Inst::AluRRR {
                alu_op: AluOPRRR::Add,
                rd: into_reg,
                rs1: spilltmp_reg2(),
                rs2: from_reg,
            });
        }
        insts
    }

    fn gen_stack_lower_bound_trap(limit_reg: Reg) -> SmallInstVec<Inst> {
        let mut insts = SmallVec::new();
        insts.push(Inst::TrapIfC {
            cc: IntCC::UnsignedLessThan,
            rs1: stack_reg(),
            rs2: limit_reg,
            trap_code: ir::TrapCode::StackOverflow,
        });
        insts
    }

    fn gen_get_stack_addr(mem: StackAMode, into_reg: Writable<Reg>, _ty: Type) -> Inst {
        Inst::LoadAddr {
            rd: into_reg,
            mem: mem.into(),
        }
    }

    fn gen_load_base_offset(into_reg: Writable<Reg>, base: Reg, offset: i32, ty: Type) -> Inst {
        let mem = AMode::RegOffset(base, offset as i64, ty);
        Inst::gen_load(into_reg, mem, ty, MemFlags::trusted())
    }

    fn gen_store_base_offset(base: Reg, offset: i32, from_reg: Reg, ty: Type) -> Inst {
        let mem = AMode::RegOffset(base, offset as i64, ty);
        Inst::gen_store(mem, from_reg, ty, MemFlags::trusted())
    }

    fn gen_sp_reg_adjust(amount: i32) -> SmallInstVec<Inst> {
        let mut insts = SmallVec::new();
        if amount == 0 {
            return insts;
        }
        insts.push(Inst::AjustSp {
            amount: amount as i64,
        });
        insts
    }

    fn gen_nominal_sp_adj(offset: i32) -> Inst {
        Inst::VirtualSPOffsetAdj {
            amount: offset as i64,
        }
    }

    fn gen_prologue_frame_setup(flags: &settings::Flags) -> SmallInstVec<Inst> {
        // add  sp,sp,-16    ;; alloc stack space for fp.
        // sd   ra,8(sp)     ;; save ra.
        // sd   fp,0(sp)     ;; store old fp.
        // mv   fp,sp        ;; set fp to sp.
        let mut insts = SmallVec::new();
        insts.push(Inst::AjustSp { amount: -16 });
        insts.push(Self::gen_store_stack(
            StackAMode::SPOffset(8, I64),
            link_reg(),
            I64,
        ));
        insts.push(Self::gen_store_stack(
            StackAMode::SPOffset(0, I64),
            fp_reg(),
            I64,
        ));
        if flags.unwind_info() {
            insts.push(Inst::Unwind {
                inst: UnwindInst::PushFrameRegs {
                    offset_upward_to_caller_sp: 16, // FP, LR
                },
            });
        }
        insts.push(Inst::Mov {
            rd: writable_fp_reg(),
            rm: stack_reg(),
            ty: I64,
        });
        insts
    }
    /// reverse of gen_prologue_frame_setup.
    fn gen_epilogue_frame_restore(_: &settings::Flags) -> SmallInstVec<Inst> {
        let mut insts = SmallVec::new();
        insts.push(Self::gen_load_stack(
            StackAMode::SPOffset(8, I64),
            writable_link_reg(),
            I64,
        ));
        insts.push(Self::gen_load_stack(
            StackAMode::SPOffset(0, I64),
            writable_fp_reg(),
            I64,
        ));
        insts.push(Inst::AjustSp { amount: 16 });
        insts
    }

    fn gen_probestack(insts: &mut SmallInstVec<Self::I>, frame_size: u32) {
        insts.extend(Inst::load_constant_u32(
            writable_a0(),
            frame_size as u64,
            &mut |_| writable_a0(),
        ));
        insts.push(Inst::Call {
            info: Box::new(CallInfo {
                dest: ExternalName::LibCall(LibCall::Probestack),
                uses: smallvec![CallArgPair {
                    vreg: a0(),
                    preg: a0(),
                }],
                defs: smallvec![],
                clobbers: PRegSet::empty(),
                opcode: Opcode::Call,
                callee_callconv: CallConv::SystemV,
                caller_callconv: CallConv::SystemV,
            }),
        });
    }
    // Returns stack bytes used as well as instructions. Does not adjust
    // nominal SP offset; abi_impl generic code will do that.
    fn gen_clobber_save(
        _call_conv: isa::CallConv,
        setup_frame: bool,
        flags: &settings::Flags,
        clobbered_callee_saves: &[Writable<RealReg>],
        fixed_frame_storage_size: u32,
        _outgoing_args_size: u32,
    ) -> (u64, SmallVec<[Inst; 16]>) {
        let mut insts = SmallVec::new();
        let clobbered_size = compute_clobber_size(&clobbered_callee_saves);
        // Adjust the stack pointer downward for clobbers and the function fixed
        // frame (spillslots and storage slots).
        let stack_size = fixed_frame_storage_size + clobbered_size;
        if flags.unwind_info() && setup_frame {
            // The *unwind* frame (but not the actual frame) starts at the
            // clobbers, just below the saved FP/LR pair.
            insts.push(Inst::Unwind {
                inst: UnwindInst::DefineNewFrame {
                    offset_downward_to_clobbers: clobbered_size,
                    offset_upward_to_caller_sp: 16, // FP, LR
                },
            });
        }
        // Store each clobbered register in order at offsets from SP,
        // placing them above the fixed frame slots.
        if stack_size > 0 {
            // since we use fp, we didn't need use UnwindInst::StackAlloc.
            let mut cur_offset = 8;
            for reg in clobbered_callee_saves {
                let r_reg = reg.to_reg();
                let ty = match r_reg.class() {
                    RegClass::Int => I64,
                    RegClass::Float => F64,
                    RegClass::Vector => unimplemented!("Vector Clobber Saves"),
                };
                if flags.unwind_info() {
                    insts.push(Inst::Unwind {
                        inst: UnwindInst::SaveReg {
                            clobber_offset: clobbered_size - cur_offset,
                            reg: r_reg,
                        },
                    });
                }
                insts.push(Self::gen_store_stack(
                    StackAMode::SPOffset(-(cur_offset as i64), ty),
                    real_reg_to_reg(reg.to_reg()),
                    ty,
                ));
                cur_offset += 8
            }
            insts.push(Inst::AjustSp {
                amount: -(stack_size as i64),
            });
        }
        (clobbered_size as u64, insts)
    }

    fn gen_clobber_restore(
        call_conv: isa::CallConv,
        sig: &Signature,
        _flags: &settings::Flags,
        clobbers: &[Writable<RealReg>],
        fixed_frame_storage_size: u32,
        _outgoing_args_size: u32,
    ) -> SmallVec<[Inst; 16]> {
        let mut insts = SmallVec::new();
        let clobbered_callee_saves =
            Self::get_clobbered_callee_saves(call_conv, _flags, sig, clobbers);
        let stack_size = fixed_frame_storage_size + compute_clobber_size(&clobbered_callee_saves);
        if stack_size > 0 {
            insts.push(Inst::AjustSp {
                amount: stack_size as i64,
            });
        }
        let mut cur_offset = 8;
        for reg in &clobbered_callee_saves {
            let rreg = reg.to_reg();
            let ty = match rreg.class() {
                RegClass::Int => I64,
                RegClass::Float => F64,
                RegClass::Vector => unimplemented!("Vector Clobber Restores"),
            };
            insts.push(Self::gen_load_stack(
                StackAMode::SPOffset(-cur_offset, ty),
                Writable::from_reg(real_reg_to_reg(reg.to_reg())),
                ty,
            ));
            cur_offset += 8
        }
        insts
    }

    fn gen_call(
        dest: &CallDest,
        uses: CallArgList,
        defs: CallRetList,
        clobbers: PRegSet,
        opcode: ir::Opcode,
        tmp: Writable<Reg>,
        callee_conv: isa::CallConv,
        caller_conv: isa::CallConv,
    ) -> SmallVec<[Self::I; 2]> {
        let mut insts = SmallVec::new();
        match &dest {
            &CallDest::ExtName(ref name, RelocDistance::Near) => insts.push(Inst::Call {
                info: Box::new(CallInfo {
                    dest: name.clone(),
                    uses,
                    defs,
                    clobbers,
                    opcode,
                    caller_callconv: caller_conv,
                    callee_callconv: callee_conv,
                }),
            }),
            &CallDest::ExtName(ref name, RelocDistance::Far) => {
                insts.push(Inst::LoadExtName {
                    rd: tmp,
                    name: Box::new(name.clone()),
                    offset: 0,
                });
                insts.push(Inst::CallInd {
                    info: Box::new(CallIndInfo {
                        rn: tmp.to_reg(),
                        uses,
                        defs,
                        clobbers,
                        opcode,
                        caller_callconv: caller_conv,
                        callee_callconv: callee_conv,
                    }),
                });
            }
            &CallDest::Reg(reg) => insts.push(Inst::CallInd {
                info: Box::new(CallIndInfo {
                    rn: *reg,
                    uses,
                    defs,
                    clobbers,
                    opcode,
                    caller_callconv: caller_conv,
                    callee_callconv: callee_conv,
                }),
            }),
        }
        insts
    }

    fn gen_memcpy<F: FnMut(Type) -> Writable<Reg>>(
        call_conv: isa::CallConv,
        dst: Reg,
        src: Reg,
        size: usize,
        mut alloc_tmp: F,
    ) -> SmallVec<[Self::I; 8]> {
        let mut insts = SmallVec::new();
        let arg0 = Writable::from_reg(x_reg(10));
        let arg1 = Writable::from_reg(x_reg(11));
        let arg2 = Writable::from_reg(x_reg(12));
        let tmp = alloc_tmp(Self::word_type());
        insts.extend(Inst::load_constant_u64(tmp, size as u64, &mut alloc_tmp).into_iter());
        insts.push(Inst::Call {
            info: Box::new(CallInfo {
                dest: ExternalName::LibCall(LibCall::Memcpy),
                uses: smallvec![
                    CallArgPair {
                        vreg: dst,
                        preg: arg0.to_reg()
                    },
                    CallArgPair {
                        vreg: src,
                        preg: arg1.to_reg()
                    },
                    CallArgPair {
                        vreg: tmp.to_reg(),
                        preg: arg2.to_reg()
                    }
                ],
                defs: smallvec![],
                clobbers: Self::get_regs_clobbered_by_call(call_conv),
                opcode: Opcode::Call,
                caller_callconv: call_conv,
                callee_callconv: call_conv,
            }),
        });
        insts
    }

    fn get_number_of_spillslots_for_value(
        rc: RegClass,
        _target_vector_bytes: u32,
        isa_flags: &RiscvFlags,
    ) -> u32 {
        // We allocate in terms of 8-byte slots.
        match rc {
            RegClass::Int => 1,
            RegClass::Float => 1,
            RegClass::Vector => (isa_flags.min_vec_reg_size() / 8) as u32,
        }
    }

    /// Get the current virtual-SP offset from an instruction-emission state.
    fn get_virtual_sp_offset_from_state(s: &EmitState) -> i64 {
        s.virtual_sp_offset
    }

    /// Get the nominal-SP-to-FP offset from an instruction-emission state.
    fn get_nominal_sp_to_fp(s: &EmitState) -> i64 {
        s.nominal_sp_to_fp
    }

    fn get_regs_clobbered_by_call(_call_conv_of_callee: isa::CallConv) -> PRegSet {
        let x_clobbers = CALLER_SAVE_X_REG
            .iter()
            .enumerate()
            .filter(|(_, clobbered)| **clobbered)
            .map(|(i, _)| px_reg(i));
        let f_clobbers = CALLER_SAVE_F_REG
            .iter()
            .enumerate()
            .filter(|(_, clobbered)| **clobbered)
            .map(|(i, _)| pf_reg(i));
        // We clobber all vector registers (and vector state) on calls.
        let v_clobbers = (0..=31).map(|i| pv_reg(i));

        let mut clobbers = PRegSet::empty();
        for clobber in x_clobbers.chain(f_clobbers).chain(v_clobbers) {
            clobbers.add(clobber);
        }
        clobbers
    }

    fn get_clobbered_callee_saves(
        call_conv: isa::CallConv,
        _flags: &settings::Flags,
        _sig: &Signature,
        regs: &[Writable<RealReg>],
    ) -> Vec<Writable<RealReg>> {
        let mut regs: Vec<Writable<RealReg>> = regs
            .iter()
            .cloned()
            .filter(|r| is_reg_saved_in_prologue(call_conv, r.to_reg()))
            .collect();

        regs.sort();
        regs
    }

    fn is_frame_setup_needed(
        is_leaf: bool,
        stack_args_size: u32,
        num_clobbered_callee_saves: usize,
        fixed_frame_storage_size: u32,
    ) -> bool {
        !is_leaf
            // The function arguments that are passed on the stack are addressed
            // relative to the Frame Pointer.
            || stack_args_size > 0
            || num_clobbered_callee_saves > 0
        || fixed_frame_storage_size > 0
    }

    fn gen_inline_probestack(insts: &mut SmallInstVec<Self::I>, frame_size: u32, guard_size: u32) {
        // Unroll at most n consecutive probes, before falling back to using a loop
        const PROBE_MAX_UNROLL: u32 = 3;
        // Number of probes that we need to perform
        let probe_count = align_to(frame_size, guard_size) / guard_size;

        if probe_count <= PROBE_MAX_UNROLL {
            Self::gen_probestack_unroll(insts, guard_size, probe_count)
        } else {
            Self::gen_probestack_loop(insts, guard_size, probe_count)
        }
    }
}

const CALLER_SAVE_X_REG: [bool; 32] = [
    false, true, false, false, false, true, true, true, // 0-7
    false, false, true, true, true, true, true, true, // 8-15
    true, true, false, false, false, false, false, false, // 16-23
    false, false, false, false, true, true, true, true, // 24-31
];
const CALLEE_SAVE_X_REG: [bool; 32] = [
    false, false, true, false, false, false, false, false, // 0-7
    true, true, false, false, false, false, false, false, // 8-15
    false, false, true, true, true, true, true, true, // 16-23
    true, true, true, true, false, false, false, false, // 24-31
];
const CALLER_SAVE_F_REG: [bool; 32] = [
    true, true, true, true, true, true, true, true, // 0-7
    false, true, true, true, true, true, true, true, // 8-15
    true, true, false, false, false, false, false, false, // 16-23
    false, false, false, false, true, true, true, true, // 24-31
];
const CALLEE_SAVE_F_REG: [bool; 32] = [
    false, false, false, false, false, false, false, false, // 0-7
    true, false, false, false, false, false, false, false, // 8-15
    false, false, true, true, true, true, true, true, // 16-23
    true, true, true, true, false, false, false, false, // 24-31
];

/// This should be the registers that must be saved by callee.
#[inline]
fn is_reg_saved_in_prologue(_conv: CallConv, reg: RealReg) -> bool {
    match reg.class() {
        RegClass::Int => CALLEE_SAVE_X_REG[reg.hw_enc() as usize],
        RegClass::Float => CALLEE_SAVE_F_REG[reg.hw_enc() as usize],
        // All vector registers are caller saved.
        RegClass::Vector => false,
    }
}

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

impl Riscv64MachineDeps {
    fn gen_probestack_unroll(insts: &mut SmallInstVec<Inst>, guard_size: u32, probe_count: u32) {
        insts.reserve(probe_count as usize);
        for i in 0..probe_count {
            let offset = (guard_size * (i + 1)) as i64;
            insts.push(Self::gen_store_stack(
                StackAMode::SPOffset(-offset, I8),
                zero_reg(),
                I32,
            ));
        }
    }

    fn gen_probestack_loop(insts: &mut SmallInstVec<Inst>, guard_size: u32, probe_count: u32) {
        insts.push(Inst::StackProbeLoop {
            guard_size,
            probe_count,
            tmp: Writable::from_reg(x_reg(28)), // t3
        });
    }
}
