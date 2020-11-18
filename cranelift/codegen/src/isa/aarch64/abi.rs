//! Implementation of a standard AArch64 ABI.

use crate::ir;
use crate::ir::types;
use crate::ir::types::*;
use crate::ir::MemFlags;
use crate::isa;
use crate::isa::aarch64::{inst::EmitState, inst::*};
use crate::machinst::*;
use crate::settings;
use crate::{CodegenError, CodegenResult};
use alloc::boxed::Box;
use alloc::vec::Vec;
use regalloc::{RealReg, Reg, RegClass, Set, Writable};
use smallvec::SmallVec;

// We use a generic implementation that factors out AArch64 and x64 ABI commonalities, because
// these ABIs are very similar.

/// Support for the AArch64 ABI from the callee side (within a function body).
pub(crate) type AArch64ABICallee = ABICalleeImpl<AArch64MachineDeps>;

/// Support for the AArch64 ABI from the caller side (at a callsite).
pub(crate) type AArch64ABICaller = ABICallerImpl<AArch64MachineDeps>;

// Spidermonkey specific ABI convention.

/// This is SpiderMonkey's `WasmTableCallSigReg`.
static BALDRDASH_SIG_REG: u8 = 10;

/// This is SpiderMonkey's `WasmTlsReg`.
static BALDRDASH_TLS_REG: u8 = 23;

/// Offset in stack-arg area to callee-TLS slot in Baldrdash-2020 calling convention.
static BALDRDASH_CALLEE_TLS_OFFSET: i64 = 0;
/// Offset in stack-arg area to caller-TLS slot in Baldrdash-2020 calling convention.
static BALDRDASH_CALLER_TLS_OFFSET: i64 = 8;

// These two lists represent the registers the JIT may *not* use at any point in generated code.
//
// So these are callee-preserved from the JIT's point of view, and every register not in this list
// has to be caller-preserved by definition.
//
// Keep these lists in sync with the NonAllocatableMask set in Spidermonkey's
// Architecture-arm64.cpp.

// Indexed by physical register number.
#[rustfmt::skip]
static BALDRDASH_JIT_CALLEE_SAVED_GPR: &[bool] = &[
    /* 0 = */ false, false, false, false, false, false, false, false,
    /* 8 = */ false, false, false, false, false, false, false, false,
    /* 16 = */ true /* x16 / ip1 */, true /* x17 / ip2 */, true /* x18 / TLS */, false,
    /* 20 = */ false, false, false, false,
    /* 24 = */ false, false, false, false,
    // There should be 28, the pseudo stack pointer in this list, however the wasm stubs trash it
    // gladly right now.
    /* 28 = */ false, false, true /* x30 = FP */, false /* x31 = SP */
];

#[rustfmt::skip]
static BALDRDASH_JIT_CALLEE_SAVED_FPU: &[bool] = &[
    /* 0 = */ false, false, false, false, false, false, false, false,
    /* 8 = */ false, false, false, false, false, false, false, false,
    /* 16 = */ false, false, false, false, false, false, false, false,
    /* 24 = */ false, false, false, false, false, false, false, true /* v31 / d31 */
];

/// This is the limit for the size of argument and return-value areas on the
/// stack. We place a reasonable limit here to avoid integer overflow issues
/// with 32-bit arithmetic: for now, 128 MB.
static STACK_ARG_RET_SIZE_LIMIT: u64 = 128 * 1024 * 1024;

