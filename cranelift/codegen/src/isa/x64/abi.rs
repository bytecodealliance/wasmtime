//! Implementation of the standard x64 ABI.

use crate::ir::types::*;
use crate::ir::{self, types, SourceLoc, TrapCode, Type};
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
                Some(ABIArg::Reg(
                    regs::r14().to_real_reg(),
                    types::I64,
                    param.extension,
                    param.purpose,
                ))
            }
            &ir::ArgumentPurpose::SignatureId => {
                // This is SpiderMonkey's `WasmTableCallSigReg`.
                Some(ABIArg::Reg(
                    regs::r10().to_real_reg(),
                    types::I64,
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
                | &ir::ArgumentPurpose::CallerTLS => {}
                _ => panic!(
                    "Unsupported argument purpose {:?} in signature: {:?}",
                    param.purpose, params
                ),
            }

            let intreg = in_int_reg(param.value_type);
            let vecreg = in_vec_reg(param.value_type);
            debug_assert!(intreg || vecreg);
            debug_assert!(!(intreg && vecreg));

            let (next_reg, candidate) = if intreg {
                let candidate = match args_or_rets {
                    ArgsOrRets::Args => get_intreg_for_arg_systemv(&call_conv, next_gpr),
                    ArgsOrRets::Rets => get_intreg_for_retval_systemv(&call_conv, next_gpr, i),
                };
                debug_assert!(candidate
                    .map(|r| r.get_class() == RegClass::I64)
                    .unwrap_or(true));
                (&mut next_gpr, candidate)
            } else {
                let candidate = match args_or_rets {
                    ArgsOrRets::Args => get_fltreg_for_arg_systemv(&call_conv, next_vreg),
                    ArgsOrRets::Rets => get_fltreg_for_retval_systemv(&call_conv, next_vreg, i),
                };
                debug_assert!(candidate
                    .map(|r| r.get_class() == RegClass::V128)
                    .unwrap_or(true));
                (&mut next_vreg, candidate)
            };

            if let Some(param) = try_fill_baldrdash_reg(call_conv, param) {
                assert!(intreg);
                ret.push(param);
            } else if let Some(reg) = candidate {
                ret.push(ABIArg::Reg(
                    reg.to_real_reg(),
                    param.value_type,
                    param.extension,
                    param.purpose,
                ));
                *next_reg += 1;
            } else {
                // Compute size. Every arg takes a minimum slot of 8 bytes. (16-byte
                // stack alignment happens separately after all args.)
                let size = (param.value_type.bits() / 8) as u64;
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
            if let Some(reg) = get_intreg_for_arg_systemv(&call_conv, next_gpr) {
                ret.push(ABIArg::Reg(
                    reg.to_real_reg(),
                    types::I64,
                    ir::ArgumentExtension::None,
                    ir::ArgumentPurpose::Normal,
                ));
            } else {
                ret.push(ABIArg::Stack(
                    next_stack as i64,
                    types::I64,
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
        Inst::load(ty, mem, into_reg, ext_kind, /* infallible */ None)
    }

    fn gen_store_stack(mem: StackAMode, from_reg: Reg, ty: Type) -> Self::I {
        Inst::store(ty, from_reg, mem, /* infallible */ None)
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
        let ext_mode = match from_bits {
            1 | 8 => ExtMode::BQ,
            16 => ExtMode::WQ,
            32 => ExtMode::LQ,
            _ => panic!("Bad extension: {} bits to {} bits", from_bits, to_bits),
        };
        if is_signed {
            Inst::movsx_rm_r(ext_mode, RegMem::reg(from_reg), to_reg, None)
        } else {
            Inst::movzx_rm_r(ext_mode, RegMem::reg(from_reg), to_reg, None)
        }
    }

    fn gen_ret() -> Self::I {
        Inst::Ret
    }

    fn gen_epilogue_placeholder() -> Self::I {
        Inst::EpiloguePlaceholder
    }

    fn gen_add_imm(into_reg: Writable<Reg>, from_reg: Reg, imm: u32) -> SmallVec<[Self::I; 4]> {
        let mut ret = SmallVec::new();
        if from_reg != into_reg.to_reg() {
            ret.push(Inst::gen_move(into_reg, from_reg, I64));
        }
        ret.push(Inst::alu_rmi_r(
            true,
            AluRmiROpcode::Add,
            RegMemImm::imm(imm),
            into_reg,
        ));
        ret
    }

    fn gen_stack_lower_bound_trap(limit_reg: Reg) -> SmallVec<[Self::I; 2]> {
        smallvec![
            Inst::cmp_rmi_r(/* bytes = */ 8, RegMemImm::reg(regs::rsp()), limit_reg),
            Inst::TrapIf {
                // NBE == "> unsigned"; args above are reversed; this tests limit_reg > rsp.
                cc: CC::NBE,
                srcloc: SourceLoc::default(),
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
        Inst::load(ty, mem, into_reg, ExtKind::None, None)
    }

    fn gen_store_base_offset(base: Reg, offset: i32, from_reg: Reg, ty: Type) -> Self::I {
        let simm32 = offset as u32;
        let mem = Amode::imm_reg(simm32, base);
        Inst::store(ty, from_reg, mem, None)
    }

    fn gen_sp_reg_adjust(amount: i32) -> SmallVec<[Self::I; 2]> {
        let (alu_op, amount) = if amount >= 0 {
            (AluRmiROpcode::Add, amount)
        } else {
            (AluRmiROpcode::Sub, -amount)
        };

        let amount = amount as u32;

        smallvec![Inst::alu_rmi_r(
            true,
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

    fn gen_prologue_frame_setup() -> SmallVec<[Self::I; 2]> {
        let r_rsp = regs::rsp();
        let r_rbp = regs::rbp();
        let w_rbp = Writable::from_reg(r_rbp);
        let mut insts = SmallVec::new();
        // RSP before the call will be 0 % 16.  So here, it is 8 % 16.
        insts.push(Inst::push64(RegMemImm::reg(r_rbp)));
        // RSP is now 0 % 16
        insts.push(Inst::mov_r_r(true, r_rsp, w_rbp));
        insts
    }

    fn gen_epilogue_frame_restore() -> SmallVec<[Self::I; 2]> {
        let mut insts = SmallVec::new();
        insts.push(Inst::mov_r_r(
            true,
            regs::rbp(),
            Writable::from_reg(regs::rsp()),
        ));
        insts.push(Inst::pop64(Writable::from_reg(regs::rbp())));
        insts
    }

    fn gen_clobber_save(
        call_conv: isa::CallConv,
        _: &settings::Flags,
        clobbers: &Set<Writable<RealReg>>,
    ) -> (u64, SmallVec<[Self::I; 16]>) {
        let mut insts = SmallVec::new();
        // Find all clobbered registers that are callee-save. These are only I64
        // registers (all XMM registers are caller-save) so we can compute the
        // total size of the needed stack space easily.
        let clobbered = get_callee_saves(&call_conv, clobbers);
        let stack_size = 8 * clobbered.len() as u32;
        // Align to 16 bytes.
        let stack_size = (stack_size + 15) & !15;
        // Adjust the stack pointer downward with one `sub rsp, IMM`
        // instruction.
        if stack_size > 0 {
            insts.push(Inst::alu_rmi_r(
                true,
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
                        /* bytes = */ 8,
                        r_reg.to_reg(),
                        Amode::imm_reg(cur_offset, regs::rsp()),
                        None,
                    ));
                    cur_offset += 8;
                }
                // No XMM regs are callee-save, so we do not need to implement
                // this.
                _ => unimplemented!(),
            }
        }

        (stack_size as u64, insts)
    }

    fn gen_clobber_restore(
        call_conv: isa::CallConv,
        flags: &settings::Flags,
        clobbers: &Set<Writable<RealReg>>,
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
                        None,
                    ));
                    cur_offset += 8;
                }
                _ => unimplemented!(),
            }
        }
        // Adjust RSP back upward.
        if stack_size > 0 {
            insts.push(Inst::alu_rmi_r(
                true,
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
                None,
            ));
        }

        insts
    }

    /// Generate a call instruction/sequence.
    fn gen_call(
        dest: &CallDest,
        uses: Vec<Reg>,
        defs: Vec<Writable<Reg>>,
        loc: SourceLoc,
        opcode: ir::Opcode,
        tmp: Writable<Reg>,
    ) -> SmallVec<[(InstIsSafepoint, Self::I); 2]> {
        let mut insts = SmallVec::new();
        match dest {
            &CallDest::ExtName(ref name, RelocDistance::Near) => {
                insts.push((
                    InstIsSafepoint::Yes,
                    Inst::call_known(name.clone(), uses, defs, loc, opcode),
                ));
            }
            &CallDest::ExtName(ref name, RelocDistance::Far) => {
                insts.push((
                    InstIsSafepoint::No,
                    Inst::LoadExtName {
                        dst: tmp,
                        name: Box::new(name.clone()),
                        offset: 0,
                        srcloc: loc,
                    },
                ));
                insts.push((
                    InstIsSafepoint::Yes,
                    Inst::call_unknown(RegMem::reg(tmp.to_reg()), uses, defs, loc, opcode),
                ));
            }
            &CallDest::Reg(reg) => {
                insts.push((
                    InstIsSafepoint::Yes,
                    Inst::call_unknown(RegMem::reg(reg), uses, defs, loc, opcode),
                ));
            }
        }
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

    fn get_caller_saves(call_conv: isa::CallConv) -> Vec<Writable<Reg>> {
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

        if call_conv.extends_baldrdash() {
            caller_saved.push(Writable::from_reg(regs::r12()));
            caller_saved.push(Writable::from_reg(regs::r13()));
            // Not r14; implicitly preserved in the entry.
            caller_saved.push(Writable::from_reg(regs::r15()));
            caller_saved.push(Writable::from_reg(regs::rbx()));
        }

        caller_saved
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
                })
            }
        }
    }
}

fn in_int_reg(ty: types::Type) -> bool {
    match ty {
        types::I8
        | types::I16
        | types::I32
        | types::I64
        | types::B1
        | types::B8
        | types::B16
        | types::B32
        | types::B64
        | types::R64 => true,
        types::R32 => panic!("unexpected 32-bits refs on x64!"),
        _ => false,
    }
}

fn in_vec_reg(ty: types::Type) -> bool {
    match ty {
        types::F32 | types::F64 => true,
        _ if ty.is_vector() => true,
        _ => false,
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
