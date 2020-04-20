//! x86 ABI implementation.

use super::super::settings as shared_settings;
use super::registers::{FPR, GPR, RU};
use super::settings as isa_settings;
use crate::abi::{legalize_args, ArgAction, ArgAssigner, ValueConversion};
use crate::cursor::{Cursor, CursorPosition, EncCursor};
use crate::ir;
use crate::ir::immediates::Imm64;
use crate::ir::stackslot::{StackOffset, StackSize};
use crate::ir::types;
use crate::ir::{
    get_probestack_funcref, AbiParam, ArgumentExtension, ArgumentLoc, ArgumentPurpose, InstBuilder,
    ValueLoc,
};
use crate::isa::{CallConv, RegClass, RegUnit, TargetIsa};
use crate::regalloc::RegisterSet;
use crate::result::CodegenResult;
use crate::stack_layout::layout_stack;
use alloc::borrow::Cow;
use core::i32;
use target_lexicon::{PointerWidth, Triple};

/// Argument registers for x86-64
static ARG_GPRS: [RU; 6] = [RU::rdi, RU::rsi, RU::rdx, RU::rcx, RU::r8, RU::r9];

/// Return value registers.
static RET_GPRS: [RU; 3] = [RU::rax, RU::rdx, RU::rcx];

/// Argument registers for x86-64, when using windows fastcall
static ARG_GPRS_WIN_FASTCALL_X64: [RU; 4] = [RU::rcx, RU::rdx, RU::r8, RU::r9];

/// Return value registers for x86-64, when using windows fastcall
static RET_GPRS_WIN_FASTCALL_X64: [RU; 1] = [RU::rax];

/// The win64 fastcall ABI uses some shadow stack space, allocated by the caller, that can be used
/// by the callee for temporary values.
///
/// [1] "Space is allocated on the call stack as a shadow store for callees to save" This shadow
/// store contains the parameters which are passed through registers (ARG_GPRS) and is eventually
/// used by the callee to save & restore the values of the arguments.
///
/// [2] https://blogs.msdn.microsoft.com/oldnewthing/20110302-00/?p=11333 "Although the x64 calling
/// convention reserves spill space for parameters, you don’t have to use them as such"
const WIN_SHADOW_STACK_SPACE: StackSize = 32;

/// Stack alignment requirement for functions.
///
/// 16 bytes is the perfect stack alignment, because:
///
/// - On Win64, "The primary exceptions are the stack pointer and malloc or alloca memory, which
/// are aligned to 16 bytes in order to aid performance".
/// - The original 32-bit x86 ELF ABI had a 4-byte aligned stack pointer, but newer versions use a
/// 16-byte aligned stack pointer.
/// - This allows using aligned loads and stores on SIMD vectors of 16 bytes that are located
/// higher up in the stack.
const STACK_ALIGNMENT: u32 = 16;

#[derive(Clone)]
struct Args {
    pointer_bytes: u8,
    pointer_bits: u8,
    pointer_type: ir::Type,
    gpr: &'static [RU],
    gpr_used: usize,
    fpr_limit: usize,
    fpr_used: usize,
    offset: u32,
    call_conv: CallConv,
    shared_flags: shared_settings::Flags,
    #[allow(dead_code)]
    isa_flags: isa_settings::Flags,
    assigning_returns: bool,
}

impl Args {
    fn new(
        bits: u8,
        gpr: &'static [RU],
        fpr_limit: usize,
        call_conv: CallConv,
        shared_flags: &shared_settings::Flags,
        isa_flags: &isa_settings::Flags,
        assigning_returns: bool,
    ) -> Self {
        let offset = if call_conv.extends_windows_fastcall() {
            WIN_SHADOW_STACK_SPACE
        } else {
            0
        };

        Self {
            pointer_bytes: bits / 8,
            pointer_bits: bits,
            pointer_type: ir::Type::int(u16::from(bits)).unwrap(),
            gpr,
            gpr_used: 0,
            fpr_limit,
            fpr_used: 0,
            offset,
            call_conv,
            shared_flags: shared_flags.clone(),
            isa_flags: isa_flags.clone(),
            assigning_returns,
        }
    }
}