/// Try to fill a Baldrdash register, returning it if it was found.
fn try_fill_baldrdash_reg(call_conv: isa::CallConv, param: &ir::AbiParam) -> Option<ABIArg> {
    if call_conv.extends_baldrdash() {
        match &param.purpose {
            &ir::ArgumentPurpose::VMContext => {
                // This is SpiderMonkey's `WasmTlsReg`.
                Some(ABIArg::Reg(
                    xreg(BALDRDASH_TLS_REG).to_real_reg(),
                    ir::types::I64,
                    param.extension,
                    param.purpose,
                ))
            }
            &ir::ArgumentPurpose::SignatureId => {
                // This is SpiderMonkey's `WasmTableCallSigReg`.
                Some(ABIArg::Reg(
                    xreg(BALDRDASH_SIG_REG).to_real_reg(),
                    ir::types::I64,
                    param.extension,
                    param.purpose,
                ))
            }
            &ir::ArgumentPurpose::CalleeTLS => {
                // This is SpiderMonkey's callee TLS slot in the extended frame of Wasm's ABI-2020.
                assert!(call_conv == isa::CallConv::Baldrdash2020);
                Some(ABIArg::Stack(
                    BALDRDASH_CALLEE_TLS_OFFSET,
                    ir::types::I64,
                    ir::ArgumentExtension::None,
                    param.purpose,
                ))
            }
            &ir::ArgumentPurpose::CallerTLS => {
                // This is SpiderMonkey's caller TLS slot in the extended frame of Wasm's ABI-2020.
                assert!(call_conv == isa::CallConv::Baldrdash2020);
                Some(ABIArg::Stack(
                    BALDRDASH_CALLER_TLS_OFFSET,
                    ir::types::I64,
                    ir::ArgumentExtension::None,
                    param.purpose,
                ))
            }
            _ => None,
        }
    } else {
        None
    }
}

impl Into<AMode> for StackAMode {
    fn into(self) -> AMode {
        match self {
            StackAMode::FPOffset(off, ty) => AMode::FPOffset(off, ty),
            StackAMode::NominalSPOffset(off, ty) => AMode::NominalSPOffset(off, ty),
            StackAMode::SPOffset(off, ty) => AMode::SPOffset(off, ty),
        }
    }
}

// Returns the size of stack space needed to store the
// `int_reg` and `vec_reg`.
fn saved_reg_stack_size(
    int_reg: &[Writable<RealReg>],
    vec_reg: &[Writable<RealReg>],
) -> (usize, usize) {
    // Round up to multiple of 2, to keep 16-byte stack alignment.
    let int_save_bytes = (int_reg.len() + (int_reg.len() & 1)) * 8;
    let vec_save_bytes = vec_reg.len() * 16;
    (int_save_bytes, vec_save_bytes)
}

/// AArch64-specific ABI behavior. This struct just serves as an implementation
/// point for the trait; it is never actually instantiated.
pub(crate) struct AArch64MachineDeps;

impl ABIMachineSpec for AArch64MachineDeps {
    type I = Inst;

    fn word_bits() -> u32 {
        64
    }

    /// Return required stack alignment in bytes.
    fn stack_align(_call_conv: isa::CallConv) -> u32 {
        16
    }

