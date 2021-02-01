//! Implementation of the standard x64 ABI.

use crate::ir::types::*;
use crate::ir::{self, types, ExternalName, LibCall, MemFlags, Opcode, TrapCode, Type};
use crate::isa;
use crate::isa::{x64::inst::*, CallConv};
use crate::machinst::abi_impl::*;
use crate::machinst::*;
use crate::settings;
use crate::{CodegenError, CodegenResult};
use alloc::boxed::Box;
use alloc::vec::Vec;
use args::*;
use regalloc::{RealReg, Reg, RegClass, Set, Writable};
use smallvec::{smallvec, SmallVec};
use std::convert::TryFrom;

/// This is the limit for the size of argument and return-value areas on the
/// stack. We place a reasonable limit here to avoid integer overflow issues
/// with 32-bit arithmetic: for now, 128 MB.
static STACK_ARG_RET_SIZE_LIMIT: u64 = 128 * 1024 * 1024;

/// Offset in stack-arg area to callee-TLS slot in Baldrdash-2020 calling convention.
static BALDRDASH_CALLEE_TLS_OFFSET: i64 = 0;
/// Offset in stack-arg area to caller-TLS slot in Baldrdash-2020 calling convention.
static BALDRDASH_CALLER_TLS_OFFSET: i64 = 8;

/// Try to fill a Baldrdash register, returning it if it was found.
fn try_fill_baldrdash_reg(call_conv: CallConv, param: &ir::AbiParam) -> Option<ABIArg> {
    if call_conv.extends_baldrdash() {
        match &param.purpose {
            &ir::ArgumentPurpose::VMContext => {
                // This is SpiderMonkey's `WasmTlsReg`.
                Some(ABIArg::Reg {
                    regs: ValueRegs::one(regs::r14().to_real_reg()),
                    ty: types::I64,
                    extension: param.extension,
                    purpose: param.purpose,
                })
            }
            &ir::ArgumentPurpose::SignatureId => {
                // This is SpiderMonkey's `WasmTableCallSigReg`.
                Some(ABIArg::Reg {
                    regs: ValueRegs::one(regs::r10().to_real_reg()),
                    ty: types::I64,
                    extension: param.extension,
                    purpose: param.purpose,
                })
            }
            &ir::ArgumentPurpose::CalleeTLS => {
                // This is SpiderMonkey's callee TLS slot in the extended frame of Wasm's ABI-2020.
                assert!(call_conv == isa::CallConv::Baldrdash2020);
                Some(ABIArg::Stack {
                    offset: BALDRDASH_CALLEE_TLS_OFFSET,
                    ty: ir::types::I64,
                    extension: ir::ArgumentExtension::None,
                    purpose: param.purpose,
                })
            }
            &ir::ArgumentPurpose::CallerTLS => {
                // This is SpiderMonkey's caller TLS slot in the extended frame of Wasm's ABI-2020.
                assert!(call_conv == isa::CallConv::Baldrdash2020);
                Some(ABIArg::Stack {
                    offset: BALDRDASH_CALLER_TLS_OFFSET,
                    ty: ir::types::I64,
                    extension: ir::ArgumentExtension::None,
                    purpose: param.purpose,
                })
            }
            _ => None,
        }
    } else {
        None
    }
}

/// Support for the x64 ABI from the callee side (within a function body).
pub(crate) type X64ABICallee = ABICalleeImpl<X64ABIMachineSpec>;

/// Support for the x64 ABI from the caller side (at a callsite).
pub(crate) type X64ABICaller = ABICallerImpl<X64ABIMachineSpec>;

/// Implementation of ABI primitives for x64.
pub(crate) struct X64ABIMachineSpec;

impl ABIMachineSpec for X64ABIMachineSpec {
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

        let mut next_gpr = 0;
        let mut next_vreg = 0;
        let mut next_stack: u64 = 0;
        let mut ret = vec![];

