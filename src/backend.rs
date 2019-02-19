#![allow(dead_code)] // for now

use microwasm::{SignlessType, I32, I64};

use self::registers::*;
use dynasmrt::x64::Assembler;
use dynasmrt::{AssemblyOffset, DynamicLabel, DynasmApi, DynasmLabelApi, ExecutableBuffer};
use error::Error;
use std::{
    iter::{self, FromIterator},
    mem,
    ops::RangeInclusive,
};

use module::{ModuleContext, RuntimeFunc};

/// Size of a pointer on the target in bytes.
const WORD_SIZE: u32 = 8;

type GPR = u8;

pub fn arg_locs(types: impl IntoIterator<Item = SignlessType>) -> Vec<CCLoc> {
    let types = types.into_iter();
    let mut out = Vec::with_capacity(types.size_hint().0);
    // TODO: VmCtx is in the first register
    let mut int_gpr_iter = INTEGER_ARGS_IN_GPRS.into_iter();
    let mut stack_idx = 0;

    for ty in types {
        match ty {
            I32 | I64 => out.push(int_gpr_iter.next().map(|&r| CCLoc::Reg(r)).unwrap_or_else(
                || {
                    let out = CCLoc::Stack(stack_idx);
                    stack_idx += 1;
                    out
                },
            )),
            _ => {}
        }
    }

    out
}

pub fn ret_locs(types: impl IntoIterator<Item = SignlessType>) -> Vec<CCLoc> {
    let types = types.into_iter();
    let mut out = Vec::with_capacity(types.size_hint().0);
    // TODO: VmCtx is in the first register
    let mut int_gpr_iter = INTEGER_RETURN_GPRS.into_iter();

    for ty in types {
        match ty {
            I32 | I64 => out.push(CCLoc::Reg(
                *int_gpr_iter
                    .next()
                    .expect("We don't support stack returns yet"),
            )),
            _ => panic!("We don't support floats yet"),
        }
    }

    out
}

#[derive(Debug, Copy, Clone)]
struct GPRs {
    bits: u16,
}

impl GPRs {
    fn new() -> Self {
        Self { bits: 0 }
    }
}

pub mod registers {
    pub const RAX: u8 = 0;
    pub const RCX: u8 = 1;
    pub const RDX: u8 = 2;
    pub const RBX: u8 = 3;
    pub const RSP: u8 = 4;
    pub const RBP: u8 = 5;
    pub const RSI: u8 = 6;
    pub const RDI: u8 = 7;
    pub const R8: u8 = 8;
    pub const R9: u8 = 9;
    pub const R10: u8 = 10;
    pub const R11: u8 = 11;
    pub const R12: u8 = 12;
    pub const R13: u8 = 13;
    pub const R14: u8 = 14;
    pub const R15: u8 = 15;
    pub const NUM_GPRS: u8 = 16;
}

extern "sysv64" fn println(len: u64, args: *const u8) {
    println!("{}", unsafe {
        std::str::from_utf8_unchecked(std::slice::from_raw_parts(args, len as usize))
    });
}

#[allow(unused_macros)]
macro_rules! asm_println {
    ($asm:expr) => {asm_println!($asm,)};
    ($asm:expr, $($args:tt)*) => {{
        use std::mem;

        let mut args = format!($($args)*).into_bytes();

        let len = args.len();
        let ptr = args.as_mut_ptr();
        mem::forget(args);

        dynasm!($asm
            ; push rdi
            ; push rsi
            ; push rdx
            ; push rcx
            ; push r8
            ; push r9
            ; push r10
            ; push r11

            ; mov rax, QWORD println as *const u8 as i64
            ; mov rdi, QWORD len as i64
            ; mov rsi, QWORD ptr as i64

            ; test rsp, 0b1111
            ; jnz >with_adjusted_stack_ptr

            ; call rax
            ; jmp >pop_rest

            ; with_adjusted_stack_ptr:
            ; push 1
            ; call rax
            ; pop r11

            ; pop_rest:
            ; pop r11
            ; pop r10
            ; pop r9
            ; pop r8
            ; pop rcx
            ; pop rdx
            ; pop rsi
            ; pop rdi
        );
    }}
}

impl GPRs {
    fn take(&mut self) -> GPR {
        let lz = self.bits.trailing_zeros();
        debug_assert!(lz < 16, "ran out of free GPRs");
        let gpr = lz as GPR;
        self.mark_used(gpr);
        gpr
    }

    fn mark_used(&mut self, gpr: GPR) {
        self.bits &= !(1 << gpr as u16);
    }

    fn release(&mut self, gpr: GPR) {
        debug_assert!(
            !self.is_free(gpr),
            "released register {} was already free",
            gpr
        );
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
    counts: [u8; NUM_GPRS as usize],
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
            counts: [1; NUM_GPRS as _],
        };

        // Give ourselves a few scratch registers to work with, for now.
        for &scratch in SCRATCH_REGS {
            result.release_scratch_gpr(scratch);
        }

        result
    }

    pub fn mark_used(&mut self, gpr: GPR) {
        self.scratch.mark_used(gpr);
        self.counts[gpr as usize] += 1;
    }

    pub fn num_usages(&self, gpr: GPR) -> u8 {
        self.counts[gpr as usize]
    }

    // TODO: Add function that takes a scratch register if possible
    //       but otherwise gives a fresh stack location.
    pub fn take_scratch_gpr(&mut self) -> GPR {
        let out = self.scratch.take();
        self.counts[out as usize] += 1;
        out
    }

    pub fn release_scratch_gpr(&mut self, gpr: GPR) {
        let c = &mut self.counts[gpr as usize];
        *c -= 1;
        if *c == 0 {
            self.scratch.release(gpr);
        }
    }

    pub fn is_free(&self, gpr: GPR) -> bool {
        self.scratch.is_free(gpr)
    }

    pub fn free_scratch(&self) -> u32 {
        self.scratch.free_count()
    }
}

#[derive(Debug, Clone)]
pub struct CallingConvention {
    stack_depth: StackDepth,
    arguments: Vec<CCLoc>,
}

impl CallingConvention {
    pub fn function_start(args: impl IntoIterator<Item = CCLoc>) -> Self {
        CallingConvention {
            // We start and return the function with stack depth 1 since we must
            // allow space for the saved return address.
            stack_depth: StackDepth(1),
            arguments: Vec::from_iter(args),
        }
    }
}

// TODO: Combine this with `ValueLocation`?
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CCLoc {
    /// Value exists in a register.
    Reg(GPR),
    /// Value exists on the stack.
    Stack(i32),
}

// TODO: Allow pushing condition codes to stack? We'd have to immediately
//       materialise them into a register if anything is pushed above them.
/// Describes location of a value.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ValueLocation {
    /// Value exists in a register.
    Reg(GPR),
    /// Value exists on the stack. Note that this offset is from the rsp as it
    /// was when we entered the function.
    Stack(i32),
    /// Value is a literal (TODO: Support more than just `i32`)
    Immediate(i64),
}

impl From<CCLoc> for ValueLocation {
    fn from(other: CCLoc) -> Self {
        match other {
            CCLoc::Reg(r) => ValueLocation::Reg(r),
            CCLoc::Stack(o) => ValueLocation::Stack(o),
        }
    }
}

impl ValueLocation {
    fn immediate(&self) -> Option<i64> {
        match self {
            ValueLocation::Immediate(i) => Some(*i),
            _ => None,
        }
    }
}