    fn compute_arg_locs(
        call_conv: isa::CallConv,
        params: &[ir::AbiParam],
        args_or_rets: ArgsOrRets,
        add_ret_area_ptr: bool,
    ) -> CodegenResult<(Vec<ABIArg>, i64, Option<usize>)> {
        let is_baldrdash = call_conv.extends_baldrdash();
        let has_baldrdash_tls = call_conv == isa::CallConv::Baldrdash2020;

        // See AArch64 ABI (https://c9x.me/compile/bib/abi-arm64.pdf), sections 5.4.
        let mut next_xreg = 0;
        let mut next_vreg = 0;
        let mut next_stack: u64 = 0;
        let mut ret = vec![];

        if args_or_rets == ArgsOrRets::Args && has_baldrdash_tls {
            // Baldrdash ABI-2020 always has two stack-arg slots reserved, for the callee and
            // caller TLS-register values, respectively.
            next_stack = 16;
        }

        // Note on return values: on the regular non-baldrdash ABI, we may return values in 8
        // registers for V128 and I64 registers independently of the number of register values
        // returned in the other class. That is, we can return values in up to 8 integer and 8
        // vector registers at once.
        // In Baldrdash, we can only use one register for return value for all the register
        // classes. That is, we can't return values in both one integer and one vector register;
        // only one return value may be in a register.

        let (max_per_class_reg_vals, mut remaining_reg_vals) = match (args_or_rets, is_baldrdash) {
            (ArgsOrRets::Args, _) => (8, 16),     // x0-x7 and v0-v7
            (ArgsOrRets::Rets, false) => (8, 16), // x0-x7 and v0-v7
            (ArgsOrRets::Rets, true) => (1, 1),   // x0 or v0, but not both
        };

        for i in 0..params.len() {
            // Process returns backward, according to the SpiderMonkey ABI (which we
            // adopt internally if `is_baldrdash` is set).
            let param = match (args_or_rets, is_baldrdash) {
                (ArgsOrRets::Args, _) => &params[i],
                (ArgsOrRets::Rets, false) => &params[i],
                (ArgsOrRets::Rets, true) => &params[params.len() - 1 - i],
            };

            // Validate "purpose".
            match &param.purpose {
                &ir::ArgumentPurpose::VMContext
                | &ir::ArgumentPurpose::Normal
                | &ir::ArgumentPurpose::StackLimit
                | &ir::ArgumentPurpose::SignatureId
                | &ir::ArgumentPurpose::CallerTLS
                | &ir::ArgumentPurpose::CalleeTLS => {}
                _ => panic!(
                    "Unsupported argument purpose {:?} in signature: {:?}",
                    param.purpose, params
                ),
            }

            assert!(
                legal_type_for_machine(param.value_type),
                "Invalid type for AArch64: {:?}",
                param.value_type
            );
            let rc = Inst::rc_for_type(param.value_type).unwrap();

            let next_reg = match rc {
                RegClass::I64 => &mut next_xreg,
                RegClass::V128 => &mut next_vreg,
                _ => panic!("Invalid register class: {:?}", rc),
            };

            if let Some(param) = try_fill_baldrdash_reg(call_conv, param) {
                assert!(rc == RegClass::I64);
                ret.push(param);
            } else if *next_reg < max_per_class_reg_vals && remaining_reg_vals > 0 {
                let reg = match rc {
                    RegClass::I64 => xreg(*next_reg),
                    RegClass::V128 => vreg(*next_reg),
                    _ => unreachable!(),
                };
                ret.push(ABIArg::Reg(
                    reg.to_real_reg(),
                    param.value_type,
                    param.extension,
                    param.purpose,
                ));
                *next_reg += 1;
                remaining_reg_vals -= 1;
            } else {
                // Compute size. Every arg takes a minimum slot of 8 bytes. (16-byte
                // stack alignment happens separately after all args.)
                let size = (ty_bits(param.value_type) / 8) as u64;
                let size = std::cmp::max(size, 8);
                // Align.
                debug_assert!(size.is_power_of_two());
                next_stack = (next_stack + size - 1) & !(size - 1);
                ret.push(ABIArg::Stack(
                    next_stack as i64,
                    param.value_type,
                    param.extension,
                    param.purpose,
                ));
                next_stack += size;
            }
        }

        if args_or_rets == ArgsOrRets::Rets && is_baldrdash {
            ret.reverse();
        }

        let extra_arg = if add_ret_area_ptr {
            debug_assert!(args_or_rets == ArgsOrRets::Args);
            if next_xreg < max_per_class_reg_vals && remaining_reg_vals > 0 {
                ret.push(ABIArg::Reg(
                    xreg(next_xreg).to_real_reg(),
                    I64,
                    ir::ArgumentExtension::None,
                    ir::ArgumentPurpose::Normal,
                ));
            } else {
                ret.push(ABIArg::Stack(
                    next_stack as i64,
                    I64,
                    ir::ArgumentExtension::None,
                    ir::ArgumentPurpose::Normal,
                ));
                next_stack += 8;
            }
            Some(ret.len() - 1)
        } else {
            None
        };

        next_stack = (next_stack + 15) & !15;

        // To avoid overflow issues, limit the arg/return size to something
        // reasonable -- here, 128 MB.
        if next_stack > STACK_ARG_RET_SIZE_LIMIT {
            return Err(CodegenError::ImplLimitExceeded);
        }

        Ok((ret, next_stack as i64, extra_arg))
    }