        if args_or_rets == ArgsOrRets::Args && has_baldrdash_tls {
            // Baldrdash ABI-2020 always has two stack-arg slots reserved, for the callee and
            // caller TLS-register values, respectively.
            next_stack = 16;
        }

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
                | &ir::ArgumentPurpose::CalleeTLS
                | &ir::ArgumentPurpose::CallerTLS
                | &ir::ArgumentPurpose::StructReturn
                | &ir::ArgumentPurpose::StructArgument(_) => {}
                _ => panic!(
                    "Unsupported argument purpose {:?} in signature: {:?}",
                    param.purpose, params
                ),
            }

            if let Some(param) = try_fill_baldrdash_reg(call_conv, param) {
                ret.push(param);
                continue;
            }

            if let ir::ArgumentPurpose::StructArgument(size) = param.purpose {
                let offset = next_stack as i64;
                let size = size as u64;
                assert!(size % 8 == 0, "StructArgument size is not properly aligned");
                next_stack += size;
                ret.push(ABIArg::StructArg {
                    offset,
                    size,
                    purpose: param.purpose,
                });
                continue;
            }

            // Find regclass(es) of the register(s) used to store a value of this type.
            let (rcs, _) = Inst::rc_for_type(param.value_type)?;
            let intreg = rcs[0] == RegClass::I64;
            let num_regs = rcs.len();
            assert!(num_regs <= 2);
            if num_regs == 2 {
                assert_eq!(rcs[0], rcs[1]);
            }

            let mut regs: SmallVec<[RealReg; 2]> = smallvec![];
            for j in 0..num_regs {
                let nextreg = if intreg {
                    match args_or_rets {
                        ArgsOrRets::Args => get_intreg_for_arg_systemv(&call_conv, next_gpr + j),
                        ArgsOrRets::Rets => {
                            get_intreg_for_retval_systemv(&call_conv, next_gpr + j, i + j)
                        }
                    }
                } else {
                    match args_or_rets {
                        ArgsOrRets::Args => get_fltreg_for_arg_systemv(&call_conv, next_vreg + j),
                        ArgsOrRets::Rets => {
                            get_fltreg_for_retval_systemv(&call_conv, next_vreg + j, i + j)
                        }
                    }
                };
                if let Some(reg) = nextreg {
                    regs.push(reg.to_real_reg());
                } else {
                    regs.clear();
                    break;
                }
            }

            if regs.len() > 0 {
                let regs = match num_regs {
                    1 => ValueRegs::one(regs[0]),
                    2 => ValueRegs::two(regs[0], regs[1]),
                    _ => panic!("More than two registers unexpected"),
                };
                ret.push(ABIArg::Reg {
                    regs,
                    ty: param.value_type,
                    extension: param.extension,
                    purpose: param.purpose,
                });
                if intreg {
                    next_gpr += num_regs;
                } else {
                    next_vreg += num_regs;
                }
            } else {
                // Compute size. Every arg takes a minimum slot of 8 bytes. (16-byte
                // stack alignment happens separately after all args.)
                let size = (param.value_type.bits() / 8) as u64;
                let size = std::cmp::max(size, 8);
                // Align.
                debug_assert!(size.is_power_of_two());
                next_stack = (next_stack + size - 1) & !(size - 1);
                ret.push(ABIArg::Stack {
                    offset: next_stack as i64,
                    ty: param.value_type,
                    extension: param.extension,
                    purpose: param.purpose,
                });
                next_stack += size;
            }
        }

        if args_or_rets == ArgsOrRets::Rets && is_baldrdash {
            ret.reverse();
        }

        let extra_arg = if add_ret_area_ptr {
            debug_assert!(args_or_rets == ArgsOrRets::Args);
            if let Some(reg) = get_intreg_for_arg_systemv(&call_conv, next_gpr) {
                ret.push(ABIArg::Reg {
                    regs: ValueRegs::one(reg.to_real_reg()),
                    ty: types::I64,
                    extension: ir::ArgumentExtension::None,
                    purpose: ir::ArgumentPurpose::Normal,
                });
            } else {
                ret.push(ABIArg::Stack {
                    offset: next_stack as i64,
                    ty: types::I64,
                    extension: ir::ArgumentExtension::None,
                    purpose: ir::ArgumentPurpose::Normal,
                });
                next_stack += 8;
            }
            Some(ret.len() - 1)
        } else {
            None
        };

        next_stack = (next_stack + 15) & !15;

        // To avoid overflow issues, limit the arg/return size to something reasonable.
        if next_stack > STACK_ARG_RET_SIZE_LIMIT {
            return Err(CodegenError::ImplLimitExceeded);
        }

        Ok((ret, next_stack as i64, extra_arg))
    }

    fn fp_to_arg_offset(call_conv: isa::CallConv, flags: &settings::Flags) -> i64 {
        if call_conv.extends_baldrdash() {
            let num_words = flags.baldrdash_prologue_words() as i64;
            debug_assert!(num_words > 0, "baldrdash must set baldrdash_prologue_words");
            num_words * 8
        } else {
            16 // frame pointer + return address.
        }
    }

    fn gen_load_stack(mem: StackAMode, into_reg: Writable<Reg>, ty: Type) -> Self::I {
        let ext_kind = match ty {
            types::B1
            | types::B8
            | types::I8
            | types::B16
            | types::I16
            | types::B32
            | types::I32 => ExtKind::SignExtend,
            types::B64 | types::I64 | types::R64 | types::F32 | types::F64 => ExtKind::None,
            _ if ty.bytes() == 16 => ExtKind::None,
            _ => panic!("load_stack({})", ty),
        };
        Inst::load(ty, mem, into_reg, ext_kind)
    }

    fn gen_store_stack(mem: StackAMode, from_reg: Reg, ty: Type) -> Self::I {
        Inst::store(ty, from_reg, mem)
    }

    fn gen_move(to_reg: Writable<Reg>, from_reg: Reg, ty: Type) -> Self::I {
        Inst::gen_move(to_reg, from_reg, ty)
    }

    /// Generate an integer-extend operation.
    fn gen_extend(
        to_reg: Writable<Reg>,
        from_reg: Reg,
        is_signed: bool,
        from_bits: u8,
        to_bits: u8,
    ) -> Self::I {
        let ext_mode = ExtMode::new(from_bits as u16, to_bits as u16)
            .expect(&format!("invalid extension: {} -> {}", from_bits, to_bits));
        if is_signed {
            Inst::movsx_rm_r(ext_mode, RegMem::reg(from_reg), to_reg)
        } else {
            Inst::movzx_rm_r(ext_mode, RegMem::reg(from_reg), to_reg)
        }
    }

    fn gen_ret() -> Self::I {
        Inst::ret()
    }

    fn gen_epilogue_placeholder() -> Self::I {
        Inst::epilogue_placeholder()
    }

    fn gen_add_imm(into_reg: Writable<Reg>, from_reg: Reg, imm: u32) -> SmallInstVec<Self::I> {
        let mut ret = SmallVec::new();
        if from_reg != into_reg.to_reg() {
            ret.push(Inst::gen_move(into_reg, from_reg, I64));
        }
        ret.push(Inst::alu_rmi_r(
            OperandSize::Size64,
            AluRmiROpcode::Add,
            RegMemImm::imm(imm),
            into_reg,
        ));
        ret
    }

    fn gen_stack_lower_bound_trap(limit_reg: Reg) -> SmallInstVec<Self::I> {
        smallvec![
            Inst::cmp_rmi_r(OperandSize::Size64, RegMemImm::reg(regs::rsp()), limit_reg),
            Inst::TrapIf {
                // NBE == "> unsigned"; args above are reversed; this tests limit_reg > rsp.
                cc: CC::NBE,
                trap_code: TrapCode::StackOverflow,
            },
        ]
    }

    fn gen_get_stack_addr(mem: StackAMode, into_reg: Writable<Reg>, _ty: Type) -> Self::I {
        let mem: SyntheticAmode = mem.into();
        Inst::lea(mem, into_reg)
    }

    fn get_stacklimit_reg() -> Reg {
        debug_assert!(
            !is_callee_save_systemv(regs::r10().to_real_reg())
                && !is_callee_save_baldrdash(regs::r10().to_real_reg())
        );

        // As per comment on trait definition, we must return a caller-save
        // register here.
        regs::r10()
    }

    fn gen_load_base_offset(into_reg: Writable<Reg>, base: Reg, offset: i32, ty: Type) -> Self::I {
        // Only ever used for I64s; if that changes, see if the ExtKind below needs to be changed.
        assert_eq!(ty, I64);
        let simm32 = offset as u32;
        let mem = Amode::imm_reg(simm32, base);
        Inst::load(ty, mem, into_reg, ExtKind::None)
    }

    fn gen_store_base_offset(base: Reg, offset: i32, from_reg: Reg, ty: Type) -> Self::I {
        let simm32 = offset as u32;
        let mem = Amode::imm_reg(simm32, base);
        Inst::store(ty, from_reg, mem)
    }

    fn gen_sp_reg_adjust(amount: i32) -> SmallInstVec<Self::I> {
        let (alu_op, amount) = if amount >= 0 {
            (AluRmiROpcode::Add, amount)
        } else {
            (AluRmiROpcode::Sub, -amount)
        };

        let amount = amount as u32;

        smallvec![Inst::alu_rmi_r(
            OperandSize::Size64,
            alu_op,
            RegMemImm::imm(amount),
            Writable::from_reg(regs::rsp()),
        )]
    }

    fn gen_nominal_sp_adj(offset: i32) -> Self::I {
        Inst::VirtualSPOffsetAdj {
            offset: offset as i64,
        }
    }

    fn gen_prologue_frame_setup() -> SmallInstVec<Self::I> {
        let r_rsp = regs::rsp();
        let r_rbp = regs::rbp();
        let w_rbp = Writable::from_reg(r_rbp);
        let mut insts = SmallVec::new();
        // RSP before the call will be 0 % 16.  So here, it is 8 % 16.
        insts.push(Inst::push64(RegMemImm::reg(r_rbp)));
        // RSP is now 0 % 16
        insts.push(Inst::mov_r_r(OperandSize::Size64, r_rsp, w_rbp));
        insts
    }

    fn gen_epilogue_frame_restore() -> SmallInstVec<Self::I> {
        let mut insts = SmallVec::new();
        insts.push(Inst::mov_r_r(
            OperandSize::Size64,
            regs::rbp(),
            Writable::from_reg(regs::rsp()),
        ));
        insts.push(Inst::pop64(Writable::from_reg(regs::rbp())));
        insts
    }

    fn gen_probestack(frame_size: u32) -> SmallInstVec<Self::I> {
        let mut insts = SmallVec::new();
        insts.push(Inst::imm(
            OperandSize::Size32,
            frame_size as u64,
            Writable::from_reg(regs::rax()),
        ));
        insts.push(Inst::CallKnown {
            dest: ExternalName::LibCall(LibCall::Probestack),
            uses: vec![regs::rax()],
            defs: vec![],
            opcode: Opcode::Call,
        });
        insts
    }

    fn gen_clobber_save(
        call_conv: isa::CallConv,
        _: &settings::Flags,
        clobbers: &Set<Writable<RealReg>>,
        fixed_frame_storage_size: u32,
        _outgoing_args_size: u32,
    ) -> (u64, SmallVec<[Self::I; 16]>) {
        let mut insts = SmallVec::new();
        // Find all clobbered registers that are callee-save. These are only I64
        // registers (all XMM registers are caller-save) so we can compute the
        // total size of the needed stack space easily.
        let clobbered = get_callee_saves(&call_conv, clobbers);
        let clobbered_size = 8 * clobbered.len() as u32;
        let stack_size = clobbered_size + fixed_frame_storage_size;
        // Align to 16 bytes.
        let stack_size = (stack_size + 15) & !15;
        let clobbered_size = stack_size - fixed_frame_storage_size;
        // Adjust the stack pointer downward with one `sub rsp, IMM`
        // instruction.
        if stack_size > 0 {
            insts.push(Inst::alu_rmi_r(
                OperandSize::Size64,
                AluRmiROpcode::Sub,
                RegMemImm::imm(stack_size),
                Writable::from_reg(regs::rsp()),
            ));
        }
        // Store each clobbered register in order at offsets from RSP.
        let mut cur_offset = 0;
        for reg in &clobbered {
            let r_reg = reg.to_reg();
            match r_reg.get_class() {
                RegClass::I64 => {
                    insts.push(Inst::mov_r_m(
                        OperandSize::Size64,
                        r_reg.to_reg(),
                        Amode::imm_reg(cur_offset, regs::rsp()),
                    ));
                    cur_offset += 8;
                }
                // No XMM regs are callee-save, so we do not need to implement
                // this.
                _ => unimplemented!(),
            }
        }

        (clobbered_size as u64, insts)
    }

    fn gen_clobber_restore(
        call_conv: isa::CallConv,
        flags: &settings::Flags,
        clobbers: &Set<Writable<RealReg>>,
        _fixed_frame_storage_size: u32,
        _outgoing_args_size: u32,
    ) -> SmallVec<[Self::I; 16]> {
        let mut insts = SmallVec::new();

        let clobbered = get_callee_saves(&call_conv, clobbers);
        let stack_size = 8 * clobbered.len() as u32;
        let stack_size = (stack_size + 15) & !15;

        // Restore regs by loading from offsets of RSP.
        let mut cur_offset = 0;
        for reg in &clobbered {
            let rreg = reg.to_reg();
            match rreg.get_class() {
                RegClass::I64 => {
                    insts.push(Inst::mov64_m_r(
                        Amode::imm_reg(cur_offset, regs::rsp()),
                        Writable::from_reg(rreg.to_reg()),
                    ));
                    cur_offset += 8;
                }
                _ => unimplemented!(),
            }
        }
        // Adjust RSP back upward.
        if stack_size > 0 {
            insts.push(Inst::alu_rmi_r(
                OperandSize::Size64,
                AluRmiROpcode::Add,
                RegMemImm::imm(stack_size),
                Writable::from_reg(regs::rsp()),
            ));
        }

        // If this is Baldrdash-2020, restore the callee (i.e., our) TLS
        // register. We may have allocated it for something else and clobbered
        // it, but the ABI expects us to leave the TLS register unchanged.
        if call_conv == isa::CallConv::Baldrdash2020 {
            let off = BALDRDASH_CALLEE_TLS_OFFSET + Self::fp_to_arg_offset(call_conv, flags);
            insts.push(Inst::mov64_m_r(
                Amode::imm_reg(off as u32, regs::rbp()),
                Writable::from_reg(regs::r14()),
            ));
        }

        insts
    }

    /// Generate a call instruction/sequence.
    fn gen_call(
        dest: &CallDest,
        uses: Vec<Reg>,
        defs: Vec<Writable<Reg>>,
        opcode: ir::Opcode,
        tmp: Writable<Reg>,
        _callee_conv: isa::CallConv,
        _caller_conv: isa::CallConv,
    ) -> SmallVec<[(InstIsSafepoint, Self::I); 2]> {
        let mut insts = SmallVec::new();
        match dest {
            &CallDest::ExtName(ref name, RelocDistance::Near) => {
                insts.push((
                    InstIsSafepoint::Yes,
                    Inst::call_known(name.clone(), uses, defs, opcode),
                ));
            }
            &CallDest::ExtName(ref name, RelocDistance::Far) => {
                insts.push((
                    InstIsSafepoint::No,
                    Inst::LoadExtName {
                        dst: tmp,
                        name: Box::new(name.clone()),
                        offset: 0,
                    },
                ));
                insts.push((
                    InstIsSafepoint::Yes,
                    Inst::call_unknown(RegMem::reg(tmp.to_reg()), uses, defs, opcode),
                ));
            }
            &CallDest::Reg(reg) => {
                insts.push((
                    InstIsSafepoint::Yes,
                    Inst::call_unknown(RegMem::reg(reg), uses, defs, opcode),
                ));
            }
        }
        insts
    }

    fn gen_memcpy(
        call_conv: isa::CallConv,
        dst: Reg,
        src: Reg,
        size: usize,
    ) -> SmallVec<[Self::I; 8]> {
        // Baldrdash should not use struct args.
        assert!(!call_conv.extends_baldrdash());
        let mut insts = SmallVec::new();
        let arg0 = get_intreg_for_arg_systemv(&call_conv, 0).unwrap();
        let arg1 = get_intreg_for_arg_systemv(&call_conv, 1).unwrap();
        let arg2 = get_intreg_for_arg_systemv(&call_conv, 2).unwrap();
        // We need a register to load the address of `memcpy()` below and we
        // don't have a lowering context to allocate a temp here; so just use a
        // register we know we are free to mutate as part of this sequence
        // (because it is clobbered by the call as per the ABI anyway).
        let memcpy_addr = get_intreg_for_arg_systemv(&call_conv, 3).unwrap();
        insts.push(Inst::gen_move(Writable::from_reg(arg0), dst, I64));
        insts.push(Inst::gen_move(Writable::from_reg(arg1), src, I64));
        insts.extend(
            Inst::gen_constant(
                ValueRegs::one(Writable::from_reg(arg2)),
                size as u128,
                I64,
                |_| panic!("tmp should not be needed"),
            )
            .into_iter(),
        );
        // We use an indirect call and a full LoadExtName because we do not have
        // information about the libcall `RelocDistance` here, so we
        // conservatively use the more flexible calling sequence.
        insts.push(Inst::LoadExtName {
            dst: Writable::from_reg(memcpy_addr),
            name: Box::new(ExternalName::LibCall(LibCall::Memcpy)),
            offset: 0,
        });
        insts.push(Inst::call_unknown(
            RegMem::reg(memcpy_addr),
            /* uses = */ vec![arg0, arg1, arg2],
            /* defs = */ Self::get_regs_clobbered_by_call(call_conv),
            Opcode::Call,
        ));
        insts
    }

    fn get_number_of_spillslots_for_value(rc: RegClass, ty: Type) -> u32 {
        // We allocate in terms of 8-byte slots.
        match (rc, ty) {
            (RegClass::I64, _) => 1,
            (RegClass::V128, types::F32) | (RegClass::V128, types::F64) => 1,
            (RegClass::V128, _) => 2,
            _ => panic!("Unexpected register class!"),
        }
    }

    fn get_virtual_sp_offset_from_state(s: &<Self::I as MachInstEmit>::State) -> i64 {
        s.virtual_sp_offset
    }

    fn get_nominal_sp_to_fp(s: &<Self::I as MachInstEmit>::State) -> i64 {
        s.nominal_sp_to_fp
    }

    fn get_regs_clobbered_by_call(call_conv_of_callee: isa::CallConv) -> Vec<Writable<Reg>> {
        let mut caller_saved = vec![
            // Systemv calling convention:
            // - GPR: all except RBX, RBP, R12 to R15 (which are callee-saved).
            Writable::from_reg(regs::rsi()),
            Writable::from_reg(regs::rdi()),
            Writable::from_reg(regs::rax()),
            Writable::from_reg(regs::rcx()),
            Writable::from_reg(regs::rdx()),
            Writable::from_reg(regs::r8()),
            Writable::from_reg(regs::r9()),
            Writable::from_reg(regs::r10()),
            Writable::from_reg(regs::r11()),
            // - XMM: all the registers!
            Writable::from_reg(regs::xmm0()),
            Writable::from_reg(regs::xmm1()),
            Writable::from_reg(regs::xmm2()),
            Writable::from_reg(regs::xmm3()),
            Writable::from_reg(regs::xmm4()),
            Writable::from_reg(regs::xmm5()),
            Writable::from_reg(regs::xmm6()),
            Writable::from_reg(regs::xmm7()),
            Writable::from_reg(regs::xmm8()),
            Writable::from_reg(regs::xmm9()),
            Writable::from_reg(regs::xmm10()),
            Writable::from_reg(regs::xmm11()),
            Writable::from_reg(regs::xmm12()),
            Writable::from_reg(regs::xmm13()),
            Writable::from_reg(regs::xmm14()),
            Writable::from_reg(regs::xmm15()),
        ];

        if call_conv_of_callee.extends_baldrdash() {
            caller_saved.push(Writable::from_reg(regs::r12()));
            caller_saved.push(Writable::from_reg(regs::r13()));
            // Not r14; implicitly preserved in the entry.
            caller_saved.push(Writable::from_reg(regs::r15()));
            caller_saved.push(Writable::from_reg(regs::rbx()));
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
            // No other supported ABI on x64 does so.
            ir::ArgumentExtension::None
        }
    }
}