impl ArgAssigner for Args {
    fn assign(&mut self, arg: &AbiParam) -> ArgAction {
        if let ArgumentPurpose::StructArgument(size) = arg.purpose {
            if self.call_conv != CallConv::SystemV {
                panic!(
                    "The sarg argument purpose is not yet implemented for non-systemv call conv {:?}",
                    self.call_conv,
                );
            }
            let loc = ArgumentLoc::Stack(self.offset as i32);
            self.offset += size;
            debug_assert!(self.offset <= i32::MAX as u32);
            return ArgAction::AssignAndChangeType(loc, types::SARG__);
        }

        let ty = arg.value_type;

        if ty.bits() > u16::from(self.pointer_bits) {
            if !self.assigning_returns && self.call_conv.extends_windows_fastcall() {
                // "Any argument that doesn't fit in 8 bytes, or isn't
                // 1, 2, 4, or 8 bytes, must be passed by reference"
                return ValueConversion::Pointer(self.pointer_type).into();
            } else if !ty.is_vector() && !ty.is_float() {
                // On SystemV large integers and booleans are broken down to fit in a register.
                return ValueConversion::IntSplit.into();
            }
        }

        // Vectors should stay in vector registers unless SIMD is not enabled--then they are split
        if ty.is_vector() {
            if self.shared_flags.enable_simd() {
                let reg = FPR.unit(self.fpr_used);
                self.fpr_used += 1;
                return ArgumentLoc::Reg(reg).into();
            }
            return ValueConversion::VectorSplit.into();
        }

        // Small integers are extended to the size of a pointer register.
        if ty.is_int() && ty.bits() < u16::from(self.pointer_bits) {
            match arg.extension {
                ArgumentExtension::None => {}
                ArgumentExtension::Uext => return ValueConversion::Uext(self.pointer_type).into(),
                ArgumentExtension::Sext => return ValueConversion::Sext(self.pointer_type).into(),
            }
        }

        // Handle special-purpose arguments.
        if ty.is_int() && self.call_conv.extends_baldrdash() {
            match arg.purpose {
                // This is SpiderMonkey's `WasmTlsReg`.
                ArgumentPurpose::VMContext => {
                    return ArgumentLoc::Reg(if self.pointer_bits == 64 {
                        RU::r14
                    } else {
                        RU::rsi
                    } as RegUnit)
                    .into();
                }
                // This is SpiderMonkey's `WasmTableCallSigReg`.
                ArgumentPurpose::SignatureId => {
                    return ArgumentLoc::Reg(if self.pointer_bits == 64 {
                        RU::r10
                    } else {
                        RU::rcx
                    } as RegUnit)
                    .into()
                }
                _ => {}
            }
        }

        // Try to use a GPR.
        if !ty.is_float() && self.gpr_used < self.gpr.len() {
            let reg = self.gpr[self.gpr_used] as RegUnit;
            self.gpr_used += 1;
            return ArgumentLoc::Reg(reg).into();
        }

        // Try to use an FPR.
        let fpr_offset = if self.call_conv.extends_windows_fastcall() {
            // Float and general registers on windows share the same parameter index.
            // The used register depends entirely on the parameter index: Even if XMM0
            // is not used for the first parameter, it cannot be used for the second parameter.
            debug_assert_eq!(self.fpr_limit, self.gpr.len());
            &mut self.gpr_used
        } else {
            &mut self.fpr_used
        };

        if ty.is_float() && *fpr_offset < self.fpr_limit {
            let reg = FPR.unit(*fpr_offset);
            *fpr_offset += 1;
            return ArgumentLoc::Reg(reg).into();
        }

        // Assign a stack location.
        let loc = ArgumentLoc::Stack(self.offset as i32);
        self.offset += u32::from(self.pointer_bytes);
        debug_assert!(self.offset <= i32::MAX as u32);
        loc.into()
    }
}

/// Legalize `sig`.
pub fn legalize_signature(
    sig: &mut Cow<ir::Signature>,
    triple: &Triple,
    _current: bool,
    shared_flags: &shared_settings::Flags,
    isa_flags: &isa_settings::Flags,
) {
    let bits;
    let mut args;

    match triple.pointer_width().unwrap() {
        PointerWidth::U16 => panic!(),
        PointerWidth::U32 => {
            bits = 32;
            args = Args::new(bits, &[], 0, sig.call_conv, shared_flags, isa_flags, false);
        }
        PointerWidth::U64 => {
            bits = 64;
            args = if sig.call_conv.extends_windows_fastcall() {
                Args::new(
                    bits,
                    &ARG_GPRS_WIN_FASTCALL_X64[..],
                    4,
                    sig.call_conv,
                    shared_flags,
                    isa_flags,
                    false,
                )
            } else {
                Args::new(
                    bits,
                    &ARG_GPRS[..],
                    8,
                    sig.call_conv,
                    shared_flags,
                    isa_flags,
                    false,
                )
            };
        }
    }

    let (ret_regs, ret_fpr_limit) = if sig.call_conv.extends_windows_fastcall() {
        // windows-x64 calling convention only uses XMM0 or RAX for return values
        (&RET_GPRS_WIN_FASTCALL_X64[..], 1)
    } else {
        (&RET_GPRS[..], 2)
    };

    let mut rets = Args::new(
        bits,
        ret_regs,
        ret_fpr_limit,
        sig.call_conv,
        shared_flags,
        isa_flags,
        true,
    );

    // If we don't have enough available return registers
    // to fit all of the return values, we need to backtrack and start
    // assigning locations all over again with a different strategy. In order to
    // do that, we need a copy of the original assigner for the returns.
    let mut backup_rets = rets.clone();

    if let Some(new_returns) = legalize_args(&sig.returns, &mut rets) {
        if new_returns
            .iter()
            .filter(|r| r.purpose == ArgumentPurpose::Normal)
            .any(|r| !r.location.is_reg())
        {
            // The return values couldn't all fit into available return
            // registers. Introduce the use of a struct-return parameter.
            debug_assert!(!sig.uses_struct_return_param());

            // We're using the first register for the return pointer parameter.
            let mut ret_ptr_param = AbiParam {
                value_type: args.pointer_type,
                purpose: ArgumentPurpose::StructReturn,
                extension: ArgumentExtension::None,
                location: ArgumentLoc::Unassigned,
                legalized_to_pointer: false,
            };
            match args.assign(&ret_ptr_param) {
                ArgAction::Assign(ArgumentLoc::Reg(reg)) => {
                    ret_ptr_param.location = ArgumentLoc::Reg(reg);
                    sig.to_mut().params.push(ret_ptr_param);
                }
                _ => unreachable!("return pointer should always get a register assignment"),
            }

            // We're using the first return register for the return pointer (like
            // sys v does).
            let mut ret_ptr_return = AbiParam {
                value_type: args.pointer_type,
                purpose: ArgumentPurpose::StructReturn,
                extension: ArgumentExtension::None,
                location: ArgumentLoc::Unassigned,
                legalized_to_pointer: false,
            };
            match backup_rets.assign(&ret_ptr_return) {
                ArgAction::Assign(ArgumentLoc::Reg(reg)) => {
                    ret_ptr_return.location = ArgumentLoc::Reg(reg);
                    sig.to_mut().returns.push(ret_ptr_return);
                }
                _ => unreachable!("return pointer should always get a register assignment"),
            }

            sig.to_mut().returns.retain(|ret| {
                // Either this is the return pointer, in which case we want to keep
                // it, or else assume that it is assigned for a reason and doesn't
                // conflict with our return pointering legalization.
                debug_assert_eq!(
                    ret.location.is_assigned(),
                    ret.purpose != ArgumentPurpose::Normal
                );
                ret.location.is_assigned()
            });

            if let Some(new_returns) = legalize_args(&sig.returns, &mut backup_rets) {
                sig.to_mut().returns = new_returns;
            }
        } else {
            sig.to_mut().returns = new_returns;
        }
    }

    if let Some(new_params) = legalize_args(&sig.params, &mut args) {
        sig.to_mut().params = new_params;
    }
}

