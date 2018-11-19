#![allow(dead_code)] // for now

use error::Error;
use dynasmrt::x64::Assembler;
use dynasmrt::{DynasmApi, DynasmLabelApi, AssemblyOffset, ExecutableBuffer, DynamicLabel};

/// Size of a pointer on the target in bytes.
const WORD_SIZE: u32 = 8;

type GPR = u8;

struct GPRs {
    bits: u16,
}

impl GPRs {
    fn new() -> Self {
        Self { bits: 0 }
    }
}

const RAX: u8 = 0;
const RCX: u8 = 1;
const RDX: u8 = 2;
const RBX: u8 = 3;
const RSP: u8 = 4;
const RBP: u8 = 5;
const RSI: u8 = 6;
const RDI: u8 = 7;
const R8: u8 = 8;
const R9: u8 = 9;
const R10: u8 = 10;
const R11: u8 = 11;
const R12: u8 = 12;
const R13: u8 = 13;
const R14: u8 = 14;
const R15: u8 = 15;

impl GPRs {
    fn take(&mut self) -> GPR {
        let lz = self.bits.trailing_zeros();
        assert!(lz < 32, "ran out of free GPRs");
        self.bits &= !(1 << lz);
        lz as GPR
    }

    fn release(&mut self, gpr: GPR) {
        assert!(
            !self.is_free(gpr),
            "released register was already free",
        );
        self.bits |= 1 << gpr;
    }

    fn is_free(&self, gpr: GPR) -> bool {
        (self.bits & (1 << gpr)) != 0
    }
}

pub struct Registers {
    scratch_gprs: GPRs,
}

impl Registers {
    pub fn new() -> Self {
        let mut result = Self {
            scratch_gprs: GPRs::new(),
        };
        // Give ourselves a few scratch registers to work with, for now.
        result.release_scratch_gpr(RAX);
        result.release_scratch_gpr(RCX);
        result.release_scratch_gpr(RDX);
        result
    }

    pub fn take_scratch_gpr(&mut self) -> GPR {
        self.scratch_gprs.take()
    }

    pub fn release_scratch_gpr(&mut self, gpr: GPR) {
        self.scratch_gprs.release(gpr);
    }
}

/// Label in code.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Label(DynamicLabel);

/// Describes location of a argument.
enum ArgLocation {
    /// Argument is passed via some register.
    Reg(GPR),
    /// Value is passed thru the stack.
    Stack(i32),
}

/// Get a location for an argument at the given position.
fn abi_loc_for_arg(pos: u32) -> ArgLocation {
    // TODO: This assumes only system-v calling convention.
    // In system-v calling convention the first 6 arguments are passed via registers. 
    // All rest arguments are passed on the stack.
    const ARGS_IN_GPRS: &'static [GPR] = &[
        RDI,
        RSI,
        RDX,
        RCX,
        R8,
        R9,
    ];

    if let Some(&reg) = ARGS_IN_GPRS.get(pos as usize) {
        ArgLocation::Reg(reg)
    } else {
        let stack_pos = pos - ARGS_IN_GPRS.len() as u32;
        // +2 is because the first argument is located right after the saved frame pointer slot 
        // and the incoming return address.
        let stack_offset = ((stack_pos + 2) * WORD_SIZE) as i32;
        ArgLocation::Stack(stack_offset)
    }
}

pub struct CodeGenSession {
    assembler: Assembler,
    func_starts: Vec<AssemblyOffset>,
}

impl CodeGenSession {
    pub fn new() -> Self {
        CodeGenSession {
            assembler: Assembler::new().unwrap(),
            func_starts: Vec::new(),
        }
    }

    pub fn new_context(&mut self) -> Context {
        let start_offset = self.assembler.offset();
        self.func_starts.push(start_offset);
        Context {
            asm: &mut self.assembler,
            start: start_offset,
            regs: Registers::new(),
            sp_depth: 0,
        }
    }

    pub fn into_translated_code_section(self) -> Result<TranslatedCodeSection, Error>  {
        let exec_buf = self.assembler
            .finalize()
            .map_err(|_asm| Error::Assembler("assembler error".to_owned()))?;
        Ok(TranslatedCodeSection { exec_buf, func_starts: self.func_starts })
    }
}

pub struct TranslatedCodeSection {
    exec_buf: ExecutableBuffer,
    func_starts: Vec<AssemblyOffset>,
}

impl TranslatedCodeSection {
    pub fn func_start(&self, idx: usize) -> *const u8 {
        let offset = self.func_starts[idx];
        self.exec_buf.ptr(offset)
    }
}

pub struct Context<'a> {
    asm: &'a mut Assembler,
    start: AssemblyOffset,
    regs: Registers,
    /// Offset from starting value of SP counted in words. Each push and pop 
    /// on the value stack increments or decrements this value by 1 respectively.
    sp_depth: u32,
}