impl From<StackAMode> for SyntheticAmode {
    fn from(amode: StackAMode) -> Self {
        // We enforce a 128 MB stack-frame size limit above, so these
        // `expect()`s should never fail.
        match amode {
            StackAMode::FPOffset(off, _ty) => {
                let off = i32::try_from(off)
                    .expect("Offset in FPOffset is greater than 2GB; should hit impl limit first");
                let simm32 = off as u32;
                SyntheticAmode::Real(Amode::ImmReg {
                    simm32,
                    base: regs::rbp(),
                    flags: MemFlags::trusted(),
                })
            }
            StackAMode::NominalSPOffset(off, _ty) => {
                let off = i32::try_from(off).expect(
                    "Offset in NominalSPOffset is greater than 2GB; should hit impl limit first",
                );
                let simm32 = off as u32;
                SyntheticAmode::nominal_sp_offset(simm32)
            }
            StackAMode::SPOffset(off, _ty) => {
                let off = i32::try_from(off)
                    .expect("Offset in SPOffset is greater than 2GB; should hit impl limit first");
                let simm32 = off as u32;
                SyntheticAmode::Real(Amode::ImmReg {
                    simm32,
                    base: regs::rsp(),
                    flags: MemFlags::trusted(),
                })
            }
        }
    }
}