    fn fp_to_arg_offset(call_conv: isa::CallConv, flags: &settings::Flags) -> i64 {
        if call_conv.extends_baldrdash() {
            let num_words = flags.baldrdash_prologue_words() as i64;
            debug_assert!(num_words > 0, "baldrdash must set baldrdash_prologue_words");
            debug_assert_eq!(num_words % 2, 0, "stack must be 16-aligned");
            num_words * 8
        } else {
            16 // frame pointer + return address.
        }
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

    fn gen_ret() -> Inst {
        Inst::Ret
    }

    fn gen_add_imm(into_reg: Writable<Reg>, from_reg: Reg, imm: u32) -> SmallVec<[Inst; 4]> {
        let imm = imm as u64;
        let mut insts = SmallVec::new();
        if let Some(imm12) = Imm12::maybe_from_u64(imm) {
            insts.push(Inst::AluRRImm12 {
                alu_op: ALUOp::Add64,
                rd: into_reg,
                rn: from_reg,
                imm12,
            });
        } else {
            let scratch2 = writable_tmp2_reg();
            assert_ne!(scratch2.to_reg(), from_reg);
            insts.extend(Inst::load_constant(scratch2, imm.into()));
            insts.push(Inst::AluRRRExtend {
                alu_op: ALUOp::Add64,
                rd: into_reg,
                rn: from_reg,
                rm: scratch2.to_reg(),
                extendop: ExtendOp::UXTX,
            });
        }
        insts
    }

    fn gen_stack_lower_bound_trap(limit_reg: Reg) -> SmallVec<[Inst; 2]> {
        let mut insts = SmallVec::new();
        insts.push(Inst::AluRRRExtend {
            alu_op: ALUOp::SubS64,
            rd: writable_zero_reg(),
            rn: stack_reg(),
            rm: limit_reg,
            extendop: ExtendOp::UXTX,
        });
        insts.push(Inst::TrapIf {
            trap_code: ir::TrapCode::StackOverflow,
            // Here `Lo` == "less than" when interpreting the two
            // operands as unsigned integers.
            kind: CondBrKind::Cond(Cond::Lo),
        });
        insts
    }

    fn gen_epilogue_placeholder() -> Inst {
        Inst::EpiloguePlaceholder
    }

    fn gen_get_stack_addr(mem: StackAMode, into_reg: Writable<Reg>, _ty: Type) -> Inst {
        let mem = mem.into();
        Inst::LoadAddr { rd: into_reg, mem }
    }

    fn get_stacklimit_reg() -> Reg {
        spilltmp_reg()
    }

    fn gen_load_base_offset(into_reg: Writable<Reg>, base: Reg, offset: i32, ty: Type) -> Inst {
        let mem = AMode::RegOffset(base, offset as i64, ty);
        Inst::gen_load(into_reg, mem, ty, MemFlags::trusted())
    }

    fn gen_store_base_offset(base: Reg, offset: i32, from_reg: Reg, ty: Type) -> Inst {
        let mem = AMode::RegOffset(base, offset as i64, ty);
        Inst::gen_store(mem, from_reg, ty, MemFlags::trusted())
    }

    fn gen_sp_reg_adjust(amount: i32) -> SmallVec<[Inst; 2]> {
        if amount == 0 {
            return SmallVec::new();
        }

        let (amount, is_sub) = if amount > 0 {
            (amount as u64, false)
        } else {
            (-amount as u64, true)
        };

        let alu_op = if is_sub { ALUOp::Sub64 } else { ALUOp::Add64 };

        let mut ret = SmallVec::new();
        if let Some(imm12) = Imm12::maybe_from_u64(amount) {
            let adj_inst = Inst::AluRRImm12 {
                alu_op,
                rd: writable_stack_reg(),
                rn: stack_reg(),
                imm12,
            };
            ret.push(adj_inst);
        } else {
            let tmp = writable_spilltmp_reg();
            let const_inst = Inst::load_constant(tmp, amount);
            let adj_inst = Inst::AluRRRExtend {
                alu_op,
                rd: writable_stack_reg(),
                rn: stack_reg(),
                rm: tmp.to_reg(),
                extendop: ExtendOp::UXTX,
            };
            ret.extend(const_inst);
            ret.push(adj_inst);
        }
        ret
    }

    fn gen_nominal_sp_adj(offset: i32) -> Inst {
        Inst::VirtualSPOffsetAdj {
            offset: offset as i64,
        }
    }

    fn gen_prologue_frame_setup() -> SmallVec<[Inst; 2]> {
        let mut insts = SmallVec::new();
        // stp fp (x29), lr (x30), [sp, #-16]!
        insts.push(Inst::StoreP64 {
            rt: fp_reg(),
            rt2: link_reg(),
            mem: PairAMode::PreIndexed(
                writable_stack_reg(),
                SImm7Scaled::maybe_from_i64(-16, types::I64).unwrap(),
            ),
            flags: MemFlags::trusted(),
        });
        // mov fp (x29), sp. This uses the ADDI rd, rs, 0 form of `MOV` because
        // the usual encoding (`ORR`) does not work with SP.
        insts.push(Inst::AluRRImm12 {
            alu_op: ALUOp::Add64,
            rd: writable_fp_reg(),
            rn: stack_reg(),
            imm12: Imm12 {
                bits: 0,
                shift12: false,
            },
        });
        insts
    }

    fn gen_epilogue_frame_restore() -> SmallVec<[Inst; 2]> {
        let mut insts = SmallVec::new();

        // MOV (alias of ORR) interprets x31 as XZR, so use an ADD here.
        // MOV to SP is an alias of ADD.
        insts.push(Inst::AluRRImm12 {
            alu_op: ALUOp::Add64,
            rd: writable_stack_reg(),
            rn: fp_reg(),
            imm12: Imm12 {
                bits: 0,
                shift12: false,
            },
        });
        insts.push(Inst::LoadP64 {
            rt: writable_fp_reg(),
            rt2: writable_link_reg(),
            mem: PairAMode::PostIndexed(
                writable_stack_reg(),
                SImm7Scaled::maybe_from_i64(16, types::I64).unwrap(),
            ),
            flags: MemFlags::trusted(),
        });

        insts
    }

    // Returns stack bytes used as well as instructions. Does not adjust
    // nominal SP offset; abi_impl generic code will do that.
    fn gen_clobber_save(
        call_conv: isa::CallConv,
        _: &settings::Flags,
        clobbers: &Set<Writable<RealReg>>,
        fixed_frame_storage_size: u32,
        _outgoing_args_size: u32,
    ) -> (u64, SmallVec<[Inst; 16]>) {
        let mut insts = SmallVec::new();
        let (clobbered_int, clobbered_vec) = get_regs_saved_in_prologue(call_conv, clobbers);

        let (int_save_bytes, vec_save_bytes) = saved_reg_stack_size(&clobbered_int, &clobbered_vec);
        let total_save_bytes = (vec_save_bytes + int_save_bytes) as i32;
        insts.extend(Self::gen_sp_reg_adjust(
            -(total_save_bytes + fixed_frame_storage_size as i32),
        ));

        for (i, reg_pair) in clobbered_int.chunks(2).enumerate() {
            let (r1, r2) = if reg_pair.len() == 2 {
                // .to_reg().to_reg(): Writable<RealReg> --> RealReg --> Reg
                (reg_pair[0].to_reg().to_reg(), reg_pair[1].to_reg().to_reg())
            } else {
                (reg_pair[0].to_reg().to_reg(), zero_reg())
            };

            debug_assert!(r1.get_class() == RegClass::I64);
            debug_assert!(r2.get_class() == RegClass::I64);

            // stp r1, r2, [sp, #(i * #16)]
            insts.push(Inst::StoreP64 {
                rt: r1,
                rt2: r2,
                mem: PairAMode::SignedOffset(
                    stack_reg(),
                    SImm7Scaled::maybe_from_i64((i * 16) as i64, types::I64).unwrap(),
                ),
                flags: MemFlags::trusted(),
            });
        }

        let vec_offset = int_save_bytes;
        for (i, reg) in clobbered_vec.iter().enumerate() {
            insts.push(Inst::FpuStore128 {
                rd: reg.to_reg().to_reg(),
                mem: AMode::Unscaled(
                    stack_reg(),
                    SImm9::maybe_from_i64((vec_offset + (i * 16)) as i64).unwrap(),
                ),
                flags: MemFlags::trusted(),
            });
        }

        (total_save_bytes as u64, insts)
    }

    fn gen_clobber_restore(
        call_conv: isa::CallConv,
        flags: &settings::Flags,
        clobbers: &Set<Writable<RealReg>>,
        _fixed_frame_storage_size: u32,
        _outgoing_args_size: u32,
    ) -> SmallVec<[Inst; 16]> {
        let mut insts = SmallVec::new();
        let (clobbered_int, clobbered_vec) = get_regs_saved_in_prologue(call_conv, clobbers);

        let (int_save_bytes, vec_save_bytes) = saved_reg_stack_size(&clobbered_int, &clobbered_vec);
        for (i, reg_pair) in clobbered_int.chunks(2).enumerate() {
            let (r1, r2) = if reg_pair.len() == 2 {
                (
                    reg_pair[0].map(|r| r.to_reg()),
                    reg_pair[1].map(|r| r.to_reg()),
                )
            } else {
                (reg_pair[0].map(|r| r.to_reg()), writable_zero_reg())
            };

            debug_assert!(r1.to_reg().get_class() == RegClass::I64);
            debug_assert!(r2.to_reg().get_class() == RegClass::I64);

            // ldp r1, r2, [sp, #(i * 16)]
            insts.push(Inst::LoadP64 {
                rt: r1,
                rt2: r2,
                mem: PairAMode::SignedOffset(
                    stack_reg(),
                    SImm7Scaled::maybe_from_i64((i * 16) as i64, types::I64).unwrap(),
                ),
                flags: MemFlags::trusted(),
            });
        }

        for (i, reg) in clobbered_vec.iter().enumerate() {
            insts.push(Inst::FpuLoad128 {
                rd: Writable::from_reg(reg.to_reg().to_reg()),
                mem: AMode::Unscaled(
                    stack_reg(),
                    SImm9::maybe_from_i64(((i * 16) + int_save_bytes) as i64).unwrap(),
                ),
                flags: MemFlags::trusted(),
            });
        }

        // For non-baldrdash calling conventions, the frame pointer
        // will be moved into the stack pointer in the epilogue, so we
        // can skip restoring the stack pointer value with this `add`.
        if call_conv.extends_baldrdash() {
            let total_save_bytes = (int_save_bytes + vec_save_bytes) as i32;
            insts.extend(Self::gen_sp_reg_adjust(total_save_bytes));
        }

        // If this is Baldrdash-2020, restore the callee (i.e., our) TLS
        // register. We may have allocated it for something else and clobbered
        // it, but the ABI expects us to leave the TLS register unchanged.
        if call_conv == isa::CallConv::Baldrdash2020 {
            let off = BALDRDASH_CALLEE_TLS_OFFSET + Self::fp_to_arg_offset(call_conv, flags);
            insts.push(Inst::gen_load(
                writable_xreg(BALDRDASH_TLS_REG),
                AMode::UnsignedOffset(fp_reg(), UImm12Scaled::maybe_from_i64(off, I64).unwrap()),
                I64,
                MemFlags::trusted(),
            ));
        }

        insts
    }

    fn gen_call(
        dest: &CallDest,
        uses: Vec<Reg>,
        defs: Vec<Writable<Reg>>,
        opcode: ir::Opcode,
        tmp: Writable<Reg>,
        callee_conv: isa::CallConv,
        caller_conv: isa::CallConv,
    ) -> SmallVec<[(InstIsSafepoint, Inst); 2]> {
        let mut insts = SmallVec::new();
        match &dest {
            &CallDest::ExtName(ref name, RelocDistance::Near) => insts.push((
                InstIsSafepoint::Yes,
                Inst::Call {
                    info: Box::new(CallInfo {
                        dest: name.clone(),
                        uses,
                        defs,
                        opcode,
                        caller_callconv: caller_conv,
                        callee_callconv: callee_conv,
                    }),
                },
            )),
            &CallDest::ExtName(ref name, RelocDistance::Far) => {
                insts.push((
                    InstIsSafepoint::No,
                    Inst::LoadExtName {
                        rd: tmp,
                        name: Box::new(name.clone()),
                        offset: 0,
                    },
                ));
                insts.push((
                    InstIsSafepoint::Yes,
                    Inst::CallInd {
                        info: Box::new(CallIndInfo {
                            rn: tmp.to_reg(),
                            uses,
                            defs,
                            opcode,
                            caller_callconv: caller_conv,
                            callee_callconv: callee_conv,
                        }),
                    },
                ));
            }
            &CallDest::Reg(reg) => insts.push((
                InstIsSafepoint::Yes,
                Inst::CallInd {
                    info: Box::new(CallIndInfo {
                        rn: *reg,
                        uses,
                        defs,
                        opcode,
                        caller_callconv: caller_conv,
                        callee_callconv: callee_conv,
                    }),
                },
            )),
        }

        insts
    }

    fn get_number_of_spillslots_for_value(rc: RegClass, ty: Type) -> u32 {
        // We allocate in terms of 8-byte slots.
        match (rc, ty) {
            (RegClass::I64, _) => 1,
            (RegClass::V128, F32) | (RegClass::V128, F64) => 1,
            (RegClass::V128, _) => 2,
            _ => panic!("Unexpected register class!"),
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

    fn get_regs_clobbered_by_call(call_conv_of_callee: isa::CallConv) -> Vec<Writable<Reg>> {
        let mut caller_saved = Vec::new();
        for i in 0..29 {
            let x = writable_xreg(i);
            if is_reg_clobbered_by_call(call_conv_of_callee, x.to_reg().to_real_reg()) {
                caller_saved.push(x);
            }
        }
        for i in 0..32 {
            let v = writable_vreg(i);
            if is_reg_clobbered_by_call(call_conv_of_callee, v.to_reg().to_real_reg()) {
                caller_saved.push(v);
            }
        }
        caller_saved
    }

    fn get_ext_mode(
        call_conv: isa::CallConv,
        specified: ir::ArgumentExtension,
    ) -> ir::ArgumentExtension {
        if call_conv.extends_baldrdash() {
            // Baldrdash (SpiderMonkey) always extends args and return values to the full register.
            specified
        } else {
            // No other supported ABI on AArch64 does so.
            ir::ArgumentExtension::None
        }
    }
}

/// Is this type supposed to be seen on this machine? E.g. references of the
/// wrong width are invalid.
fn legal_type_for_machine(ty: Type) -> bool {
    match ty {
        R32 => false,
        _ => true,
    }
}

/// Is the given register saved in the prologue if clobbered, i.e., is it a
/// callee-save?
fn is_reg_saved_in_prologue(call_conv: isa::CallConv, r: RealReg) -> bool {
    if call_conv.extends_baldrdash() {
        match r.get_class() {
            RegClass::I64 => {
                let enc = r.get_hw_encoding();
                return BALDRDASH_JIT_CALLEE_SAVED_GPR[enc];
            }
            RegClass::V128 => {
                let enc = r.get_hw_encoding();
                return BALDRDASH_JIT_CALLEE_SAVED_FPU[enc];
            }
            _ => unimplemented!("baldrdash callee saved on non-i64 reg classes"),
        };
    }

    match r.get_class() {
        RegClass::I64 => {
            // x19 - x28 inclusive are callee-saves.
            r.get_hw_encoding() >= 19 && r.get_hw_encoding() <= 28
        }
        RegClass::V128 => {
            // v8 - v15 inclusive are callee-saves.
            r.get_hw_encoding() >= 8 && r.get_hw_encoding() <= 15
        }
        _ => panic!("Unexpected RegClass"),
    }
}

/// Return the set of all integer and vector registers that must be saved in the
/// prologue and restored in the epilogue, given the set of all registers
/// written by the function's body.
fn get_regs_saved_in_prologue(
    call_conv: isa::CallConv,
    regs: &Set<Writable<RealReg>>,
) -> (Vec<Writable<RealReg>>, Vec<Writable<RealReg>>) {
    let mut int_saves = vec![];
    let mut vec_saves = vec![];
    for &reg in regs.iter() {
        if is_reg_saved_in_prologue(call_conv, reg.to_reg()) {
            match reg.to_reg().get_class() {
                RegClass::I64 => int_saves.push(reg),
                RegClass::V128 => vec_saves.push(reg),
                _ => panic!("Unexpected RegClass"),
            }
        }
    }
    // Sort registers for deterministic code output. We can do an unstable sort because the
    // registers will be unique (there are no dups).
    int_saves.sort_unstable_by_key(|r| r.to_reg().get_index());
    vec_saves.sort_unstable_by_key(|r| r.to_reg().get_index());
    (int_saves, vec_saves)
}

fn is_reg_clobbered_by_call(call_conv_of_callee: isa::CallConv, r: RealReg) -> bool {
    if call_conv_of_callee.extends_baldrdash() {
        match r.get_class() {
            RegClass::I64 => {
                let enc = r.get_hw_encoding();
                if !BALDRDASH_JIT_CALLEE_SAVED_GPR[enc] {
                    return true;
                }
                // Otherwise, fall through to preserve native's ABI caller-saved.
            }
            RegClass::V128 => {
                let enc = r.get_hw_encoding();
                if !BALDRDASH_JIT_CALLEE_SAVED_FPU[enc] {
                    return true;
                }
                // Otherwise, fall through to preserve native's ABI caller-saved.
            }
            _ => unimplemented!("baldrdash callee saved on non-i64 reg classes"),
        };
    }

    match r.get_class() {
        RegClass::I64 => {
            // x0 - x17 inclusive are caller-saves.
            r.get_hw_encoding() <= 17
        }
        RegClass::V128 => {
            // v0 - v7 inclusive and v16 - v31 inclusive are caller-saves. The
            // upper 64 bits of v8 - v15 inclusive are also caller-saves.
            // However, because we cannot currently represent partial registers
            // to regalloc.rs, we indicate here that every vector register is
            // caller-save. Because this function is used at *callsites*,
            // approximating in this direction (save more than necessary) is
            // conservative and thus safe.
            //
            // Note that we set the 'not included in clobber set' flag in the
            // regalloc.rs API when a call instruction's callee has the same ABI
            // as the caller (the current function body); this is safe (anything
            // clobbered by callee can be clobbered by caller as well) and
            // avoids unnecessary saves of v8-v15 in the prologue even though we
            // include them as defs here.
            true
        }
        _ => panic!("Unexpected RegClass"),
    }
}