/// Get register class for a type appearing in a legalized signature.
pub fn regclass_for_abi_type(ty: ir::Type) -> RegClass {
    if ty.is_int() || ty.is_bool() || ty.is_ref() {
        GPR
    } else {
        FPR
    }
}

/// Get the set of allocatable registers for `func`.
pub fn allocatable_registers(triple: &Triple, flags: &shared_settings::Flags) -> RegisterSet {
    let mut regs = RegisterSet::new();
    regs.take(GPR, RU::rsp as RegUnit);
    regs.take(GPR, RU::rbp as RegUnit);

    // 32-bit arch only has 8 registers.
    if triple.pointer_width().unwrap() != PointerWidth::U64 {
        for i in 8..16 {
            regs.take(GPR, GPR.unit(i));
            regs.take(FPR, FPR.unit(i));
        }
        if flags.enable_pinned_reg() {
            unimplemented!("Pinned register not implemented on x86-32.");
        }
    } else {
        // Choose r15 as the pinned register on 64-bits: it is non-volatile on native ABIs and
        // isn't the fixed output register of any instruction.
        if flags.enable_pinned_reg() {
            regs.take(GPR, RU::r15 as RegUnit);
        }
    }

    regs
}

/// Get the set of callee-saved general-purpose registers.
fn callee_saved_gprs(isa: &dyn TargetIsa, call_conv: CallConv) -> &'static [RU] {
    match isa.triple().pointer_width().unwrap() {
        PointerWidth::U16 => panic!(),
        PointerWidth::U32 => &[RU::rbx, RU::rsi, RU::rdi],
        PointerWidth::U64 => {
            if call_conv.extends_windows_fastcall() {
                // "registers RBX, RBP, RDI, RSI, RSP, R12, R13, R14, R15, and XMM6-15 are
                // considered nonvolatile and must be saved and restored by a function that uses
                //  them."
                // as per https://docs.microsoft.com/en-us/cpp/build/x64-calling-convention
                // RSP & RBP are not listed below, since they are restored automatically during
                // a function call. If that wasn't the case, function calls (RET) would not work.
                &[
                    RU::rbx,
                    RU::rdi,
                    RU::rsi,
                    RU::r12,
                    RU::r13,
                    RU::r14,
                    RU::r15,
                ]
            } else {
                &[RU::rbx, RU::r12, RU::r13, RU::r14, RU::r15]
            }
        }
    }
}

/// Get the set of callee-saved floating-point (SIMD) registers.
fn callee_saved_fprs(isa: &dyn TargetIsa, call_conv: CallConv) -> &'static [RU] {
    match isa.triple().pointer_width().unwrap() {
        PointerWidth::U16 => panic!(),
        PointerWidth::U32 => &[],
        PointerWidth::U64 => {
            if call_conv.extends_windows_fastcall() {
                // "registers RBX, ... , and XMM6-15 are considered nonvolatile and must be saved
                //  and restored by a function that uses them."
                // as per https://docs.microsoft.com/en-us/cpp/build/x64-calling-convention as of
                // February 5th, 2020.
                &[
                    RU::xmm6,
                    RU::xmm7,
                    RU::xmm8,
                    RU::xmm9,
                    RU::xmm10,
                    RU::xmm11,
                    RU::xmm12,
                    RU::xmm13,
                    RU::xmm14,
                    RU::xmm15,
                ]
            } else {
                &[]
            }
        }
    }
}