fn get_intreg_for_arg_systemv(call_conv: &CallConv, idx: usize) -> Option<Reg> {
    match call_conv {
        CallConv::Fast
        | CallConv::Cold
        | CallConv::SystemV
        | CallConv::BaldrdashSystemV
        | CallConv::Baldrdash2020 => {}
        _ => panic!("int args only supported for SysV calling convention"),
    };
    match idx {
        0 => Some(regs::rdi()),
        1 => Some(regs::rsi()),
        2 => Some(regs::rdx()),
        3 => Some(regs::rcx()),
        4 => Some(regs::r8()),
        5 => Some(regs::r9()),
        _ => None,
    }
}

fn get_fltreg_for_arg_systemv(call_conv: &CallConv, idx: usize) -> Option<Reg> {
    match call_conv {
        CallConv::Fast
        | CallConv::Cold
        | CallConv::SystemV
        | CallConv::BaldrdashSystemV
        | CallConv::Baldrdash2020 => {}
        _ => panic!("float args only supported for SysV calling convention"),
    };
    match idx {
        0 => Some(regs::xmm0()),
        1 => Some(regs::xmm1()),
        2 => Some(regs::xmm2()),
        3 => Some(regs::xmm3()),
        4 => Some(regs::xmm4()),
        5 => Some(regs::xmm5()),
        6 => Some(regs::xmm6()),
        7 => Some(regs::xmm7()),
        _ => None,
    }
}