// TODO: This assumes only system-v calling convention.
// In system-v calling convention the first 6 arguments are passed via registers.
// All rest arguments are passed on the stack.
const INTEGER_ARGS_IN_GPRS: &[GPR] = &[RSI, RDX, RCX, R8, R9];
const INTEGER_RETURN_GPRS: &[GPR] = &[RAX, RDX];
// List of scratch registers taken from https://wiki.osdev.org/System_V_ABI
const SCRATCH_REGS: &[GPR] = &[RSI, RDX, RCX, R8, R9, RAX, R10, R11];
const VMCTX: GPR = RDI;

#[must_use]
#[derive(Debug, Clone)]
pub struct FunctionEnd {
    should_generate_epilogue: bool,
}

pub struct CodeGenSession<'a, M> {
    assembler: Assembler,
    pub module_context: &'a M,
    func_starts: Vec<(Option<AssemblyOffset>, DynamicLabel)>,
}

impl<'a, M> CodeGenSession<'a, M> {
    pub fn new(func_count: u32, module_context: &'a M) -> Self {
        let mut assembler = Assembler::new().unwrap();
        let func_starts = iter::repeat_with(|| (None, assembler.new_dynamic_label()))
            .take(func_count as usize)
            .collect::<Vec<_>>();

        CodeGenSession {
            assembler,
            func_starts,
            module_context,
        }
    }

    pub fn new_context(&mut self, func_idx: u32) -> Context<'_, M> {
        {
            let func_start = &mut self.func_starts[func_idx as usize];

            // At this point we know the exact start address of this function. Save it
            // and define dynamic label at this location.
            func_start.0 = Some(self.assembler.offset());
            self.assembler.dynamic_label(func_start.1);
        }

        Context {
            asm: &mut self.assembler,
            func_starts: &self.func_starts,
            trap_label: None,
            block_state: Default::default(),
            module_context: self.module_context,
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
            // TODO
            relocatable_accesses: vec![],
        })
    }
}

#[derive(Debug)]
struct RelocateAddress {
    reg: Option<GPR>,
    imm: usize,
}

#[derive(Debug)]
struct RelocateAccess {
    position: AssemblyOffset,
    dst_reg: GPR,
    address: RelocateAddress,
}

#[derive(Debug)]
pub struct UninitializedCodeSection(TranslatedCodeSection);

#[derive(Debug)]
pub struct TranslatedCodeSection {
    exec_buf: ExecutableBuffer,
    func_starts: Vec<AssemblyOffset>,
    relocatable_accesses: Vec<RelocateAccess>,
}

impl TranslatedCodeSection {
    pub fn func_start(&self, idx: usize) -> *const u8 {
        let offset = self.func_starts[idx];
        self.exec_buf.ptr(offset)
    }

    pub fn func_range(&self, idx: usize) -> std::ops::Range<usize> {
        let end = self
            .func_starts
            .get(idx + 1)
            .map(|i| i.0)
            .unwrap_or(self.exec_buf.len());

        self.func_starts[idx].0..end
    }

    pub fn funcs<'a>(&'a self) -> impl Iterator<Item = std::ops::Range<usize>> + 'a {
        (0..self.func_starts.len()).map(move |i| self.func_range(i))
    }

    pub fn buffer(&self) -> &[u8] {
        &*self.exec_buf
    }

    pub fn disassemble(&self) {
        ::disassemble::disassemble(&*self.exec_buf).unwrap();
    }
}

/// A value on the logical stack. The logical stack is the value stack as it
/// is visible to the WebAssembly, whereas the physical stack is the stack as
/// it exists on the machine (i.e. as offsets in memory relative to `rsp`).
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum StackValue {
    /// This value has a "real" location, either in a register, on the stack,
    /// in an immediate, etc.
    Value(ValueLocation),
    /// This value is on the physical stack and so should be accessed
    /// with the `pop` instruction.
    // TODO: This complicates a lot of our code, it'd be great if we could get rid of it.
    Pop,
}

