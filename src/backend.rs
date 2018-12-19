#![allow(dead_code)] // for now

// Since we want this to be linear-time, we never want to iterate over a `Vec`. `ArrayVec`s have a hard,
// small maximum size and so we can consider iterating over them to be essentially constant-time.
use arrayvec::ArrayVec;

use dynasmrt::x64::Assembler;
use dynasmrt::{AssemblyOffset, DynamicLabel, DynasmApi, DynasmLabelApi, ExecutableBuffer};
use error::Error;
use std::iter;

/// Size of a pointer on the target in bytes.
const WORD_SIZE: u32 = 8;

type GPR = u8;

#[derive(Debug, Copy, Clone)]
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
const NUM_GPRS: u8 = 16;

impl GPRs {
    fn take(&mut self) -> GPR {
        let lz = self.bits.trailing_zeros();
        assert!(lz < 16, "ran out of free GPRs");
        let gpr = lz as GPR;
        self.mark_used(gpr);
        gpr
    }

    fn mark_used(&mut self, gpr: GPR) {
        self.bits &= !(1 << gpr as u16);
    }

    fn release(&mut self, gpr: GPR) {
        assert!(!self.is_free(gpr), "released register was already free",);
        self.bits |= 1 << gpr;
    }

    fn free_count(&self) -> u32 {
        self.bits.count_ones()
    }

    fn is_free(&self, gpr: GPR) -> bool {
        (self.bits & (1 << gpr)) != 0
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Registers {
    scratch: GPRs,
}

impl Default for Registers {
    fn default() -> Self {
        Self::new()
    }
}

impl Registers {
    pub fn new() -> Self {
        let mut result = Self {
            scratch: GPRs::new(),
        };
        // Give ourselves a few scratch registers to work with, for now.
        for &scratch in SCRATCH_REGS {
            result.release_scratch_gpr(scratch);
        }

        result
    }

    pub fn mark_used(&mut self, gpr: GPR) {
        self.scratch.mark_used(gpr);
    }

    // TODO: Add function that takes a scratch register if possible
    //       but otherwise gives a fresh stack location.
    pub fn take_scratch_gpr(&mut self) -> GPR {
        self.scratch.take()
    }

    pub fn release_scratch_gpr(&mut self, gpr: GPR) {
        self.scratch.release(gpr);
    }

    pub fn is_free(&self, gpr: GPR) -> bool {
        self.scratch.is_free(gpr)
    }

    pub fn free_scratch(&self) -> u32 {
        self.scratch.free_count()
    }
}

// TODO: Allow pushing condition codes to stack? We'd have to immediately
//       materialise them into a register if anything is pushed above them.
/// Describes location of a value.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum ValueLocation {
    /// Value exists in a register.
    Reg(GPR),
    /// Value exists on the stack. This is an offset relative to the
    /// first local, and so will have to be adjusted with `adjusted_offset`
    /// before reading (as RSP may have been changed by `push`/`pop`).
    Stack(i32),
    /// Value is a literal (TODO: Support more than just `i32`)
    Immediate(i32),
}

// TODO: This assumes only system-v calling convention.
// In system-v calling convention the first 6 arguments are passed via registers.
// All rest arguments are passed on the stack.
const ARGS_IN_GPRS: &[GPR] = &[RDI, RSI, RDX, RCX, R8, R9];
// RAX is reserved for return values. In the future we want a system to allow
// use of specific registers by saving/restoring them. This would allow using
// RAX as a scratch register when we're not calling a function, and would also
// allow us to call instructions that require specific registers.
//
// List of scratch registers taken from https://wiki.osdev.org/System_V_ABI
const SCRATCH_REGS: &[GPR] = &[RAX, R10, R11];

pub struct CodeGenSession {
    assembler: Assembler,
    func_starts: Vec<(Option<AssemblyOffset>, DynamicLabel)>,
}

impl CodeGenSession {
    pub fn new(func_count: u32) -> Self {
        let mut assembler = Assembler::new().unwrap();
        let func_starts = iter::repeat_with(|| (None, assembler.new_dynamic_label()))
            .take(func_count as usize)
            .collect::<Vec<_>>();

        CodeGenSession {
            assembler,
            func_starts,
        }
    }

    pub fn new_context(&mut self, func_idx: u32) -> Context {
        {
            let func_start = &mut self.func_starts[func_idx as usize];

            // At this point we now the exact start address of this function. Save it
            // and define dynamic label at this location.
            func_start.0 = Some(self.assembler.offset());
            self.assembler.dynamic_label(func_start.1);
        }

        Context {
            asm: &mut self.assembler,
            func_starts: &self.func_starts,
            block_state: Default::default(),
        }
    }