/// Get the set of callee-saved registers that are used.
fn callee_saved_regs_used(isa: &dyn TargetIsa, func: &ir::Function) -> RegisterSet {
    let mut all_callee_saved = RegisterSet::empty();
    for reg in callee_saved_gprs(isa, func.signature.call_conv) {
        all_callee_saved.free(GPR, *reg as RegUnit);
    }
    for reg in callee_saved_fprs(isa, func.signature.call_conv) {
        all_callee_saved.free(FPR, *reg as RegUnit);
    }

    let mut used = RegisterSet::empty();
    for value_loc in func.locations.values() {
        // Note that `value_loc` here contains only a single unit of a potentially multi-unit
        // register. We don't use registers that overlap each other in the x86 ISA, but in others
        // we do. So this should not be blindly reused.
        if let ValueLoc::Reg(ru) = *value_loc {
            if GPR.contains(ru) {
                if !used.is_avail(GPR, ru) {
                    used.free(GPR, ru);
                }
            } else if FPR.contains(ru) {
                if !used.is_avail(FPR, ru) {
                    used.free(FPR, ru);
                }
            }
        }
    }

    // regmove and regfill instructions may temporarily divert values into other registers,
    // and these are not reflected in `func.locations`. Scan the function for such instructions
    // and note which callee-saved registers they use.
    //
    // TODO: Consider re-evaluating how regmove/regfill/regspill work and whether it's possible
    // to avoid this step.
    for block in &func.layout {
        for inst in func.layout.block_insts(block) {
            match func.dfg[inst] {
                ir::instructions::InstructionData::RegMove { dst, .. }
                | ir::instructions::InstructionData::RegFill { dst, .. } => {
                    if GPR.contains(dst) {
                        if !used.is_avail(GPR, dst) {
                            used.free(GPR, dst);
                        }
                    } else if FPR.contains(dst) {
                        if !used.is_avail(FPR, dst) {
                            used.free(FPR, dst);
                        }
                    }
                }
                _ => (),
            }
        }
    }

    used.intersect(&all_callee_saved);
    used
}

pub fn prologue_epilogue(func: &mut ir::Function, isa: &dyn TargetIsa) -> CodegenResult<()> {
    match func.signature.call_conv {
        // For now, just translate fast and cold as system_v.
        CallConv::Fast | CallConv::Cold | CallConv::SystemV => {
            system_v_prologue_epilogue(func, isa)
        }
        CallConv::WindowsFastcall => fastcall_prologue_epilogue(func, isa),
        CallConv::BaldrdashSystemV | CallConv::BaldrdashWindows => {
            baldrdash_prologue_epilogue(func, isa)
        }
        CallConv::Probestack => unimplemented!("probestack calling convention"),
    }
}

fn baldrdash_prologue_epilogue(func: &mut ir::Function, isa: &dyn TargetIsa) -> CodegenResult<()> {
    debug_assert!(
        !isa.flags().enable_probestack(),
        "baldrdash does not expect cranelift to emit stack probes"
    );

    let word_size = StackSize::from(isa.pointer_bytes());
    let shadow_store_size = if func.signature.call_conv.extends_windows_fastcall() {
        WIN_SHADOW_STACK_SPACE
    } else {
        0
    };

    let bytes =
        StackSize::from(isa.flags().baldrdash_prologue_words()) * word_size + shadow_store_size;

    let mut ss = ir::StackSlotData::new(ir::StackSlotKind::IncomingArg, bytes);
    ss.offset = Some(-(bytes as StackOffset));
    func.stack_slots.push(ss);

    let is_leaf = func.is_leaf();
    layout_stack(&mut func.stack_slots, is_leaf, STACK_ALIGNMENT)?;
    Ok(())
}