impl<'a> Context<'a> {
    /// Returns the offset of the first instruction.
    fn start(&self) -> AssemblyOffset {
        self.start
    }
}

/// Create a new undefined label.
pub fn create_label(ctx: &mut Context) -> Label {
    Label(ctx.asm.new_dynamic_label())
}

/// Define the given label at the current position.
/// 
/// Multiple labels can be defined at the same position. However, a label 
/// can be defined only once.
pub fn define_label(ctx: &mut Context, label: Label) {
    ctx.asm.dynamic_label(label.0);
}

fn push_i32(ctx: &mut Context, gpr: GPR) {
    // For now, do an actual push (and pop below). In the future, we could
    // do on-the-fly register allocation here.
    ctx.sp_depth += 1;
    dynasm!(ctx.asm
        ; push Rq(gpr)
    );
    ctx.regs.release_scratch_gpr(gpr);
}

fn pop_i32(ctx: &mut Context) -> GPR {
    ctx.sp_depth -= 1;
    let gpr = ctx.regs.take_scratch_gpr();
    dynasm!(ctx.asm
        ; pop Rq(gpr)
    );
    gpr
}

pub fn add_i32(ctx: &mut Context) {
    let op0 = pop_i32(ctx);
    let op1 = pop_i32(ctx);
    dynasm!(ctx.asm
        ; add Rd(op0), Rd(op1)
    );
    push_i32(ctx, op0);
    ctx.regs.release_scratch_gpr(op1);
}

fn sp_relative_offset(ctx: &mut Context, slot_idx: u32) -> i32 {
    ((ctx.sp_depth as i32) + slot_idx as i32) * WORD_SIZE as i32
}

pub fn get_local_i32(ctx: &mut Context, local_idx: u32) {
    let gpr = ctx.regs.take_scratch_gpr();
    let offset = sp_relative_offset(ctx, local_idx);
    dynasm!(ctx.asm
        ; mov Rq(gpr), [rsp + offset]
    );
    push_i32(ctx, gpr);
}

pub fn store_i32(ctx: &mut Context, local_idx: u32) {
    let gpr = pop_i32(ctx);
    let offset = sp_relative_offset(ctx, local_idx);
    dynasm!(ctx.asm
        ; mov [rsp + offset], Rq(gpr)
    );
    ctx.regs.release_scratch_gpr(gpr);
}

pub fn relop_eq_i32(ctx: &mut Context) {
    let right = pop_i32(ctx);
    let left = pop_i32(ctx);
    let result = ctx.regs.take_scratch_gpr();
    dynasm!(ctx.asm
        ; xor Rq(result), Rq(result)
        ; cmp Rd(left), Rd(right)
        ; sete Rb(result)
    );
    push_i32(ctx, result);
    ctx.regs.release_scratch_gpr(left);
    ctx.regs.release_scratch_gpr(right);
}

pub fn prepare_return_value(ctx: &mut Context) {
    let ret_gpr = pop_i32(ctx);
    if ret_gpr != RAX {
        dynasm!(ctx.asm
            ; mov Rq(RAX), Rq(ret_gpr)
        );
        ctx.regs.release_scratch_gpr(ret_gpr);
    }
}

pub fn copy_incoming_arg(ctx: &mut Context, arg_pos: u32) {
    let loc = abi_loc_for_arg(arg_pos);

    // First, ensure the argument is in a register.
    let reg = match loc {
        ArgLocation::Reg(reg) => reg,
        ArgLocation::Stack(offset) => {
            assert!(
                ctx.regs.scratch_gprs.is_free(RAX),
                "we assume that RAX can be used as a scratch register for now",
            );
            dynasm!(ctx.asm
                ; mov Rq(RAX), [rsp + offset]
            );
            RAX
        }
    };

    // And then move a value from a register into local variable area on the stack.
    let offset = sp_relative_offset(ctx, arg_pos);
    dynasm!(ctx.asm
        ; mov [rsp + offset], Rq(reg) 
    );
}

pub fn prologue(ctx: &mut Context, stack_slots: u32) {
    // Align stack slots to the nearest even number. This is required
    // by x86-64 ABI.
    let aligned_stack_slots = (stack_slots + 1) & !1;

    let framesize: i32 = aligned_stack_slots as i32 * WORD_SIZE as i32;
    dynasm!(ctx.asm
        ; push rbp
        ; mov rbp, rsp
        ; sub rsp, framesize
    );
    ctx.sp_depth += aligned_stack_slots - stack_slots;
}

pub fn epilogue(ctx: &mut Context) {
    assert_eq!(ctx.sp_depth, 0, "imbalanced pushes and pops detected");
    dynasm!(ctx.asm
        ; mov rsp, rbp
        ; pop rbp
        ; ret
    );
}

pub fn trap(ctx: &mut Context) {
    dynasm!(ctx.asm
        ; ud2
    );
}
