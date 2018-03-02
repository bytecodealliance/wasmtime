//! Intel ABI implementation.

use ir;
use isa::{RegClass, RegUnit, TargetIsa};
use regalloc::AllocatableSet;
use settings as shared_settings;
use super::registers::{GPR, FPR, RU};
use abi::{ArgAction, ValueConversion, ArgAssigner, legalize_args};
use ir::{AbiParam, ArgumentPurpose, ArgumentLoc, ArgumentExtension, CallConv, InstBuilder};
use ir::stackslot::{StackSize, StackOffset};
use ir::immediates::Imm64;
use stack_layout::layout_stack;
use std::i32;
use cursor::{Cursor, EncCursor, CursorPosition};
use result;


/// Argument registers for x86-64
static ARG_GPRS: [RU; 6] = [RU::rdi, RU::rsi, RU::rdx, RU::rcx, RU::r8, RU::r9];

/// Return value registers.
static RET_GPRS: [RU; 3] = [RU::rax, RU::rdx, RU::rcx];

struct Args {
    pointer_bytes: u32,
    pointer_bits: u16,
    pointer_type: ir::Type,
    gpr: &'static [RU],
    gpr_used: usize,
    fpr_limit: usize,
    fpr_used: usize,
    offset: u32,
    call_conv: CallConv,
}

impl Args {
    fn new(bits: u16, gpr: &'static [RU], fpr_limit: usize, call_conv: CallConv) -> Args {
        Args {
            pointer_bytes: u32::from(bits) / 8,
            pointer_bits: bits,
            pointer_type: ir::Type::int(bits).unwrap(),
            gpr,
            gpr_used: 0,
            fpr_limit,
            fpr_used: 0,
            offset: 0,
            call_conv: call_conv,
        }
    }
}