fn get_intreg_for_retval_systemv(
    call_conv: &CallConv,
    intreg_idx: usize,
    retval_idx: usize,
) -> Option<Reg> {
    match call_conv {
        CallConv::Fast | CallConv::Cold | CallConv::SystemV => match intreg_idx {
            0 => Some(regs::rax()),
            1 => Some(regs::rdx()),
            _ => None,
        },
        CallConv::BaldrdashSystemV | CallConv::Baldrdash2020 => {
            if intreg_idx == 0 && retval_idx == 0 {
                Some(regs::rax())
            } else {
                None
            }
        }
        CallConv::WindowsFastcall | CallConv::BaldrdashWindows | CallConv::Probestack => todo!(),
    }
}

fn get_fltreg_for_retval_systemv(
    call_conv: &CallConv,
    fltreg_idx: usize,
    retval_idx: usize,
) -> Option<Reg> {
    match call_conv {
        CallConv::Fast | CallConv::Cold | CallConv::SystemV => match fltreg_idx {
            0 => Some(regs::xmm0()),
            1 => Some(regs::xmm1()),
            _ => None,
        },
        CallConv::BaldrdashSystemV | CallConv::Baldrdash2020 => {
            if fltreg_idx == 0 && retval_idx == 0 {
                Some(regs::xmm0())
            } else {
                None
            }
        }
        CallConv::WindowsFastcall | CallConv::BaldrdashWindows | CallConv::Probestack => todo!(),
    }
}