/// Implementation of the fastcall-based Win64 calling convention described at [1]
/// [1] https://docs.microsoft.com/en-us/cpp/build/x64-calling-convention
fn fastcall_prologue_epilogue(func: &mut ir::Function, isa: &dyn TargetIsa) -> CodegenResult<()> {
    if isa.triple().pointer_width().unwrap() != PointerWidth::U64 {
        panic!("TODO: windows-fastcall: x86-32 not implemented yet");
    }

    // The reserved stack area is composed of:
    //   return address + frame pointer + all callee-saved registers
    //
    // Pushing the return address is an implicit function of the `call`
    // instruction. Each of the others we will then push explicitly. Then we
    // will adjust the stack pointer to make room for the rest of the required
    // space for this frame.
    let csrs = callee_saved_regs_used(isa, func);
    let gpsr_stack_size = ((csrs.iter(GPR).len() + 2) * isa.pointer_bytes() as usize) as u32;
    let fpsr_stack_size = (csrs.iter(FPR).len() * types::F64X2.bytes() as usize) as u32;
    let mut csr_stack_size = gpsr_stack_size + fpsr_stack_size;

    // FPRs must be saved with 16-byte alignment; because they follow the GPRs on the stack, align if needed
    if fpsr_stack_size > 0 {
        csr_stack_size = (csr_stack_size + 15) & !15;
    }

    func.create_stack_slot(ir::StackSlotData {
        kind: ir::StackSlotKind::IncomingArg,
        size: csr_stack_size,
        offset: Some(-(csr_stack_size as StackOffset)),
    });

    let is_leaf = func.is_leaf();

    // If not a leaf function, allocate an explicit stack slot at the end of the space for the callee's shadow space
    if !is_leaf {
        // TODO: eventually use the caller-provided shadow store as spill slot space when laying out the stack
        func.create_stack_slot(ir::StackSlotData {
            kind: ir::StackSlotKind::ExplicitSlot,
            size: WIN_SHADOW_STACK_SPACE,
            offset: None,
        });
    }

    let total_stack_size = layout_stack(&mut func.stack_slots, is_leaf, STACK_ALIGNMENT)? as i32;

    // Subtract the GPR saved register size from the local size because pushes are used for the saves
    let local_stack_size = i64::from(total_stack_size - gpsr_stack_size as i32);

    // Add CSRs to function signature
    let reg_type = isa.pointer_type();
    let sp_arg_index = if fpsr_stack_size > 0 {
        let sp_arg = ir::AbiParam::special_reg(
            reg_type,
            ir::ArgumentPurpose::CalleeSaved,
            RU::rsp as RegUnit,
        );
        let index = func.signature.params.len();
        func.signature.params.push(sp_arg);
        Some(index)
    } else {
        None
    };
    let fp_arg = ir::AbiParam::special_reg(
        reg_type,
        ir::ArgumentPurpose::FramePointer,
        RU::rbp as RegUnit,
    );
    func.signature.params.push(fp_arg);
    func.signature.returns.push(fp_arg);

    for gp_csr in csrs.iter(GPR) {
        let csr_arg = ir::AbiParam::special_reg(reg_type, ir::ArgumentPurpose::CalleeSaved, gp_csr);
        func.signature.params.push(csr_arg);
        func.signature.returns.push(csr_arg);
    }

    for fp_csr in csrs.iter(FPR) {
        // The calling convention described in
        // https://docs.microsoft.com/en-us/cpp/build/x64-calling-convention only requires
        // preserving the low 128 bits of XMM6-XMM15.
        let csr_arg =
            ir::AbiParam::special_reg(types::F64X2, ir::ArgumentPurpose::CalleeSaved, fp_csr);
        func.signature.params.push(csr_arg);
        func.signature.returns.push(csr_arg);
    }

    // Set up the cursor and insert the prologue
    let entry_block = func.layout.entry_block().expect("missing entry block");
    let mut pos = EncCursor::new(func, isa).at_first_insertion_point(entry_block);
    insert_common_prologue(
        &mut pos,
        local_stack_size,
        reg_type,
        &csrs,
        sp_arg_index.is_some(),
        isa,
    );

    // Reset the cursor and insert the epilogue
    let mut pos = pos.at_position(CursorPosition::Nowhere);
    insert_common_epilogues(&mut pos, local_stack_size, reg_type, &csrs, sp_arg_index);

    Ok(())
}

/// Insert a System V-compatible prologue and epilogue.
fn system_v_prologue_epilogue(func: &mut ir::Function, isa: &dyn TargetIsa) -> CodegenResult<()> {
    let pointer_width = isa.triple().pointer_width().unwrap();
    let word_size = pointer_width.bytes() as usize;

    let csrs = callee_saved_regs_used(isa, func);
    assert!(
        csrs.iter(FPR).len() == 0,
        "SysV ABI does not have callee-save SIMD registers"
    );

    // The reserved stack area is composed of:
    //   return address + frame pointer + all callee-saved registers
    //
    // Pushing the return address is an implicit function of the `call`
    // instruction. Each of the others we will then push explicitly. Then we
    // will adjust the stack pointer to make room for the rest of the required
    // space for this frame.
    let csr_stack_size = ((csrs.iter(GPR).len() + 2) * word_size) as i32;
    func.create_stack_slot(ir::StackSlotData {
        kind: ir::StackSlotKind::IncomingArg,
        size: csr_stack_size as u32,
        offset: Some(-csr_stack_size),
    });

    let is_leaf = func.is_leaf();
    let total_stack_size = layout_stack(&mut func.stack_slots, is_leaf, STACK_ALIGNMENT)? as i32;
    let local_stack_size = i64::from(total_stack_size - csr_stack_size);

    // Add CSRs to function signature
    let reg_type = ir::Type::int(u16::from(pointer_width.bits())).unwrap();
    // On X86-32 all parameters, including vmctx, are passed on stack, and we need
    // to extract vmctx from the stack before we can save the frame pointer.
    let sp_arg_index = if isa.pointer_bits() == 32 {
        let sp_arg = ir::AbiParam::special_reg(
            reg_type,
            ir::ArgumentPurpose::CalleeSaved,
            RU::rsp as RegUnit,
        );
        let index = func.signature.params.len();
        func.signature.params.push(sp_arg);
        Some(index)
    } else {
        None
    };
    let fp_arg = ir::AbiParam::special_reg(
        reg_type,
        ir::ArgumentPurpose::FramePointer,
        RU::rbp as RegUnit,
    );
    func.signature.params.push(fp_arg);
    func.signature.returns.push(fp_arg);

    for csr in csrs.iter(GPR) {
        let csr_arg = ir::AbiParam::special_reg(reg_type, ir::ArgumentPurpose::CalleeSaved, csr);
        func.signature.params.push(csr_arg);
        func.signature.returns.push(csr_arg);
    }

    // Set up the cursor and insert the prologue
    let entry_block = func.layout.entry_block().expect("missing entry block");
    let mut pos = EncCursor::new(func, isa).at_first_insertion_point(entry_block);
    insert_common_prologue(
        &mut pos,
        local_stack_size,
        reg_type,
        &csrs,
        sp_arg_index.is_some(),
        isa,
    );

    // Reset the cursor and insert the epilogue
    let mut pos = pos.at_position(CursorPosition::Nowhere);
    insert_common_epilogues(&mut pos, local_stack_size, reg_type, &csrs, sp_arg_index);

    Ok(())
}

