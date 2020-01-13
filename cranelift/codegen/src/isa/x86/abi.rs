//! x86 ABI implementation.

use super::super::settings as shared_settings;
#[cfg(feature = "unwind")]
use super::fde::emit_fde;
use super::registers::{FPR, GPR, RU};
use super::settings as isa_settings;
#[cfg(feature = "unwind")]
use super::unwind::UnwindInfo;
use crate::abi::{legalize_args, ArgAction, ArgAssigner, ValueConversion};
#[cfg(feature = "unwind")]
use crate::binemit::{FrameUnwindKind, FrameUnwindSink};
use crate::cursor::{Cursor, CursorPosition, EncCursor};
use crate::ir;
use crate::ir::immediates::Imm64;
use crate::ir::stackslot::{StackOffset, StackSize};
use crate::ir::{
    get_probestack_funcref, AbiParam, ArgumentExtension, ArgumentLoc, ArgumentPurpose,
    FrameLayoutChange, InstBuilder, ValueLoc,
};
use crate::isa::{CallConv, RegClass, RegUnit, TargetIsa};
use crate::regalloc::RegisterSet;
use crate::result::CodegenResult;
use crate::stack_layout::layout_stack;
use alloc::borrow::Cow;
use core::i32;
use std::boxed::Box;
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
const WIN_SHADOW_STACK_SPACE: i32 = 32;

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
}

impl Args {
    fn new(
        bits: u8,
        gpr: &'static [RU],
        fpr_limit: usize,
        call_conv: CallConv,
        shared_flags: &shared_settings::Flags,
        isa_flags: &isa_settings::Flags,
    ) -> Self {
        let offset = if call_conv.extends_windows_fastcall() {
            WIN_SHADOW_STACK_SPACE
        } else {
            0
        } as u32;

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
        }
    }
}