impl StackValue {
    /// Returns either the location that this value can be accessed at
    /// if possible. If this value is `Pop`, you can only access it by
    /// popping the physical stack and so this function returns `None`.
    ///
    /// Of course, we could calculate the location of the value on the
    /// physical stack, but that would be unncessary computation for
    /// our usecases.
    fn location(&self) -> Option<ValueLocation> {
        match *self {
            StackValue::Value(loc) => Some(loc),
            StackValue::Pop => None,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct BlockState {
    stack: Stack,
    depth: StackDepth,
    regs: Registers,
}

type Stack = Vec<ValueLocation>;

pub enum MemoryAccessMode {
    /// This is slower than using `Unchecked` mode, but works in
    /// any scenario (the most important scenario being when we're
    /// running on a system that can't index much more memory than
    /// the Wasm).
    Checked,
    /// This means that checks are _not emitted by the compiler_!
    /// If you're using WebAssembly to run untrusted code, you
    /// _must_ delegate bounds checking somehow (probably by
    /// allocating 2^33 bytes of memory with the second half set
    /// to unreadable/unwriteable/unexecutable)
    Unchecked,
}

pub struct Context<'a, M> {
    asm: &'a mut Assembler,
    module_context: &'a M,
    func_starts: &'a Vec<(Option<AssemblyOffset>, DynamicLabel)>,
    /// Each push and pop on the value stack increments or decrements this value by 1 respectively.
    pub block_state: BlockState,
    trap_label: Option<Label>,
}

/// Label in code.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Label(DynamicLabel);

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

macro_rules! unop {
    ($name:ident, $instr:ident, $reg_ty:ident, $typ:ty, $const_fallback:expr) => {
        pub fn $name(&mut self) {
            let val = self.pop();

            let out_val = match val {
                ValueLocation::Immediate(imm) => ValueLocation::Immediate($const_fallback(imm as $typ) as _),
                ValueLocation::Stack(offset) => {
                    let offset = self.adjusted_offset(offset);
                    let temp = self.block_state.regs.take_scratch_gpr();
                    dynasm!(self.asm
                        ; $instr $reg_ty(temp), [rsp + offset]
                    );
                    ValueLocation::Reg(temp)
                }
                ValueLocation::Reg(reg) => {
                    let temp = self.block_state.regs.take_scratch_gpr();
                    dynasm!(self.asm
                        ; $instr $reg_ty(temp), $reg_ty(reg)
                    );
                    ValueLocation::Reg(temp)
                }
            };

            self.push(out_val);
        }
    }
}

// TODO: Support immediate `count` parameters
macro_rules! shift {
    ($name:ident, $reg_ty:ident, $instr:ident, $const_fallback:expr) => {
        pub fn $name(&mut self) {
            enum RestoreRcx {
                MoveValBack(GPR),
                PopRcx,
            }

            let mut count = self.pop();
            let mut val = self.pop();

            if val == ValueLocation::Reg(RCX) {
                val = ValueLocation::Reg(self.into_temp_reg(val));
            }

            // TODO: Maybe allocate `RCX`, write `count` to it and then free `count`.
            //       Once we've implemented refcounting this will do the right thing
            //       for free.
            let temp_rcx = match count {
                ValueLocation::Reg(RCX) => {None}
                other => {
                    let out = if self.block_state.regs.is_free(RCX) {
                        None
                    } else {
                        let new_reg = self.block_state.regs.take_scratch_gpr();
                        dynasm!(self.asm
                            ; mov Rq(new_reg), rcx
                        );
                        Some(new_reg)
                    };

                    match other {
                        ValueLocation::Reg(gpr) => {
                            dynasm!(self.asm
                                ; mov cl, Rb(gpr)
                            );
                        }
                        ValueLocation::Stack(offset) => {
                            let offset = self.adjusted_offset(offset);
                            dynasm!(self.asm
                                ; mov cl, [rsp + offset]
                            );
                        }
                        ValueLocation::Immediate(imm) => {
                            dynasm!(self.asm
                                ; mov cl, imm as i8
                            );
                        }
                    }

                    out
                }
            };

            self.free_value(count);
            self.block_state.regs.mark_used(RCX);
            count = ValueLocation::Reg(RCX);

            let reg = self.into_reg(val);

            dynasm!(self.asm
                ; $instr $reg_ty(reg), cl
            );

            self.free_value(count);

            if let Some(gpr) = temp_rcx {
                dynasm!(self.asm
                    ; mov rcx, Rq(gpr)
                );
                self.block_state.regs.release_scratch_gpr(gpr);
            }

            self.push(ValueLocation::Reg(reg));
        }
    }
}

macro_rules! cmp_i32 {
    ($name:ident, $instr:ident, $reverse_instr:ident, $const_fallback:expr) => {
        pub fn $name(&mut self) {
            let right = self.pop();
            let mut left = self.pop();

            let out = if let Some(i) = left.immediate() {
                match right {
                    ValueLocation::Stack(offset) => {
                        let result = self.block_state.regs.take_scratch_gpr();
                        let offset = self.adjusted_offset(offset);

                        dynasm!(self.asm
                            ; xor Rd(result), Rd(result)
                            ; cmp DWORD [rsp + offset], i as i32
                            ; $reverse_instr Rb(result)
                        );
                        ValueLocation::Reg(result)
                    }
                    ValueLocation::Reg(rreg) => {
                        let result = self.block_state.regs.take_scratch_gpr();
                        dynasm!(self.asm
                            ; xor Rd(result), Rd(result)
                            ; cmp Rd(rreg), i as i32
                            ; $reverse_instr Rb(result)
                        );
                        ValueLocation::Reg(result)
                    }
                    ValueLocation::Immediate(right) => {
                        ValueLocation::Immediate(if $const_fallback(i as i32, right as i32) { 1 } else { 0 })
                    }
                }
            } else {
                let lreg = self.into_reg(left);
                // TODO: Make `into_reg` take an `&mut`?
                left = ValueLocation::Reg(lreg);
                let result = self.block_state.regs.take_scratch_gpr();

                match right {
                    ValueLocation::Stack(offset) => {
                        let offset = self.adjusted_offset(offset);
                        dynasm!(self.asm
                            ; xor Rd(result), Rd(result)
                            ; cmp Rd(lreg), [rsp + offset]
                            ; $instr Rb(result)
                        );
                    }
                    ValueLocation::Reg(rreg) => {
                        dynasm!(self.asm
                            ; xor Rd(result), Rd(result)
                            ; cmp Rd(lreg), Rd(rreg)
                            ; $instr Rb(result)
                        );
                    }
                    ValueLocation::Immediate(i) => {
                        dynasm!(self.asm
                            ; xor Rd(result), Rd(result)
                            ; cmp Rd(lreg), i as i32
                            ; $instr Rb(result)
                        );
                    }
                }

                ValueLocation::Reg(result)
            };

            self.free_value(left);
            self.free_value(right);

            self.push(out);
        }
    }
}

macro_rules! cmp_i64 {
    ($name:ident, $instr:ident, $reverse_instr:ident, $const_fallback:expr) => {
        pub fn $name(&mut self) {
            let right = self.pop();
            let mut left = self.pop();

            let out = if let Some(i) = left.immediate() {
                match right {
                    ValueLocation::Stack(offset) => {
                        let result = self.block_state.regs.take_scratch_gpr();
                        let offset = self.adjusted_offset(offset);
                        if let Some(i) = i.try_into() {
                            dynasm!(self.asm
                                ; xor Rd(result), Rd(result)
                                ; cmp QWORD [rsp + offset], i
                                ; $reverse_instr Rb(result)
                            );
                        } else {
                            unimplemented!("Unsupported `cmp` with large 64-bit immediate operand");
                        }
                        ValueLocation::Reg(result)
                    }
                    ValueLocation::Reg(rreg) => {
                        let result = self.block_state.regs.take_scratch_gpr();
                        if let Some(i) = i.try_into() {
                            dynasm!(self.asm
                                ; xor Rd(result), Rd(result)
                                ; cmp Rq(rreg), i
                                ; $reverse_instr Rb(result)
                            );
                        } else {
                            unimplemented!("Unsupported `cmp` with large 64-bit immediate operand");
                        }
                        ValueLocation::Reg(result)
                    }
                    ValueLocation::Immediate(right) => {
                        ValueLocation::Immediate(if $const_fallback(i, right) { 1 } else { 0 })
                    }
                }
            } else {
                let lreg = self.into_reg(left);
                left = ValueLocation::Reg(lreg);

                let result = self.block_state.regs.take_scratch_gpr();

                match right {
                    ValueLocation::Stack(offset) => {
                        let offset = self.adjusted_offset(offset);
                        dynasm!(self.asm
                            ; xor Rd(result), Rd(result)
                            ; cmp Rq(lreg), [rsp + offset]
                            ; $instr Rb(result)
                        );
                    }
                    ValueLocation::Reg(rreg) => {
                        dynasm!(self.asm
                            ; xor Rd(result), Rd(result)
                            ; cmp Rq(lreg), Rq(rreg)
                            ; $instr Rb(result)
                        );
                    }
                    ValueLocation::Immediate(i) => {
                        if let Some(i) = i.try_into() {
                            dynasm!(self.asm
                                ; xor Rd(result), Rd(result)
                                ; cmp Rq(lreg), i
                                ; $instr Rb(result)
                            );
                        } else {
                            unimplemented!("Unsupported `cmp` with large 64-bit immediate operand");
                        }
                    }
                }

                ValueLocation::Reg(result)
            };

            self.free_value(left);
            self.free_value(right);
            self.push(out);
        }
    }
}

macro_rules! commutative_binop_i32 {
    ($name:ident, $instr:ident, $const_fallback:expr) => {
        pub fn $name(&mut self) {
            let op0 = self.pop();
            let op1 = self.pop();

            if let Some(i1) = op1.immediate() {
                if let Some(i0) = op0.immediate() {
                    self.push(ValueLocation::Immediate($const_fallback(i1 as i32, i0 as i32) as _));
                    return;
                }
            }

            let (op1, op0) = match op1 {
                ValueLocation::Reg(_) => (self.into_temp_reg(op1), op0),
                _ => if op0.immediate().is_some() {
                    (self.into_temp_reg(op1), op0)
                } else {
                    (self.into_temp_reg(op0), op1)
                }
            };

            match op0 {
                ValueLocation::Reg(reg) => {
                    dynasm!(self.asm
                        ; $instr Rd(op1), Rd(reg)
                    );
                }
                ValueLocation::Stack(offset) => {
                    let offset = self.adjusted_offset(offset);
                    dynasm!(self.asm
                        ; $instr Rd(op1), [rsp + offset]
                    );
                }
                ValueLocation::Immediate(i) => {
                    dynasm!(self.asm
                        ; $instr Rd(op1), i as i32
                    );
                }
            }

            self.free_value(op0);
            self.push(ValueLocation::Reg(op1));
        }
    }
}

macro_rules! commutative_binop_i64 {
    ($name:ident, $instr:ident, $const_fallback:expr) => {
        pub fn $name(&mut self) {
            let op0 = self.pop();
            let op1 = self.pop();

            if let Some(i1) = op1.immediate() {
                if let Some(i0) = op0.immediate() {
                    self.block_state.stack.push(ValueLocation::Immediate($const_fallback(i1, i0)));
                    return;
                }
            }

            let (op1, op0) = match op1 {
                ValueLocation::Reg(reg) => (reg, op0),
                _ => if op0.immediate().is_some() {
                    (self.into_temp_reg(op1), op0)
                } else {
                    (self.into_temp_reg(op0), op1)
                }
            };

            match op0 {
                ValueLocation::Reg(reg) => {
                    dynasm!(self.asm
                        ; $instr Rq(op1), Rq(reg)
                    );
                }
                ValueLocation::Stack(offset) => {
                    let offset = self.adjusted_offset(offset);
                    dynasm!(self.asm
                        ; $instr Rq(op1), [rsp + offset]
                    );
                }
                ValueLocation::Immediate(i) => {
                    if let Some(i) = i.try_into() {
                        dynasm!(self.asm
                            ; $instr Rq(op1), i
                        );
                    } else {
                        let scratch = self.block_state.regs.take_scratch_gpr();

                        dynasm!(self.asm
                            ; mov Rq(scratch), QWORD i
                            ; $instr Rq(op1), Rq(scratch)
                        );

                        self.block_state.regs.release_scratch_gpr(scratch);
                    }
                }
            }

            self.free_value(op0);
            self.push(ValueLocation::Reg(op1));
        }
    }
}

macro_rules! load {
    ($name:ident, $reg_ty:ident, $instruction_name:expr) => {
        pub fn $name(&mut self, offset: u32) -> Result<(), Error> {
            fn load_to_reg<_M: ModuleContext>(
                ctx: &mut Context<_M>,
                dst: GPR,
                (offset, runtime_offset): (i32, Result<i32, GPR>)
            ) {
                let vmctx_mem_ptr_offset = ctx.module_context.offset_of_memory_ptr() as i32;
                let mem_ptr_reg = ctx.block_state.regs.take_scratch_gpr();
                dynasm!(ctx.asm
                    ; mov Rq(mem_ptr_reg), [Rq(VMCTX) + vmctx_mem_ptr_offset]
                );
                match runtime_offset {
                    Ok(imm) => {
                        dynasm!(ctx.asm
                            ; mov $reg_ty(dst), [Rq(mem_ptr_reg) + offset + imm]
                        );
                    }
                    Err(offset_reg) => {
                        dynasm!(ctx.asm
                            ; mov $reg_ty(dst), [Rq(mem_ptr_reg) + Rq(offset_reg) + offset]
                        );
                    }
                }
                ctx.block_state.regs.release_scratch_gpr(mem_ptr_reg);
            }

            assert!(offset <= i32::max_value() as u32);

            let base = self.pop();

            let temp = self.block_state.regs.take_scratch_gpr();

            match base {
                ValueLocation::Immediate(i) => {
                    let val = if let Some(i) = i.try_into() {
                        Ok(i)
                    } else {
                        Err(self.into_temp_reg(base))
                    };

                    load_to_reg(self, temp, (offset as _, val));

                    if let Err(r) = val {
                        self.block_state.regs.release_scratch_gpr(r);
                    }
                }
                base => {
                    let gpr = self.into_reg(base);
                    load_to_reg(self, temp, (offset as _, Err(gpr)));
                    self.block_state.regs.release_scratch_gpr(gpr);
                }
            }

            self.push(ValueLocation::Reg(temp));

            Ok(())
        }
    }
}

macro_rules! store {
    ($name:ident, $reg_ty:ident, $size:ident, $instruction_name:expr) => {
        pub fn $name(&mut self, offset: u32) -> Result<(), Error> {
            fn store_from_reg<_M: ModuleContext>(
                ctx: &mut Context<_M>,
                src: GPR,
                (offset, runtime_offset): (i32, Result<i32, GPR>)
            ) {
                let vmctx_mem_ptr_offset = ctx.module_context.offset_of_memory_ptr() as i32;
                let mem_ptr_reg = ctx.block_state.regs.take_scratch_gpr();
                dynasm!(ctx.asm
                    ; mov Rq(mem_ptr_reg), [Rq(VMCTX) + vmctx_mem_ptr_offset]
                );
                match runtime_offset {
                    Ok(imm) => {
                        dynasm!(ctx.asm
                            ; mov [Rq(mem_ptr_reg) + offset + imm], $reg_ty(src)
                        );
                    }
                    Err(offset_reg) => {
                        dynasm!(ctx.asm
                            ; mov [Rq(mem_ptr_reg) + Rq(offset_reg) + offset], $reg_ty(src)
                        );
                    }
                }
                ctx.block_state.regs.release_scratch_gpr(mem_ptr_reg);
            }

            assert!(offset <= i32::max_value() as u32);

            let src = self.pop();
            let base = self.pop();

            let src_reg = self.into_reg(src);

            match base {
                ValueLocation::Immediate(i) => {
                    let val = if let Some(i) = i.try_into() {
                        Ok(i)
                    } else {
                        Err(self.into_temp_reg(base))
                    };

                    store_from_reg(self, src_reg, (offset as i32, val));

                    if let Err(r) = val {
                        self.block_state.regs.release_scratch_gpr(r);
                    }
                }
                base => {
                    let gpr = self.into_reg(base);
                    store_from_reg(self, src_reg, (offset as i32, Err(gpr)));
                    self.block_state.regs.release_scratch_gpr(gpr);
                }
            }

            self.block_state.regs.release_scratch_gpr(src_reg);

            Ok(())
        }
    }
}

trait TryInto<O> {
    fn try_into(self) -> Option<O>;
}

impl TryInto<i64> for u64 {
    fn try_into(self) -> Option<i64> {
        let max = i64::max_value() as u64;

        if self <= max {
            Some(self as i64)
        } else {
            None
        }
    }
}

impl TryInto<i32> for i64 {
    fn try_into(self) -> Option<i32> {
        let min = i32::min_value() as i64;
        let max = i32::max_value() as i64;

        if self >= min && self <= max {
            Some(self as i32)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct VirtualCallingConvention {
    stack: Stack,
    depth: StackDepth,
}

impl<M: ModuleContext> Context<'_, M> {
    pub fn debug(&mut self, d: std::fmt::Arguments) {
        asm_println!(self.asm, "{}", d);
    }

    pub fn virtual_calling_convention(&self) -> VirtualCallingConvention {
        VirtualCallingConvention {
            stack: self.block_state.stack.clone(),
            depth: self.block_state.depth,
        }
    }

    /// Create a new undefined label.
    pub fn create_label(&mut self) -> Label {
        Label(self.asm.new_dynamic_label())
    }

    pub fn define_host_fn(&mut self, host_fn: *const u8) {
        dynasm!(self.asm
            ; mov rax, QWORD host_fn as i64
            ; call rax
            ; ret
        );
    }

    fn adjusted_offset(&self, offset: i32) -> i32 {
        (self.block_state.depth.0 as i32 + offset) * WORD_SIZE as i32
    }

    cmp_i32!(i32_eq, sete, sete, |a, b| a == b);
    cmp_i32!(i32_neq, setne, setne, |a, b| a != b);
    // `dynasm-rs` inexplicably doesn't support setb but `setnae` (and `setc`) are synonymous
    cmp_i32!(i32_lt_u, setnae, seta, |a, b| (a as u32) < (b as u32));
    cmp_i32!(i32_le_u, setbe, setae, |a, b| (a as u32) <= (b as u32));
    cmp_i32!(i32_gt_u, seta, setnae, |a, b| (a as u32) > (b as u32));
    cmp_i32!(i32_ge_u, setae, setna, |a, b| (a as u32) >= (b as u32));
    cmp_i32!(i32_lt_s, setl, setnle, |a, b| a < b);
    cmp_i32!(i32_le_s, setle, setnl, |a, b| a <= b);
    cmp_i32!(i32_gt_s, setg, setnge, |a, b| a > b);
    cmp_i32!(i32_ge_s, setge, setng, |a, b| a >= b);

    cmp_i64!(i64_eq, sete, sete, |a, b| a == b);
    cmp_i64!(i64_neq, setne, setne, |a, b| a != b);
    // `dynasm-rs` inexplicably doesn't support setb but `setnae` (and `setc`) are synonymous
    cmp_i64!(i64_lt_u, setnae, seta, |a, b| (a as u64) < (b as u64));
    cmp_i64!(i64_le_u, setbe, setae, |a, b| (a as u64) <= (b as u64));
    cmp_i64!(i64_gt_u, seta, setnae, |a, b| (a as u64) > (b as u64));
    cmp_i64!(i64_ge_u, setae, setna, |a, b| (a as u64) >= (b as u64));
    cmp_i64!(i64_lt_s, setl, setnle, |a, b| a < b);
    cmp_i64!(i64_le_s, setle, setnl, |a, b| a <= b);
    cmp_i64!(i64_gt_s, setg, setnge, |a, b| a > b);
    cmp_i64!(i64_ge_s, setge, setng, |a, b| a >= b);

    // TODO: Should we do this logic in `eq` and just have this delegate to `eq`?
    //       That would mean that `eqz` and `eq` with a const 0 argument don't
    //       result in different code. It would also allow us to generate better
    //       code for `neq` and `gt_u` with const 0 operand
    pub fn i32_eqz(&mut self) {
        let val = self.pop();

        if let ValueLocation::Immediate(i) = val {
            self.push(ValueLocation::Immediate(if i == 0 { 1 } else { 0 }));
            return;
        }

        let reg = self.into_reg(val);
        let out = self.block_state.regs.take_scratch_gpr();

        dynasm!(self.asm
            ; xor Rd(out), Rd(out)
            ; test Rd(reg), Rd(reg)
            ; setz Rb(out)
        );

        self.block_state.regs.release_scratch_gpr(reg);

        self.push(ValueLocation::Reg(out));
    }

    pub fn i64_eqz(&mut self) {
        let val = self.pop();

        if let ValueLocation::Immediate(i) = val {
            self.push(ValueLocation::Immediate(if i == 0 { 1 } else { 0 }));
            return;
        }

        let reg = self.into_reg(val);
        let out = self.block_state.regs.take_scratch_gpr();

        dynasm!(self.asm
            ; xor Rd(out), Rd(out)
            ; test Rq(reg), Rq(reg)
            ; setz Rb(out)
        );

        self.block_state.regs.release_scratch_gpr(reg);

        self.push(ValueLocation::Reg(out));
    }

    /// Pops i32 predicate and branches to the specified label
    /// if the predicate is equal to zero.
    pub fn br_if_false(&mut self, label: Label, f: impl FnOnce(&mut Self)) {
        let val = self.pop();

        f(self);

        let predicate = self.into_reg(val);

        dynasm!(self.asm
            ; test Rd(predicate), Rd(predicate)
            ; jz =>label.0
        );

        self.block_state.regs.release_scratch_gpr(predicate);
    }

    /// Pops i32 predicate and branches to the specified label
    /// if the predicate is not equal to zero.
    pub fn br_if_true(&mut self, label: Label, f: impl FnOnce(&mut Self)) {
        let val = self.pop();

        f(self);

        let predicate = self.into_reg(val);

        dynasm!(self.asm
            ; test Rd(predicate), Rd(predicate)
            ; jnz =>label.0
        );

        self.block_state.regs.release_scratch_gpr(predicate);
    }

    /// Branch unconditionally to the specified label.
    pub fn br(&mut self, label: Label) {
        dynasm!(self.asm
            ; jmp =>label.0
        );
    }

    fn set_stack_depth_preserve_flags(&mut self, depth: StackDepth) {
        if self.block_state.depth.0 < depth.0 {
            // TODO: We need to preserve ZF on `br_if` so we use `push`/`pop` but that isn't
            //       necessary on (for example) `br`.
            for _ in 0..depth.0 - self.block_state.depth.0 {
                dynasm!(self.asm
                    ; push rax
                );
            }
        } else if self.block_state.depth.0 > depth.0 {
            let trash = self.block_state.regs.take_scratch_gpr();
            // TODO: We need to preserve ZF on `br_if` so we use `push`/`pop` but that isn't
            //       necessary on (for example) `br`.
            for _ in 0..self.block_state.depth.0 - depth.0 {
                dynasm!(self.asm
                    ; pop Rq(trash)
                );
            }
        }

        self.block_state.depth = depth;
    }

    fn set_stack_depth(&mut self, depth: StackDepth) {
        if self.block_state.depth.0 != depth.0 {
            let diff = depth.0 as i32 - self.block_state.depth.0 as i32;
            if diff.abs() == 1 {
                self.set_stack_depth_preserve_flags(depth);
            } else {
                dynasm!(self.asm
                    ; add rsp, (self.block_state.depth.0 as i32 - depth.0 as i32) * WORD_SIZE as i32
                );

                self.block_state.depth = depth;
            }
        }
    }

    pub fn pass_block_args(&mut self, cc: &CallingConvention) {
        let args = &cc.arguments;
        for (remaining, &dst) in args.iter().enumerate().rev() {
            if let CCLoc::Reg(r) = dst {
                if !self.block_state.regs.is_free(r)
                    && *self.block_state.stack.last().unwrap() != ValueLocation::Reg(r)
                {
                    // TODO: This would be made simpler and more efficient with a proper SSE
                    //       representation.
                    self.save_regs(&[r], ..=remaining);
                }

                self.block_state.regs.mark_used(r);
            }
            self.pop_into(dst.into());
        }

        self.set_stack_depth(cc.stack_depth);
    }

    /// Puts all stack values into "real" locations so that they can i.e. be set to different
    /// values on different iterations of a loop
    pub fn serialize_args(&mut self, count: u32) -> CallingConvention {
        let mut out = Vec::with_capacity(count as _);

        for _ in 0..count {
            let val = self.pop();
            // TODO: We can use stack slots for values already on the stack but we
            //       don't refcount stack slots right now
            let loc = CCLoc::Reg(self.into_temp_reg(val));

            out.push(loc);
        }

        out.reverse();

        CallingConvention {
            stack_depth: self.block_state.depth,
            arguments: out,
        }
    }

    fn immediate_to_reg(&mut self, reg: GPR, val: i64) {
        if (val as u64) <= u32::max_value() as u64 {
            dynasm!(self.asm
                ; mov Rd(reg), val as i32
            );
        } else {
            dynasm!(self.asm
                ; mov Rq(reg), QWORD val
            );
        }
    }

    // The `&` and `&mut` aren't necessary (`ValueLocation` is copy) but it ensures that we don't get
    // the arguments the wrong way around. In the future we want to have a `ReadLocation` and `WriteLocation`
    // so we statically can't write to a literal so this will become a non-issue.
    fn copy_value(&mut self, src: &ValueLocation, dst: &mut ValueLocation) {
        match (*src, *dst) {
            (ValueLocation::Stack(in_offset), ValueLocation::Stack(out_offset)) => {
                let in_offset = self.adjusted_offset(in_offset);
                let out_offset = self.adjusted_offset(out_offset);
                if in_offset != out_offset {
                    let gpr = self.block_state.regs.take_scratch_gpr();
                    dynasm!(self.asm
                        ; mov Rq(gpr), [rsp + in_offset]
                        ; mov [rsp + out_offset], Rq(gpr)
                    );
                    self.block_state.regs.release_scratch_gpr(gpr);
                }
            }
            (ValueLocation::Reg(in_reg), ValueLocation::Stack(out_offset)) => {
                let out_offset = self.adjusted_offset(out_offset);
                dynasm!(self.asm
                    ; mov [rsp + out_offset], Rq(in_reg)
                );
            }
            (ValueLocation::Immediate(i), ValueLocation::Stack(out_offset)) => {
                let out_offset = self.adjusted_offset(out_offset);
                if (i as u64) <= u32::max_value() as u64 {
                    dynasm!(self.asm
                        ; mov DWORD [rsp + out_offset], i as i32
                    );
                } else {
                    let scratch = self.block_state.regs.take_scratch_gpr();

                    dynasm!(self.asm
                        ; mov Rq(scratch), QWORD i
                        ; mov [rsp + out_offset], Rq(scratch)
                    );

                    self.block_state.regs.release_scratch_gpr(scratch);
                }
            }
            (ValueLocation::Stack(in_offset), ValueLocation::Reg(out_reg)) => {
                let in_offset = self.adjusted_offset(in_offset);
                dynasm!(self.asm
                    ; mov Rq(out_reg), [rsp + in_offset]
                );
            }
            (ValueLocation::Reg(in_reg), ValueLocation::Reg(out_reg)) => {
                if in_reg != out_reg {
                    dynasm!(self.asm
                        ; mov Rq(out_reg), Rq(in_reg)
                    );
                }
            }
            (ValueLocation::Immediate(i), ValueLocation::Reg(out_reg)) => {
                self.immediate_to_reg(out_reg, i);
            }
            // TODO: Have separate `ReadLocation` and `WriteLocation`?
            (_, ValueLocation::Immediate(_)) => panic!("Tried to copy to an immediate value!"),
        }
    }

    /// Define the given label at the current position.
    ///
    /// Multiple labels can be defined at the same position. However, a label
    /// can be defined only once.
    pub fn define_label(&mut self, label: Label) {
        self.asm.dynamic_label(label.0);
    }

    pub fn set_state(&mut self, state: VirtualCallingConvention) {
        self.block_state.regs = Registers::new();
        for elem in &state.stack {
            if let ValueLocation::Reg(r) = elem {
                self.block_state.regs.mark_used(*r);
            }
        }
        self.block_state.stack = state.stack;
        self.block_state.depth = state.depth;
    }

    pub fn apply_cc(&mut self, cc: &CallingConvention) {
        let stack = cc.arguments.iter();

        self.block_state.stack = Vec::with_capacity(stack.size_hint().0);
        self.block_state.regs = Registers::new();

        for &elem in stack {
            if let CCLoc::Reg(r) = elem {
                self.block_state.regs.mark_used(r);
            }

            self.block_state.stack.push(elem.into());
        }

        self.block_state.depth = cc.stack_depth;
    }

    load!(i32_load, Rd, "i32.load");
    load!(i64_load, Rq, "i64.load");
    store!(i32_store, Rd, DWORD, "i32.store");
    store!(i64_store, Rq, QWORD, "i64.store");

    fn push_physical(&mut self, value: ValueLocation) -> ValueLocation {
        self.block_state.depth.reserve(1);
        match value {
            ValueLocation::Reg(gpr) => {
                // TODO: Proper stack allocation scheme
                dynasm!(self.asm
                    ; push Rq(gpr)
                );
                self.block_state.regs.release_scratch_gpr(gpr);
            }
            ValueLocation::Stack(o) => {
                let offset = self.adjusted_offset(o);
                dynasm!(self.asm
                    ; push QWORD [rsp + offset]
                );
            }
            ValueLocation::Immediate(imm) => {
                let gpr = self.block_state.regs.take_scratch_gpr();
                dynasm!(self.asm
                    ; mov Rq(gpr), QWORD imm
                    ; push Rq(gpr)
                );
                self.block_state.regs.release_scratch_gpr(gpr);
            }
        }
        ValueLocation::Stack(-(self.block_state.depth.0 as i32))
    }

    fn push(&mut self, value: ValueLocation) {
        self.block_state.stack.push(value);
    }

    fn pop(&mut self) -> ValueLocation {
        self.block_state.stack.pop().expect("Stack is empty")
    }

    pub fn drop(&mut self, range: RangeInclusive<u32>) {
        let mut repush = Vec::with_capacity(*range.start() as _);

        for _ in 0..*range.start() {
            repush.push(self.pop());
        }

        for _ in range {
            let val = self.pop();
            self.free_value(val);
        }

        for v in repush.into_iter().rev() {
            self.push(v);
        }
    }

    fn pop_into(&mut self, dst: ValueLocation) {
        let val = self.pop();
        self.copy_value(&val, &mut { dst });
        self.free_value(val);
    }

    fn free_value(&mut self, val: ValueLocation) {
        match val {
            ValueLocation::Reg(r) => {
                self.block_state.regs.release_scratch_gpr(r);
            }
            // TODO: Refcounted stack slots
            _ => {}
        }
    }

    /// Puts this value into a register so that it can be efficiently read
    fn into_reg(&mut self, val: ValueLocation) -> GPR {
        match val {
            ValueLocation::Reg(r) => r,
            ValueLocation::Immediate(i) => {
                let scratch = self.block_state.regs.take_scratch_gpr();
                self.immediate_to_reg(scratch, i);
                scratch
            }
            ValueLocation::Stack(offset) => {
                // TODO: We can use `pop` if the number of usages is 1
                //       Even better, with an SSE-like `Value` abstraction
                //       we can make it so we only load it once.
                let offset = self.adjusted_offset(offset);
                let scratch = self.block_state.regs.take_scratch_gpr();

                dynasm!(self.asm
                    ; mov Rq(scratch), [rsp + offset]
                );

                scratch
            }
        }
    }

    /// Puts this value into a temporary register so that operations
    /// on that register don't write to a local.
    fn into_temp_reg(&mut self, val: ValueLocation) -> GPR {
        match val {
            ValueLocation::Reg(r) => {
                if self.block_state.regs.num_usages(r) <= 1 {
                    assert_eq!(self.block_state.regs.num_usages(r), 1);
                    r
                } else {
                    let new_reg = self.block_state.regs.take_scratch_gpr();
                    self.block_state.regs.release_scratch_gpr(r);
                    dynasm!(self.asm
                        ; mov Rq(new_reg), Rq(r)
                    );
                    new_reg
                }
            }
            val => self.into_reg(val),
        }
    }

    unop!(i32_clz, lzcnt, Rd, u32, u32::leading_zeros);
    unop!(i64_clz, lzcnt, Rq, u64, |a: u64| a.leading_zeros() as u64);
    unop!(i32_ctz, tzcnt, Rd, u32, u32::trailing_zeros);
    unop!(i64_ctz, tzcnt, Rq, u64, |a: u64| a.trailing_zeros() as u64);
    unop!(i32_popcnt, popcnt, Rd, u32, u32::count_ones);
    unop!(i64_popcnt, popcnt, Rq, u64, |a: u64| a.count_ones() as u64);

    // TODO: Use `lea` when the LHS operand isn't a temporary but both of the operands
    //       are in registers.
    commutative_binop_i32!(i32_add, add, |a, b| (a as i32).wrapping_add(b as i32));
    commutative_binop_i32!(i32_and, and, |a, b| a & b);
    commutative_binop_i32!(i32_or, or, |a, b| a | b);
    commutative_binop_i32!(i32_xor, xor, |a, b| a ^ b);

    commutative_binop_i64!(i64_add, add, i64::wrapping_add);
    commutative_binop_i64!(i64_and, and, |a, b| a & b);
    commutative_binop_i64!(i64_or, or, |a, b| a | b);
    commutative_binop_i64!(i64_xor, xor, |a, b| a ^ b);

    shift!(i32_shl, Rd, shl, |a, b| (a as i32).wrapping_shl(b as _));
    shift!(i32_shr_s, Rd, sar, |a, b| (a as i32).wrapping_shr(b as _));
    shift!(i32_shr_u, Rd, shr, |a, b| (a as u32).wrapping_shr(b as _));
    shift!(i32_rotl, Rd, rol, |a, b| (a as u32).rotate_left(b as _));
    shift!(i32_rotr, Rd, ror, |a, b| (a as u32).rotate_right(b as _));

    shift!(i64_shl, Rq, shl, |a, b| (a as i64).wrapping_shl(b as _));
    shift!(i64_shr_s, Rq, sar, |a, b| (a as i64).wrapping_shr(b as _));
    shift!(i64_shr_u, Rq, shr, |a, b| (a as u64).wrapping_shr(b as _));
    shift!(i64_rotl, Rq, rol, |a, b| (a as u64).rotate_left(b as _));
    shift!(i64_rotr, Rq, ror, |a, b| (a as u64).rotate_right(b as _));

    // `sub` is not commutative, so we have to handle it differently (we _must_ use the `op1`
    // temp register as the output)
    pub fn i64_sub(&mut self) {
        let op0 = self.pop();
        let op1 = self.pop();

        if let Some(i1) = op1.immediate() {
            if let Some(i0) = op0.immediate() {
                self.push(ValueLocation::Immediate(i1 - i0));
                return;
            }
        }

        let op1 = self.into_temp_reg(op1);
        match op0 {
            ValueLocation::Reg(reg) => {
                dynasm!(self.asm
                    ; sub Rq(op1), Rq(reg)
                );
            }
            ValueLocation::Stack(offset) => {
                let offset = self.adjusted_offset(offset);
                dynasm!(self.asm
                    ; sub Rq(op1), [rsp + offset]
                );
            }
            ValueLocation::Immediate(i) => {
                if let Some(i) = i.try_into() {
                    dynasm!(self.asm
                        ; sub Rq(op1), i
                    );
                } else {
                    unimplemented!(concat!(
                        "Unsupported `sub` with large 64-bit immediate operand"
                    ));
                }
            }
        }

        self.push(ValueLocation::Reg(op1));
        self.free_value(op0);
    }

    // `i64_mul` needs to be seperate because the immediate form of the instruction
    // has a different syntax to the immediate form of the other instructions.
    pub fn i64_mul(&mut self) {
        let op0 = self.pop();
        let op1 = self.pop();

        if let Some(i1) = op1.immediate() {
            if let Some(i0) = op0.immediate() {
                self.block_state
                    .stack
                    .push(ValueLocation::Immediate(i64::wrapping_mul(i1, i0)));
                return;
            }
        }

        let (op1, op0) = match op1 {
            ValueLocation::Reg(_) => (self.into_temp_reg(op1), op0),
            _ => {
                if op0.immediate().is_some() {
                    (self.into_temp_reg(op1), op0)
                } else {
                    (self.into_temp_reg(op0), op1)
                }
            }
        };

        match op0 {
            ValueLocation::Reg(reg) => {
                dynasm!(self.asm
                    ; imul Rq(op1), Rq(reg)
                );
            }
            ValueLocation::Stack(offset) => {
                let offset = self.adjusted_offset(offset);
                dynasm!(self.asm
                    ; imul Rq(op1), [rsp + offset]
                );
            }
            ValueLocation::Immediate(i) => {
                if let Some(i) = i.try_into() {
                    dynasm!(self.asm
                        ; imul Rq(op1), Rq(op1), i
                    );
                } else {
                    unimplemented!(concat!(
                        "Unsupported `imul` with large 64-bit immediate operand"
                    ));
                }
            }
        }

        self.push(ValueLocation::Reg(op1));
        self.free_value(op0);
    }

    // `sub` is not commutative, so we have to handle it differently (we _must_ use the `op1`
    // temp register as the output)
    pub fn i32_sub(&mut self) {
        let op0 = self.pop();
        let op1 = self.pop();

        if let Some(i1) = op1.immediate() {
            if let Some(i0) = op0.immediate() {
                self.block_state
                    .stack
                    .push(ValueLocation::Immediate(i1 - i0));
                return;
            }
        }

        let op1 = self.into_temp_reg(op1);
        match op0 {
            ValueLocation::Reg(reg) => {
                dynasm!(self.asm
                    ; sub Rd(op1), Rd(reg)
                );
            }
            ValueLocation::Stack(offset) => {
                let offset = self.adjusted_offset(offset);
                dynasm!(self.asm
                    ; sub Rd(op1), [rsp + offset]
                );
            }
            ValueLocation::Immediate(i) => {
                dynasm!(self.asm
                    ; sub Rd(op1), i as i32
                );
            }
        }

        self.push(ValueLocation::Reg(op1));
        self.free_value(op0);
    }

    // `i32_mul` needs to be seperate because the immediate form of the instruction
    // has a different syntax to the immediate form of the other instructions.
    pub fn i32_mul(&mut self) {
        let op0 = self.pop();
        let op1 = self.pop();

        if let Some(i1) = op1.immediate() {
            if let Some(i0) = op0.immediate() {
                self.push(ValueLocation::Immediate(
                    i32::wrapping_mul(i1 as i32, i0 as i32) as _,
                ));
                return;
            }
        }

        let (op1, op0) = match op1 {
            ValueLocation::Reg(_) => (self.into_temp_reg(op1), op0),
            _ => {
                if op0.immediate().is_some() {
                    (self.into_temp_reg(op1), op0)
                } else {
                    (self.into_temp_reg(op0), op1)
                }
            }
        };

        match op0 {
            ValueLocation::Reg(reg) => {
                dynasm!(self.asm
                    ; imul Rd(op1), Rd(reg)
                );
            }
            ValueLocation::Stack(offset) => {
                let offset = self.adjusted_offset(offset);
                dynasm!(self.asm
                    ; imul Rd(op1), [rsp + offset]
                );
            }
            ValueLocation::Immediate(i) => {
                dynasm!(self.asm
                    ; imul Rd(op1), Rd(op1), i as i32
                );
            }
        }

        self.push(ValueLocation::Reg(op1));
        self.free_value(op0);
    }

    pub fn select(&mut self) {
        let cond = self.pop();
        let else_ = self.pop();
        let then = self.pop();

        match cond {
            ValueLocation::Immediate(i) => {
                if i == 0 {
                    self.push(else_);
                } else {
                    self.push(then);
                }

                return;
            }
            other => {
                let reg = self.into_reg(other);

                dynasm!(self.asm
                    ; test Rd(reg), Rd(reg)
                );

                self.block_state.regs.release_scratch_gpr(reg);
            }
        }

        let out = self.block_state.regs.take_scratch_gpr();

        // TODO: Can do this better for variables on stack
        let reg = self.into_reg(else_);
        dynasm!(self.asm
            ; cmovz Rq(out), Rq(reg)
        );
        self.block_state.regs.release_scratch_gpr(reg);
        let reg = self.into_reg(then);
        dynasm!(self.asm
            ; cmovnz Rq(out), Rq(reg)
        );
        self.block_state.regs.release_scratch_gpr(reg);

        self.push(ValueLocation::Reg(out));
    }

    pub fn pick(&mut self, depth: u32) {
        let idx = self.block_state.stack.len() - 1 - depth as usize;
        let v = self.block_state.stack[idx];

        match v {
            ValueLocation::Reg(r) => {
                self.block_state.regs.mark_used(r);
            }
            _ => {}
        }

        self.block_state.stack.push(v);
    }

    pub fn i32_literal(&mut self, imm: i32) {
        self.push(ValueLocation::Immediate(imm as _));
    }

    pub fn i64_literal(&mut self, imm: i64) {
        self.push(ValueLocation::Immediate(imm));
    }

    // TODO: Use `ArrayVec`?
    // TODO: This inefficiently duplicates registers but it's not really possible
    //       to double up stack space right now.
    /// Saves volatile (i.e. caller-saved) registers before a function call, if they are used.
    fn save_volatile(&mut self, bounds: impl std::ops::RangeBounds<usize>) {
        self.save_regs(SCRATCH_REGS, bounds);
    }

    fn save_regs<I>(&mut self, regs: &I, bounds: impl std::ops::RangeBounds<usize>)
    where
        for<'a> &'a I: IntoIterator<Item = &'a GPR>,
        I: ?Sized,
    {
        use std::ops::Bound::*;

        let mut stack = mem::replace(&mut self.block_state.stack, vec![]);
        let (start, end) = (
            match bounds.end_bound() {
                Unbounded => 0,
                Included(v) => stack.len() - 1 - v,
                Excluded(v) => stack.len() - v,
            },
            match bounds.start_bound() {
                Unbounded => stack.len(),
                Included(v) => stack.len() - v,
                Excluded(v) => stack.len() - 1 - v,
            },
        );
        for val in stack[start..end].iter_mut() {
            if let ValueLocation::Reg(vreg) = *val {
                if regs.into_iter().any(|r| *r == vreg) {
                    *val = self.push_physical(*val);
                }
            }
        }

        mem::replace(&mut self.block_state.stack, stack);
    }

    /// Write the arguments to the callee to the registers and the stack using the SystemV
    /// calling convention.
    fn pass_outgoing_args(&mut self, out_locs: &[CCLoc]) {
        self.save_volatile(out_locs.len()..);

        // TODO: Do alignment here
        let total_stack_space = out_locs
            .iter()
            .flat_map(|&l| {
                if let CCLoc::Stack(offset) = l {
                    if offset > 0 {
                        Some(offset as u32)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .max()
            .unwrap_or(0);
        let depth = self.block_state.depth.0 + total_stack_space;

        let mut pending = Vec::<(ValueLocation, ValueLocation)>::new();

        for &loc in out_locs.iter().rev() {
            let val = self.pop();

            match loc {
                CCLoc::Stack(offset) => {
                    let offset = self.adjusted_offset(offset as i32 - depth as i32);

                    if offset == -(WORD_SIZE as i32) {
                        self.push_physical(val);
                    } else {
                        let gpr = self.into_reg(val);
                        dynasm!(self.asm
                            ; mov [rsp + offset], Rq(gpr)
                        );
                        self.block_state.regs.release_scratch_gpr(gpr);
                    }
                }
                CCLoc::Reg(r) => {
                    if val == ValueLocation::Reg(r) {
                        self.free_value(val);
                    } else if self.block_state.regs.is_free(r) {
                        self.copy_value(&val, &mut loc.into());
                        self.free_value(val);
                    } else {
                        pending.push((val, loc.into()));
                    }
                }
            }
        }

        let mut try_count = 10;
        while !pending.is_empty() {
            try_count -= 1;

            if try_count == 0 {
                unimplemented!("We can't handle cycles in the register allocation right now");
            }

            for (src, dst) in mem::replace(&mut pending, vec![]) {
                if let ValueLocation::Reg(r) = dst {
                    if !self.block_state.regs.is_free(r) {
                        pending.push((src, dst));
                        continue;
                    }
                }
                self.copy_value(&src, &mut { dst });
                self.free_value(src);
            }
        }

        self.set_stack_depth(StackDepth(depth));
    }

    // TODO: Multiple returns
    fn push_function_return(&mut self, arity: u32) {
        if arity == 0 {
            return;
        }
        debug_assert_eq!(arity, 1);
        self.block_state.regs.mark_used(RAX);
        self.push(ValueLocation::Reg(RAX));
    }

    // TODO: Do return types properly
    pub fn call_indirect(
        &mut self,
        signature_hash: u32,
        arg_types: impl IntoIterator<Item = SignlessType>,
        return_arity: u32,
    ) {
        debug_assert!(
            return_arity == 0 || return_arity == 1,
            "We don't support multiple return yet"
        );

        let locs = arg_locs(arg_types);

        for &loc in &locs {
            if let CCLoc::Reg(r) = loc {
                self.block_state.regs.mark_used(r);
            }
        }

        let callee = self.pop();
        let callee = self.into_temp_reg(callee);
        let temp0 = self.block_state.regs.take_scratch_gpr();

        for &loc in &locs {
            if let CCLoc::Reg(r) = loc {
                self.block_state.regs.release_scratch_gpr(r);
            }
        }

        self.pass_outgoing_args(&locs);

        let fail = self.trap_label().0;

        // TODO: Consider generating a single trap function and jumping to that instead.
        dynasm!(self.asm
            ; cmp Rd(callee), [Rq(VMCTX) + self.module_context.offset_of_funcs_len() as i32]
            ; jae =>fail
            ; imul Rd(callee), Rd(callee), mem::size_of::<RuntimeFunc>() as i32
            ; mov Rq(temp0), [Rq(VMCTX) + self.module_context.offset_of_funcs_ptr() as i32]
            ; cmp DWORD [
                Rq(temp0) +
                    Rq(callee) +
                    RuntimeFunc::offset_of_sig_hash() as i32
            ], signature_hash as i32
            ; jne =>fail
        );

        dynasm!(self.asm
            ; call QWORD [
                Rq(temp0) +
                    Rq(callee) +
                    RuntimeFunc::offset_of_func_start() as i32
            ]
        );

        self.block_state.regs.release_scratch_gpr(temp0);
        self.block_state.regs.release_scratch_gpr(callee);

        self.push_function_return(return_arity);
    }

    pub fn swap(&mut self, depth: u32) {
        let last = self.block_state.stack.len() - 1;
        self.block_state.stack.swap(last, last - depth as usize);
    }

    /// Call a function with the given index
    pub fn call_direct(
        &mut self,
        index: u32,
        arg_types: impl IntoIterator<Item = SignlessType>,
        return_arity: u32,
    ) {
        debug_assert!(
            return_arity == 0 || return_arity == 1,
            "We don't support multiple return yet"
        );

        self.pass_outgoing_args(&arg_locs(arg_types));

        let label = &self.func_starts[index as usize].1;
        dynasm!(self.asm
            ; call =>*label
        );

        self.push_function_return(return_arity);
    }

    // TODO: Reserve space to store RBX, RBP, and R12..R15 so we can use them
    //       as scratch registers
    // TODO: Allow use of unused argument registers as scratch registers.
    /// Writes the function prologue and stores the arguments as locals
    pub fn start_function(&mut self, params: impl IntoIterator<Item = SignlessType>) {
        let locs = Vec::from_iter(arg_locs(params));
        self.apply_cc(&CallingConvention::function_start(locs));
    }

    pub fn ret(&mut self) {
        dynasm!(self.asm
            ; ret
        );
    }

    /// Writes the function epilogue (right now all this does is add the trap label that the
    /// conditional traps in `call_indirect` use)
    pub fn epilogue(&mut self) {
        if let Some(l) = self.trap_label {
            self.define_label(l);
            dynasm!(self.asm
                ; ud2
            );
        }
    }

    pub fn trap(&mut self) {
        dynasm!(self.asm
            ; ud2
        );
    }

    #[must_use]
    fn trap_label(&mut self) -> Label {
        if let Some(l) = self.trap_label {
            return l;
        }

        let label = self.create_label();
        self.trap_label = Some(label);
        label
    }
}