/// Insert the prologue for a given function.
/// This is used by common calling conventions such as System V.
fn insert_common_prologue(
    pos: &mut EncCursor,
    stack_size: i64,
    reg_type: ir::types::Type,
    csrs: &RegisterSet,
    has_sp_param: bool,
    isa: &dyn TargetIsa,
) {
    let sp = if has_sp_param {
        let block = pos.current_block().expect("missing block under cursor");
        let sp = pos.func.dfg.append_block_param(block, reg_type);
        pos.func.locations[sp] = ir::ValueLoc::Reg(RU::rsp as RegUnit);
        Some(sp)
    } else {
        None
    };

    // If this is a leaf function with zero stack, then there's no need to
    // insert a stack check since it can't overflow anything and
    // forward-progress is guarantee so long as loop are handled anyway.
    //
    // If this has a stack size it could stack overflow, or if it isn't a leaf
    // it could be part of a long call chain which we need to check anyway.
    //
    // First we look for the stack limit as a special argument to the function,
    // and failing that we see if a custom stack limit factory has been provided
    // which will be used to likely calculate the stack limit from the arguments
    // or perhaps constants.
    if stack_size > 0 || !pos.func.is_leaf() {
        let scratch = ir::ValueLoc::Reg(RU::rax as RegUnit);
        let stack_limit_arg = match pos.func.special_param(ArgumentPurpose::StackLimit) {
            Some(arg) => {
                let copy = pos.ins().copy(arg);
                pos.func.locations[copy] = scratch;
                Some(copy)
            }
            None => pos
                .func
                .stack_limit
                .map(|gv| interpret_gv(pos, gv, sp, scratch)),
        };
        if let Some(stack_limit_arg) = stack_limit_arg {
            insert_stack_check(pos, stack_size, stack_limit_arg);
        }
    }

    // Append param to entry block
    let block = pos.current_block().expect("missing block under cursor");
    let fp = pos.func.dfg.append_block_param(block, reg_type);
    pos.func.locations[fp] = ir::ValueLoc::Reg(RU::rbp as RegUnit);

    pos.ins().x86_push(fp);

    let mov_sp_inst = pos
        .ins()
        .copy_special(RU::rsp as RegUnit, RU::rbp as RegUnit);

    let mut last_csr_push = None;
    for reg in csrs.iter(GPR) {
        // Append param to entry block
        let csr_arg = pos.func.dfg.append_block_param(block, reg_type);

        // Assign it a location
        pos.func.locations[csr_arg] = ir::ValueLoc::Reg(reg);
        last_csr_push = Some(pos.ins().x86_push(csr_arg));
    }

    // Allocate stack frame storage.
    let mut adjust_sp_inst = None;
    if stack_size > 0 {
        if isa.flags().enable_probestack() && stack_size > (1 << isa.flags().probestack_size_log2())
        {
            // Emit a stack probe.
            let rax = RU::rax as RegUnit;
            let rax_val = ir::ValueLoc::Reg(rax);

            // The probestack function expects its input in %rax.
            let arg = pos.ins().iconst(reg_type, stack_size);
            pos.func.locations[arg] = rax_val;

            // Call the probestack function.
            let callee = get_probestack_funcref(pos.func, reg_type, rax, isa);

            // Make the call.
            let call = if !isa.flags().is_pic()
                && isa.triple().pointer_width().unwrap() == PointerWidth::U64
                && !pos.func.dfg.ext_funcs[callee].colocated
            {
                // 64-bit non-PIC non-colocated calls need to be legalized to call_indirect.
                // Use r11 as it may be clobbered under all supported calling conventions.
                let r11 = RU::r11 as RegUnit;
                let sig = pos.func.dfg.ext_funcs[callee].signature;
                let addr = pos.ins().func_addr(reg_type, callee);
                pos.func.locations[addr] = ir::ValueLoc::Reg(r11);
                pos.ins().call_indirect(sig, addr, &[arg])
            } else {
                // Otherwise just do a normal call.
                pos.ins().call(callee, &[arg])
            };

            // If the probestack function doesn't adjust sp, do it ourselves.
            if !isa.flags().probestack_func_adjusts_sp() {
                let result = pos.func.dfg.inst_results(call)[0];
                pos.func.locations[result] = rax_val;
                adjust_sp_inst = Some(pos.ins().adjust_sp_down(result));
            }
        } else {
            // Simply decrement the stack pointer.
            adjust_sp_inst = Some(pos.ins().adjust_sp_down_imm(Imm64::new(stack_size)));
        }
    }

    // With the stack pointer adjusted, save any callee-saved floating point registers via offset
    // FPR saves are at the highest addresses of the local frame allocation, immediately following the GPR pushes
    let mut last_fpr_save = None;

    for (i, reg) in csrs.iter(FPR).enumerate() {
        // Append param to entry block
        let csr_arg = pos.func.dfg.append_block_param(block, types::F64X2);

        // Since regalloc has already run, we must assign a location.
        pos.func.locations[csr_arg] = ir::ValueLoc::Reg(reg);

        // Offset to where the register is saved relative to RSP, accounting for FPR save alignment
        let offset = ((i + 1) * types::F64X2.bytes() as usize) as i64
            + (stack_size % types::F64X2.bytes() as i64);

        last_fpr_save = Some(pos.ins().store(
            ir::MemFlags::trusted(),
            csr_arg,
            sp.expect("FPR save requires SP param"),
            (stack_size - offset) as i32,
        ));
    }

    pos.func.prologue_end = Some(
        last_fpr_save
            .or(adjust_sp_inst)
            .or(last_csr_push)
            .unwrap_or(mov_sp_inst),
    );
}