impl ArgAssigner for Args {
    fn assign(&mut self, arg: &AbiParam) -> ArgAction {
        let ty = arg.value_type;

        // Vectors should stay in vector registers unless SIMD is not enabled--then they are split
        if ty.is_vector() {
            if self.shared_flags.enable_simd() {
                let reg = FPR.unit(self.fpr_used);
                self.fpr_used += 1;
                return ArgumentLoc::Reg(reg).into();
            }
            return ValueConversion::VectorSplit.into();
        }

        // Large integers and booleans are broken down to fit in a register.
        if !ty.is_float() && ty.bits() > u16::from(self.pointer_bits) {
            return ValueConversion::IntSplit.into();
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
            args = Args::new(bits, &[], 0, sig.call_conv, shared_flags, isa_flags);
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
                )
            } else {
                Args::new(
                    bits,
                    &ARG_GPRS[..],
                    8,
                    sig.call_conv,
                    shared_flags,
                    isa_flags,
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
    );

    let sig_is_multi_return = sig.is_multi_return();

    // If this is a multi-value return and we don't have enough available return
    // registers to fit all of the return values, we need to backtrack and start
    // assigning locations all over again with a different strategy. In order to
    // do that, we need a copy of the original assigner for the returns.
    let backup_rets_for_struct_return = if sig_is_multi_return {
        Some(rets.clone())
    } else {
        None
    };

    if let Some(new_returns) = legalize_args(&sig.returns, &mut rets) {
        if sig.is_multi_return()
            && new_returns
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
            };
            match args.assign(&ret_ptr_param) {
                ArgAction::Assign(ArgumentLoc::Reg(reg)) => {
                    ret_ptr_param.location = ArgumentLoc::Reg(reg);
                    sig.to_mut().params.push(ret_ptr_param);
                }
                _ => unreachable!("return pointer should always get a register assignment"),
            }

            let mut backup_rets = backup_rets_for_struct_return.unwrap();

            // We're using the first return register for the return pointer (like
            // sys v does).
            let mut ret_ptr_return = AbiParam {
                value_type: args.pointer_type,
                purpose: ArgumentPurpose::StructReturn,
                extension: ArgumentExtension::None,
                location: ArgumentLoc::Unassigned,
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
    if ty.is_int() || ty.is_bool() {
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

/// Get the set of callee-saved registers.
fn callee_saved_gprs(isa: &dyn TargetIsa, call_conv: CallConv) -> &'static [RU] {
    match isa.triple().pointer_width().unwrap() {
        PointerWidth::U16 => panic!(),
        PointerWidth::U32 => &[RU::rbx, RU::rsi, RU::rdi],
        PointerWidth::U64 => {
            if call_conv.extends_windows_fastcall() {
                // "registers RBX, RBP, RDI, RSI, RSP, R12, R13, R14, R15 are considered nonvolatile
                //  and must be saved and restored by a function that uses them."
                // as per https://docs.microsoft.com/en-us/cpp/build/x64-calling-convention
                // RSP & RSB are not listed below, since they are restored automatically during
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

/// Get the set of callee-saved registers that are used.
fn callee_saved_gprs_used(isa: &dyn TargetIsa, func: &ir::Function) -> RegisterSet {
    let mut all_callee_saved = RegisterSet::empty();
    for reg in callee_saved_gprs(isa, func.signature.call_conv) {
        all_callee_saved.free(GPR, *reg as RegUnit);
    }

    let mut used = RegisterSet::empty();
    for value_loc in func.locations.values() {
        // Note that `value_loc` here contains only a single unit of a potentially multi-unit
        // register. We don't use registers that overlap each other in the x86 ISA, but in others
        // we do. So this should not be blindly reused.
        if let ValueLoc::Reg(ru) = *value_loc {
            if !used.is_avail(GPR, ru) {
                used.free(GPR, ru);
            }
        }
    }

    // regmove and regfill instructions may temporarily divert values into other registers,
    // and these are not reflected in `func.locations`. Scan the function for such instructions
    // and note which callee-saved registers they use.
    //
    // TODO: Consider re-evaluating how regmove/regfill/regspill work and whether it's possible
    // to avoid this step.
    for ebb in &func.layout {
        for inst in func.layout.ebb_insts(ebb) {
            match func.dfg[inst] {
                ir::instructions::InstructionData::RegMove { dst, .. }
                | ir::instructions::InstructionData::RegFill { dst, .. } => {
                    if !used.is_avail(GPR, dst) {
                        used.free(GPR, dst);
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
        !isa.flags().probestack_enabled(),
        "baldrdash does not expect cranelift to emit stack probes"
    );

    let word_size = StackSize::from(isa.pointer_bytes());
    let shadow_store_size = if func.signature.call_conv.extends_windows_fastcall() {
        WIN_SHADOW_STACK_SPACE as u32
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

/// CFAState is cranelift's model of the call frame layout at any particular point in a function.
/// It describes the call frame's layout in terms of a call frame address, where it is with respect
/// to the start of the call frame, and the where the top of the stack is with respect to it.
///
/// Changes in this layout are used to derive appropriate `ir::FrameLayoutChange` to record for
/// relevant instructions.
#[derive(Clone)]
struct CFAState {
    /// The register from which we can derive the call frame address. On x86_64, this is typically
    /// `rbp`, but at function entry and exit may be `rsp` while the call frame is being
    /// established.
    cf_ptr_reg: RegUnit,
    /// Given that `cf_ptr_reg` is a register containing a pointer to some memory, `cf_ptr_offset`
    /// is the offset from that pointer to the address of the start of this function's call frame.
    ///
    /// For a concrete x86_64 example, we will start this at 8 - the call frame begins immediately
    /// before the return address. This will typically then be set to 16, after pushing `rbp` to
    /// preserve the parent call frame. It is very unlikely the offset should be anything other
    /// than one or two pointer widths.
    cf_ptr_offset: isize,
    /// The offset between the start of the call frame and the current stack pointer. This is
    /// primarily useful to point to where on the stack preserved registers are, but is maintained
    /// through the whole function for consistency.
    current_depth: isize,
}

/// Implementation of the fastcall-based Win64 calling convention described at [1]
/// [1] https://docs.microsoft.com/en-us/cpp/build/x64-calling-convention
fn fastcall_prologue_epilogue(func: &mut ir::Function, isa: &dyn TargetIsa) -> CodegenResult<()> {
    if isa.triple().pointer_width().unwrap() != PointerWidth::U64 {
        panic!("TODO: windows-fastcall: x86-32 not implemented yet");
    }

    let csrs = callee_saved_gprs_used(isa, func);

    // The reserved stack area is composed of:
    //   return address + frame pointer + all callee-saved registers + shadow space
    //
    // Pushing the return address is an implicit function of the `call`
    // instruction. Each of the others we will then push explicitly. Then we
    // will adjust the stack pointer to make room for the rest of the required
    // space for this frame.
    let word_size = isa.pointer_bytes() as usize;
    let csr_stack_size = ((csrs.iter(GPR).len() + 2) * word_size) as i32;

    // TODO: eventually use the 32 bytes (shadow store) as spill slot. This currently doesn't work
    //       since cranelift does not support spill slots before incoming args

    func.create_stack_slot(ir::StackSlotData {
        kind: ir::StackSlotKind::IncomingArg,
        size: csr_stack_size as u32,
        offset: Some(-(WIN_SHADOW_STACK_SPACE + csr_stack_size)),
    });

    let is_leaf = func.is_leaf();
    let total_stack_size = layout_stack(&mut func.stack_slots, is_leaf, STACK_ALIGNMENT)? as i32;
    let local_stack_size = i64::from(total_stack_size - csr_stack_size);

    // Add CSRs to function signature
    let reg_type = isa.pointer_type();
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
    let entry_ebb = func.layout.entry_block().expect("missing entry block");
    let mut pos = EncCursor::new(func, isa).at_first_insertion_point(entry_ebb);
    let prologue_cfa_state =
        insert_common_prologue(&mut pos, local_stack_size, reg_type, &csrs, isa);

    // Reset the cursor and insert the epilogue
    let mut pos = pos.at_position(CursorPosition::Nowhere);
    insert_common_epilogues(
        &mut pos,
        local_stack_size,
        reg_type,
        &csrs,
        isa,
        prologue_cfa_state,
    );

    Ok(())
}

/// Insert a System V-compatible prologue and epilogue.
fn system_v_prologue_epilogue(func: &mut ir::Function, isa: &dyn TargetIsa) -> CodegenResult<()> {
    let pointer_width = isa.triple().pointer_width().unwrap();
    let word_size = pointer_width.bytes() as usize;

    let csrs = callee_saved_gprs_used(isa, func);

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
    let entry_ebb = func.layout.entry_block().expect("missing entry block");
    let mut pos = EncCursor::new(func, isa).at_first_insertion_point(entry_ebb);
    let prologue_cfa_state =
        insert_common_prologue(&mut pos, local_stack_size, reg_type, &csrs, isa);

    // Reset the cursor and insert the epilogue
    let mut pos = pos.at_position(CursorPosition::Nowhere);
    insert_common_epilogues(
        &mut pos,
        local_stack_size,
        reg_type,
        &csrs,
        isa,
        prologue_cfa_state,
    );

    Ok(())
}

/// Insert the prologue for a given function.
/// This is used by common calling conventions such as System V.
fn insert_common_prologue(
    pos: &mut EncCursor,
    stack_size: i64,
    reg_type: ir::types::Type,
    csrs: &RegisterSet,
    isa: &dyn TargetIsa,
) -> Option<CFAState> {
    let word_size = isa.pointer_bytes() as isize;
    if stack_size > 0 {
        // Check if there is a special stack limit parameter. If so insert stack check.
        if let Some(stack_limit_arg) = pos.func.special_param(ArgumentPurpose::StackLimit) {
            // Total stack size is the size of all stack area used by the function, including
            // pushed CSRs, frame pointer.
            // Also, the size of a return address, implicitly pushed by a x86 `call` instruction,
            // also should be accounted for.
            // TODO: Check if the function body actually contains a `call` instruction.
            let total_stack_size = (csrs.iter(GPR).len() + 1 + 1) as i64 * word_size as i64;

            insert_stack_check(pos, total_stack_size, stack_limit_arg);
        }
    }

    let mut cfa_state = if let Some(ref mut frame_layout) = pos.func.frame_layout {
        let cfa_state = CFAState {
            cf_ptr_reg: RU::rsp as RegUnit,
            cf_ptr_offset: word_size,
            current_depth: -word_size,
        };

        frame_layout.initial = vec![
            FrameLayoutChange::CallFrameAddressAt {
                reg: cfa_state.cf_ptr_reg,
                offset: cfa_state.cf_ptr_offset,
            },
            FrameLayoutChange::ReturnAddressAt {
                cfa_offset: cfa_state.current_depth,
            },
        ]
        .into_boxed_slice();

        Some(cfa_state)
    } else {
        None
    };

    // Append param to entry EBB
    let ebb = pos.current_ebb().expect("missing ebb under cursor");
    let fp = pos.func.dfg.append_ebb_param(ebb, reg_type);
    pos.func.locations[fp] = ir::ValueLoc::Reg(RU::rbp as RegUnit);

    let push_fp_inst = pos.ins().x86_push(fp);

    if let Some(ref mut frame_layout) = pos.func.frame_layout {
        let cfa_state = cfa_state
            .as_mut()
            .expect("cfa state exists when recording frame layout");
        cfa_state.current_depth -= word_size;
        cfa_state.cf_ptr_offset += word_size;
        frame_layout.instructions.insert(
            push_fp_inst,
            vec![
                FrameLayoutChange::CallFrameAddressAt {
                    reg: cfa_state.cf_ptr_reg,
                    offset: cfa_state.cf_ptr_offset,
                },
                FrameLayoutChange::RegAt {
                    reg: RU::rbp as RegUnit,
                    cfa_offset: cfa_state.current_depth,
                },
            ]
            .into_boxed_slice(),
        );
    }

    let mov_sp_inst = pos
        .ins()
        .copy_special(RU::rsp as RegUnit, RU::rbp as RegUnit);

    if let Some(ref mut frame_layout) = pos.func.frame_layout {
        let mut cfa_state = cfa_state
            .as_mut()
            .expect("cfa state exists when recording frame layout");
        cfa_state.cf_ptr_reg = RU::rbp as RegUnit;
        frame_layout.instructions.insert(
            mov_sp_inst,
            vec![FrameLayoutChange::CallFrameAddressAt {
                reg: cfa_state.cf_ptr_reg,
                offset: cfa_state.cf_ptr_offset,
            }]
            .into_boxed_slice(),
        );
    }

    for reg in csrs.iter(GPR) {
        // Append param to entry EBB
        let csr_arg = pos.func.dfg.append_ebb_param(ebb, reg_type);

        // Assign it a location
        pos.func.locations[csr_arg] = ir::ValueLoc::Reg(reg);

        // Remember it so we can push it momentarily
        let reg_push_inst = pos.ins().x86_push(csr_arg);

        if let Some(ref mut frame_layout) = pos.func.frame_layout {
            let mut cfa_state = cfa_state
                .as_mut()
                .expect("cfa state exists when recording frame layout");
            cfa_state.current_depth -= word_size;
            frame_layout.instructions.insert(
                reg_push_inst,
                vec![FrameLayoutChange::RegAt {
                    reg,
                    cfa_offset: cfa_state.current_depth,
                }]
                .into_boxed_slice(),
            );
        }
    }

    // Allocate stack frame storage.
    if stack_size > 0 {
        if isa.flags().probestack_enabled()
            && stack_size > (1 << isa.flags().probestack_size_log2())
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
                pos.func.prologue_end = Some(pos.ins().adjust_sp_down(result));
            }
        } else {
            // Simply decrement the stack pointer.
            pos.func.prologue_end = Some(pos.ins().adjust_sp_down_imm(Imm64::new(stack_size)));
        }
    }

    cfa_state
}

/// Insert a check that generates a trap if the stack pointer goes
/// below a value in `stack_limit_arg`.
fn insert_stack_check(pos: &mut EncCursor, stack_size: i64, stack_limit_arg: ir::Value) {
    use crate::ir::condcodes::IntCC;

    // Copy `stack_limit_arg` into a %rax and use it for calculating
    // a SP threshold.
    let stack_limit_copy = pos.ins().copy(stack_limit_arg);
    pos.func.locations[stack_limit_copy] = ir::ValueLoc::Reg(RU::rax as RegUnit);
    let sp_threshold = pos.ins().iadd_imm(stack_limit_copy, stack_size);
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
    isa: &dyn TargetIsa,
    cfa_state: Option<CFAState>,
) {
    while let Some(ebb) = pos.next_ebb() {
        pos.goto_last_inst(ebb);
        if let Some(inst) = pos.current_inst() {
            if pos.func.dfg[inst].opcode().is_return() {
                if let (Some(ref mut frame_layout), ref func_layout) =
                    (pos.func.frame_layout.as_mut(), &pos.func.layout)
                {
                    // Figure out if we need to insert end-of-function-aware frame layout information.
                    let following_inst = func_layout
                        .next_ebb(ebb)
                        .and_then(|next_ebb| func_layout.first_inst(next_ebb));

                    if let Some(following_inst) = following_inst {
                        frame_layout
                            .instructions
                            .insert(inst, vec![FrameLayoutChange::Preserve].into_boxed_slice());
                        frame_layout.instructions.insert(
                            following_inst,
                            vec![FrameLayoutChange::Restore].into_boxed_slice(),
                        );
                    }
                }

                insert_common_epilogue(
                    inst,
                    stack_size,
                    pos,
                    reg_type,
                    csrs,
                    isa,
                    cfa_state.clone(),
                );
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
    isa: &dyn TargetIsa,
    mut cfa_state: Option<CFAState>,
) {
    let word_size = isa.pointer_bytes() as isize;
    if stack_size > 0 {
        pos.ins().adjust_sp_up_imm(Imm64::new(stack_size));
    }

    // Pop all the callee-saved registers, stepping backward each time to
    // preserve the correct order.
    let fp_ret = pos.ins().x86_pop(reg_type);
    let fp_pop_inst = pos.built_inst();

    if let Some(ref mut cfa_state) = cfa_state.as_mut() {
        // Account for CFA state in the reverse of `insert_common_prologue`.
        cfa_state.current_depth += word_size;
        cfa_state.cf_ptr_offset -= word_size;
        // And now that we're going to overwrite `rbp`, `rsp` is the only way to get to the call frame.
        // We don't apply a frame layout change *yet* because we check that at return the depth is
        // exactly one `word_size`.
        cfa_state.cf_ptr_reg = RU::rsp as RegUnit;
    }

    pos.prev_inst();

    pos.func.locations[fp_ret] = ir::ValueLoc::Reg(RU::rbp as RegUnit);
    pos.func.dfg.append_inst_arg(inst, fp_ret);

    for reg in csrs.iter(GPR) {
        let csr_ret = pos.ins().x86_pop(reg_type);
        if let Some(ref mut cfa_state) = cfa_state.as_mut() {
            // Note: don't bother recording a frame layout change because the popped value is
            // still correct in memory, and won't be overwritten until we've returned where the
            // current frame's layout would no longer matter. Only adjust `current_depth` for a
            // consistency check later.
            cfa_state.current_depth += word_size;
        }
        pos.prev_inst();

        pos.func.locations[csr_ret] = ir::ValueLoc::Reg(reg);
        pos.func.dfg.append_inst_arg(inst, csr_ret);
    }

    if let Some(ref mut frame_layout) = pos.func.frame_layout {
        let cfa_state = cfa_state
            .as_mut()
            .expect("cfa state exists when recording frame layout");
        // Validity checks - if we accounted correctly, CFA state at a return will match CFA state
        // at the entry of a function.
        //
        // Current_depth starts assuming a return address is pushed, and cf_ptr_offset is one
        // pointer below current_depth.
        assert_eq!(cfa_state.current_depth, -word_size);
        assert_eq!(cfa_state.cf_ptr_offset, word_size);

        let new_cfa = FrameLayoutChange::CallFrameAddressAt {
            reg: cfa_state.cf_ptr_reg,
            offset: cfa_state.cf_ptr_offset,
        };

        frame_layout
            .instructions
            .entry(fp_pop_inst)
            .and_modify(|insts| {
                *insts = insts
                    .iter()
                    .cloned()
                    .chain(std::iter::once(new_cfa))
                    .collect::<Box<[_]>>();
            })
            .or_insert_with(|| Box::new([new_cfa]));
    }
}

#[cfg(feature = "unwind")]
pub fn emit_unwind_info(
    func: &ir::Function,
    isa: &dyn TargetIsa,
    kind: FrameUnwindKind,
    sink: &mut dyn FrameUnwindSink,
) {
    match kind {
        FrameUnwindKind::Fastcall => {
            // Assumption: RBP is being used as the frame pointer
            // In the future, Windows fastcall codegen should usually omit the frame pointer
            if let Some(info) = UnwindInfo::try_from_func(func, isa, Some(RU::rbp.into())) {
                info.emit(sink);
            }
        }
        FrameUnwindKind::Libunwind => {
            if func.frame_layout.is_some() {
                emit_fde(func, isa, sink);
            }
        }
    }
}