impl ArgAssigner for Args {
    fn assign(&mut self, arg: &AbiParam) -> ArgAction {
        let ty = arg.value_type;

        // Check for a legal type.
        // We don't support SIMD yet, so break all vectors down.
        if ty.is_vector() {
            return ValueConversion::VectorSplit.into();
        }

        // Large integers and booleans are broken down to fit in a register.
        if !ty.is_float() && ty.bits() > self.pointer_bits {
            return ValueConversion::IntSplit.into();
        }

        // Small integers are extended to the size of a pointer register.
        if ty.is_int() && ty.bits() < self.pointer_bits {
            match arg.extension {
                ArgumentExtension::None => {}
                ArgumentExtension::Uext => return ValueConversion::Uext(self.pointer_type).into(),
                ArgumentExtension::Sext => return ValueConversion::Sext(self.pointer_type).into(),
            }
        }

        // Handle special-purpose arguments.
        if ty.is_int() && self.call_conv == CallConv::SpiderWASM {
            match arg.purpose {
                // This is SpiderMonkey's `WasmTlsReg`.
                ArgumentPurpose::VMContext => {
                    return ArgumentLoc::Reg(if self.pointer_bits == 64 {
                        RU::r14
                    } else {
                        RU::rsi
                    } as RegUnit).into()
                }
                // This is SpiderMonkey's `WasmTableCallSigReg`.
                ArgumentPurpose::SignatureId => return ArgumentLoc::Reg(RU::rbx as RegUnit).into(),
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
        if ty.is_float() && self.fpr_used < self.fpr_limit {
            let reg = FPR.unit(self.fpr_used);
            self.fpr_used += 1;
            return ArgumentLoc::Reg(reg).into();
        }

        // Assign a stack location.
        let loc = ArgumentLoc::Stack(self.offset as i32);
        self.offset += self.pointer_bytes;
        assert!(self.offset <= i32::MAX as u32);
        loc.into()
    }
}

/// Legalize `sig`.
pub fn legalize_signature(sig: &mut ir::Signature, flags: &shared_settings::Flags, _current: bool) {
    let bits;
    let mut args;

    if flags.is_64bit() {
        bits = 64;
        args = Args::new(bits, &ARG_GPRS, 8, sig.call_conv);
    } else {
        bits = 32;
        args = Args::new(bits, &[], 0, sig.call_conv);
    }

    legalize_args(&mut sig.params, &mut args);

    let mut rets = Args::new(bits, &RET_GPRS, 2, sig.call_conv);
    legalize_args(&mut sig.returns, &mut rets);
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
pub fn allocatable_registers(
    _func: &ir::Function,
    flags: &shared_settings::Flags,
) -> AllocatableSet {
    let mut regs = AllocatableSet::new();
    regs.take(GPR, RU::rsp as RegUnit);
    regs.take(GPR, RU::rbp as RegUnit);

    // 32-bit arch only has 8 registers.
    if !flags.is_64bit() {
        for i in 8..16 {
            regs.take(GPR, GPR.unit(i));
            regs.take(FPR, FPR.unit(i));
        }
    }

    regs
}

/// Get the set of callee-saved registers.
pub fn callee_saved_registers(flags: &shared_settings::Flags) -> &'static [RU] {
    if flags.is_64bit() {
        &[RU::rbx, RU::r12, RU::r13, RU::r14, RU::r15]
    } else {
        &[RU::rbx, RU::rsi, RU::rdi]
    }
}

pub fn prologue_epilogue(func: &mut ir::Function, isa: &TargetIsa) -> result::CtonResult {
    match func.signature.call_conv {
        ir::CallConv::Native => native_prologue_epilogue(func, isa),
        ir::CallConv::SpiderWASM => spiderwasm_prologue_epilogue(func, isa),
    }
}

pub fn spiderwasm_prologue_epilogue(
    func: &mut ir::Function,
    isa: &TargetIsa,
) -> result::CtonResult {
    // Spiderwasm on 32-bit x86 always aligns its stack pointer to 16 bytes.
    let stack_align = 16;
    let word_size = if isa.flags().is_64bit() { 8 } else { 4 };
    let bytes = StackSize::from(isa.flags().spiderwasm_prologue_words()) * word_size;

    let mut ss = ir::StackSlotData::new(ir::StackSlotKind::IncomingArg, bytes);
    ss.offset = -(bytes as StackOffset);
    func.stack_slots.push(ss);

    layout_stack(&mut func.stack_slots, stack_align)?;
    Ok(())
}

/// Insert a System V-compatible prologue and epilogue.
pub fn native_prologue_epilogue(func: &mut ir::Function, isa: &TargetIsa) -> result::CtonResult {
    // The original 32-bit x86 ELF ABI had a 4-byte aligned stack pointer, but
    // newer versions use a 16-byte aligned stack pointer.
    let stack_align = 16;
    let word_size = if isa.flags().is_64bit() { 8 } else { 4 };
    let csr_type = if isa.flags().is_64bit() {
        ir::types::I64
    } else {
        ir::types::I32
    };
    let csrs = callee_saved_registers(isa.flags());

    // The reserved stack area is composed of:
    //   return address + frame pointer + all callee-saved registers
    //
    // Pushing the return address is an implicit function of the `call`
    // instruction. Each of the others we will then push explicitly. Then we
    // will adjust the stack pointer to make room for the rest of the required
    // space for this frame.
    let csr_stack_size = ((csrs.len() + 2) * word_size as usize) as i32;
    func.create_stack_slot(ir::StackSlotData {
        kind: ir::StackSlotKind::IncomingArg,
        size: csr_stack_size as u32,
        offset: -csr_stack_size,
    });

    let total_stack_size = layout_stack(&mut func.stack_slots, stack_align)? as i32;
    let local_stack_size = i64::from(total_stack_size - csr_stack_size);

    // Add CSRs to function signature
    let fp_arg = ir::AbiParam::special_reg(
        csr_type,
        ir::ArgumentPurpose::FramePointer,
        RU::rbp as RegUnit,
    );
    func.signature.params.push(fp_arg);
    func.signature.returns.push(fp_arg);

    for csr in csrs.iter() {
        let csr_arg =
            ir::AbiParam::special_reg(csr_type, ir::ArgumentPurpose::CalleeSaved, *csr as RegUnit);
        func.signature.params.push(csr_arg);
        func.signature.returns.push(csr_arg);
    }

    // Set up the cursor and insert the prologue
    let entry_ebb = func.layout.entry_block().expect("missing entry block");
    let mut pos = EncCursor::new(func, isa).at_first_insertion_point(entry_ebb);
    insert_native_prologue(&mut pos, local_stack_size, csr_type, csrs);

    // Reset the cursor and insert the epilogue
    let mut pos = pos.at_position(CursorPosition::Nowhere);
    insert_native_epilogues(&mut pos, local_stack_size, csr_type, csrs);

    Ok(())
}

/// Insert the prologue for a given function.
fn insert_native_prologue(
    pos: &mut EncCursor,
    stack_size: i64,
    csr_type: ir::types::Type,
    csrs: &'static [RU],
) {
    // Append param to entry EBB
    let ebb = pos.current_ebb().expect("missing ebb under cursor");
    let fp = pos.func.dfg.append_ebb_param(ebb, csr_type);
    pos.func.locations[fp] = ir::ValueLoc::Reg(RU::rbp as RegUnit);

    pos.ins().x86_push(fp);
    pos.ins().copy_special(
        RU::rsp as RegUnit,
        RU::rbp as RegUnit,
    );

    for reg in csrs.iter() {
        // Append param to entry EBB
        let csr_arg = pos.func.dfg.append_ebb_param(ebb, csr_type);

        // Assign it a location
        pos.func.locations[csr_arg] = ir::ValueLoc::Reg(*reg as RegUnit);

        // Remember it so we can push it momentarily
        pos.ins().x86_push(csr_arg);
    }

    if stack_size > 0 {
        pos.ins().adjust_sp_imm(Imm64::new(-stack_size));
    }
}

/// Find all `return` instructions and insert epilogues before them.
fn insert_native_epilogues(
    pos: &mut EncCursor,
    stack_size: i64,
    csr_type: ir::types::Type,
    csrs: &'static [RU],
) {
    while let Some(ebb) = pos.next_ebb() {
        pos.goto_last_inst(ebb);
        if let Some(inst) = pos.current_inst() {
            if pos.func.dfg[inst].opcode().is_return() {
                insert_native_epilogue(inst, stack_size, pos, csr_type, csrs);
            }
        }
    }
}

/// Insert an epilogue given a specific `return` instruction.
fn insert_native_epilogue(
    inst: ir::Inst,
    stack_size: i64,
    pos: &mut EncCursor,
    csr_type: ir::types::Type,
    csrs: &'static [RU],
) {
    if stack_size > 0 {
        pos.ins().adjust_sp_imm(Imm64::new(stack_size));
    }

    // Pop all the callee-saved registers, stepping backward each time to
    // preserve the correct order.
    let fp_ret = pos.ins().x86_pop(csr_type);
    pos.prev_inst();

    pos.func.locations[fp_ret] = ir::ValueLoc::Reg(RU::rbp as RegUnit);
    pos.func.dfg.append_inst_arg(inst, fp_ret);

    for reg in csrs.iter() {
        let csr_ret = pos.ins().x86_pop(csr_type);
        pos.prev_inst();

        pos.func.locations[csr_ret] = ir::ValueLoc::Reg(*reg as RegUnit);
        pos.func.dfg.append_inst_arg(inst, csr_ret);
    }
}