/// Inserts code necessary to calculate `gv`.
///
/// Note that this is typically done with `ins().global_value(...)` but that
/// requires legalization to run to encode it, and we're running super late
/// here in the backend where legalization isn't possible. To get around this
/// we manually interpret the `gv` specified and do register allocation for
/// intermediate values.
///
/// This is an incomplete implementation of loading `GlobalValue` values to get
/// compared to the stack pointer, but currently it serves enough functionality
/// to get this implemented in `wasmtime` itself. This'll likely get expanded a
/// bit over time!
fn interpret_gv(
    pos: &mut EncCursor,
    gv: ir::GlobalValue,
    sp: Option<ir::Value>,
    scratch: ir::ValueLoc,
) -> ir::Value {
    match pos.func.global_values[gv] {
        ir::GlobalValueData::VMContext => {
            let vmctx_index = pos
                .func
                .signature
                .special_param_index(ir::ArgumentPurpose::VMContext)
                .expect("no vmcontext parameter found");
            match pos.func.signature.params[vmctx_index] {
                AbiParam {
                    location: ArgumentLoc::Reg(_),
                    ..
                } => {
                    let entry = pos.func.layout.entry_block().unwrap();
                    pos.func.dfg.block_params(entry)[vmctx_index]
                }
                AbiParam {
                    location: ArgumentLoc::Stack(offset),
                    value_type,
                    ..
                } => {
                    let offset =
                        offset + i32::from(pos.isa.pointer_bytes() * (1 + vmctx_index as u8));
                    // The following access can be marked `trusted` because it is a load of an argument. We
                    // know it is safe because it was safe to write it in preparing this function call.
                    let ret =
                        pos.ins()
                            .load(value_type, ir::MemFlags::trusted(), sp.unwrap(), offset);
                    pos.func.locations[ret] = scratch;
                    return ret;
                }
                AbiParam {
                    location: ArgumentLoc::Unassigned,
                    ..
                } => unreachable!(),
            }
        }
        ir::GlobalValueData::Load {
            base,
            offset,
            global_type,
            readonly: _,
        } => {
            let base = interpret_gv(pos, base, sp, scratch);
            let ret = pos
                .ins()
                .load(global_type, ir::MemFlags::trusted(), base, offset);
            pos.func.locations[ret] = scratch;
            return ret;
        }
        ref other => panic!("global value for stack limit not supported: {}", other),
    }
}

/// Insert a check that generates a trap if the stack pointer goes
/// below a value in `stack_limit_arg`.
fn insert_stack_check(pos: &mut EncCursor, stack_size: i64, stack_limit_arg: ir::Value) {
    use crate::ir::condcodes::IntCC;

    // Our stack pointer, after subtracting `stack_size`, must not be below
    // `stack_limit_arg`. To do this we're going to add `stack_size` to
    // `stack_limit_arg` and see if the stack pointer is below that. The
    // `stack_size + stack_limit_arg` computation might overflow, however, due
    // to how stack limits may be loaded and set externally to trigger a trap.
    //
    // To handle this we'll need an extra comparison to see if the stack
    // pointer is already below `stack_limit_arg`. Most of the time this
    // isn't necessary though since the stack limit which triggers a trap is
    // likely a sentinel somewhere around `usize::max_value()`. In that case
    // only conditionally emit this pre-flight check. That way most functions
    // only have the one comparison, but are also guaranteed that if we add
    // `stack_size` to `stack_limit_arg` is won't overflow.
    //
    // This does mean that code generators which use this stack check
    // functionality need to ensure that values stored into the stack limit
    // will never overflow if this threshold is added.
    if stack_size >= 32 * 1024 {
        let cflags = pos.ins().ifcmp_sp(stack_limit_arg);
        pos.func.locations[cflags] = ir::ValueLoc::Reg(RU::rflags as RegUnit);
        pos.ins().trapif(
            IntCC::UnsignedGreaterThanOrEqual,
            cflags,
            ir::TrapCode::StackOverflow,
        );
    }

    // Copy `stack_limit_arg` into a %rax and use it for calculating
    // a SP threshold.
    let sp_threshold = pos.ins().iadd_imm(stack_limit_arg, stack_size);
    pos.func.locations[sp_threshold] = ir::ValueLoc::Reg(RU::rax as RegUnit);

    // If the stack pointer currently reaches the SP threshold or below it then after opening
    // the current stack frame, the current stack pointer will reach the limit.
    let cflags = pos.ins().ifcmp_sp(sp_threshold);
    pos.func.locations[cflags] = ir::ValueLoc::Reg(RU::rflags as RegUnit);
    pos.ins().trapif(
        IntCC::UnsignedGreaterThanOrEqual,
        cflags,
        ir::TrapCode::StackOverflow,
    );
}