fn is_callee_save_systemv(r: RealReg) -> bool {
    use regs::*;
    match r.get_class() {
        RegClass::I64 => match r.get_hw_encoding() as u8 {
            ENC_RBX | ENC_RBP | ENC_R12 | ENC_R13 | ENC_R14 | ENC_R15 => true,
            _ => false,
        },
        RegClass::V128 => false,
        _ => unimplemented!(),
    }
}

fn is_callee_save_baldrdash(r: RealReg) -> bool {
    use regs::*;
    match r.get_class() {
        RegClass::I64 => {
            if r.get_hw_encoding() as u8 == ENC_R14 {
                // r14 is the WasmTlsReg and is preserved implicitly.
                false
            } else {
                // Defer to native for the other ones.
                is_callee_save_systemv(r)
            }
        }
        RegClass::V128 => false,
        _ => unimplemented!(),
    }
}

fn get_callee_saves(call_conv: &CallConv, regs: &Set<Writable<RealReg>>) -> Vec<Writable<RealReg>> {
    let mut regs: Vec<Writable<RealReg>> = match call_conv {
        CallConv::BaldrdashSystemV | CallConv::Baldrdash2020 => regs
            .iter()
            .cloned()
            .filter(|r| is_callee_save_baldrdash(r.to_reg()))
            .collect(),
        CallConv::BaldrdashWindows => {
            todo!("baldrdash windows");
        }
        CallConv::Fast | CallConv::Cold | CallConv::SystemV => regs
            .iter()
            .cloned()
            .filter(|r| is_callee_save_systemv(r.to_reg()))
            .collect(),
        CallConv::WindowsFastcall => todo!("windows fastcall"),
        CallConv::Probestack => todo!("probestack?"),
    };
    // Sort registers for deterministic code output. We can do an unstable sort because the
    // registers will be unique (there are no dups).
    regs.sort_unstable_by_key(|r| r.to_reg().get_index());
    regs
}