    pub fn into_translated_code_section(self) -> Result<TranslatedCodeSection, Error> {
        let exec_buf = self
            .assembler
            .finalize()
            .map_err(|_asm| Error::Assembler("assembler error".to_owned()))?;
        let func_starts = self
            .func_starts
            .iter()
            .map(|(offset, _)| offset.unwrap())
            .collect::<Vec<_>>();
        Ok(TranslatedCodeSection {
            exec_buf,
            func_starts,
        })
    }
}

#[derive(Debug)]
pub struct TranslatedCodeSection {
    exec_buf: ExecutableBuffer,
    func_starts: Vec<AssemblyOffset>,
}

impl TranslatedCodeSection {
    pub fn func_start(&self, idx: usize) -> *const u8 {
        let offset = self.func_starts[idx];
        self.exec_buf.ptr(offset)
    }

    pub fn disassemble(&self) {
        ::disassemble::disassemble(&*self.exec_buf).unwrap();
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum Value {
    Local(u32),
    Temp(GPR),
    Immediate(i32),
}

impl Value {
    fn immediate(&self) -> Option<i32> {
        match *self {
            Value::Immediate(i) => Some(i),
            _ => None,
        }
    }

    fn location(&self, locals: &Locals) -> ValueLocation {
        match *self {
            Value::Local(loc) => locals.get(loc),
            Value::Temp(reg) => ValueLocation::Reg(reg),
            Value::Immediate(reg) => ValueLocation::Immediate(reg),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum StackValue {
    Local(u32),
    Temp(GPR),
    Immediate(i32),
    Pop,
}

impl StackValue {
    fn location(&self, locals: &Locals) -> Option<ValueLocation> {
        match *self {
            StackValue::Local(loc) => Some(locals.get(loc)),
            StackValue::Immediate(i) => Some(ValueLocation::Immediate(i)),
            StackValue::Temp(reg) => Some(ValueLocation::Reg(reg)),
            StackValue::Pop => None,
        }
    }
}

#[derive(Debug, Default, Clone)]
struct Locals {
    // TODO: Store all places that the value can be read, so we can optimise
    //       passing (register) arguments along into a noop after saving their
    //       values.
    register_arguments: ArrayVec<[ValueLocation; ARGS_IN_GPRS.len()]>,
    num_stack_args: u32,
    num_local_stack_slots: u32,
}

impl Locals {
    fn get(&self, index: u32) -> ValueLocation {
        self.register_arguments
            .get(index as usize)
            .cloned()
            .unwrap_or_else(|| {
                let stack_index = index - self.register_arguments.len() as u32;
                if stack_index < self.num_stack_args {
                    ValueLocation::Stack(
                        ((stack_index + self.num_local_stack_slots + 2) * WORD_SIZE) as _,
                    )
                } else {
                    let stack_index = stack_index - self.num_stack_args;
                    ValueLocation::Stack((stack_index * WORD_SIZE) as _)
                }
            })
    }
}

#[derive(Debug, Default, Clone)]
pub struct BlockState {
    stack: Stack,
    // TODO: `BitVec`
    stack_map: Vec<bool>,
    depth: StackDepth,
    return_register: Option<GPR>,
    regs: Registers,
    /// This is the _current_ locals, since we can shuffle them about during function calls.
    /// We will restore this to be the same state as the `Locals` in `Context` at the end
    /// of a block.
    locals: Locals,
    parent_locals: Locals,
}

fn adjusted_offset(ctx: &mut Context, offset: i32) -> i32 {
    (ctx.block_state.depth.0 * WORD_SIZE) as i32 + offset
}

type Stack = Vec<StackValue>;

pub struct Context<'a> {
    asm: &'a mut Assembler,
    func_starts: &'a Vec<(Option<AssemblyOffset>, DynamicLabel)>,
    /// Each push and pop on the value stack increments or decrements this value by 1 respectively.
    block_state: BlockState,
}

impl<'a> Context<'a> {}

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
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct StackDepth(u32);

impl StackDepth {
    pub fn reserve(&mut self, slots: u32) {
        self.0 += slots;
    }

    pub fn free(&mut self, slots: u32) {
        self.0 -= slots;
    }
}

fn expand_stack(ctx: &mut Context, by: u32) {
    use std::iter;

    if by == 0 {
        return;
    }

    let new_stack_size = (ctx.block_state.stack_map.len() + by as usize).next_power_of_two();
    let additional_elements = new_stack_size - ctx.block_state.stack_map.len();
    ctx.block_state
        .stack_map
        .extend(iter::repeat(false).take(additional_elements));

    dynasm!(ctx.asm
        ; sub rsp, additional_elements as i32
    );
}

// TODO: Make this generic over `Vec` or `ArrayVec`?
fn stack_slots(ctx: &mut Context, count: u32) -> Vec<i32> {
    let mut out = Vec::with_capacity(count as usize);

    let offset_if_taken = |(i, is_taken): (usize, bool)| {
        if !is_taken {
            Some(i as i32 * WORD_SIZE as i32)
        } else {
            None
        }
    };

    out.extend(
        ctx.block_state
            .stack_map
            .iter()
            .cloned()
            .enumerate()
            .filter_map(offset_if_taken),
    );

    let remaining = count as usize - out.len();

    if remaining > 0 {
        expand_stack(ctx, remaining as u32);
        out.extend(
            ctx.block_state
                .stack_map
                .iter()
                .cloned()
                .enumerate()
                .filter_map(offset_if_taken),
        );
    }

    out
}

fn stack_slot(ctx: &mut Context) -> i32 {
    if let Some(pos) = ctx
        .block_state
        .stack_map
        .iter()
        .position(|is_taken| !is_taken)
    {
        ctx.block_state.stack_map[pos] = true;
        pos as i32 * WORD_SIZE as i32
    } else {
        expand_stack(ctx, 1);
        stack_slot(ctx)
    }
}

// We use `put` instead of `pop` since with `BrIf` it's possible
// that the block will continue after returning.
pub fn return_from_block(ctx: &mut Context, arity: u32, is_function_end: bool) {
    // This should just be an optimisation, passing `false` should always result
    // in correct code.
    if !is_function_end {
        restore_locals(ctx);
    }

    if arity == 0 {
        return;
    }

    let stack_top = *ctx.block_state.stack.last().expect("Stack is empty");
    if let Some(reg) = ctx.block_state.return_register {
        put_stack_val_into(ctx, stack_top, ValueLocation::Reg(reg));
    } else {
        let out_reg = match stack_top {
            StackValue::Temp(r) => r,
            other => {
                let new_scratch = ctx.block_state.regs.take_scratch_gpr();
                put_stack_val_into(ctx, other, ValueLocation::Reg(new_scratch));
                new_scratch
            }
        };

        ctx.block_state.return_register = Some(out_reg);
    }
}

pub fn start_block(ctx: &mut Context) -> BlockState {
    // free_return_register(ctx, arity);
    let current_state = ctx.block_state.clone();
    ctx.block_state.parent_locals = ctx.block_state.locals.clone();
    ctx.block_state.return_register = None;
    current_state
}

// To start the next subblock of a block (for `if..then..else..end`).
// The only difference is that choices we made in the first subblock
// (for now only the return register) must be maintained in the next
// subblocks.
pub fn reset_block(ctx: &mut Context, parent_block_state: BlockState) {
    let return_reg = ctx.block_state.return_register;

    ctx.block_state = parent_block_state;

    ctx.block_state.return_register = return_reg;
}

pub fn end_block(ctx: &mut Context, parent_block_state: BlockState) {
    // TODO: This is currently never called, but is important for if we want to
    //       have a more complex stack spilling scheme.
    if ctx.block_state.depth != parent_block_state.depth {
        dynasm!(ctx.asm
            ; add rsp, (ctx.block_state.depth.0 - parent_block_state.depth.0) as i32
        );
    }

    let return_reg = ctx.block_state.return_register;
    ctx.block_state = parent_block_state;

    if let Some(reg) = return_reg {
        ctx.block_state.regs.mark_used(reg);
        ctx.block_state.stack.push(StackValue::Temp(reg));
    }
}

// TODO: We should be able to have arbitrary return registers. For blocks with multiple
//       return points we can just choose the first one that we encounter and then always
//       use that one. This will mean that `(block ...)` is no less efficient than `...`
//       alone, and you only pay for the shuffling of registers in the case that you use
//       `BrIf` or similar.
fn push_return_value(ctx: &mut Context, arity: u32) {
    if arity == 0 {
        return;
    }
    assert_eq!(arity, 1);
    ctx.block_state.regs.mark_used(RAX);
    ctx.block_state.stack.push(StackValue::Temp(RAX));
}

fn restore_locals(ctx: &mut Context) {
    for (src, dst) in ctx
        .block_state
        .locals
        .register_arguments
        .clone()
        .iter()
        .zip(&ctx.block_state.parent_locals.register_arguments.clone())
    {
        copy_value(ctx, *src, *dst);
    }
}

fn push_i32(ctx: &mut Context, value: Value) {
    let stack_loc = match value {
        Value::Local(loc) => StackValue::Local(loc),
        Value::Immediate(i) => StackValue::Immediate(i),
        Value::Temp(gpr) => {
            if ctx.block_state.regs.free_scratch() >= 1 {
                StackValue::Temp(gpr)
            } else {
                ctx.block_state.depth.reserve(1);
                // TODO: Proper stack allocation scheme
                dynasm!(ctx.asm
                    ; push Rq(gpr)
                );
                ctx.block_state.regs.release_scratch_gpr(gpr);
                StackValue::Pop
            }
        }
    };

    ctx.block_state.stack.push(stack_loc);
}

fn pop_i32(ctx: &mut Context) -> Value {
    match ctx.block_state.stack.pop().expect("Stack is empty") {
        StackValue::Local(loc) => Value::Local(loc),
        StackValue::Immediate(i) => Value::Immediate(i),
        StackValue::Temp(reg) => Value::Temp(reg),
        StackValue::Pop => {
            ctx.block_state.depth.free(1);
            let gpr = ctx.block_state.regs.take_scratch_gpr();
            dynasm!(ctx.asm
                ; pop Rq(gpr)
            );
            Value::Temp(gpr)
        }
    }
}

/// Warning: this _will_ pop the runtime stack, but will _not_ pop the compile-time
///          stack. It's specifically for mid-block breaks like `Br` and `BrIf`.
fn put_stack_val_into(ctx: &mut Context, val: StackValue, dst: ValueLocation) {
    let to_move = match val {
        StackValue::Local(loc) => Value::Local(loc),
        StackValue::Immediate(i) => Value::Immediate(i),
        StackValue::Temp(reg) => Value::Temp(reg),
        StackValue::Pop => {
            ctx.block_state.depth.free(1);
            match dst {
                ValueLocation::Reg(r) => dynasm!(ctx.asm
                    ; pop Rq(r)
                ),
                ValueLocation::Stack(offset) => {
                    let offset = adjusted_offset(ctx, offset);
                    dynasm!(ctx.asm
                        ; pop QWORD [rsp + offset]
                    )
                }
                ValueLocation::Immediate(_) => panic!("Tried to write to literal!"),
            }

            // DO NOT DO A `copy_val`
            return;
        }
    };

    let src = to_move.location(&ctx.block_state.locals);
    copy_value(ctx, src, dst);
    if src != dst {
        free_value(ctx, to_move);
    }
}

pub fn drop(ctx: &mut Context) {
    match ctx.block_state.stack.pop().expect("Stack is empty") {
        StackValue::Pop => {
            dynasm!(ctx.asm
            ; add rsp, WORD_SIZE as i32
            );
        }
        StackValue::Temp(gpr) => free_value(ctx, Value::Temp(gpr)),
        _ => {}
    }
}

fn pop_i32_into(ctx: &mut Context, dst: ValueLocation) {
    let val = ctx.block_state.stack.pop().expect("Stack is empty");
    put_stack_val_into(ctx, val, dst);
}

fn free_value(ctx: &mut Context, val: Value) {
    match val {
        Value::Temp(reg) => ctx.block_state.regs.release_scratch_gpr(reg),
        Value::Local(_) | Value::Immediate(_) => {}
    }
}

/// Puts this value into a register so that it can be efficiently read
fn into_reg(ctx: &mut Context, val: Value) -> GPR {
    match val.location(&ctx.block_state.locals) {
        ValueLocation::Stack(offset) => {
            let offset = adjusted_offset(ctx, offset);
            let scratch = ctx.block_state.regs.take_scratch_gpr();
            dynasm!(ctx.asm
                ; mov Rq(scratch), [rsp + offset]
            );
            scratch
        }
        ValueLocation::Immediate(i) => {
            let scratch = ctx.block_state.regs.take_scratch_gpr();
            dynasm!(ctx.asm
                ; mov Rq(scratch), i
            );
            scratch
        }
        ValueLocation::Reg(reg) => reg,
    }
}

/// Puts this value into a temporary register so that operations
/// on that register don't write to a local.
fn into_temp_reg(ctx: &mut Context, val: Value) -> GPR {
    match val {
        Value::Local(loc) => {
            let scratch = ctx.block_state.regs.take_scratch_gpr();

            match ctx.block_state.locals.get(loc) {
                ValueLocation::Stack(offset) => {
                    let offset = adjusted_offset(ctx, offset);
                    dynasm!(ctx.asm
                        ; mov Rq(scratch), [rsp + offset]
                    );
                }
                ValueLocation::Reg(reg) => {
                    dynasm!(ctx.asm
                        ; mov Rq(scratch), Rq(reg)
                    );
                }
                ValueLocation::Immediate(_) => {
                    panic!("We shouldn't be storing immediates in locals for now")
                }
            }

            scratch
        }
        Value::Immediate(i) => {
            let scratch = ctx.block_state.regs.take_scratch_gpr();

            dynasm!(ctx.asm
                ; mov Rq(scratch), i
            );

            scratch
        }
        Value::Temp(reg) => reg,
    }
}

macro_rules! commutative_binop {
    ($name:ident, $instr:ident, $const_fallback:expr) => {
        pub fn $name(ctx: &mut Context) {
            let op0 = pop_i32(ctx);
            let op1 = pop_i32(ctx);

            if let Some(i1) = op1.immediate() {
                if let Some(i0) = op0.immediate() {
                    ctx.block_state.stack.push(StackValue::Immediate($const_fallback(i1, i0)));
                    return;
                }
            }

            let (op1, op0) = match op1 {
                Value::Temp(reg) => (reg, op0),
                _ => if op0.immediate().is_some() {
                    (into_temp_reg(ctx, op1), op0)
                } else {
                    (into_temp_reg(ctx, op0), op1)
                }
            };

            match op0.location(&ctx.block_state.locals) {
                ValueLocation::Reg(reg) => {
                    dynasm!(ctx.asm
                        ; $instr Rd(op1), Rd(reg)
                    );
                }
                ValueLocation::Stack(offset) => {
                    let offset = adjusted_offset(ctx, offset);
                    dynasm!(ctx.asm
                        ; $instr Rd(op1), [rsp + offset]
                    );
                }
                ValueLocation::Immediate(i) => {
                    dynasm!(ctx.asm
                        ; $instr Rd(op1), i
                    );
                }
            }

            ctx.block_state.stack.push(StackValue::Temp(op1));
            free_value(ctx, op0);
        }
    }
}

commutative_binop!(i32_add, add, i32::wrapping_add);
commutative_binop!(i32_and, and, |a, b| a & b);
commutative_binop!(i32_or, or, |a, b| a | b);
commutative_binop!(i32_xor, xor, |a, b| a ^ b);

// `i32_mul` needs to be seperate because the immediate form of the instruction
// has a different syntax to the immediate form of the other instructions.
pub fn i32_mul(ctx: &mut Context) {
    let op0 = pop_i32(ctx);
    let op1 = pop_i32(ctx);

    if let Some(i1) = op1.immediate() {
        if let Some(i0) = op0.immediate() {
            ctx.block_state
                .stack
                .push(StackValue::Immediate(i32::wrapping_mul(i1, i0)));
            return;
        }
    }

    let (op1, op0) = match op1 {
        Value::Temp(reg) => (reg, op0),
        _ => {
            if op0.immediate().is_some() {
                (into_temp_reg(ctx, op1), op0)
            } else {
                (into_temp_reg(ctx, op0), op1)
            }
        }
    };

    match op0.location(&ctx.block_state.locals) {
        ValueLocation::Reg(reg) => {
            dynasm!(ctx.asm
                ; imul Rd(op1), Rd(reg)
            );
        }
        ValueLocation::Stack(offset) => {
            let offset = adjusted_offset(ctx, offset);
            dynasm!(ctx.asm
                ; imul Rd(op1), [rsp + offset]
            );
        }
        ValueLocation::Immediate(i) => {
            dynasm!(ctx.asm
                ; imul Rd(op1), Rd(op1), i
            );
        }
    }

    ctx.block_state.stack.push(StackValue::Temp(op1));
    free_value(ctx, op0);
}

// `sub` is not commutative, so we have to handle it differently (we _must_ use the `op1`
// temp register as the output)
pub fn i32_sub(ctx: &mut Context) {
    let op0 = pop_i32(ctx);
    let op1 = pop_i32(ctx);

    if let Some(i1) = op1.immediate() {
        if let Some(i0) = op0.immediate() {
            ctx.block_state.stack.push(StackValue::Immediate(i1 - i0));
            return;
        }
    }

    let op1 = into_temp_reg(ctx, op1);
    match op0.location(&ctx.block_state.locals) {
        ValueLocation::Reg(reg) => {
            dynasm!(ctx.asm
                ; sub Rd(op1), Rd(reg)
            );
        }
        ValueLocation::Stack(offset) => {
            let offset = adjusted_offset(ctx, offset);
            dynasm!(ctx.asm
                ; sub Rd(op1), [rsp + offset]
            );
        }
        ValueLocation::Immediate(i) => {
            dynasm!(ctx.asm
                ; sub Rd(op1), i
            );
        }
    }

    ctx.block_state.stack.push(StackValue::Temp(op1));
    free_value(ctx, op0);
}

pub fn get_local_i32(ctx: &mut Context, local_idx: u32) {
    push_i32(ctx, Value::Local(local_idx));
}

// TODO: We can put locals that were spilled to the stack
//       back into registers here.
pub fn set_local_i32(ctx: &mut Context, local_idx: u32) {
    let val = pop_i32(ctx);
    let val_loc = val.location(&ctx.block_state.locals);
    let dst_loc = ctx.block_state.parent_locals.get(local_idx);

    materialize_local(ctx, local_idx);

    if let Some(cur) = ctx
        .block_state
        .locals
        .register_arguments
        .get_mut(local_idx as usize)
    {
        *cur = dst_loc;
    }

    copy_value(ctx, val_loc, dst_loc);
    free_value(ctx, val);
}

fn materialize_local(ctx: &mut Context, local_idx: u32) {
    // TODO: With real stack allocation we can make this constant-time. We can have a kind of
    //       on-the-fly SSA transformation where we mark each `StackValue::Local` with an ID
    //       that increases with each assignment (this can be stored in block state and so
    //       is reset when the block ends). We then refcount the storage associated with each
    //       "value ID" and in `pop` we free up slots whose refcount hits 0. This means we
    //       can have even cleaner assembly than we currently do while giving us back
    //       linear runtime.
    for index in (0..ctx.block_state.stack.len()).rev() {
        match ctx.block_state.stack[index] {
            // For now it's impossible for a local to be in RAX but that might be
            // possible in the future, so we check both cases.
            StackValue::Local(i) if i == local_idx => {
                ctx.block_state.depth.reserve(1);
                ctx.block_state.stack[index] = StackValue::Pop;
                match ctx.block_state.locals.get(local_idx) {
                    ValueLocation::Reg(r) => dynasm!(ctx.asm
                        ; push Rq(r)
                    ),
                    ValueLocation::Stack(offset) => {
                        let offset = adjusted_offset(ctx, offset);
                        dynasm!(ctx.asm
                            ; push QWORD [rsp + offset]
                        )
                    }
                    _ => unreachable!(),
                }
            }
            StackValue::Pop => {
                // We don't need to fail if the `Pop` is lower in the stack than the last instance of this
                // local, but we might as well fail for now since we want to reimplement this using proper
                // stack allocation anyway.
                panic!("Tried to materialize local but the stack already contains elements");
            }
            _ => {}
        }
    }
}

pub fn literal_i32(ctx: &mut Context, imm: i32) {
    push_i32(ctx, Value::Immediate(imm));
}

macro_rules! cmp {
    ($name:ident, $instr:ident, $const_fallback:expr) => {
        pub fn $name(ctx: &mut Context) {
            let right = pop_i32(ctx);
            let left = pop_i32(ctx);

            let out = if let Some(i) = left.immediate() {
                match right.location(&ctx.block_state.locals) {
                    ValueLocation::Stack(offset) => {
                        let result = ctx.block_state.regs.take_scratch_gpr();
                        let offset = adjusted_offset(ctx, offset);
                        dynasm!(ctx.asm
                            ; xor Rq(result), Rq(result)
                            ; cmp DWORD [rsp + offset], i
                            ; $instr Rb(result)
                        );
                        Value::Temp(result)
                    }
                    ValueLocation::Reg(rreg) => {
                        let result = ctx.block_state.regs.take_scratch_gpr();
                        dynasm!(ctx.asm
                            ; xor Rq(result), Rq(result)
                            ; cmp Rd(rreg), i
                            ; $instr Rb(result)
                        );
                        Value::Temp(result)
                    }
                    ValueLocation::Immediate(right) => {
                        Value::Immediate(if $const_fallback(i, right) { 1 } else { 0 })
                    }
                }
            } else {
                let lreg = into_reg(ctx, left);
                let result = ctx.block_state.regs.take_scratch_gpr();

                match right.location(&ctx.block_state.locals) {
                    ValueLocation::Stack(offset) => {
                        let offset = adjusted_offset(ctx, offset);
                        dynasm!(ctx.asm
                            ; xor Rq(result), Rq(result)
                            ; cmp Rd(lreg), [rsp + offset]
                            ; $instr Rb(result)
                        );
                    }
                    ValueLocation::Reg(rreg) => {
                        dynasm!(ctx.asm
                            ; xor Rq(result), Rq(result)
                            ; cmp Rd(lreg), Rd(rreg)
                            ; $instr Rb(result)
                        );
                    }
                    ValueLocation::Immediate(i) => {
                        dynasm!(ctx.asm
                            ; xor Rq(result), Rq(result)
                            ; cmp Rd(lreg), i
                            ; $instr Rb(result)
                        );
                    }
                }

                Value::Temp(result)
            };

            push_i32(ctx, out);
            free_value(ctx, left);
            free_value(ctx, right);
        }
    }
}

cmp!(i32_eq, sete, |a, b| a == b);
cmp!(i32_neq, setne, |a, b| a != b);
// TODO: `dynasm-rs` inexplicably doesn't support setb
cmp!(i32_lt_u, setnae, |a, b| (a as u32) < (b as u32));
cmp!(i32_le_u, setbe, |a, b| (a as u32) <= (b as u32));
cmp!(i32_gt_u, seta, |a, b| (a as u32) > (b as u32));
cmp!(i32_ge_u, setae, |a, b| (a as u32) >= (b as u32));
cmp!(i32_lt_s, setl, |a, b| a < b);
cmp!(i32_le_s, setle, |a, b| a <= b);
cmp!(i32_gt_s, setg, |a, b| a == b);
cmp!(i32_ge_s, setge, |a, b| a == b);

/// Pops i32 predicate and branches to the specified label
/// if the predicate is equal to zero.
pub fn jump_if_false(ctx: &mut Context, label: Label) {
    let val = pop_i32(ctx);
    let predicate = into_temp_reg(ctx, val);
    dynasm!(ctx.asm
        ; test Rd(predicate), Rd(predicate)
        ; je =>label.0
    );
    ctx.block_state.regs.release_scratch_gpr(predicate);
}

/// Branch unconditionally to the specified label.
pub fn br(ctx: &mut Context, label: Label) {
    dynasm!(ctx.asm
        ; jmp =>label.0
    );
}

fn copy_value(ctx: &mut Context, src: ValueLocation, dst: ValueLocation) {
    match (src, dst) {
        (ValueLocation::Stack(in_offset), ValueLocation::Stack(out_offset)) => {
            let in_offset = adjusted_offset(ctx, in_offset);
            let out_offset = adjusted_offset(ctx, out_offset);
            if in_offset != out_offset {
                let gpr = ctx.block_state.regs.take_scratch_gpr();
                dynasm!(ctx.asm
                    ; mov Rq(gpr), [rsp + in_offset]
                    ; mov [rsp + out_offset], Rq(gpr)
                );
                ctx.block_state.regs.release_scratch_gpr(gpr);
            }
        }
        (ValueLocation::Reg(in_reg), ValueLocation::Stack(out_offset)) => {
            let out_offset = adjusted_offset(ctx, out_offset);
            dynasm!(ctx.asm
                ; mov [rsp + out_offset], Rq(in_reg)
            );
        }
        (ValueLocation::Immediate(i), ValueLocation::Stack(out_offset)) => {
            let out_offset = adjusted_offset(ctx, out_offset);
            dynasm!(ctx.asm
                ; mov DWORD [rsp + out_offset], i
            );
        }
        (ValueLocation::Stack(in_offset), ValueLocation::Reg(out_reg)) => {
            let in_offset = adjusted_offset(ctx, in_offset);
            dynasm!(ctx.asm
                ; mov Rq(out_reg), [rsp + in_offset]
            );
        }
        (ValueLocation::Reg(in_reg), ValueLocation::Reg(out_reg)) => {
            if in_reg != out_reg {
                dynasm!(ctx.asm
                    ; mov Rq(out_reg), Rq(in_reg)
                );
            }
        }
        (ValueLocation::Immediate(i), ValueLocation::Reg(out_reg)) => {
            dynasm!(ctx.asm
                ; mov Rq(out_reg), i
            );
        }
        (_, ValueLocation::Immediate(_)) => panic!("Tried to copy to an immediate value!"),
    }
}

#[must_use]
pub struct CallCleanup {
    restore_registers: ArrayVec<[GPR; SCRATCH_REGS.len()]>,
    stack_depth: i32,
}

/// Make sure that any argument registers that will be used by the call are free
/// by storing them to the stack.
///
/// Unfortunately, we can't elide this store if we're just passing arguments on
/// because these registers are caller-saved and so the callee can use them as
/// scratch space.
fn free_arg_registers(ctx: &mut Context, count: u32) {
    if count == 0 {
        return;
    }

    // This is bound to the maximum size of the `ArrayVec` amd so can be considered to have constant
    // runtime
    for i in 0..ctx.block_state.locals.register_arguments.len() {
        match ctx.block_state.locals.register_arguments[i] {
            ValueLocation::Reg(reg) => {
                if ARGS_IN_GPRS.contains(&reg) {
                    let dst = ValueLocation::Stack(
                        ((ctx.block_state.locals.num_local_stack_slots - 1 - i as u32) * WORD_SIZE)
                            as _,
                    );
                    copy_value(ctx, ValueLocation::Reg(reg), dst);
                    ctx.block_state.locals.register_arguments[i] = dst;
                }
            }
            _ => {}
        }
    }
}

fn free_return_register(ctx: &mut Context, count: u32) {
    if count == 0 {
        return;
    }

    free_register(ctx, RAX);
}

fn free_register(ctx: &mut Context, reg: GPR) {
    let mut to_repush = 0;
    let mut out = None;

    if ctx.block_state.regs.is_free(reg) {
        return;
    }

    // TODO: With real stack allocation we can make this constant-time
    for stack_val in ctx.block_state.stack.iter_mut().rev() {
        match stack_val.location(&ctx.block_state.locals) {
            // For now it's impossible for a local to be in RAX but that might be
            // possible in the future, so we check both cases.
            Some(ValueLocation::Reg(r)) if r == reg => {
                ctx.block_state.depth.reserve(1);
                *stack_val = StackValue::Pop;

                out = Some(*stack_val);

                break;
            }
            Some(_) => {}
            None => {
                to_repush += 1;
            }
        }
    }

    if let Some(out) = out {
        match out {
            StackValue::Temp(gpr) => {
                dynasm!(ctx.asm
                    ; mov Rq(gpr), rax
                );
            }
            StackValue::Pop => {
                // TODO: Ideally we should do proper stack allocation so we
                //       don't have to check this at all (i.e. order on the
                //       physical stack and order on the logical stack should
                //       be independent).
                assert_eq!(to_repush, 0);
                dynasm!(ctx.asm
                    ; push Rq(reg)
                );
            }
            _ => unreachable!(),
        }
        ctx.block_state.regs.release_scratch_gpr(reg);
    }
}

// TODO: Use `ArrayVec`?
/// Saves volatile (i.e. caller-saved) registers before a function call, if they are used.
fn save_volatile(ctx: &mut Context) -> ArrayVec<[GPR; SCRATCH_REGS.len()]> {
    let mut out = ArrayVec::new();

    // TODO: If there are no `StackValue::Pop`s that need to be popped
    //       before we reach our `Temp` value, we can set the `StackValue`
    //       for the register to be restored to `StackValue::Pop` (and
    //       release the register!) instead of restoring it.
    for &reg in SCRATCH_REGS.iter() {
        if !ctx.block_state.regs.is_free(reg) {
            dynasm!(ctx.asm
                ; push Rq(reg)
            );
            out.push(reg);
        }
    }

    out
}

/// Write the arguments to the callee to the registers and the stack using the SystemV
/// calling convention.
fn pass_outgoing_args(ctx: &mut Context, arity: u32, return_arity: u32) -> CallCleanup {
    let num_stack_args = (arity as usize).saturating_sub(ARGS_IN_GPRS.len()) as i32;

    free_arg_registers(ctx, arity);

    // We pop stack arguments first - arguments are RTL
    if num_stack_args > 0 {
        let size = num_stack_args * WORD_SIZE as i32;

        // Reserve space for the outgoing stack arguments (so we don't
        // stomp on any locals or the value stack).
        dynasm!(ctx.asm
            ; sub rsp, size
        );
        ctx.block_state.depth.reserve(num_stack_args as u32);

        for stack_slot in (0..num_stack_args).rev() {
            // Since the stack offset is from the bottom of the locals
            // and we want to start from the actual RSP (so `offset = 0`
            // writes to `[rsp]`), we subtract our current depth.
            //
            // We might want to do this in the future by having a separate
            // `AbsoluteValueLocation` and `RelativeValueLocation`.
            let offset =
                stack_slot * WORD_SIZE as i32 - ctx.block_state.depth.0 as i32 * WORD_SIZE as i32;
            pop_i32_into(ctx, ValueLocation::Stack(offset));
        }
    }

    for reg in ARGS_IN_GPRS[..(arity as usize).min(ARGS_IN_GPRS.len())]
        .iter()
        .rev()
    {
        pop_i32_into(ctx, ValueLocation::Reg(*reg));
    }

    // We do this before doing `save_volatile`, since otherwise we'll trample the return value
    // of the call when we pop back.
    free_return_register(ctx, return_arity);

    CallCleanup {
        stack_depth: num_stack_args,
        restore_registers: save_volatile(ctx),
    }
}

/// Frees up the stack space used for stack-passed arguments and restores the value
/// of volatile (i.e. caller-saved) registers to the state that they were in before
/// the call.
fn post_call_cleanup(ctx: &mut Context, mut cleanup: CallCleanup) {
    if cleanup.stack_depth > 0 {
        let size = cleanup.stack_depth * WORD_SIZE as i32;
        dynasm!(ctx.asm
            ; add rsp, size
        );
    }

    for reg in cleanup.restore_registers.drain(..).rev() {
        dynasm!(ctx.asm
            ; pop Rq(reg)
        );
    }
}

/// Call a function with the given index
pub fn call_direct(ctx: &mut Context, index: u32, arg_arity: u32, return_arity: u32) {
    assert!(
        return_arity == 0 || return_arity == 1,
        "We don't support multiple return yet"
    );

    let cleanup = pass_outgoing_args(ctx, arg_arity, return_arity);

    let label = &ctx.func_starts[index as usize].1;
    dynasm!(ctx.asm
        ; call =>*label
    );

    post_call_cleanup(ctx, cleanup);
    push_return_value(ctx, return_arity);
}

#[must_use]
pub struct Function {
    should_generate_epilogue: bool,
}

// TODO: Reserve space to store RBX, RBP, and R12..R15 so we can use them
//       as scratch registers
// TODO: Allow use of unused argument registers as scratch registers.
/// Writes the function prologue and stores the arguments as locals
pub fn start_function(ctx: &mut Context, arguments: u32, locals: u32) -> Function {
    let reg_args = &ARGS_IN_GPRS[..(arguments as usize).min(ARGS_IN_GPRS.len())];

    // We need space to store the register arguments if we need to call a function
    // and overwrite these registers so we add `reg_args.len()`
    let stack_slots = locals + reg_args.len() as u32;
    // Align stack slots to the nearest even number. This is required
    // by x86-64 ABI.
    let aligned_stack_slots = (stack_slots + 1) & !1;
    let frame_size: i32 = aligned_stack_slots as i32 * WORD_SIZE as i32;

    ctx.block_state.locals.register_arguments =
        reg_args.iter().cloned().map(ValueLocation::Reg).collect();
    ctx.block_state.locals.num_stack_args = arguments.saturating_sub(ARGS_IN_GPRS.len() as _);
    ctx.block_state.locals.num_local_stack_slots = stack_slots;
    ctx.block_state.return_register = Some(RAX);

    ctx.block_state.parent_locals = ctx.block_state.locals.clone();

    // ctx.block_state.depth.reserve(aligned_stack_slots - locals);
    let should_generate_epilogue = frame_size > 0;
    if should_generate_epilogue {
        dynasm!(ctx.asm
            ; push rbp
            ; mov rbp, rsp
            ; sub rsp, frame_size
        );
    }

    Function {
        should_generate_epilogue,
    }
}

/// Writes the function epilogue, restoring the stack pointer and returning to the
/// caller.
pub fn epilogue(ctx: &mut Context, func: Function) {
    // We don't need to clean up the stack - RSP is restored and
    // the calling function has its own register stack and will
    // stomp on the registers from our stack if necessary.
    if func.should_generate_epilogue {
        dynasm!(ctx.asm
            ; mov rsp, rbp
            ; pop rbp
        );
    }

    dynasm!(ctx.asm
        ; ret
    );
}

pub fn trap(ctx: &mut Context) {
    dynasm!(ctx.asm
        ; ud2
    );
}
