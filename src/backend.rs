#![allow(dead_code)] // for now

use dynasmrt::x64::Assembler;
use dynasmrt::{AssemblyOffset, DynamicLabel, DynasmApi, DynasmLabelApi, ExecutableBuffer};
use error::Error;
use std::iter;

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
        assert!(!self.is_free(gpr), "released register was already free",);
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

/// Describes location of a argument.
#[derive(Debug)]
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
    const ARGS_IN_GPRS: &'static [GPR] = &[RDI, RSI, RDX, RCX, R8, R9];

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

/// Records data about the function.
struct FuncDef {
    /// Offset to the start of the function. None, until the exact offset is known.
    ///
    /// Used to calculate the address for calling this function.
    /// TODO: This field will not be needed if dynasm gain ability to return `AssemblyOffset` for the
    /// defined labels.
    offset: Option<AssemblyOffset>,
    /// Dynamic label can be used to designate target of calls
    /// before knowning the actual address of the function.
    label: DynamicLabel,
}

impl FuncDef {
    fn new(asm: &mut Assembler) -> FuncDef {
        FuncDef {
            offset: None,
            label: asm.new_dynamic_label(),
        }
    }
}

pub struct CodeGenSession {
    assembler: Assembler,
    func_defs: Vec<FuncDef>,
}

impl CodeGenSession {
    pub fn new(func_count: u32) -> Self {
        let mut assembler = Assembler::new().unwrap();
        let func_defs = iter::repeat_with(|| FuncDef::new(&mut assembler))
            .take(func_count as usize)
            .collect::<Vec<_>>();

        CodeGenSession {
            assembler,
            func_defs,
        }
    }

    pub fn new_context(&mut self, func_idx: u32) -> Context {
        {
            let func_start = &mut self.func_defs[func_idx as usize];

            // At this point we know the exact start address of this function. Save it
            // and define dynamic label at this location.
            func_start.offset = Some(self.assembler.offset());
            self.assembler.dynamic_label(func_start.label);
        }

        Context {
            asm: &mut self.assembler,
            func_defs: &self.func_defs,
            regs: Registers::new(),
            sp_depth: StackDepth(0),
        }
    }

    pub fn into_translated_code_section(self) -> Result<TranslatedCodeSection, Error> {
        let exec_buf = self
            .assembler
            .finalize()
            .map_err(|_asm| Error::Assembler("assembler error".to_owned()))?;
        let func_defs = self
            .func_defs
            .iter()
            .map(|FuncDef { offset, .. }| offset.unwrap())
            .collect::<Vec<_>>();
        Ok(TranslatedCodeSection {
            exec_buf,
            func_defs,
        })
    }
}

pub struct TranslatedCodeSection {
    exec_buf: ExecutableBuffer,
    func_defs: Vec<AssemblyOffset>,
}

impl TranslatedCodeSection {
    pub fn func_start(&self, idx: usize) -> *const u8 {
        let offset = self.func_defs[idx];
        self.exec_buf.ptr(offset)
    }
}

pub struct Context<'a> {
    asm: &'a mut Assembler,
    func_defs: &'a Vec<FuncDef>,
    regs: Registers,
    /// Each push and pop on the value stack increments or decrements this value by 1 respectively.
    sp_depth: StackDepth,
}

/// Label in code.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Label(DynamicLabel);

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

/// Offset from starting value of SP counted in words.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct StackDepth(u32);

impl StackDepth {
    pub fn reserve(&mut self, slots: u32) {
        self.0 += slots;
    }

    pub fn free(&mut self, slots: u32) {
        self.0 -= slots;
    }
}

pub fn current_stack_depth(ctx: &Context) -> StackDepth {
    ctx.sp_depth
}

pub fn restore_stack_depth(ctx: &mut Context, stack_depth: StackDepth) {
    ctx.sp_depth = stack_depth;
}

fn push_i32(ctx: &mut Context, gpr: GPR) {
    // For now, do an actual push (and pop below). In the future, we could
    // do on-the-fly register allocation here.
    ctx.sp_depth.reserve(1);
    dynasm!(ctx.asm
        ; push Rq(gpr)
    );
    ctx.regs.release_scratch_gpr(gpr);
}

fn pop_i32(ctx: &mut Context) -> GPR {
    ctx.sp_depth.free(1);
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
    ((ctx.sp_depth.0 as i32) + slot_idx as i32) * WORD_SIZE as i32
}

pub fn get_local_i32(ctx: &mut Context, local_idx: u32) {
    let gpr = ctx.regs.take_scratch_gpr();
    let offset = sp_relative_offset(ctx, local_idx);
    dynasm!(ctx.asm
        ; mov Rq(gpr), [rsp + offset]
    );
    push_i32(ctx, gpr);
}

pub fn set_local_i32(ctx: &mut Context, local_idx: u32) {
    let gpr = pop_i32(ctx);
    let offset = sp_relative_offset(ctx, local_idx);
    dynasm!(ctx.asm
        ; mov [rsp + offset], Rq(gpr)
    );
    ctx.regs.release_scratch_gpr(gpr);
}

pub fn literal_i32(ctx: &mut Context, imm: i32) {
    let gpr = ctx.regs.take_scratch_gpr();
    dynasm!(ctx.asm
        ; mov Rd(gpr), imm
    );
    push_i32(ctx, gpr);
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

/// Pops i32 predicate and branches to the specified label
/// if the predicate is equal to zero.
pub fn pop_and_breq(ctx: &mut Context, label: Label) {
    let predicate = pop_i32(ctx);
    dynasm!(ctx.asm
        ; test Rd(predicate), Rd(predicate)
        ; je =>label.0
    );
    ctx.regs.release_scratch_gpr(predicate);
}

/// Branch unconditionally to the specified label.
pub fn br(ctx: &mut Context, label: Label) {
    dynasm!(ctx.asm
        ; jmp =>label.0
    );
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

pub fn pass_outgoing_args(ctx: &mut Context, arity: u32) {
    for arg_pos in (0..arity).rev() {
        ctx.sp_depth.free(1);

        let loc = abi_loc_for_arg(arg_pos);
        match loc {
            ArgLocation::Reg(gpr) => {
                dynasm!(ctx.asm
                    ; pop Rq(gpr)
                );
            }
            _ => unimplemented!("don't know how to pass argument {} via {:?}", arg_pos, loc),
        }
    }
}

pub fn call_direct(ctx: &mut Context, index: u32, return_arity: u32) {
    assert!(return_arity == 0 || return_arity == 1);

    let label = &ctx.func_defs[index as usize].label;
    dynasm!(ctx.asm
        ; call =>*label
    );

    if return_arity == 1 {
        dynasm!(ctx.asm
            ; push rax
        );
        ctx.sp_depth.reserve(1);
    }
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
    ctx.sp_depth.reserve(aligned_stack_slots - stack_slots);
}

pub fn epilogue(ctx: &mut Context) {
    // TODO: This doesn't work with stack alignment.
    // assert_eq!(
    //     ctx.sp_depth,
    //     StackDepth(0),
    //     "imbalanced pushes and pops detected"
    // );
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