/// Find all `return` instructions and insert epilogues before them.
fn insert_common_epilogues(
    pos: &mut EncCursor,
    stack_size: i64,
    reg_type: ir::types::Type,
    csrs: &RegisterSet,
    sp_arg_index: Option<usize>,
) {
    while let Some(block) = pos.next_block() {
        pos.goto_last_inst(block);
        if let Some(inst) = pos.current_inst() {
            if pos.func.dfg[inst].opcode().is_return() {
                insert_common_epilogue(inst, stack_size, pos, reg_type, csrs, sp_arg_index);
            }
        }
    }
}

/// Insert an epilogue given a specific `return` instruction.
/// This is used by common calling conventions such as System V.
fn insert_common_epilogue(
    inst: ir::Inst,
    stack_size: i64,
    pos: &mut EncCursor,
    reg_type: ir::types::Type,
    csrs: &RegisterSet,
    sp_arg_index: Option<usize>,
) {
    // Insert the pop of the frame pointer
    let fp_pop = pos.ins().x86_pop(reg_type);
    let fp_pop_inst = pos.prev_inst().unwrap();
    pos.func.locations[fp_pop] = ir::ValueLoc::Reg(RU::rbp as RegUnit);
    pos.func.dfg.append_inst_arg(inst, fp_pop);

    // Insert the CSR pops
    let mut first_csr_pop_inst = None;
    for reg in csrs.iter(GPR) {
        let csr_pop = pos.ins().x86_pop(reg_type);
        first_csr_pop_inst = pos.prev_inst();
        assert!(first_csr_pop_inst.is_some());
        pos.func.locations[csr_pop] = ir::ValueLoc::Reg(reg);
        pos.func.dfg.append_inst_arg(inst, csr_pop);
    }

    // Insert the adjustment of SP
    let mut sp_adjust_inst = None;
    if stack_size > 0 {
        pos.ins().adjust_sp_up_imm(Imm64::new(stack_size));
        sp_adjust_inst = pos.prev_inst();
        assert!(sp_adjust_inst.is_some());
    }

    let mut first_fpr_load = None;
    if let Some(index) = sp_arg_index {
        let sp = pos
            .func
            .dfg
            .block_params(pos.func.layout.entry_block().unwrap())[index];

        // Insert the FPR loads (unlike the GPRs, which are stack pops, these are in-order loads)
        for (i, reg) in csrs.iter(FPR).enumerate() {
            // Offset to where the register is saved relative to RSP, accounting for FPR save alignment
            let offset = ((i + 1) * types::F64X2.bytes() as usize) as i64
                + (stack_size % types::F64X2.bytes() as i64);

            let value = pos.ins().load(
                types::F64X2,
                ir::MemFlags::trusted(),
                sp,
                (stack_size - offset) as i32,
            );

            first_fpr_load.get_or_insert(pos.current_inst().expect("current inst"));

            pos.func.locations[value] = ir::ValueLoc::Reg(reg);
            pos.func.dfg.append_inst_arg(inst, value);
        }
    } else {
        assert!(csrs.iter(FPR).len() == 0);
    }

    pos.func.epilogues_start.push(
        first_fpr_load
            .or(sp_adjust_inst)
            .or(first_csr_pop_inst)
            .unwrap_or(fp_pop_inst),
    );
}

#[cfg(feature = "unwind")]
pub fn create_unwind_info(
    func: &ir::Function,
    isa: &dyn TargetIsa,
) -> CodegenResult<Option<crate::isa::unwind::UnwindInfo>> {
    use crate::isa::unwind::UnwindInfo;

    // Assumption: RBP is being used as the frame pointer for both calling conventions
    // In the future, we should be omitting frame pointer as an optimization, so this will change
    Ok(match func.signature.call_conv {
        CallConv::Fast | CallConv::Cold | CallConv::SystemV => {
            super::unwind::systemv::create_unwind_info(func, isa, Some(RU::rbp.into()))?
                .map(|u| UnwindInfo::SystemV(u))
        }
        CallConv::WindowsFastcall => {
            super::unwind::winx64::create_unwind_info(func, isa)?.map(|u| UnwindInfo::WindowsX64(u))
        }
        _ => None,
    })
}
