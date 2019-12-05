#![allow(clippy::float_cmp)]

use self::registers::*;
use crate::error::Error;
use crate::microwasm::{BrTarget, Ieee32, Ieee64, SignlessType, Type, Value, F32, F64, I32, I64};
use crate::module::ModuleContext;
use cranelift_codegen::{binemit, ir};
use dynasm::dynasm;
use dynasmrt::x64::Assembler;
use dynasmrt::{AssemblyOffset, DynamicLabel, DynasmApi, DynasmLabelApi, ExecutableBuffer};
use either::Either;
use more_asserts::assert_le;
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    convert::{TryFrom, TryInto},
    fmt::Display,
    iter::{self, FromIterator},
    mem,
    ops::RangeInclusive,
};

// TODO: Get rid of this! It's a total hack.
mod magic {
    use cranelift_codegen::ir;

    /// Compute an `ir::ExternalName` for the `memory.grow` libcall for
    /// 32-bit locally-defined memories.
    pub fn get_memory32_grow_name() -> ir::ExternalName {
        ir::ExternalName::user(1, 0)
    }

    /// Compute an `ir::ExternalName` for the `memory.grow` libcall for
    /// 32-bit imported memories.
    pub fn get_imported_memory32_grow_name() -> ir::ExternalName {
        ir::ExternalName::user(1, 1)
    }

    /// Compute an `ir::ExternalName` for the `memory.size` libcall for
    /// 32-bit locally-defined memories.
    pub fn get_memory32_size_name() -> ir::ExternalName {
        ir::ExternalName::user(1, 2)
    }

    /// Compute an `ir::ExternalName` for the `memory.size` libcall for
    /// 32-bit imported memories.
    pub fn get_imported_memory32_size_name() -> ir::ExternalName {
        ir::ExternalName::user(1, 3)
    }
}

/// Size of a pointer on the target in bytes.
const WORD_SIZE: u32 = 8;

type RegId = u8;

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum GPR {
    Rq(RegId),
    Rx(RegId),
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum GPRType {
    Rq,
    Rx,
}

impl From<SignlessType> for GPRType {
    fn from(other: SignlessType) -> GPRType {
        match other {
            I32 | I64 => GPRType::Rq,
            F32 | F64 => GPRType::Rx,
        }
    }
}

impl From<SignlessType> for Option<GPRType> {
    fn from(other: SignlessType) -> Self {
        Some(other.into())
    }
}

impl GPR {
    fn type_(self) -> GPRType {
        match self {
            GPR::Rq(_) => GPRType::Rq,
            GPR::Rx(_) => GPRType::Rx,
        }
    }

    fn rq(self) -> Option<RegId> {
        match self {
            GPR::Rq(r) => Some(r),
            GPR::Rx(_) => None,
        }
    }

    fn rx(self) -> Option<RegId> {
        match self {
            GPR::Rx(r) => Some(r),
            GPR::Rq(_) => None,
        }
    }
}

pub fn arg_locs(types: impl IntoIterator<Item = SignlessType>) -> Vec<CCLoc> {
    let types = types.into_iter();
    let mut out = Vec::with_capacity(types.size_hint().0);
    // TODO: VmCtx is in the first register
    let mut int_gpr_iter = INTEGER_ARGS_IN_GPRS.iter();
    let mut float_gpr_iter = FLOAT_ARGS_IN_GPRS.iter();
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
            F32 | F64 => out.push(
                float_gpr_iter
                    .next()
                    .map(|&r| CCLoc::Reg(r))
                    .expect("Float args on stack not yet supported"),
            ),
        }
    }

    out
}

pub fn ret_locs(types: impl IntoIterator<Item = SignlessType>) -> Vec<CCLoc> {
    let types = types.into_iter();
    let mut out = Vec::with_capacity(types.size_hint().0);
    // TODO: VmCtx is in the first register
    let mut int_gpr_iter = INTEGER_RETURN_GPRS.iter();
    let mut float_gpr_iter = FLOAT_RETURN_GPRS.iter();

    for ty in types {
        match ty {
            I32 | I64 => out.push(CCLoc::Reg(
                *int_gpr_iter
                    .next()
                    .expect("We don't support stack returns yet"),
            )),
            F32 | F64 => out.push(CCLoc::Reg(
                *float_gpr_iter
                    .next()
                    .expect("We don't support stack returns yet"),
            )),
        }
    }

    out
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct GPRs {
    bits: u16,
}

impl GPRs {
    fn new() -> Self {
        Self { bits: 0 }
    }
}

#[allow(dead_code)]
pub mod registers {
    use super::{RegId, GPR};

    pub mod rq {
        use super::RegId;

        pub const RAX: RegId = 0;
        pub const RCX: RegId = 1;
        pub const RDX: RegId = 2;
        pub const RBX: RegId = 3;
        pub const RSP: RegId = 4;
        pub const RBP: RegId = 5;
        pub const RSI: RegId = 6;
        pub const RDI: RegId = 7;
        pub const R8: RegId = 8;
        pub const R9: RegId = 9;
        pub const R10: RegId = 10;
        pub const R11: RegId = 11;
        pub const R12: RegId = 12;
        pub const R13: RegId = 13;
        pub const R14: RegId = 14;
        pub const R15: RegId = 15;
    }

    pub const RAX: GPR = GPR::Rq(self::rq::RAX);
    pub const RCX: GPR = GPR::Rq(self::rq::RCX);
    pub const RDX: GPR = GPR::Rq(self::rq::RDX);
    pub const RBX: GPR = GPR::Rq(self::rq::RBX);
    pub const RSP: GPR = GPR::Rq(self::rq::RSP);
    pub const RBP: GPR = GPR::Rq(self::rq::RBP);
    pub const RSI: GPR = GPR::Rq(self::rq::RSI);
    pub const RDI: GPR = GPR::Rq(self::rq::RDI);
    pub const R8: GPR = GPR::Rq(self::rq::R8);
    pub const R9: GPR = GPR::Rq(self::rq::R9);
    pub const R10: GPR = GPR::Rq(self::rq::R10);
    pub const R11: GPR = GPR::Rq(self::rq::R11);
    pub const R12: GPR = GPR::Rq(self::rq::R12);
    pub const R13: GPR = GPR::Rq(self::rq::R13);
    pub const R14: GPR = GPR::Rq(self::rq::R14);
    pub const R15: GPR = GPR::Rq(self::rq::R15);

    pub const XMM0: GPR = GPR::Rx(0);
    pub const XMM1: GPR = GPR::Rx(1);
    pub const XMM2: GPR = GPR::Rx(2);
    pub const XMM3: GPR = GPR::Rx(3);
    pub const XMM4: GPR = GPR::Rx(4);
    pub const XMM5: GPR = GPR::Rx(5);
    pub const XMM6: GPR = GPR::Rx(6);
    pub const XMM7: GPR = GPR::Rx(7);
    pub const XMM8: GPR = GPR::Rx(8);
    pub const XMM9: GPR = GPR::Rx(9);
    pub const XMM10: GPR = GPR::Rx(10);
    pub const XMM11: GPR = GPR::Rx(11);
    pub const XMM12: GPR = GPR::Rx(12);
    pub const XMM13: GPR = GPR::Rx(13);
    pub const XMM14: GPR = GPR::Rx(14);
    pub const XMM15: GPR = GPR::Rx(15);

    pub const NUM_GPRS: u8 = 16;
}

const SIGN_MASK_F64: u64 = 0b1000000000000000000000000000000000000000000000000000000000000000;
const REST_MASK_F64: u64 = !SIGN_MASK_F64;
const SIGN_MASK_F32: u32 = 0b10000000000000000000000000000000;
const REST_MASK_F32: u32 = !SIGN_MASK_F32;

impl GPRs {
    fn take(&mut self) -> Option<RegId> {
        let lz = self.bits.trailing_zeros();
        if lz < 16 {
            let gpr = lz as RegId;
            self.mark_used(gpr);
            Some(gpr)
        } else {
            None
        }
    }

    fn mark_used(&mut self, gpr: RegId) {
        self.bits &= !(1 << gpr as u16);
    }

    fn release(&mut self, gpr: RegId) {
        debug_assert!(
            !self.is_free(gpr),
            "released register {} was already free",
            gpr
        );
        self.bits |= 1 << gpr;
    }

    fn is_free(self, gpr: RegId) -> bool {
        (self.bits & (1 << gpr)) != 0
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Registers {
    /// Registers at 64 bits and below (al/ah/ax/eax/rax, for example)
    scratch_64: (GPRs, [u8; NUM_GPRS as usize]),
    /// Registers at 128 bits (xmm0, for example)
    scratch_128: (GPRs, [u8; NUM_GPRS as usize]),
}

impl Default for Registers {
    fn default() -> Self {
        Self::new()
    }
}

impl Registers {
    pub fn new() -> Self {
        let mut result = Self {
            scratch_64: (GPRs::new(), [1; NUM_GPRS as _]),
            scratch_128: (GPRs::new(), [1; NUM_GPRS as _]),
        };

        // Give ourselves a few scratch registers to work with, for now.
        for &scratch in SCRATCH_REGS {
            result.release(scratch);
        }

        result
    }

    fn scratch_counts_mut(&mut self, gpr: GPR) -> (u8, &mut (GPRs, [u8; NUM_GPRS as usize])) {
        match gpr {
            GPR::Rq(r) => (r, &mut self.scratch_64),
            GPR::Rx(r) => (r, &mut self.scratch_128),
        }
    }

    fn scratch_counts(&self, gpr: GPR) -> (u8, &(GPRs, [u8; NUM_GPRS as usize])) {
        match gpr {
            GPR::Rq(r) => (r, &self.scratch_64),
            GPR::Rx(r) => (r, &self.scratch_128),
        }
    }

    pub fn mark_used(&mut self, gpr: GPR) {
        let (gpr, scratch_counts) = self.scratch_counts_mut(gpr);
        scratch_counts.0.mark_used(gpr);
        scratch_counts.1[gpr as usize] += 1;
    }

    pub fn num_usages(&self, gpr: GPR) -> u8 {
        let (gpr, scratch_counts) = self.scratch_counts(gpr);
        scratch_counts.1[gpr as usize]
    }

    pub fn take(&mut self, ty: impl Into<GPRType>) -> Option<GPR> {
        let (mk_gpr, scratch_counts) = match ty.into() {
            GPRType::Rq => (GPR::Rq as fn(_) -> _, &mut self.scratch_64),
            GPRType::Rx => (GPR::Rx as fn(_) -> _, &mut self.scratch_128),
        };

        let out = scratch_counts.0.take()?;
        scratch_counts.1[out as usize] += 1;
        Some(mk_gpr(out))
    }

    pub fn release(&mut self, gpr: GPR) -> Result<(), Error> {
        let (gpr, scratch_counts) = self.scratch_counts_mut(gpr);
        let c = &mut scratch_counts.1[gpr as usize];
        *c = match c.checked_sub(1) {
            Some(e) => e,
            None => {
                return Err(Error::Microwasm(
                    format!("Double-freed register: {}", gpr).to_string(),
                ))
            }
        };
        if *c == 0 {
            scratch_counts.0.release(gpr);
        }
        Ok(())
    }

    pub fn is_free(&self, gpr: GPR) -> bool {
        let (gpr, scratch_counts) = self.scratch_counts(gpr);
        scratch_counts.0.is_free(gpr)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockCallingConvention {
    pub stack_depth: StackDepth,
    pub arguments: Vec<CCLoc>,
}

impl BlockCallingConvention {
    pub fn function_start(args: impl IntoIterator<Item = CCLoc>) -> Self {
        BlockCallingConvention {
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

impl CCLoc {
    fn try_from(other: ValueLocation) -> Option<Self> {
        match other {
            ValueLocation::Reg(reg) => Some(CCLoc::Reg(reg)),
            ValueLocation::Stack(offset) => Some(CCLoc::Stack(offset)),
            _ => None,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CondCode {
    CF0,
    CF1,
    ZF0,
    ZF1,
    CF0AndZF0,
    CF1OrZF1,
    ZF0AndSFEqOF,
    ZF1OrSFNeOF,
    SFEqOF,
    SFNeOF,
}

mod cc {
    use super::CondCode;

    pub const EQUAL: CondCode = CondCode::ZF0;
    pub const NOT_EQUAL: CondCode = CondCode::ZF1;
    pub const GE_U: CondCode = CondCode::CF0;
    pub const LT_U: CondCode = CondCode::CF1;
    pub const GT_U: CondCode = CondCode::CF0AndZF0;
    pub const LE_U: CondCode = CondCode::CF1OrZF1;
    pub const GE_S: CondCode = CondCode::SFEqOF;
    pub const LT_S: CondCode = CondCode::SFNeOF;
    pub const GT_S: CondCode = CondCode::ZF0AndSFEqOF;
    pub const LE_S: CondCode = CondCode::ZF1OrSFNeOF;
}

impl std::ops::Not for CondCode {
    type Output = Self;

    fn not(self) -> Self {
        use CondCode::*;

        match self {
            CF0 => CF1,
            CF1 => CF0,
            ZF0 => ZF1,
            ZF1 => ZF0,
            CF0AndZF0 => CF1OrZF1,
            CF1OrZF1 => CF0AndZF0,
            ZF0AndSFEqOF => ZF1OrSFNeOF,
            ZF1OrSFNeOF => ZF0AndSFEqOF,
            SFEqOF => SFNeOF,
            SFNeOF => SFEqOF,
        }
    }
}

/// Describes location of a value.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ValueLocation {
    /// Value exists in a register.
    Reg(GPR),
    /// Value exists on the stack. Note that this offset is from the rsp as it
    /// was when we entered the function.
    Stack(i32),
    /// Value is a literal
    Immediate(Value),
    /// Value is a set condition code
    Cond(CondCode),
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
    fn stack(self) -> Option<i32> {
        match self {
            ValueLocation::Stack(o) => Some(o),
            _ => None,
        }
    }

    fn reg(self) -> Option<GPR> {
        match self {
            ValueLocation::Reg(r) => Some(r),
            _ => None,
        }
    }

    fn immediate(self) -> Option<Value> {
        match self {
            ValueLocation::Immediate(i) => Some(i),
            _ => None,
        }
    }

    fn imm_i32(self) -> Option<i32> {
        self.immediate().and_then(Value::as_i32)
    }

    fn imm_i64(self) -> Option<i64> {
        self.immediate().and_then(Value::as_i64)
    }

    fn imm_f32(self) -> Option<Ieee32> {
        self.immediate().and_then(Value::as_f32)
    }

    fn imm_f64(self) -> Option<Ieee64> {
        self.immediate().and_then(Value::as_f64)
    }
}

// TODO: This assumes only system-v calling convention.
// In system-v calling convention the first 6 arguments are passed via registers.
// All rest arguments are passed on the stack.
const INTEGER_ARGS_IN_GPRS: &[GPR] = &[RSI, RDX, RCX, R8, R9];
const INTEGER_RETURN_GPRS: &[GPR] = &[RAX, RDX];
const FLOAT_ARGS_IN_GPRS: &[GPR] = &[XMM0, XMM1, XMM2, XMM3, XMM4, XMM5, XMM6, XMM7];
const FLOAT_RETURN_GPRS: &[GPR] = &[XMM0, XMM1];
// List of scratch registers taken from https://wiki.osdev.org/System_V_ABI
const SCRATCH_REGS: &[GPR] = &[
    RSI, RDX, RCX, R8, R9, RAX, R10, R11, XMM0, XMM1, XMM2, XMM3, XMM4, XMM5, XMM6, XMM7, XMM8,
    XMM9, XMM10, XMM11, XMM12, XMM13, XMM14, XMM15,
];
const VMCTX: RegId = rq::RDI;

#[must_use]
#[derive(Debug, Clone)]
pub struct FunctionEnd {
    should_generate_epilogue: bool,
}

pub struct CodeGenSession<'module, M> {
    assembler: Assembler,
    pub module_context: &'module M,
    pub op_offset_map: Vec<(AssemblyOffset, Box<dyn Display + Send + Sync>)>,
    labels: Labels,
    func_starts: Vec<(Option<AssemblyOffset>, DynamicLabel)>,
}

impl<'module, M> CodeGenSession<'module, M> {
    pub fn new(func_count: u32, module_context: &'module M) -> Self {
        let mut assembler = Assembler::new().unwrap();
        let func_starts = iter::repeat_with(|| (None, assembler.new_dynamic_label()))
            .take(func_count as usize)
            .collect::<Vec<_>>();

        CodeGenSession {
            assembler,
            op_offset_map: Default::default(),
            labels: Default::default(),
            func_starts,
            module_context,
        }
    }

    pub fn new_context<'this>(
        &'this mut self,
        func_idx: u32,
        reloc_sink: &'this mut dyn binemit::RelocSink,
    ) -> Context<'this, M> {
        {
            let func_start = &mut self.func_starts[func_idx as usize];

            // At this point we know the exact start address of this function. Save it
            // and define dynamic label at this location.
            func_start.0 = Some(self.assembler.offset());
            self.assembler.dynamic_label(func_start.1);
        }

        Context {
            asm: &mut self.assembler,
            current_function: func_idx,
            reloc_sink,
            func_starts: &self.func_starts,
            labels: &mut self.labels,
            block_state: Default::default(),
            module_context: self.module_context,
        }
    }

    fn finalize(&mut self) {
        let mut values = self.labels.values_mut().collect::<Vec<_>>();
        values.sort_unstable_by_key(|(_, align, _)| *align);
        for (label, align, func) in values {
            if let Some(mut func) = func.take() {
                dynasm!(self.assembler
                    ; .align *align as usize
                );
                self.assembler.dynamic_label(label.0);
                func(&mut self.assembler);
            }
        }
    }

    pub fn into_translated_code_section(mut self) -> Result<TranslatedCodeSection, Error> {
        self.finalize();
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
            op_offset_map: self.op_offset_map,
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

pub struct TranslatedCodeSection {
    exec_buf: ExecutableBuffer,
    func_starts: Vec<AssemblyOffset>,
    #[allow(dead_code)]
    relocatable_accesses: Vec<RelocateAccess>,
    op_offset_map: Vec<(AssemblyOffset, Box<dyn Display + Send + Sync>)>,
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
            .unwrap_or_else(|| self.exec_buf.len());

        self.func_starts[idx].0..end
    }

    pub fn funcs<'a>(&'a self) -> impl Iterator<Item = std::ops::Range<usize>> + 'a {
        (0..self.func_starts.len()).map(move |i| self.func_range(i))
    }

    pub fn buffer(&self) -> &[u8] {
        &*self.exec_buf
    }

    pub fn disassemble(&self) {
        crate::disassemble::disassemble(&*self.exec_buf, &self.op_offset_map).unwrap();
    }
}

#[derive(Debug, Default, Clone)]
pub struct BlockState {
    pub stack: Stack,
    pub depth: StackDepth,
    pub regs: Registers,
}

type Stack = Vec<ValueLocation>;

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
enum LabelValue {
    I32(i32),
    I64(i64),
}

impl From<Value> for LabelValue {
    fn from(other: Value) -> LabelValue {
        match other {
            Value::I32(v) => LabelValue::I32(v),
            Value::I64(v) => LabelValue::I64(v),
            Value::F32(v) => LabelValue::I32(v.to_bits() as _),
            Value::F64(v) => LabelValue::I64(v.to_bits() as _),
        }
    }
}

type Labels = HashMap<
    (u32, Either<TypeId, (LabelValue, Option<LabelValue>)>),
    (Label, u32, Option<Box<dyn FnMut(&mut Assembler)>>),
>;

pub struct Context<'this, M> {
    pub asm: &'this mut Assembler,
    reloc_sink: &'this mut dyn binemit::RelocSink,
    module_context: &'this M,
    current_function: u32,
    func_starts: &'this Vec<(Option<AssemblyOffset>, DynamicLabel)>,
    /// Each push and pop on the value stack increments or decrements this value by 1 respectively.
    pub block_state: BlockState,
    labels: &'this mut Labels,
}

/// Label in code.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Label(DynamicLabel);

/// Offset from starting value of SP counted in words.
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct StackDepth(u32);

impl StackDepth {
    pub fn reserve(&mut self, slots: u32) {
        self.0 = self.0.checked_add(slots).unwrap();
    }

    pub fn free(&mut self, slots: u32) {
        self.0 = self.0.checked_sub(slots).unwrap();
    }
}

macro_rules! int_div {
    ($full_div_s:ident, $full_div_u:ident, $div_u:ident, $div_s:ident, $rem_u:ident, $rem_s:ident, $imm_fn:ident, $signed_ty:ty, $unsigned_ty:ty, $reg_ty:tt, $pointer_ty:tt) => {
        // TODO: Fast div using mul for constant divisor? It looks like LLVM doesn't do that for us when
        //       emitting Wasm.
        pub fn $div_u(&mut self) -> Result<(), Error>{
            let divisor = self.pop()?;
            let dividend = self.pop()?;

            if let (Some(dividend), Some(divisor)) = (dividend.$imm_fn(), divisor.$imm_fn()) {
                if divisor == 0 {
                    self.trap();
                    self.push(ValueLocation::Immediate((0 as $unsigned_ty).into()));
                } else {
                    self.push(ValueLocation::Immediate(
                        <$unsigned_ty>::wrapping_div(dividend as _, divisor as _).into(),
                    ));
                }

                return Ok(())
            }

            let (div, rem, saved) = self.$full_div_u(divisor, dividend)?;

            self.free_value(rem)?;

            let div = match div {
                ValueLocation::Reg(div)  => {
                    if saved.clone().any(|dst| dst == div) {
                        let new = self.take_reg(I32).unwrap();
                        dynasm!(self.asm
                            ; mov Rq(new.rq().unwrap()), Rq(div.rq().unwrap())
                        );
                        self.block_state.regs.release(div)?;
                        ValueLocation::Reg(new)
                    } else {
                        ValueLocation::Reg(div)
                    }
                }
                _ => div,
            };

            self.cleanup_gprs(saved);

            self.push(div);
            Ok(())
        }

        // TODO: Fast div using mul for constant divisor? It looks like LLVM doesn't do that for us when
        //       emitting Wasm.
        pub fn $div_s(&mut self) -> Result<(), Error>{
            let divisor = self.pop()?;
            let dividend = self.pop()?;

            if let (Some(dividend), Some(divisor)) = (dividend.$imm_fn(), divisor.$imm_fn()) {
                if divisor == 0 {
                    self.trap();
                    self.push(ValueLocation::Immediate((0 as $signed_ty).into()));
                } else {
                    self.push(ValueLocation::Immediate(
                        <$signed_ty>::wrapping_div(dividend, divisor).into(),
                    ));
                }

                return Ok(())
            }

            let (div, rem, saved) = self.$full_div_s(divisor, dividend)?;

            self.free_value(rem)?;

            let div = match div {
                ValueLocation::Reg(div)  => {
                    if saved.clone().any(|dst| dst == div) {
                        let new = self.take_reg(I32).unwrap();
                        dynasm!(self.asm
                            ; mov Rq(new.rq().unwrap()), Rq(div.rq().unwrap())
                        );
                        self.block_state.regs.release(div)?;
                        ValueLocation::Reg(new)
                    } else {
                        ValueLocation::Reg(div)
                    }
                }
                _ => div,
            };

            self.cleanup_gprs(saved);

            self.push(div);
            Ok(())
        }

        pub fn $rem_u(&mut self) -> Result<(), Error>{
            let divisor = self.pop()?;
            let dividend = self.pop()?;

            if let (Some(dividend), Some(divisor)) = (dividend.$imm_fn(), divisor.$imm_fn()) {
                if divisor == 0 {
                    self.trap();
                    self.push(ValueLocation::Immediate((0 as $unsigned_ty).into()));
                } else {
                    self.push(ValueLocation::Immediate(
                        (dividend as $unsigned_ty % divisor as $unsigned_ty).into(),
                    ));
                }
                return Ok(());
            }

            let (div, rem, saved) = self.$full_div_u(divisor, dividend)?;

            self.free_value(div)?;

            let rem = match rem {
                ValueLocation::Reg(rem)  => {
                    if saved.clone().any(|dst| dst == rem) {
                        let new = self.take_reg(I32).unwrap();
                        dynasm!(self.asm
                            ; mov Rq(new.rq().unwrap()), Rq(rem.rq().unwrap())
                        );
                        self.block_state.regs.release(rem)?;
                        ValueLocation::Reg(new)
                    } else {
                        ValueLocation::Reg(rem)
                    }
                }
                _ => rem,
            };

            self.cleanup_gprs(saved);

            self.push(rem);
            Ok(())
        }

        pub fn $rem_s(&mut self) -> Result<(), Error>{
            let mut divisor = self.pop()?;
            let dividend = self.pop()?;

            if let (Some(dividend), Some(divisor)) = (dividend.$imm_fn(), divisor.$imm_fn()) {
                if divisor == 0 {
                    self.trap();
                    self.push(ValueLocation::Immediate((0 as $signed_ty).into()));
                } else {
                    self.push(ValueLocation::Immediate((dividend % divisor).into()));
                }
                return Ok(());
            }

            let is_neg1 = self.create_label();

            let current_depth = self.block_state.depth.clone();

            // TODO: This could cause segfaults because of implicit push/pop
            let gen_neg1_case = match divisor {
                ValueLocation::Immediate(_) => {
                    if divisor.$imm_fn().unwrap() == -1 {
                        self.push(ValueLocation::Immediate((-1 as $signed_ty).into()));
                        self.free_value(dividend)?;
                        return Ok(());
                    }

                    false
                }
                ValueLocation::Reg(_) => {
                    let reg = self.into_reg(GPRType::Rq, &mut divisor).unwrap();
                    dynasm!(self.asm
                        ; cmp $reg_ty(reg.rq().unwrap()), -1
                    );
                    // TODO: We could choose `current_depth` as the depth here instead but we currently
                    //       don't for simplicity
                    self.set_stack_depth(current_depth.clone())?;
                    dynasm!(self.asm
                        ; je =>is_neg1.0
                    );

                    true
                }
                ValueLocation::Stack(offset) => {
                    let offset = self.adjusted_offset(offset);
                    dynasm!(self.asm
                        ; cmp $pointer_ty [rsp + offset], -1
                    );
                    self.set_stack_depth(current_depth.clone())?;
                    dynasm!(self.asm
                        ; je =>is_neg1.0
                    );

                    true
                }
                ValueLocation::Cond(_) => {
                    // `cc` can never be `-1`, only `0` and `1`
                    false
                }
            };

            let (div, rem, saved) = self.$full_div_s(divisor, dividend)?;

            self.free_value(div)?;

            let rem = match rem {
                ValueLocation::Reg(rem) => {
                    if saved.clone().any(|dst| dst == rem) {
                        let new = self.take_reg(I32).unwrap();
                        dynasm!(self.asm
                            ; mov Rq(new.rq().unwrap()), Rq(rem.rq().unwrap())
                        );
                        self.block_state.regs.release(rem)?;
                        ValueLocation::Reg(new)
                    } else {
                        ValueLocation::Reg(rem)
                    }
                }
                _ => rem,
            };

            self.cleanup_gprs(saved);

            if gen_neg1_case {
                let ret = self.create_label();
                self.set_stack_depth(current_depth.clone())?;
                dynasm!(self.asm
                    ; jmp =>ret.0
                );
                self.define_label(is_neg1);

                let dst_ccloc = match CCLoc::try_from(rem) {
                    None => {
                        return Err(Error::Microwasm(
                            "$rem_s Programmer error".to_string(),
                        ))
                    }
                    Some(o) => o,
                };

                self.copy_value(
                    ValueLocation::Immediate((0 as $signed_ty).into()),
                    dst_ccloc
                )?;

                self.set_stack_depth(current_depth.clone())?;
                self.define_label(ret);
            }

            self.push(rem);
            Ok(())
        }
    }
}

macro_rules! unop {
    ($name:ident, $instr:ident, $reg_ty:tt, $typ:ty, $const_fallback:expr) => {
        pub fn $name(&mut self) -> Result<(), Error>{
            let mut val = self.pop()?;

            let out_val = match val {
                ValueLocation::Immediate(imm) =>
                    ValueLocation::Immediate(
                        ($const_fallback(imm.as_int().unwrap() as $typ) as $typ).into()
                    ),
                ValueLocation::Stack(offset) => {
                    let offset = self.adjusted_offset(offset);
                    let temp = self.take_reg(Type::for_::<$typ>()).unwrap();
                    dynasm!(self.asm
                        ; $instr $reg_ty(temp.rq().unwrap()), [rsp + offset]
                    );
                    ValueLocation::Reg(temp)
                }
                ValueLocation::Reg(_) | ValueLocation::Cond(_) => {
                    let reg = self.into_reg(GPRType::Rq, &mut val).unwrap();
                    let temp = self.take_reg(Type::for_::<$typ>()).unwrap();
                    dynasm!(self.asm
                        ; $instr $reg_ty(temp.rq().unwrap()), $reg_ty(reg.rq().unwrap())
                    );
                    ValueLocation::Reg(temp)
                }
            };

            self.free_value(val)?;
            self.push(out_val);
            Ok(())
        }
    }
}

macro_rules! conversion {
    (
        $name:ident,
        $instr:ident,
        $in_reg_ty:tt,
        $in_reg_fn:ident,
        $out_reg_ty:tt,
        $out_reg_fn:ident,
        $in_typ:ty,
        $out_typ:ty,
        $const_ty_fn:ident,
        $const_fallback:expr
    ) => {
        pub fn $name(&mut self) -> Result<(), Error>{
            let mut val = self.pop()?;

            let out_val = match val {
                ValueLocation::Immediate(imm) =>
                    ValueLocation::Immediate(
                        $const_fallback(imm.$const_ty_fn().unwrap()).into()
                    ),
                ValueLocation::Stack(offset) => {
                    let offset = self.adjusted_offset(offset);
                    let temp = self.take_reg(Type::for_::<$out_typ>()).unwrap();
                    dynasm!(self.asm
                        ; $instr $out_reg_ty(temp.$out_reg_fn().unwrap()), [rsp + offset]
                    );

                    ValueLocation::Reg(temp)
                }
                ValueLocation::Reg(_) | ValueLocation::Cond(_) => {
                    let reg = self.into_reg(Type::for_::<$in_typ>(), &mut val).unwrap();
                    let temp = self.take_reg(Type::for_::<$out_typ>()).unwrap();

                    dynasm!(self.asm
                        ; $instr $out_reg_ty(temp.$out_reg_fn().unwrap()), $in_reg_ty(reg.$in_reg_fn().unwrap())
                    );

                    ValueLocation::Reg(temp)
                }
            };

            self.free_value(val)?;

            self.push(out_val);
            Ok(())
        }
    }
}

// TODO: Support immediate `count` parameters
macro_rules! shift {
    ($name:ident, $reg_ty:tt, $instr:ident, $const_fallback:expr, $ty:expr) => {
        pub fn $name(&mut self) -> Result<(), Error>{
            let mut count = self.pop()?;
            let mut val = self.pop()?;

            if let Some(imm) = count.immediate() {
                if let Some(imm) = imm.as_int() {
                    if let Ok(imm) = i8::try_from(imm) {
                        let reg = self.into_temp_reg($ty, &mut val).unwrap();

                        dynasm!(self.asm
                            ; $instr $reg_ty(reg.rq().unwrap()), imm
                        );
                        self.push(ValueLocation::Reg(reg));
                        return Ok(());
                    }
                }
            }

            if val == ValueLocation::Reg(RCX) {
                let new = self.take_reg($ty).unwrap();
                self.copy_value(val, CCLoc::Reg(new))?;
                self.free_value(val)?;
                val = ValueLocation::Reg(new);
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
                        let new_reg = self.take_reg(I32).unwrap();
                        dynasm!(self.asm
                            ; mov Rq(new_reg.rq().unwrap()), rcx
                        );
                        Some(new_reg)
                    };

                    match other {
                        ValueLocation::Reg(_) | ValueLocation::Cond(_) => {
                            let gpr = self.into_reg(I32, &mut count).unwrap();
                            dynasm!(self.asm
                                ; mov cl, Rb(gpr.rq().unwrap())
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
                                ; mov cl, imm.as_int().unwrap() as i8
                            );
                        }
                    }

                    out
                }
            };

            self.free_value(count)?;
            self.block_state.regs.mark_used(RCX);
            count = ValueLocation::Reg(RCX);

            let reg = self.into_temp_reg($ty, &mut val).unwrap();

            dynasm!(self.asm
                ; $instr $reg_ty(reg.rq().unwrap()), cl
            );

            self.free_value(count)?;

            if let Some(gpr) = temp_rcx {
                dynasm!(self.asm
                    ; mov rcx, Rq(gpr.rq().unwrap())
                );
                self.block_state.regs.release(gpr)?;
            }

            self.push(val);
            Ok(())
        }
    }
}

macro_rules! cmp_i32 {
    ($name:ident, $flags:expr, $reverse_flags:expr, $const_fallback:expr) => {
        pub fn $name(&mut self) -> Result<(), Error>{
            let mut right = self.pop()?;
            let mut left = self.pop()?;

            let out = if let Some(i) = left.imm_i32() {
                match right {
                    ValueLocation::Stack(offset) => {
                        let offset = self.adjusted_offset(offset);

                        dynasm!(self.asm
                            ; cmp DWORD [rsp + offset], i
                        );
                        ValueLocation::Cond($reverse_flags)
                    }
                    ValueLocation::Reg(_) | ValueLocation::Cond(_) => {
                        let rreg = self.into_reg(I32, &mut right).unwrap();
                        dynasm!(self.asm
                            ; cmp Rd(rreg.rq().unwrap()), i
                        );
                        ValueLocation::Cond($reverse_flags)
                    }
                    ValueLocation::Immediate(right) => {
                        ValueLocation::Immediate(
                            (if $const_fallback(i, right.as_i32().unwrap()) {
                                1i32
                            } else {
                                0i32
                            }).into()
                        )
                    }
                }
            } else {
                let lreg = self.into_reg(I32, &mut left).unwrap();

                match right {
                    ValueLocation::Stack(offset) => {
                        let offset = self.adjusted_offset(offset);
                        dynasm!(self.asm
                            ; cmp Rd(lreg.rq().unwrap()), [rsp + offset]
                        );
                    }
                    ValueLocation::Reg(_) | ValueLocation::Cond(_) => {
                        let rreg = self.into_reg(I32, &mut right).unwrap();
                        dynasm!(self.asm
                            ; cmp Rd(lreg.rq().unwrap()), Rd(rreg.rq().unwrap())
                        );
                    }
                    ValueLocation::Immediate(i) => {
                        dynasm!(self.asm
                            ; cmp Rd(lreg.rq().unwrap()), i.as_i32().unwrap()
                        );
                    }
                }

                ValueLocation::Cond($flags)
            };

            self.free_value(left)?;
            self.free_value(right)?;

            self.push(out);
            Ok(())
        }
    }
}

macro_rules! cmp_i64 {
    ($name:ident, $flags:expr, $reverse_flags:expr, $const_fallback:expr) => {
        pub fn $name(&mut self) -> Result<(), Error> {
            let mut right = self.pop()?;
            let mut left = self.pop()?;

            let out = if let Some(i) = left.imm_i64() {
                match right {
                    ValueLocation::Stack(offset) => {
                        let offset = self.adjusted_offset(offset);
                        if let Some(i) = i.try_into().ok() {
                            dynasm!(self.asm
                                ; cmp QWORD [rsp + offset], i
                            );
                        } else {
                            let lreg = self.into_reg(I32, &mut left).unwrap();
                            dynasm!(self.asm
                                ; cmp QWORD [rsp + offset], Rq(lreg.rq().unwrap())
                            );
                        }
                        ValueLocation::Cond($reverse_flags)
                    }
                    ValueLocation::Reg(_) | ValueLocation::Cond(_) => {
                        let rreg = self.into_reg(I32, &mut right).unwrap();
                        if let Some(i) = i.try_into().ok() {
                            dynasm!(self.asm
                                ; cmp Rq(rreg.rq().unwrap()), i
                            );
                        } else {
                            let lreg = self.into_reg(I32, &mut left).unwrap();
                            dynasm!(self.asm
                                ; cmp Rq(rreg.rq().unwrap()), Rq(lreg.rq().unwrap())
                            );
                        }
                        ValueLocation::Cond($reverse_flags)
                    }
                    ValueLocation::Immediate(right) => {
                        ValueLocation::Immediate(
                            (if $const_fallback(i, right.as_i64().unwrap()) {
                                1i32
                            } else {
                                0i32
                            }).into()
                        )
                    }
                }
            } else {
                let lreg = self.into_reg(I64, &mut left).unwrap();

                match right {
                    ValueLocation::Stack(offset) => {
                        let offset = self.adjusted_offset(offset);
                        dynasm!(self.asm
                            ; cmp Rq(lreg.rq().unwrap()), [rsp + offset]
                        );
                    }
                    ValueLocation::Reg(_) | ValueLocation::Cond(_) => {
                        let rreg = self.into_reg(I32, &mut right).unwrap();
                        dynasm!(self.asm
                            ; cmp Rq(lreg.rq().unwrap()), Rq(rreg.rq().unwrap())
                        );
                    }
                    ValueLocation::Immediate(i) => {
                        let i = i.as_i64().unwrap();
                        if let Some(i) = i.try_into().ok() {
                            dynasm!(self.asm
                                    ; cmp Rq(lreg.rq().unwrap()), i
                            );
                        } else {
                            let rreg = self.into_reg(I32, &mut right).unwrap();
                            dynasm!(self.asm
                                ; cmp Rq(lreg.rq().unwrap()), Rq(rreg.rq().unwrap())
                            );
                        }
                    }
                }

                ValueLocation::Cond($flags)
            };

            self.free_value(left)?;
            self.free_value(right)?;
            self.push(out);
            Ok(())
        }
    }
}

macro_rules! cmp_f32 {
    ($name:ident, $reverse_name:ident, $instr:ident, $const_fallback:expr) => {
        cmp_float!(
            comiss,
            f32,
            imm_f32,
            $name,
            $reverse_name,
            $instr,
            $const_fallback
        );
    };
}

macro_rules! eq_float {
    ($name:ident, $instr:ident, $imm_fn:ident, $const_fallback:expr) => {
        pub fn $name(&mut self) -> Result<(), Error>{
            let right = self.pop()?;
            let left = self.pop()?;

            if let Some(right) = right.immediate() {
                if let Some(left) = left.immediate() {
                    self.push(ValueLocation::Immediate(
                        if $const_fallback(left.$imm_fn().unwrap(), right.$imm_fn().unwrap()) {
                            1u32
                        } else {
                            0
                        }.into()
                    ));
                    return Ok(());
                }
            }

            let (mut left, mut right) = match left {
                ValueLocation::Reg(r) if self.block_state.regs.num_usages(r) <= 1 => (left, right),
                _ =>  (right, left)
            };

            let lreg = self.into_temp_reg(GPRType::Rx, &mut left).unwrap();
            let rreg = self.into_reg(GPRType::Rx, &mut right).unwrap();
            let out = self.take_reg(I32).unwrap();

            dynasm!(self.asm
                ; $instr Rx(lreg.rx().unwrap()), Rx(rreg.rx().unwrap())
                ; movd Rd(out.rq().unwrap()), Rx(lreg.rx().unwrap())
                ; and Rd(out.rq().unwrap()), 1
            );

            self.push(ValueLocation::Reg(out));
            self.free_value(left)?;
            self.free_value(right)?;
            Ok(())
        }

    }
}

macro_rules! minmax_float {
    (
        $name:ident,
        $instr:ident,
        $cmpinstr:ident,
        $addinstr:ident,
        $combineinstr:ident,
        $imm_fn:ident,
        $const_fallback:expr
    ) => {
        pub fn $name(&mut self) -> Result<(), Error>{
            let right = self.pop()?;
            let left = self.pop()?;

            if let Some(right) = right.immediate() {
                if let Some(left) = left.immediate() {
                    self.push(ValueLocation::Immediate(
                        $const_fallback(left.$imm_fn().unwrap(), right.$imm_fn().unwrap()).into()
                    ));
                    return Ok(());
                }
            }

            let (mut left, mut right) = match left {
                ValueLocation::Reg(r) if self.block_state.regs.num_usages(r) <= 1 => (left, right),
                _ =>  (right, left)
            };

            let lreg = self.into_temp_reg(GPRType::Rx, &mut left).unwrap();
            let rreg = self.into_reg(GPRType::Rx, &mut right).unwrap();

            dynasm!(self.asm
                ; $cmpinstr Rx(lreg.rx().unwrap()), Rx(rreg.rx().unwrap())
                ; je >equal
                ; $instr Rx(lreg.rx().unwrap()), Rx(rreg.rx().unwrap())
                ; jmp >ret
            ; equal:
                ; jnp >equal_but_not_parity
                ; $addinstr Rx(lreg.rx().unwrap()), Rx(rreg.rx().unwrap())
                ; jmp >ret
            ; equal_but_not_parity:
                ; $combineinstr Rx(lreg.rx().unwrap()), Rx(rreg.rx().unwrap())
            ; ret:
            );

            self.push(left);
            self.free_value(right)?;
            Ok(())
        }

    }
}

macro_rules! cmp_f64 {
    ($name:ident, $reverse_name:ident, $instr:ident, $const_fallback:expr) => {
        cmp_float!(
            comisd,
            f64,
            imm_f64,
            $name,
            $reverse_name,
            $instr,
            $const_fallback
        );
    };
}

macro_rules! cmp_float {
    (@helper $cmp_instr:ident, $ty:ty, $imm_fn:ident, $self:expr, $left:expr, $right:expr, $instr:ident, $const_fallback:expr) => {{
        let (left, right, this) = ($left, $right, $self);
        if let (Some(left), Some(right)) = (left.$imm_fn(), right.$imm_fn()) {
            if $const_fallback(<$ty>::from_bits(left.to_bits()), <$ty>::from_bits(right.to_bits())) {
                ValueLocation::Immediate(1i32.into())
            } else {
                ValueLocation::Immediate(0i32.into())
            }
        } else {
            let lreg = this.into_reg(GPRType::Rx, left).unwrap();
            let result = this.take_reg(I32).unwrap();

            match right {
                ValueLocation::Stack(offset) => {
                    let offset = this.adjusted_offset(*offset);

                    dynasm!(this.asm
                        ; xor Rq(result.rq().unwrap()), Rq(result.rq().unwrap())
                        ; $cmp_instr Rx(lreg.rx().unwrap()), [rsp + offset]
                        ; $instr Rb(result.rq().unwrap())
                    );
                }
                right => {
                    let rreg = this.into_reg(GPRType::Rx, right).unwrap();

                    dynasm!(this.asm
                        ; xor Rq(result.rq().unwrap()), Rq(result.rq().unwrap())
                        ; $cmp_instr Rx(lreg.rx().unwrap()), Rx(rreg.rx().unwrap())
                        ; $instr Rb(result.rq().unwrap())
                    );
                }
            }

            ValueLocation::Reg(result)
        }
    }};
    ($cmp_instr:ident, $ty:ty, $imm_fn:ident, $name:ident, $reverse_name:ident, $instr:ident, $const_fallback:expr) => {
        pub fn $name(&mut self) -> Result<(), Error> {
            let mut right = self.pop()?;
            let mut left = self.pop()?;

            let out = cmp_float!(@helper
                $cmp_instr,
                $ty,
                $imm_fn,
                &mut *self,
                &mut left,
                &mut right,
                $instr,
                $const_fallback
            );

            self.free_value(left)?;
            self.free_value(right)?;

            self.push(out);
            Ok(())
        }

        pub fn $reverse_name(&mut self) -> Result<(), Error> {
            let mut right = self.pop()?;
            let mut left = self.pop()?;

            let out = cmp_float!(@helper
                $cmp_instr,
                $ty,
                $imm_fn,
                &mut *self,
                &mut right,
                &mut left,
                $instr,
                $const_fallback
            );

            self.free_value(left)?;
            self.free_value(right)?;

            self.push(out);
            Ok(())
        }
    };
}

macro_rules! binop_i32 {
    ($name:ident, $instr:ident, $const_fallback:expr) => {
        binop!(
            $name,
            $instr,
            $const_fallback,
            Rd,
            rq,
            I32,
            imm_i32,
            |this: &mut Context<_>, op1: GPR, i| dynasm!(this.asm
                ; $instr Rd(op1.rq().unwrap()), i
            )
        );
    };
}

macro_rules! commutative_binop_i32 {
    ($name:ident, $instr:ident, $const_fallback:expr) => {
        commutative_binop!(
            $name,
            $instr,
            $const_fallback,
            Rd,
            rq,
            I32,
            imm_i32,
            |this: &mut Context<_>, op1: GPR, i| dynasm!(this.asm
                ; $instr Rd(op1.rq().unwrap()), i
            )
        );
    };
}

macro_rules! binop_i64 {
    ($name:ident, $instr:ident, $const_fallback:expr) => {
        binop!(
            $name,
            $instr,
            $const_fallback,
            Rq,
            rq,
            I64,
            imm_i64,
            |this: &mut Context<_>, op1: GPR, i| dynasm!(this.asm
                ; $instr Rq(op1.rq().unwrap()), i
            )
        );
    };
}

macro_rules! commutative_binop_i64 {
    ($name:ident, $instr:ident, $const_fallback:expr) => {
        commutative_binop!(
            $name,
            $instr,
            $const_fallback,
            Rq,
            rq,
            I64,
            imm_i64,
            |this: &mut Context<_>, op1: GPR, i| dynasm!(this.asm
                ; $instr Rq(op1.rq().unwrap()), i
            )
        );
    };
}

macro_rules! binop_f32 {
    ($name:ident, $instr:ident, $const_fallback:expr) => {
        binop!(
            $name,
            $instr,
            |a: Ieee32, b: Ieee32| Ieee32::from_bits(
                $const_fallback(f32::from_bits(a.to_bits()), f32::from_bits(b.to_bits())).to_bits()
            ),
            Rx,
            rx,
            F32,
            imm_f32,
            |_, _, _: i32| unreachable!()
        );
    };
}

macro_rules! commutative_binop_f32 {
    ($name:ident, $instr:ident, $const_fallback:expr) => {
        commutative_binop!(
            $name,
            $instr,
            |a: Ieee32, b: Ieee32| Ieee32::from_bits(
                $const_fallback(f32::from_bits(a.to_bits()), f32::from_bits(b.to_bits())).to_bits()
            ),
            Rx,
            rx,
            F32,
            imm_f32,
            |_, _, _: i32| unreachable!()
        );
    };
}

macro_rules! binop_f64 {
    ($name:ident, $instr:ident, $const_fallback:expr) => {
        binop!(
            $name,
            $instr,
            |a: Ieee64, b: Ieee64| Ieee64::from_bits(
                $const_fallback(f64::from_bits(a.to_bits()), f64::from_bits(b.to_bits())).to_bits()
            ),
            Rx,
            rx,
            F64,
            imm_f64,
            |_, _, _: i32| unreachable!()
        );
    };
}

macro_rules! commutative_binop_f64 {
    ($name:ident, $instr:ident, $const_fallback:expr) => {
        commutative_binop!(
            $name,
            $instr,
            |a: Ieee64, b: Ieee64| Ieee64::from_bits(
                $const_fallback(f64::from_bits(a.to_bits()), f64::from_bits(b.to_bits())).to_bits()
            ),
            Rx,
            rx,
            F64,
            imm_f64,
            |_, _, _: i32| unreachable!()
        );
    };
}
macro_rules! commutative_binop {
    ($name:ident, $instr:ident, $const_fallback:expr, $reg_ty:tt, $reg_fn:ident, $ty:expr, $imm_fn:ident, $direct_imm:expr) => {
        binop!(
            $name,
            $instr,
            $const_fallback,
            $reg_ty,
            $reg_fn,
            $ty,
            $imm_fn,
            $direct_imm,
            |op1: ValueLocation, op0: ValueLocation| match op1 {
                ValueLocation::Reg(_) => (op1, op0),
                _ => {
                    if op0.immediate().is_some() {
                        (op1, op0)
                    } else {
                        (op0, op1)
                    }
                }
            }
        );
    };
}

macro_rules! binop {
    ($name:ident, $instr:ident, $const_fallback:expr, $reg_ty:tt, $reg_fn:ident, $ty:expr, $imm_fn:ident, $direct_imm:expr) => {
        binop!($name, $instr, $const_fallback, $reg_ty, $reg_fn, $ty, $imm_fn, $direct_imm, |a, b| (a, b));
    };
    ($name:ident, $instr:ident, $const_fallback:expr, $reg_ty:tt, $reg_fn:ident, $ty:expr, $imm_fn:ident, $direct_imm:expr, $map_op:expr) => {
        pub fn $name(&mut self) -> Result<(), Error> {
            let right = self.pop()?;
            let left = self.pop()?;

            if let Some(i1) = left.$imm_fn() {
                if let Some(i0) = right.$imm_fn() {
                    self.block_state.stack.push(ValueLocation::Immediate($const_fallback(i1, i0).into()));
                    return Ok(());
                }
            }

            let (mut left, mut right) = $map_op(left, right);
            let lreg = self.into_temp_reg($ty, &mut left).unwrap();

            match right {
                ValueLocation::Reg(_) | ValueLocation::Cond(_) => {
                    // This handles the case where we (for example) have a float in an `Rq` reg
                    let right_reg = self.into_reg($ty, &mut right).unwrap();
                    dynasm!(self.asm
                        ; $instr $reg_ty(lreg.$reg_fn().unwrap()), $reg_ty(right_reg.$reg_fn().unwrap())
                    );
                }
                ValueLocation::Stack(offset) => {
                    let offset = self.adjusted_offset(offset);
                    dynasm!(self.asm
                        ; $instr $reg_ty(lreg.$reg_fn().unwrap()), [rsp + offset]
                    );
                }
                ValueLocation::Immediate(i) => {
                    if let Some(i) = i.as_int().and_then(|i| i.try_into().ok()) {
                        $direct_imm(&mut *self, lreg, i);
                    } else {
                        let scratch = self.take_reg($ty).unwrap();
                        self.immediate_to_reg(scratch, i);

                        dynasm!(self.asm
                            ; $instr $reg_ty(lreg.$reg_fn().unwrap()), $reg_ty(scratch.$reg_fn().unwrap())
                        );

                        self.block_state.regs.release(scratch)?;
                    }
                }
            }

            self.free_value(right)?;
            self.push(left);
            Ok(())
        }
    }
}

macro_rules! load {
    (@inner $name:ident, $rtype:expr, $reg_ty:tt, $emit_fn:expr) => {
        pub fn $name(&mut self, offset: u32) -> Result<(), Error> {
            fn load_to_reg<_M: ModuleContext>(
                ctx: &mut Context<_M>,
                dst: GPR,
                (offset, runtime_offset): (i32, Result<i32, GPR>)
            ) -> Result<(), Error> {
                let mem_index = 0;
                let reg_offset = ctx.module_context
                    .defined_memory_index(mem_index)
                    .map(|index| (
                        None,
                        ctx.module_context.vmctx_vmmemory_definition(index) as i32
                    ));
                let (reg, mem_offset) = reg_offset.unwrap_or_else(|| {
                    let reg = ctx.take_reg(I64).unwrap();

                    dynasm!(ctx.asm
                        ; mov Rq(reg.rq().unwrap()), [
                            Rq(VMCTX) + ctx.module_context.vmctx_vmmemory_import_from(mem_index) as i32
                        ]
                    );

                    (Some(reg), 0)
                });

                let vmctx = GPR::Rq(VMCTX);

                if ctx.module_context.emit_memory_bounds_check() {
                    let trap_label = ctx.trap_label();
                    let addr_reg = match runtime_offset {
                        Ok(imm) => {
                            let addr_reg = ctx.take_reg(I64).unwrap();
                            dynasm!(ctx.asm
                                ; mov Rq(addr_reg.rq().unwrap()), QWORD imm as i64 + offset as i64
                            );
                            addr_reg
                        }
                        Err(gpr) => {
                            if offset == 0 {
                                ctx.to_reg(I32, ValueLocation::Reg(gpr)).unwrap()
                            } else if offset > 0 {
                                let addr_reg = ctx.take_reg(I64).unwrap();
                                dynasm!(ctx.asm
                                    ; lea Rq(addr_reg.rq().unwrap()), [Rq(gpr.rq().unwrap()) + offset]
                                );
                                addr_reg
                            } else {
                                let addr_reg = ctx.take_reg(I64).unwrap();
                                let offset_reg = ctx.take_reg(I64).unwrap();
                                dynasm!(ctx.asm
                                    ; mov Rd(offset_reg.rq().unwrap()), offset
                                    ; mov Rq(addr_reg.rq().unwrap()), Rq(gpr.rq().unwrap())
                                    ; add Rq(addr_reg.rq().unwrap()), Rq(offset_reg.rq().unwrap())
                                );
                                ctx.block_state.regs.release(offset_reg)?;
                                addr_reg
                            }
                        }
                    };
                    dynasm!(ctx.asm
                        ; cmp [
                            Rq(reg.unwrap_or(vmctx).rq().unwrap()) +
                                mem_offset +
                                ctx.module_context.vmmemory_definition_current_length() as i32
                        ], Rq(addr_reg.rq().unwrap())
                        ; jna =>trap_label.0
                    );
                    ctx.block_state.regs.release(addr_reg)?;
                }

                let mem_ptr_reg = ctx.take_reg(I64).unwrap();
                dynasm!(ctx.asm
                    ; mov Rq(mem_ptr_reg.rq().unwrap()), [
                        Rq(reg.unwrap_or(vmctx).rq().unwrap()) +
                            mem_offset +
                            ctx.module_context.vmmemory_definition_base() as i32
                    ]
                );
                if let Some(reg) = reg {
                    ctx.block_state.regs.release(reg)?;
                }
                $emit_fn(ctx, dst, mem_ptr_reg, runtime_offset, offset);
                ctx.block_state.regs.release(mem_ptr_reg)?;
                Ok(())
            }

            let base = self.pop()?;

            let temp = self.take_reg($rtype).unwrap();

            match base {
                ValueLocation::Immediate(i) => {
                    load_to_reg(self, temp, (offset as _, Ok(i.as_i32().unwrap())))?;
                }
                mut base => {
                    let gpr = self.into_reg(I32, &mut base).unwrap();
                    load_to_reg(self, temp, (offset as _, Err(gpr)))?;
                    self.free_value(base)?;
                }
            }

            self.push(ValueLocation::Reg(temp));
            Ok(())
        }
    };
    ($name:ident, $rtype:expr, $reg_ty:tt, NONE, $rq_instr:ident, $ty:ident) => {
        load!(@inner
            $name,
            $rtype,
            $reg_ty,
            |ctx: &mut Context<_>, dst: GPR, mem_ptr_reg: GPR, runtime_offset: Result<i32, GPR>, offset: i32| {
                match runtime_offset {
                    Ok(imm) => {
                        dynasm!(ctx.asm
                            ; $rq_instr $reg_ty(dst.rq().unwrap()), $ty [Rq(mem_ptr_reg.rq().unwrap()) + offset + imm]
                        );
                    }
                    Err(offset_reg) => {
                        dynasm!(ctx.asm
                            ; $rq_instr $reg_ty(dst.rq().unwrap()), $ty [Rq(mem_ptr_reg.rq().unwrap()) + Rq(offset_reg.rq().unwrap()) + offset]
                        );
                    }
                }
            }
        );
    };
    ($name:ident, $rtype:expr, $reg_ty:tt, $xmm_instr:ident, $rq_instr:ident, $ty:ident) => {
        load!(@inner
            $name,
            $rtype,
            $reg_ty,
            |ctx: &mut Context<_>, dst: GPR, mem_ptr_reg: GPR, runtime_offset: Result<i32, GPR>, offset: i32| {
                match (dst, runtime_offset) {
                    (GPR::Rq(r), Ok(imm)) => {
                        dynasm!(ctx.asm
                            ; $rq_instr $reg_ty(r), $ty [Rq(mem_ptr_reg.rq().unwrap()) + offset + imm]
                        );
                    }
                    (GPR::Rx(r), Ok(imm)) => {
                        if let Some(combined) = offset.checked_add(imm) {
                            dynasm!(ctx.asm
                                ; $xmm_instr Rx(r), $ty [Rq(mem_ptr_reg.rq().unwrap()) + combined]
                            );
                        } else {
                            let offset_reg = ctx.take_reg(GPRType::Rq).unwrap();
                            dynasm!(ctx.asm
                                ; mov Rq(offset_reg.rq().unwrap()), offset
                                ; $xmm_instr Rx(r), $ty [
                                    Rq(mem_ptr_reg.rq().unwrap()) +
                                    Rq(offset_reg.rq().unwrap()) +
                                    imm
                                ]
                            );
                            ctx.block_state.regs.release(offset_reg);
                        }
                    }
                    (GPR::Rq(r), Err(offset_reg)) => {
                        dynasm!(ctx.asm
                            ; $rq_instr $reg_ty(r), $ty [Rq(mem_ptr_reg.rq().unwrap()) + Rq(offset_reg.rq().unwrap()) + offset]
                        );
                    }
                    (GPR::Rx(r), Err(offset_reg)) => {
                        dynasm!(ctx.asm
                            ; $xmm_instr Rx(r), $ty [Rq(mem_ptr_reg.rq().unwrap()) + Rq(offset_reg.rq().unwrap()) + offset]
                        );
                    }
                }
            }
        );
    };
}

macro_rules! store {
    (@inner $name:ident, $int_reg_ty:tt, $match_offset:expr, $size:ident) => {
        pub fn $name(&mut self, offset: u32) -> Result<(), Error>{
            fn store_from_reg<_M: ModuleContext>(
                ctx: &mut Context<_M>,
                src: GPR,
                (offset, runtime_offset): (i32, Result<i32, GPR>)
            ) -> Result<(), Error> {
                let mem_index = 0;
                let reg_offset = ctx.module_context
                    .defined_memory_index(mem_index)
                    .map(|index| (
                        None,
                        ctx.module_context.vmctx_vmmemory_definition(index) as i32
                    ));
                let (reg, mem_offset) = reg_offset.unwrap_or_else(|| {
                    let reg = ctx.take_reg(I64).unwrap();

                    dynasm!(ctx.asm
                        ; mov Rq(reg.rq().unwrap()), [
                            Rq(VMCTX) + ctx.module_context.vmctx_vmmemory_import_from(mem_index) as i32
                        ]
                    );

                    (Some(reg), 0)
                });

                let vmctx = GPR::Rq(VMCTX);

                if ctx.module_context.emit_memory_bounds_check() {
                    let trap_label = ctx.trap_label();
                    let addr_reg = match runtime_offset {
                        Ok(imm) => {
                            let addr_reg = ctx.take_reg(I64).unwrap();
                            dynasm!(ctx.asm
                                ; mov Rq(addr_reg.rq().unwrap()), QWORD imm as i64 + offset as i64
                            );
                            addr_reg
                        }
                        Err(gpr) => {
                            if offset == 0 {
                                ctx.to_reg(I32, ValueLocation::Reg(gpr)).unwrap()
                            } else if offset > 0 {
                                let addr_reg = ctx.take_reg(I64).unwrap();
                                dynasm!(ctx.asm
                                    ; lea Rq(addr_reg.rq().unwrap()), [Rq(gpr.rq().unwrap()) + offset]
                                );
                                addr_reg
                            } else {
                                let addr_reg = ctx.take_reg(I64).unwrap();
                                let offset_reg = ctx.take_reg(I64).unwrap();
                                dynasm!(ctx.asm
                                    ; mov Rd(offset_reg.rq().unwrap()), offset
                                    ; mov Rq(addr_reg.rq().unwrap()), Rq(gpr.rq().unwrap())
                                    ; add Rq(addr_reg.rq().unwrap()), Rq(offset_reg.rq().unwrap())
                                );
                                ctx.block_state.regs.release(offset_reg)?;
                                addr_reg
                            }
                        }
                    };
                    dynasm!(ctx.asm
                        ; cmp Rq(addr_reg.rq().unwrap()), [
                            Rq(reg.unwrap_or(vmctx).rq().unwrap()) +
                                mem_offset +
                                ctx.module_context.vmmemory_definition_current_length() as i32
                        ]
                        ; jae =>trap_label.0
                    );
                    ctx.block_state.regs.release(addr_reg)?;
                }

                let mem_ptr_reg = ctx.take_reg(I64).unwrap();
                dynasm!(ctx.asm
                    ; mov Rq(mem_ptr_reg.rq().unwrap()), [
                        Rq(reg.unwrap_or(vmctx).rq().unwrap()) +
                            mem_offset +
                            ctx.module_context.vmmemory_definition_base() as i32
                    ]
                );
                if let Some(reg) = reg {
                    ctx.block_state.regs.release(reg)?;
                }
                let src = $match_offset(ctx, mem_ptr_reg, runtime_offset, offset, src);
                ctx.block_state.regs.release(mem_ptr_reg)?;
                ctx.block_state.regs.release(src)?;
                Ok(())
            }

            assert_le!(offset, i32::max_value() as u32);

            let mut src = self.pop()?;
            let base = self.pop()?;

            // `store_from_reg` frees `src`
            // TODO: Would it be better to free it outside `store_from_reg`?
            let src_reg = self.into_reg(None, &mut src).unwrap();

            match base {
                ValueLocation::Immediate(i) => {
                    store_from_reg(self, src_reg, (offset as i32, Ok(i.as_i32().unwrap())))?
                }
                mut base => {
                    let gpr = self.into_reg(I32, &mut base).unwrap();
                    store_from_reg(self, src_reg, (offset as i32, Err(gpr)))?;
                    self.free_value(base)?;
                }
            }
            Ok(())
        }
    };
    ($name:ident, $int_reg_ty:tt, NONE, $size:ident) => {
        store!(@inner
            $name,
            $int_reg_ty,
            |ctx: &mut Context<_>, mem_ptr_reg: GPR, runtime_offset: Result<i32, GPR>, offset: i32, src| {
                let src_reg = ctx.into_temp_reg(GPRType::Rq, &mut ValueLocation::Reg(src)).unwrap();

                match runtime_offset {
                    Ok(imm) => {
                        dynasm!(ctx.asm
                            ; mov [Rq(mem_ptr_reg.rq().unwrap()) + offset + imm], $int_reg_ty(src_reg.rq().unwrap())
                        );
                    }
                    Err(offset_reg) => {
                        dynasm!(ctx.asm
                            ; mov [Rq(mem_ptr_reg.rq().unwrap()) + Rq(offset_reg.rq().unwrap()) + offset], $int_reg_ty(src_reg.rq().unwrap())
                        );
                    }
                }

                src_reg
            },
            $size
        );
    };
    ($name:ident, $int_reg_ty:tt, $xmm_instr:ident, $size:ident) => {
        store!(@inner
            $name,
            $int_reg_ty,
            |ctx: &mut Context<_>, mem_ptr_reg: GPR, runtime_offset: Result<i32, GPR>, offset: i32, src| {
                match (runtime_offset, src) {
                    (Ok(imm), GPR::Rq(r)) => {
                        dynasm!(ctx.asm
                            ; mov [Rq(mem_ptr_reg.rq().unwrap()) + offset + imm], $int_reg_ty(r)
                        );
                    }
                    (Ok(imm), GPR::Rx(r)) => {
                        dynasm!(ctx.asm
                            ; $xmm_instr [Rq(mem_ptr_reg.rq().unwrap()) + offset + imm], Rx(r)
                        );
                    }
                    (Err(offset_reg), GPR::Rq(r)) => {
                        dynasm!(ctx.asm
                            ; mov [Rq(mem_ptr_reg.rq().unwrap()) + Rq(offset_reg.rq().unwrap()) + offset], $int_reg_ty(r)
                        );
                    }
                    (Err(offset_reg), GPR::Rx(r)) => {
                        dynasm!(ctx.asm
                            ; $xmm_instr [Rq(mem_ptr_reg.rq().unwrap()) + Rq(offset_reg.rq().unwrap()) + offset], Rx(r)
                        );
                    }
                }

                src
            },
            $size
        );
    };
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualCallingConvention {
    pub stack: Stack,
    pub depth: StackDepth,
}

impl<'this, M: ModuleContext> Context<'this, M> {
    fn free_reg(&mut self, type_: GPRType) -> Result<bool, Error> {
        let pos = if let Some(pos) = self
            .block_state
            .stack
            .iter()
            .position(|r| r.reg().map(|reg| reg.type_() == type_).unwrap_or(false))
        {
            pos
        } else {
            return Ok(false);
        };

        let old_loc = self.block_state.stack[pos];
        let new_loc = self.push_physical(old_loc)?;
        self.block_state.stack[pos] = new_loc;

        let reg = old_loc.reg().unwrap();

        for elem in &mut self.block_state.stack[pos + 1..] {
            if *elem == old_loc {
                *elem = new_loc;
                self.block_state.regs.release(reg)?;
            }
        }

        Ok(true)
    }

    fn take_reg(&mut self, r: impl Into<GPRType>) -> Option<GPR> {
        let r = r.into();
        loop {
            if let Some(gpr) = self.block_state.regs.take(r) {
                break Some(gpr);
            }

            if self.free_reg(r) == Ok(false) {
                break None;
            }
        }
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

    cmp_i32!(i32_eq, cc::EQUAL, cc::EQUAL, |a, b| a == b);
    cmp_i32!(i32_neq, cc::NOT_EQUAL, cc::NOT_EQUAL, |a, b| a != b);
    // `dynasm-rs` inexplicably doesn't support setb but `setnae` (and `setc`) are synonymous
    cmp_i32!(i32_lt_u, cc::LT_U, cc::GT_U, |a, b| (a as u32) < (b as u32));
    cmp_i32!(i32_le_u, cc::LE_U, cc::GE_U, |a, b| (a as u32)
        <= (b as u32));
    cmp_i32!(i32_gt_u, cc::GT_U, cc::LT_U, |a, b| (a as u32) > (b as u32));
    cmp_i32!(i32_ge_u, cc::GE_U, cc::LE_U, |a, b| (a as u32)
        >= (b as u32));
    cmp_i32!(i32_lt_s, cc::LT_S, cc::GT_S, |a, b| a < b);
    cmp_i32!(i32_le_s, cc::LE_S, cc::GE_S, |a, b| a <= b);
    cmp_i32!(i32_gt_s, cc::GT_S, cc::LT_S, |a, b| a > b);
    cmp_i32!(i32_ge_s, cc::GE_S, cc::LE_S, |a, b| a >= b);

    cmp_i64!(i64_eq, cc::EQUAL, cc::EQUAL, |a, b| a == b);
    cmp_i64!(i64_neq, cc::NOT_EQUAL, cc::NOT_EQUAL, |a, b| a != b);
    // `dynasm-rs` inexplicably doesn't support setb but `setnae` (and `setc`) are synonymous
    cmp_i64!(i64_lt_u, cc::LT_U, cc::GT_U, |a, b| (a as u64) < (b as u64));
    cmp_i64!(i64_le_u, cc::LE_U, cc::GE_U, |a, b| (a as u64)
        <= (b as u64));
    cmp_i64!(i64_gt_u, cc::GT_U, cc::LT_U, |a, b| (a as u64) > (b as u64));
    cmp_i64!(i64_ge_u, cc::GE_U, cc::LE_U, |a, b| (a as u64)
        >= (b as u64));
    cmp_i64!(i64_lt_s, cc::LT_S, cc::GT_S, |a, b| a < b);
    cmp_i64!(i64_le_s, cc::LE_S, cc::GE_S, |a, b| a <= b);
    cmp_i64!(i64_gt_s, cc::GT_S, cc::LT_S, |a, b| a > b);
    cmp_i64!(i64_ge_s, cc::GE_S, cc::LE_S, |a, b| a >= b);

    cmp_f32!(f32_gt, f32_lt, seta, |a, b| a > b);
    cmp_f32!(f32_ge, f32_le, setnc, |a, b| a >= b);
    eq_float!(
        f32_eq,
        cmpeqss,
        as_f32,
        |a: Ieee32, b: Ieee32| f32::from_bits(a.to_bits()) == f32::from_bits(b.to_bits())
    );
    eq_float!(
        f32_ne,
        cmpneqss,
        as_f32,
        |a: Ieee32, b: Ieee32| f32::from_bits(a.to_bits()) != f32::from_bits(b.to_bits())
    );

    cmp_f64!(f64_gt, f64_lt, seta, |a, b| a > b);
    cmp_f64!(f64_ge, f64_le, setnc, |a, b| a >= b);
    eq_float!(
        f64_eq,
        cmpeqsd,
        as_f64,
        |a: Ieee64, b: Ieee64| f64::from_bits(a.to_bits()) == f64::from_bits(b.to_bits())
    );
    eq_float!(
        f64_ne,
        cmpneqsd,
        as_f64,
        |a: Ieee64, b: Ieee64| f64::from_bits(a.to_bits()) != f64::from_bits(b.to_bits())
    );

    // TODO: Should we do this logic in `eq` and just have this delegate to `eq`?
    //       That would mean that `eqz` and `eq` with a const 0 argument don't
    //       result in different code. It would also allow us to generate better
    //       code for `neq` and `gt_u` with const 0 operand
    pub fn i32_eqz(&mut self) -> Result<(), Error> {
        let mut val = self.pop()?;

        if let ValueLocation::Immediate(Value::I32(i)) = val {
            self.push(ValueLocation::Immediate(
                (if i == 0 { 1i32 } else { 0 }).into(),
            ));
            return Ok(());
        }

        if let ValueLocation::Cond(loc) = val {
            self.push(ValueLocation::Cond(!loc));
            return Ok(());
        }

        let reg = self.into_reg(I32, &mut val).unwrap();
        let out = self.take_reg(I32).unwrap();

        dynasm!(self.asm
            ; xor Rd(out.rq().unwrap()), Rd(out.rq().unwrap())
            ; test Rd(reg.rq().unwrap()), Rd(reg.rq().unwrap())
            ; setz Rb(out.rq().unwrap())
        );

        self.free_value(val)?;

        self.push(ValueLocation::Reg(out));
        Ok(())
    }

    pub fn i64_eqz(&mut self) -> Result<(), Error> {
        let mut val = self.pop()?;

        if let ValueLocation::Immediate(Value::I64(i)) = val {
            self.push(ValueLocation::Immediate(
                (if i == 0 { 1i32 } else { 0 }).into(),
            ));
            return Ok(());
        }

        if let ValueLocation::Cond(loc) = val {
            self.push(ValueLocation::Cond(!loc));
            return Ok(());
        }

        let reg = self.into_reg(I64, &mut val).unwrap();
        let out = self.take_reg(I64).unwrap();

        dynasm!(self.asm
            ; xor Rd(out.rq().unwrap()), Rd(out.rq().unwrap())
            ; test Rq(reg.rq().unwrap()), Rq(reg.rq().unwrap())
            ; setz Rb(out.rq().unwrap())
        );

        self.free_value(val)?;

        self.push(ValueLocation::Reg(out));
        Ok(())
    }

    fn br_on_cond_code(&mut self, label: Label, cond: CondCode) {
        match cond {
            cc::EQUAL => dynasm!(self.asm
                ; je =>label.0
            ),
            cc::NOT_EQUAL => dynasm!(self.asm
                ; jne =>label.0
            ),
            cc::GT_U => dynasm!(self.asm
                ; ja =>label.0
            ),
            cc::GE_U => dynasm!(self.asm
                ; jae =>label.0
            ),
            cc::LT_U => dynasm!(self.asm
                ; jb =>label.0
            ),
            cc::LE_U => dynasm!(self.asm
                ; jbe =>label.0
            ),
            cc::GT_S => dynasm!(self.asm
                ; jg =>label.0
            ),
            cc::GE_S => dynasm!(self.asm
                ; jge =>label.0
            ),
            cc::LT_S => dynasm!(self.asm
                ; jl =>label.0
            ),
            cc::LE_S => dynasm!(self.asm
                ; jle =>label.0
            ),
        }
    }

    /// Pops i32 predicate and branches to the specified label
    /// if the predicate is equal to zero.
    pub fn br_if_false(
        &mut self,
        target: impl Into<BrTarget<Label>>,
        pass_args: impl FnOnce(&mut Self) -> Result<(), Error>,
    ) -> Result<(), Error> {
        let mut val = self.pop()?;
        let label = target
            .into()
            .label()
            .copied()
            .unwrap_or_else(|| self.ret_label());

        let cond = match val {
            ValueLocation::Cond(cc) => !cc,
            _ => {
                let predicate = self.into_reg(I32, &mut val).unwrap();
                dynasm!(self.asm
                    ; test Rd(predicate.rq().unwrap()), Rd(predicate.rq().unwrap())
                );

                CondCode::ZF0
            }
        };

        self.free_value(val)?;

        pass_args(self)?;

        self.br_on_cond_code(label, cond);
        Ok(())
    }

    /// Pops i32 predicate and branches to the specified label
    /// if the predicate is not equal to zero.
    pub fn br_if_true(
        &mut self,
        target: impl Into<BrTarget<Label>>,
        pass_args: impl FnOnce(&mut Self) -> Result<(), Error>,
    ) -> Result<(), Error> {
        let mut val = self.pop()?;
        let label = target
            .into()
            .label()
            .copied()
            .unwrap_or_else(|| self.ret_label());

        let cond = match val {
            ValueLocation::Cond(cc) => cc,
            _ => {
                let predicate = self.into_reg(I32, &mut val).unwrap();
                dynasm!(self.asm
                    ; test Rd(predicate.rq().unwrap()), Rd(predicate.rq().unwrap())
                );

                CondCode::ZF1
            }
        };

        self.free_value(val)?;

        pass_args(self)?;

        self.br_on_cond_code(label, cond);
        Ok(())
    }

    /// Branch unconditionally to the specified label.
    pub fn br(&mut self, label: impl Into<BrTarget<Label>>) {
        match label.into() {
            BrTarget::Return => self.ret(),
            BrTarget::Label(label) => dynasm!(self.asm
                ; jmp =>label.0
            ),
        }
    }

    /// If `default` is `None` then the default is just continuing execution
    pub fn br_table<I>(
        &mut self,
        targets: I,
        default: Option<BrTarget<Label>>,
        pass_args: impl FnOnce(&mut Self) -> Result<(), Error>,
    ) -> Result<(), Error>
    where
        I: IntoIterator<Item = Option<BrTarget<Label>>>,
        I::IntoIter: ExactSizeIterator,
    {
        let mut targets = targets.into_iter();
        let count = targets.len();

        let mut selector = self.pop()?;

        pass_args(self)?;

        if let Some(imm) = selector.imm_i32() {
            if let Some(target) = targets.nth(imm as _).or(Some(default)).and_then(|a| a) {
                match target {
                    BrTarget::Label(label) => self.br(label),
                    BrTarget::Return => {
                        dynasm!(self.asm
                            ; ret
                        );
                    }
                }
            }
        } else {
            let end_label = self.create_label();

            if count > 0 {
                let temp = match self.into_temp_reg(GPRType::Rq, &mut selector) {
                    Some(r) => Ok((r, false)),
                    None => {
                        self.push_physical(ValueLocation::Reg(RAX))?;
                        self.block_state.regs.mark_used(RAX);
                        Ok((RAX, true))
                    }
                };

                let (selector_reg, pop_selector) = match temp {
                    Err(e) => return Err(e),
                    Ok(a) => a,
                };

                let (tmp, pop_tmp) = if let Some(reg) = self.take_reg(I64) {
                    (reg, false)
                } else {
                    let out_reg = if selector_reg == RAX { RCX } else { RAX };

                    self.push_physical(ValueLocation::Reg(out_reg))?;
                    self.block_state.regs.mark_used(out_reg);

                    (out_reg, true)
                };

                self.immediate_to_reg(tmp, (count as u32).into());
                dynasm!(self.asm
                    ; cmp Rq(selector_reg.rq().unwrap()), Rq(tmp.rq().unwrap())
                    ; cmova Rq(selector_reg.rq().unwrap()), Rq(tmp.rq().unwrap())
                    ; lea Rq(tmp.rq().unwrap()), [>start_label]
                    ; lea Rq(selector_reg.rq().unwrap()), [
                        Rq(selector_reg.rq().unwrap()) * 5
                    ]
                    ; add Rq(selector_reg.rq().unwrap()), Rq(tmp.rq().unwrap())
                );

                if pop_tmp {
                    dynasm!(self.asm
                        ; pop Rq(tmp.rq().unwrap())
                    );
                } else {
                    self.block_state.regs.release(tmp)?;
                }

                if pop_selector {
                    dynasm!(self.asm
                        ; pop Rq(selector_reg.rq().unwrap())
                    );
                }

                dynasm!(self.asm
                    ; jmp Rq(selector_reg.rq().unwrap())
                ; start_label:
                );

                for target in targets {
                    let label = target
                        .map(|target| self.target_to_label(target))
                        .unwrap_or(end_label);
                    dynasm!(self.asm
                        ; jmp =>label.0
                    );
                }
            }

            if let Some(def) = default {
                match def {
                    BrTarget::Label(label) => dynasm!(self.asm
                        ; jmp =>label.0
                    ),
                    BrTarget::Return => dynasm!(self.asm
                        ; ret
                    ),
                }
            }

            self.define_label(end_label);
        }

        self.free_value(selector)?;
        Ok(())
    }

    fn set_stack_depth(&mut self, depth: StackDepth) -> Result<(), Error> {
        if self.block_state.depth.0 != depth.0 {
            let diff = depth.0 as i32 - self.block_state.depth.0 as i32;
            let emit_lea = if diff.abs() == 1 {
                if self.block_state.depth.0 < depth.0 {
                    for _ in 0..diff {
                        dynasm!(self.asm
                            ; push rax
                        );
                    }

                    false
                } else if self.block_state.depth.0 > depth.0 {
                    if let Some(trash) = self.take_reg(I64) {
                        for _ in 0..self.block_state.depth.0 - depth.0 {
                            dynasm!(self.asm
                                ; pop Rq(trash.rq().unwrap())
                            );
                        }
                        self.block_state.regs.release(trash)?;

                        false
                    } else {
                        true
                    }
                } else {
                    false
                }
            } else {
                true
            };

            if emit_lea {
                dynasm!(self.asm
                    ; lea rsp, [rsp + (self.block_state.depth.0 as i32 - depth.0 as i32) * WORD_SIZE as i32]
                );
            }

            self.block_state.depth = depth;
        }
        Ok(())
    }

    fn do_pass_block_args(&mut self, cc: &BlockCallingConvention) -> Result<(), Error> {
        let args = &cc.arguments;
        for &dst in args.iter().rev().take(self.block_state.stack.len()) {
            if let CCLoc::Reg(r) = dst {
                if !self.block_state.regs.is_free(r)
                    && *self.block_state.stack.last().unwrap() != ValueLocation::Reg(r)
                {
                    // TODO: This would be made simpler and more efficient with a proper SSE
                    //       representation.
                    self.save_regs(&[r], ..)?;
                }

                self.block_state.regs.mark_used(r);
            }
            self.pop_into(dst)?;
        }
        Ok(())
    }

    pub fn pass_block_args(&mut self, cc: &BlockCallingConvention) -> Result<(), Error> {
        self.do_pass_block_args(cc)?;
        self.set_stack_depth(cc.stack_depth)?;
        Ok(())
    }

    pub fn serialize_block_args(
        &mut self,
        cc: &BlockCallingConvention,
        params: u32,
    ) -> Result<BlockCallingConvention, Error> {
        self.do_pass_block_args(cc)?;

        let mut out_args = cc.arguments.clone();

        out_args.reverse();

        while out_args.len() < params as usize {
            let mut val = self.pop()?;

            // TODO: We can use stack slots for values already on the stack but we
            //       don't refcount stack slots right now
            let ccloc = self.into_temp_loc(None, &mut val)?;
            out_args.push(ccloc);
        }

        out_args.reverse();

        self.set_stack_depth(cc.stack_depth)?;

        Ok(BlockCallingConvention {
            stack_depth: cc.stack_depth,
            arguments: out_args,
        })
    }

    /// Puts all stack values into "real" locations so that they can i.e. be set to different
    /// values on different iterations of a loop
    pub fn serialize_args(&mut self, count: u32) -> Result<BlockCallingConvention, Error> {
        let mut out = Vec::with_capacity(count as _);

        // TODO: We can make this more efficient now that `pop` isn't so complicated
        for _ in 0..count {
            let mut val = self.pop()?;
            // TODO: We can use stack slots for values already on the stack but we
            //       don't refcount stack slots right now
            let loc = self.into_temp_loc(None, &mut val)?;

            out.push(loc);
        }

        out.reverse();

        Ok(BlockCallingConvention {
            stack_depth: self.block_state.depth,
            arguments: out,
        })
    }

    pub fn get_global(&mut self, global_idx: u32) -> Result<(), Error> {
        let (reg, offset) = self
            .module_context
            .defined_global_index(global_idx)
            .map(|defined_global_index| {
                (
                    None,
                    self.module_context
                        .vmctx_vmglobal_definition(defined_global_index),
                )
            })
            .unwrap_or_else(|| {
                let reg = self.take_reg(I64).unwrap();

                dynasm!(self.asm
                    ; mov Rq(reg.rq().unwrap()), [
                        Rq(VMCTX) +
                            self.module_context.vmctx_vmglobal_import_from(global_idx) as i32
                    ]
                );

                (Some(reg), 0)
            });

        let out = self.take_reg(GPRType::Rq).unwrap();
        let vmctx = GPR::Rq(VMCTX);

        // TODO: Are globals necessarily aligned to 128 bits? We can load directly to an XMM reg if so
        dynasm!(self.asm
            ; mov Rq(out.rq().unwrap()), [Rq(reg.unwrap_or(vmctx).rq().unwrap()) + offset as i32]
        );

        if let Some(reg) = reg {
            self.block_state.regs.release(reg)?;
        }

        self.push(ValueLocation::Reg(out));
        Ok(())
    }

    pub fn set_global(&mut self, global_idx: u32) -> Result<(), Error> {
        let mut val = self.pop()?;
        let (reg, offset) = self
            .module_context
            .defined_global_index(global_idx)
            .map(|defined_global_index| {
                (
                    None,
                    self.module_context
                        .vmctx_vmglobal_definition(defined_global_index),
                )
            })
            .unwrap_or_else(|| {
                let reg = self.take_reg(I64).unwrap();

                dynasm!(self.asm
                    ; mov Rq(reg.rq().unwrap()), [
                        Rq(VMCTX) +
                            self.module_context.vmctx_vmglobal_import_from(global_idx) as i32
                    ]
                );

                (Some(reg), 0)
            });

        let val_reg = self.into_reg(GPRType::Rq, &mut val).unwrap();
        let vmctx = GPR::Rq(VMCTX);

        // We always use `Rq` (even for floats) since the globals are not necessarily aligned to 128 bits
        dynasm!(self.asm
            ; mov [
                Rq(reg.unwrap_or(vmctx).rq().unwrap()) + offset as i32
            ], Rq(val_reg.rq().unwrap())
        );

        if let Some(reg) = reg {
            self.block_state.regs.release(reg)?;
        }

        self.free_value(val)?;
        Ok(())
    }

    fn immediate_to_reg(&mut self, reg: GPR, val: Value) {
        match reg {
            GPR::Rq(r) => {
                let val = val.as_bytes();
                if (val as u64) <= u32::max_value() as u64 {
                    dynasm!(self.asm
                        ; mov Rd(r), val as i32
                    );
                } else {
                    dynasm!(self.asm
                        ; mov Rq(r), QWORD val
                    );
                }
            }
            GPR::Rx(r) => {
                let label = self.aligned_label(16, LabelValue::from(val));
                dynasm!(self.asm
                    ; movq Rx(r), [=>label.0]
                );
            }
        }
    }

    // The `&` and `&mut` aren't necessary (`ValueLocation` is copy) but it ensures that we don't get
    // the arguments the wrong way around. In the future we want to have a `ReadLocation` and `WriteLocation`
    // so we statically can't write to a literal so this will become a non-issue.
    fn copy_value(&mut self, src: ValueLocation, dst: CCLoc) -> Result<(), Error> {
        match (src, dst) {
            (ValueLocation::Cond(cond), CCLoc::Stack(o)) => {
                let offset = self.adjusted_offset(o);

                dynasm!(self.asm
                    ; mov QWORD [rsp + offset], DWORD 0
                );

                match cond {
                    cc::EQUAL => dynasm!(self.asm
                        ; sete [rsp + offset]
                    ),
                    cc::NOT_EQUAL => dynasm!(self.asm
                        ; setne [rsp + offset]
                    ),
                    cc::GT_U => dynasm!(self.asm
                        ; seta [rsp + offset]
                    ),
                    cc::GE_U => dynasm!(self.asm
                        ; setae [rsp + offset]
                    ),
                    cc::LT_U => dynasm!(self.asm
                        ; setb [rsp + offset]
                    ),
                    cc::LE_U => dynasm!(self.asm
                        ; setbe [rsp + offset]
                    ),
                    cc::GT_S => dynasm!(self.asm
                        ; setg [rsp + offset]
                    ),
                    cc::GE_S => dynasm!(self.asm
                        ; setge [rsp + offset]
                    ),
                    cc::LT_S => dynasm!(self.asm
                        ; setl [rsp + offset]
                    ),
                    cc::LE_S => dynasm!(self.asm
                        ; setle [rsp + offset]
                    ),
                }
            }
            (ValueLocation::Cond(cond), CCLoc::Reg(reg)) => match reg {
                GPR::Rq(r) => {
                    dynasm!(self.asm
                        ; mov Rq(r), 0
                    );

                    match cond {
                        cc::EQUAL => dynasm!(self.asm
                            ; sete Rb(r)
                        ),
                        cc::NOT_EQUAL => dynasm!(self.asm
                            ; setne Rb(r)
                        ),
                        cc::GT_U => dynasm!(self.asm
                            ; seta Rb(r)
                        ),
                        cc::GE_U => dynasm!(self.asm
                            ; setae Rb(r)
                        ),
                        cc::LT_U => dynasm!(self.asm
                            ; setb Rb(r)
                        ),
                        cc::LE_U => dynasm!(self.asm
                            ; setbe Rb(r)
                        ),
                        cc::GT_S => dynasm!(self.asm
                            ; setg Rb(r)
                        ),
                        cc::GE_S => dynasm!(self.asm
                            ; setge Rb(r)
                        ),
                        cc::LT_S => dynasm!(self.asm
                            ; setl Rb(r)
                        ),
                        cc::LE_S => dynasm!(self.asm
                            ; setle Rb(r)
                        ),
                    }
                }
                GPR::Rx(_) => {
                    let temp = CCLoc::Reg(self.take_reg(I32).unwrap());
                    self.copy_value(src, temp)?;
                    let temp = temp.into();
                    self.copy_value(temp, dst)?;
                    self.free_value(temp)?;
                }
            },
            (ValueLocation::Stack(in_offset), CCLoc::Stack(out_offset)) => {
                let in_offset = self.adjusted_offset(in_offset);
                let out_offset = self.adjusted_offset(out_offset);
                if in_offset != out_offset {
                    if let Some(gpr) = self.take_reg(I64) {
                        dynasm!(self.asm
                            ; mov Rq(gpr.rq().unwrap()), [rsp + in_offset]
                            ; mov [rsp + out_offset], Rq(gpr.rq().unwrap())
                        );
                        self.block_state.regs.release(gpr)?;
                    } else {
                        dynasm!(self.asm
                            ; push rax
                            ; mov rax, [rsp + in_offset + WORD_SIZE as i32]
                            ; mov [rsp + out_offset + WORD_SIZE as i32], rax
                            ; pop rax
                        );
                    }
                }
            }
            // TODO: XMM registers
            (ValueLocation::Reg(in_reg), CCLoc::Stack(out_offset)) => {
                let out_offset = self.adjusted_offset(out_offset);
                match in_reg {
                    GPR::Rq(in_reg) => {
                        // We can always use `Rq` here for now because stack slots are in multiples of
                        // 8 bytes
                        dynasm!(self.asm
                            ; mov [rsp + out_offset], Rq(in_reg)
                        );
                    }
                    GPR::Rx(in_reg) => {
                        // We can always use `movq` here for now because stack slots are in multiples of
                        // 8 bytes
                        dynasm!(self.asm
                            ; movq [rsp + out_offset], Rx(in_reg)
                        );
                    }
                }
            }
            (ValueLocation::Immediate(i), CCLoc::Stack(out_offset)) => {
                // TODO: Floats
                let i = i.as_bytes();
                let out_offset = self.adjusted_offset(out_offset);
                if (i as u64) <= u32::max_value() as u64 {
                    dynasm!(self.asm
                        ; mov DWORD [rsp + out_offset], i as i32
                    );
                } else if let Some(scratch) = self.take_reg(I64) {
                    dynasm!(self.asm
                        ; mov Rq(scratch.rq().unwrap()), QWORD i
                        ; mov [rsp + out_offset], Rq(scratch.rq().unwrap())
                    );

                    self.block_state.regs.release(scratch)?;
                } else {
                    dynasm!(self.asm
                        ; push rax
                        ; mov rax, QWORD i
                        ; mov [rsp + out_offset + WORD_SIZE as i32], rax
                        ; pop rax
                    );
                }
            }
            (ValueLocation::Stack(in_offset), CCLoc::Reg(out_reg)) => {
                let in_offset = self.adjusted_offset(in_offset);
                match out_reg {
                    GPR::Rq(out_reg) => {
                        // We can always use `Rq` here for now because stack slots are in multiples of
                        // 8 bytes
                        dynasm!(self.asm
                            ; mov Rq(out_reg), [rsp + in_offset]
                        );
                    }
                    GPR::Rx(out_reg) => {
                        // We can always use `movq` here for now because stack slots are in multiples of
                        // 8 bytes
                        dynasm!(self.asm
                            ; movq Rx(out_reg), [rsp + in_offset]
                        );
                    }
                }
            }
            (ValueLocation::Reg(in_reg), CCLoc::Reg(out_reg)) => {
                if in_reg != out_reg {
                    match (in_reg, out_reg) {
                        (GPR::Rq(in_reg), GPR::Rq(out_reg)) => {
                            dynasm!(self.asm
                                ; mov Rq(out_reg), Rq(in_reg)
                            );
                        }
                        (GPR::Rx(in_reg), GPR::Rq(out_reg)) => {
                            dynasm!(self.asm
                                ; movq Rq(out_reg), Rx(in_reg)
                            );
                        }
                        (GPR::Rq(in_reg), GPR::Rx(out_reg)) => {
                            dynasm!(self.asm
                                ; movq Rx(out_reg), Rq(in_reg)
                            );
                        }
                        (GPR::Rx(in_reg), GPR::Rx(out_reg)) => {
                            dynasm!(self.asm
                                ; movapd Rx(out_reg), Rx(in_reg)
                            );
                        }
                    }
                }
            }
            (ValueLocation::Immediate(i), CCLoc::Reg(out_reg)) => {
                // TODO: Floats
                self.immediate_to_reg(out_reg, i);
            }
        }
        Ok(())
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

    pub fn apply_cc(&mut self, cc: &BlockCallingConvention) {
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

    load!(i32_load, GPRType::Rq, Rd, movd, mov, DWORD);
    load!(i64_load, GPRType::Rq, Rq, movq, mov, QWORD);
    load!(f32_load, GPRType::Rx, Rd, movd, mov, DWORD);
    load!(f64_load, GPRType::Rx, Rq, movq, mov, QWORD);

    load!(i32_load8_u, GPRType::Rq, Rd, NONE, movzx, BYTE);
    load!(i32_load8_s, GPRType::Rq, Rd, NONE, movsx, BYTE);
    load!(i32_load16_u, GPRType::Rq, Rd, NONE, movzx, WORD);
    load!(i32_load16_s, GPRType::Rq, Rd, NONE, movsx, WORD);

    load!(i64_load8_u, GPRType::Rq, Rq, NONE, movzx, BYTE);
    load!(i64_load8_s, GPRType::Rq, Rq, NONE, movsx, BYTE);
    load!(i64_load16_u, GPRType::Rq, Rq, NONE, movzx, WORD);
    load!(i64_load16_s, GPRType::Rq, Rq, NONE, movsx, WORD);
    load!(i64_load32_u, GPRType::Rq, Rd, movd, mov, DWORD);
    load!(i64_load32_s, GPRType::Rq, Rq, NONE, movsxd, DWORD);

    store!(store8, Rb, NONE, DWORD);
    store!(store16, Rw, NONE, QWORD);
    store!(store32, Rd, movd, DWORD);
    store!(store64, Rq, movq, QWORD);

    fn push_physical(&mut self, mut value: ValueLocation) -> Result<ValueLocation, Error> {
        let out_offset = -(self.block_state.depth.0 as i32 + 1);
        match value {
            ValueLocation::Reg(_) | ValueLocation::Immediate(_) | ValueLocation::Cond(_) => {
                if let Some(gpr) = self.into_reg(GPRType::Rq, &mut value) {
                    dynasm!(self.asm
                        ; push Rq(gpr.rq().unwrap())
                    );
                } else {
                    dynasm!(self.asm
                        ; push rax
                    );

                    self.copy_value(value, CCLoc::Stack(out_offset))?;
                }

                self.free_value(value)?;
            }
            ValueLocation::Stack(o) => {
                let offset = self.adjusted_offset(o);
                dynasm!(self.asm
                    ; push QWORD [rsp + offset]
                );
            }
        }

        self.block_state.depth.reserve(1);

        Ok(ValueLocation::Stack(out_offset))
    }

    fn push(&mut self, value: ValueLocation) {
        if let Some(mut top) = self.block_state.stack.pop() {
            if let ValueLocation::Cond(_) = top {
                self.into_reg(I32, &mut top).unwrap();
            }

            self.block_state.stack.push(top);
        }

        self.block_state.stack.push(value);
    }

    fn pop(&mut self) -> Result<ValueLocation, Error> {
        match self.block_state.stack.pop() {
            Some(v) => Ok(v),
            None => return Err(Error::Microwasm(
                        "Stack is empty - pop impossible".to_string(),
                    )),
        }
    }

    pub fn drop(&mut self, range: RangeInclusive<u32>) -> Result<(), Error> {
        let mut repush = Vec::with_capacity(*range.start() as _);

        for _ in 0..*range.start() {
            let v = self.pop()?;
            repush.push(v);
        }

        for _ in range {
            let val = self.pop()?;
            self.free_value(val)?;
        }

        for v in repush.into_iter().rev() {
            self.push(v);
        }
        Ok(())
    }

    fn pop_into(&mut self, dst: CCLoc) -> Result<(), Error> {
        let val = self.pop()?;
        self.copy_value(val, dst)?;
        self.free_value(val)?;
        Ok(())
    }

    fn free_value(&mut self, val: ValueLocation) -> Result<(), Error> {
        match val {
            ValueLocation::Reg(r) => {
                self.block_state.regs.release(r)?;
            }
            // TODO: Refcounted stack slots
            _ => {}
        }
        Ok(())
    }

    /// Puts this value into a register so that it can be efficiently read
    fn into_reg(&mut self, ty: impl Into<Option<GPRType>>, val: &mut ValueLocation) -> Option<GPR> {
        let out = self.to_reg(ty, *val)?;
        self.free_value(*val);
        *val = ValueLocation::Reg(out);
        Some(out)
    }

    /// Clones this value into a register so that it can be efficiently read
    fn to_reg(&mut self, ty: impl Into<Option<GPRType>>, val: ValueLocation) -> Option<GPR> {
        let ty = ty.into();
        match val {
            ValueLocation::Reg(r) if ty.map(|t| t == r.type_()).unwrap_or(true) => {
                self.block_state.regs.mark_used(r);
                Some(r)
            }
            val => {
                let scratch = self.take_reg(ty.unwrap_or(GPRType::Rq))?;

                self.copy_value(val, CCLoc::Reg(scratch));

                Some(scratch)
            }
        }
    }

    /// Puts this value into a temporary register so that operations
    /// on that register don't write to a local.
    fn into_temp_reg(
        &mut self,
        ty: impl Into<Option<GPRType>>,
        val: &mut ValueLocation,
    ) -> Option<GPR> {
        let out = self.to_temp_reg(ty, *val)?;
        self.free_value(*val);
        *val = ValueLocation::Reg(out);
        Some(out)
    }

    fn into_temp_loc(
        &mut self,
        ty: impl Into<Option<GPRType>>,
        val: &mut ValueLocation,
    ) -> Result<CCLoc, Error> {
        match val {
            _ => Ok({
                if let Some(gpr) = self.into_temp_reg(ty, val) {
                    CCLoc::Reg(gpr)
                } else {
                    let out = CCLoc::Stack(self.push_physical(*val)?.stack().unwrap());
                    *val = out.into();
                    out
                }
            }),
        }
    }

    /// Clones this value into a temporary register so that operations
    /// on that register don't write to a local.
    fn to_temp_reg(&mut self, ty: impl Into<Option<GPRType>>, val: ValueLocation) -> Option<GPR> {
        // If we have `None` as the type then it always matches (`.unwrap_or(true)`)
        match val {
            ValueLocation::Reg(r) => {
                let ty = ty.into();
                let type_matches = ty.map(|t| t == r.type_()).unwrap_or(true);

                if self.block_state.regs.num_usages(r) <= 1 && type_matches {
                    self.block_state.regs.mark_used(r);
                    Some(r)
                } else {
                    let scratch = self.take_reg(ty.unwrap_or(GPRType::Rq))?;

                    self.copy_value(val, CCLoc::Reg(scratch));

                    Some(scratch)
                }
            }
            val => self.to_reg(ty, val),
        }
    }

    pub fn f32_neg(&mut self) -> Result<(), Error> {
        let mut val = self.pop()?;

        let out = if let Some(i) = val.imm_f32() {
            ValueLocation::Immediate(
                Ieee32::from_bits((-f32::from_bits(i.to_bits())).to_bits()).into(),
            )
        } else {
            let reg = self.into_temp_reg(GPRType::Rx, &mut val).unwrap();
            let const_label = self.aligned_label(16, LabelValue::I32(SIGN_MASK_F32 as i32));

            dynasm!(self.asm
                ; xorps Rx(reg.rx().unwrap()), [=>const_label.0]
            );

            val
        };

        self.push(out);
        Ok(())
    }

    pub fn f64_neg(&mut self) -> Result<(), Error> {
        let mut val = self.pop()?;

        let out = if let Some(i) = val.imm_f64() {
            ValueLocation::Immediate(
                Ieee64::from_bits((-f64::from_bits(i.to_bits())).to_bits()).into(),
            )
        } else {
            let reg = self.into_temp_reg(GPRType::Rx, &mut val).unwrap();
            let const_label = self.aligned_label(16, LabelValue::I64(SIGN_MASK_F64 as i64));

            dynasm!(self.asm
                ; xorpd Rx(reg.rx().unwrap()), [=>const_label.0]
            );

            val
        };

        self.push(out);
        Ok(())
    }

    pub fn f32_abs(&mut self) -> Result<(), Error> {
        let mut val = self.pop()?;

        let out = if let Some(i) = val.imm_f32() {
            ValueLocation::Immediate(
                Ieee32::from_bits(f32::from_bits(i.to_bits()).abs().to_bits()).into(),
            )
        } else {
            let reg = self.into_temp_reg(GPRType::Rx, &mut val).unwrap();
            let const_label = self.aligned_label(16, LabelValue::I32(REST_MASK_F32 as i32));

            dynasm!(self.asm
                ; andps Rx(reg.rx().unwrap()), [=>const_label.0]
            );

            val
        };

        self.push(out);
        Ok(())
    }

    pub fn f64_abs(&mut self) -> Result<(), Error> {
        let mut val = self.pop()?;

        let out = if let Some(i) = val.imm_f64() {
            ValueLocation::Immediate(
                Ieee64::from_bits(f64::from_bits(i.to_bits()).abs().to_bits()).into(),
            )
        } else {
            let reg = self.into_temp_reg(GPRType::Rx, &mut val).unwrap();
            let const_label = self.aligned_label(16, LabelValue::I64(REST_MASK_F64 as i64));

            dynasm!(self.asm
                ; andps Rx(reg.rx().unwrap()), [=>const_label.0]
            );

            val
        };

        self.push(out);
        Ok(())
    }

    pub fn f32_sqrt(&mut self) -> Result<(), Error> {
        let mut val = self.pop()?;

        let out = if let Some(i) = val.imm_f32() {
            ValueLocation::Immediate(
                Ieee32::from_bits(f32::from_bits(i.to_bits()).sqrt().to_bits()).into(),
            )
        } else {
            let reg = self.into_temp_reg(GPRType::Rx, &mut val).unwrap();

            dynasm!(self.asm
                ; sqrtss Rx(reg.rx().unwrap()), Rx(reg.rx().unwrap())
            );

            val
        };

        self.push(out);
        Ok(())
    }

    pub fn f64_sqrt(&mut self) -> Result<(), Error> {
        let mut val = self.pop()?;

        let out = if let Some(i) = val.imm_f64() {
            ValueLocation::Immediate(
                Ieee64::from_bits(f64::from_bits(i.to_bits()).sqrt().to_bits()).into(),
            )
        } else {
            let reg = self.into_temp_reg(GPRType::Rx, &mut val).unwrap();

            dynasm!(self.asm
                ; sqrtsd Rx(reg.rx().unwrap()), Rx(reg.rx().unwrap())
            );

            ValueLocation::Reg(reg)
        };

        self.push(out);
        Ok(())
    }

    pub fn f32_copysign(&mut self) -> Result<(), Error> {
        let mut right = self.pop()?;
        let mut left = self.pop()?;

        let out = if let (Some(left), Some(right)) = (left.imm_f32(), right.imm_f32()) {
            ValueLocation::Immediate(
                Ieee32::from_bits(
                    (left.to_bits() & REST_MASK_F32) | (right.to_bits() & SIGN_MASK_F32),
                )
                .into(),
            )
        } else {
            let lreg = self.into_temp_reg(GPRType::Rx, &mut left).unwrap();
            let rreg = self.into_reg(GPRType::Rx, &mut right).unwrap();
            let sign_mask = self.aligned_label(16, LabelValue::I32(SIGN_MASK_F32 as i32));
            let rest_mask = self.aligned_label(16, LabelValue::I32(REST_MASK_F32 as i32));

            dynasm!(self.asm
                ; andps Rx(rreg.rx().unwrap()), [=>sign_mask.0]
                ; andps Rx(lreg.rx().unwrap()), [=>rest_mask.0]
                ; orps  Rx(lreg.rx().unwrap()), Rx(rreg.rx().unwrap())
            );

            self.free_value(right)?;

            left
        };

        self.push(out);
        Ok(())
    }

    pub fn f64_copysign(&mut self) -> Result<(), Error> {
        let mut right = self.pop()?;
        let mut left = self.pop()?;

        let out = if let (Some(left), Some(right)) = (left.imm_f64(), right.imm_f64()) {
            ValueLocation::Immediate(
                Ieee64::from_bits(
                    (left.to_bits() & REST_MASK_F64) | (right.to_bits() & SIGN_MASK_F64),
                )
                .into(),
            )
        } else {
            let lreg = self.into_temp_reg(GPRType::Rx, &mut left).unwrap();
            let rreg = self.into_reg(GPRType::Rx, &mut right).unwrap();
            let sign_mask = self.aligned_label(16, LabelValue::I64(SIGN_MASK_F64 as i64));
            let rest_mask = self.aligned_label(16, LabelValue::I64(REST_MASK_F64 as i64));

            dynasm!(self.asm
                ; andpd Rx(rreg.rx().unwrap()), [=>sign_mask.0]
                ; andpd Rx(lreg.rx().unwrap()), [=>rest_mask.0]
                ; orpd  Rx(lreg.rx().unwrap()), Rx(rreg.rx().unwrap())
            );

            self.free_value(right)?;

            left
        };

        self.push(out);
        Ok(())
    }

    pub fn i32_clz(&mut self) -> Result<(), Error> {
        let mut val = self.pop()?;

        let out_val = match val {
            ValueLocation::Immediate(imm) => {
                ValueLocation::Immediate(imm.as_i32().unwrap().leading_zeros().into())
            }
            ValueLocation::Stack(offset) => {
                let offset = self.adjusted_offset(offset);
                let temp = self.take_reg(I32).unwrap();

                if is_x86_feature_detected!("lzcnt") {
                    dynasm!(self.asm
                        ; lzcnt Rd(temp.rq().unwrap()), [rsp + offset]
                    );
                    ValueLocation::Reg(temp)
                } else {
                    let temp_2 = self.take_reg(I32).unwrap();

                    dynasm!(self.asm
                        ; bsr Rd(temp.rq().unwrap()), [rsp + offset]
                        ; mov Rd(temp_2.rq().unwrap()), DWORD 0x3fu64 as _
                        ; cmove Rd(temp.rq().unwrap()), Rd(temp_2.rq().unwrap())
                        ; mov Rd(temp_2.rq().unwrap()), DWORD 0x1fu64 as _
                        ; xor Rd(temp.rq().unwrap()), Rd(temp_2.rq().unwrap())
                    );
                    self.free_value(ValueLocation::Reg(temp_2))?;
                    ValueLocation::Reg(temp)
                }
            }
            ValueLocation::Reg(_) | ValueLocation::Cond(_) => {
                let reg = self.into_reg(GPRType::Rq, &mut val).unwrap();
                let temp = self.take_reg(I32).unwrap();

                if is_x86_feature_detected!("lzcnt") {
                    dynasm!(self.asm
                        ; lzcnt Rd(temp.rq().unwrap()), Rd(reg.rq().unwrap())
                    );
                    ValueLocation::Reg(temp)
                } else {
                    dynasm!(self.asm
                        ; bsr Rd(temp.rq().unwrap()), Rd(reg.rq().unwrap())
                        ; mov Rd(reg.rq().unwrap()), DWORD 0x3fu64 as _
                        ; cmove Rd(temp.rq().unwrap()), Rd(reg.rq().unwrap())
                        ; mov Rd(reg.rq().unwrap()), DWORD 0x1fu64 as _
                        ; xor Rd(temp.rq().unwrap()), Rd(reg.rq().unwrap())
                    );
                    ValueLocation::Reg(temp)
                }
            }
        };

        self.free_value(val)?;
        self.push(out_val);
        Ok(())
    }

    pub fn i64_clz(&mut self) -> Result<(), Error> {
        let mut val = self.pop()?;

        let out_val = match val {
            ValueLocation::Immediate(imm) => {
                ValueLocation::Immediate((imm.as_i64().unwrap().leading_zeros() as u64).into())
            }
            ValueLocation::Stack(offset) => {
                let offset = self.adjusted_offset(offset);
                let temp = self.take_reg(I64).unwrap();

                if is_x86_feature_detected!("lzcnt") {
                    dynasm!(self.asm
                        ; lzcnt Rq(temp.rq().unwrap()), [rsp + offset]
                    );
                    ValueLocation::Reg(temp)
                } else {
                    let temp_2 = self.take_reg(I64).unwrap();

                    dynasm!(self.asm
                        ; bsr Rq(temp.rq().unwrap()), [rsp + offset]
                        ; mov Rq(temp_2.rq().unwrap()), QWORD 0x7fu64 as _
                        ; cmove Rq(temp.rq().unwrap()), Rq(temp_2.rq().unwrap())
                        ; mov Rq(temp_2.rq().unwrap()), QWORD 0x3fu64 as _
                        ; xor Rq(temp.rq().unwrap()), Rq(temp_2.rq().unwrap())
                    );
                    self.free_value(ValueLocation::Reg(temp_2))?;
                    ValueLocation::Reg(temp)
                }
            }
            ValueLocation::Reg(_) | ValueLocation::Cond(_) => {
                let reg = self.into_reg(GPRType::Rq, &mut val).unwrap();
                let temp = self.take_reg(I64).unwrap();

                if is_x86_feature_detected!("lzcnt") {
                    dynasm!(self.asm
                        ; lzcnt Rq(temp.rq().unwrap()), Rq(reg.rq().unwrap())
                    );
                    ValueLocation::Reg(temp)
                } else {
                    dynasm!(self.asm
                        ; bsr Rq(temp.rq().unwrap()), Rq(reg.rq().unwrap())
                        ; mov Rq(reg.rq().unwrap()), QWORD 0x7fu64 as _
                        ; cmove Rq(temp.rq().unwrap()), Rq(reg.rq().unwrap())
                        ; mov Rq(reg.rq().unwrap()), QWORD 0x3fu64 as _
                        ; xor Rq(temp.rq().unwrap()), Rq(reg.rq().unwrap())
                    );
                    ValueLocation::Reg(temp)
                }
            }
        };

        self.free_value(val)?;
        self.push(out_val);
        Ok(())
    }

    pub fn i32_ctz(&mut self) -> Result<(), Error> {
        let mut val = self.pop()?;

        let out_val = match val {
            ValueLocation::Immediate(imm) => {
                ValueLocation::Immediate(imm.as_i32().unwrap().trailing_zeros().into())
            }
            ValueLocation::Stack(offset) => {
                let offset = self.adjusted_offset(offset);
                let temp = self.take_reg(I32).unwrap();

                if is_x86_feature_detected!("lzcnt") {
                    dynasm!(self.asm
                        ; tzcnt Rd(temp.rq().unwrap()), [rsp + offset]
                    );
                    ValueLocation::Reg(temp)
                } else {
                    let temp_zero_val = self.take_reg(I32).unwrap();

                    dynasm!(self.asm
                        ; bsf Rd(temp.rq().unwrap()), [rsp + offset]
                        ; mov Rd(temp_zero_val.rq().unwrap()), DWORD 0x20u32 as _
                        ; cmove Rd(temp.rq().unwrap()), Rd(temp_zero_val.rq().unwrap())
                    );
                    self.free_value(ValueLocation::Reg(temp_zero_val))?;
                    ValueLocation::Reg(temp)
                }
            }
            ValueLocation::Reg(_) | ValueLocation::Cond(_) => {
                let reg = self.into_reg(GPRType::Rq, &mut val).unwrap();
                let temp = self.take_reg(I32).unwrap();

                if is_x86_feature_detected!("lzcnt") {
                    dynasm!(self.asm
                        ; tzcnt Rd(temp.rq().unwrap()), Rd(reg.rq().unwrap())
                    );
                    ValueLocation::Reg(temp)
                } else {
                    dynasm!(self.asm
                        ; bsf Rd(temp.rq().unwrap()), Rd(reg.rq().unwrap())
                        ; mov Rd(reg.rq().unwrap()), DWORD 0x20u32 as _
                        ; cmove Rd(temp.rq().unwrap()), Rd(reg.rq().unwrap())
                    );
                    ValueLocation::Reg(temp)
                }
            }
        };

        self.free_value(val)?;
        self.push(out_val);
        Ok(())
    }

    pub fn i64_ctz(&mut self) -> Result<(), Error> {
        let mut val = self.pop()?;

        let out_val = match val {
            ValueLocation::Immediate(imm) => {
                ValueLocation::Immediate((imm.as_i64().unwrap().trailing_zeros() as u64).into())
            }
            ValueLocation::Stack(offset) => {
                let offset = self.adjusted_offset(offset);
                let temp = self.take_reg(I64).unwrap();

                if is_x86_feature_detected!("lzcnt") {
                    dynasm!(self.asm
                        ; tzcnt Rq(temp.rq().unwrap()), [rsp + offset]
                    );
                    ValueLocation::Reg(temp)
                } else {
                    let temp_zero_val = self.take_reg(I64).unwrap();

                    dynasm!(self.asm
                        ; bsf Rq(temp.rq().unwrap()), [rsp + offset]
                        ; mov Rq(temp_zero_val.rq().unwrap()), QWORD 0x40u64 as _
                        ; cmove Rq(temp.rq().unwrap()), Rq(temp_zero_val.rq().unwrap())
                    );
                    self.free_value(ValueLocation::Reg(temp_zero_val))?;
                    ValueLocation::Reg(temp)
                }
            }
            ValueLocation::Reg(_) | ValueLocation::Cond(_) => {
                let reg = self.into_reg(GPRType::Rq, &mut val).unwrap();
                let temp = self.take_reg(I64).unwrap();

                dynasm!(self.asm
                    ; bsf Rq(temp.rq().unwrap()), Rq(reg.rq().unwrap())
                    ; mov Rq(reg.rq().unwrap()), QWORD 0x40u64 as _
                    ; cmove Rq(temp.rq().unwrap()), Rq(reg.rq().unwrap())
                );
                ValueLocation::Reg(temp)
            }
        };

        self.free_value(val)?;
        self.push(out_val);
        Ok(())
    }

    pub fn i32_extend_u(&mut self) -> Result<(), Error> {
        let val = self.pop()?;

        let out = if let ValueLocation::Immediate(imm) = val {
            ValueLocation::Immediate((imm.as_i32().unwrap() as u32 as u64).into())
        } else {
            let new_reg = self.take_reg(I64).unwrap();

            // TODO: Track set-ness of bits - we can make this a no-op in most cases
            //       but we have to make this unconditional just in case this value
            //       came from a truncate.
            match val {
                ValueLocation::Reg(GPR::Rx(rxreg)) => {
                    dynasm!(self.asm
                        ; movd Rd(new_reg.rq().unwrap()), Rx(rxreg)
                    );
                }
                ValueLocation::Reg(GPR::Rq(rqreg)) => {
                    dynasm!(self.asm
                        ; mov Rd(new_reg.rq().unwrap()), Rd(rqreg)
                    );
                }
                ValueLocation::Stack(offset) => {
                    let offset = self.adjusted_offset(offset);

                    dynasm!(self.asm
                        ; mov Rd(new_reg.rq().unwrap()), [rsp + offset]
                    );
                }
                ValueLocation::Cond(_) => self.copy_value(val, CCLoc::Reg(new_reg))?,
                ValueLocation::Immediate(_) => unreachable!(),
            }

            ValueLocation::Reg(new_reg)
        };

        self.free_value(val)?;

        self.push(out);
        Ok(())
    }

    pub fn i32_extend_s(&mut self) -> Result<(), Error> {
        let val = self.pop()?;

        self.free_value(val)?;
        let new_reg = self.take_reg(I64).unwrap();

        let out = if let ValueLocation::Immediate(imm) = val {
            self.block_state.regs.release(new_reg)?;
            ValueLocation::Immediate((imm.as_i32().unwrap() as i64).into())
        } else {
            match val {
                ValueLocation::Reg(GPR::Rx(rxreg)) => {
                    dynasm!(self.asm
                        ; movd Rd(new_reg.rq().unwrap()), Rx(rxreg)
                        ; movsxd Rq(new_reg.rq().unwrap()), Rd(new_reg.rq().unwrap())
                    );
                }
                ValueLocation::Reg(GPR::Rq(rqreg)) => {
                    dynasm!(self.asm
                        ; movsxd Rq(new_reg.rq().unwrap()), Rd(rqreg)
                    );
                }
                ValueLocation::Stack(offset) => {
                    let offset = self.adjusted_offset(offset);

                    dynasm!(self.asm
                        ; movsxd Rq(new_reg.rq().unwrap()), DWORD [rsp + offset]
                    );
                }
                _ => {
                    return Err(Error::Microwasm(
                        "i32_extend_s unreachable code".to_string(),
                    ))
                }
            }

            ValueLocation::Reg(new_reg)
        };

        self.push(out);
        Ok(())
    }

    unop!(i32_popcnt, popcnt, Rd, u32, u32::count_ones);
    conversion!(
        f64_from_f32,
        cvtss2sd,
        Rx,
        rx,
        Rx,
        rx,
        f32,
        f64,
        as_f32,
        |a: Ieee32| Ieee64::from_bits((f32::from_bits(a.to_bits()) as f64).to_bits())
    );
    conversion!(
        f32_from_f64,
        cvtsd2ss,
        Rx,
        rx,
        Rx,
        rx,
        f64,
        f32,
        as_f64,
        |a: Ieee64| Ieee32::from_bits((f64::from_bits(a.to_bits()) as f32).to_bits())
    );
    pub fn i32_truncate_f32_s(&mut self) -> Result<(), Error> {
        let mut val = self.pop()?;

        let out_val = match val {
            ValueLocation::Immediate(imm) => ValueLocation::Immediate(
                (f32::from_bits(imm.as_f32().unwrap().to_bits()) as i32).into(),
            ),
            _ => {
                let reg = self.into_reg(F32, &mut val).unwrap();
                let temp = self.take_reg(I32).unwrap();

                let sign_mask = self.aligned_label(4, LabelValue::I32(SIGN_MASK_F32 as i32));
                let float_cmp_mask = self.aligned_label(16, LabelValue::I32(0xcf000000u32 as i32));
                let zero = self.aligned_label(16, LabelValue::I32(0));
                let trap_label = self.trap_label();

                dynasm!(self.asm
                    ; cvttss2si Rd(temp.rq().unwrap()), Rx(reg.rx().unwrap())
                    ; cmp Rd(temp.rq().unwrap()), [=>sign_mask.0]
                    ; jne >ret
                    ; ucomiss Rx(reg.rx().unwrap()), Rx(reg.rx().unwrap())
                    ; jp =>trap_label.0
                    ; ucomiss Rx(reg.rx().unwrap()), [=>float_cmp_mask.0]
                    ; jnae =>trap_label.0
                    ; ucomiss Rx(reg.rx().unwrap()), [=>zero.0]
                    ; jnb =>trap_label.0
                ; ret:
                );

                ValueLocation::Reg(temp)
            }
        };

        self.free_value(val)?;

        self.push(out_val);
        Ok(())
    }

    pub fn i32_truncate_f32_u(&mut self) -> Result<(), Error> {
        let mut val = self.pop()?;

        let out_val = match val {
            ValueLocation::Immediate(imm) => ValueLocation::Immediate(
                (f32::from_bits(imm.as_f32().unwrap().to_bits()) as i32).into(),
            ),
            _ => {
                let reg = self.into_temp_reg(F32, &mut val).unwrap();
                let temp = self.take_reg(I32).unwrap();

                let sign_mask = self.aligned_label(4, LabelValue::I32(SIGN_MASK_F32 as i32));
                let float_cmp_mask = self.aligned_label(16, LabelValue::I32(0x4f000000u32 as i32));
                let trap_label = self.trap_label();

                dynasm!(self.asm
                    ; ucomiss Rx(reg.rx().unwrap()), [=>float_cmp_mask.0]
                    ; jae >else_
                    ; jp =>trap_label.0
                    ; cvttss2si Rd(temp.rq().unwrap()), Rx(reg.rx().unwrap())
                    ; test Rd(temp.rq().unwrap()), Rd(temp.rq().unwrap())
                    ; js =>trap_label.0
                    ; jmp >ret
                ; else_:
                    ; subss Rx(reg.rx().unwrap()), [=>float_cmp_mask.0]
                    ; cvttss2si Rd(temp.rq().unwrap()), Rx(reg.rx().unwrap())
                    ; test Rd(temp.rq().unwrap()), Rd(temp.rq().unwrap())
                    ; js =>trap_label.0
                    ; add Rq(temp.rq().unwrap()), [=>sign_mask.0]
                ; ret:
                );

                ValueLocation::Reg(temp)
            }
        };

        self.free_value(val)?;

        self.push(out_val);
        Ok(())
    }

    pub fn i32_truncate_f64_s(&mut self) -> Result<(), Error> {
        let mut val = self.pop()?;

        let out_val = match val {
            ValueLocation::Immediate(imm) => ValueLocation::Immediate(
                (f64::from_bits(imm.as_f64().unwrap().to_bits()) as i32).into(),
            ),
            _ => {
                let reg = self.into_reg(F32, &mut val).unwrap();
                let temp = self.take_reg(I32).unwrap();

                let sign_mask = self.aligned_label(4, LabelValue::I32(SIGN_MASK_F32 as i32));
                let float_cmp_mask =
                    self.aligned_label(16, LabelValue::I64(0xc1e0000000200000u64 as i64));
                let zero = self.aligned_label(16, LabelValue::I64(0));
                let trap_label = self.trap_label();

                dynasm!(self.asm
                    ; cvttsd2si Rd(temp.rq().unwrap()), Rx(reg.rx().unwrap())
                    ; cmp Rd(temp.rq().unwrap()), [=>sign_mask.0]
                    ; jne >ret
                    ; ucomisd Rx(reg.rx().unwrap()), Rx(reg.rx().unwrap())
                    ; jp =>trap_label.0
                    ; ucomisd Rx(reg.rx().unwrap()), [=>float_cmp_mask.0]
                    ; jna =>trap_label.0
                    ; ucomisd Rx(reg.rx().unwrap()), [=>zero.0]
                    ; jnb =>trap_label.0
                ; ret:
                );

                ValueLocation::Reg(temp)
            }
        };

        self.free_value(val)?;

        self.push(out_val);
        Ok(())
    }

    pub fn i32_truncate_f64_u(&mut self) -> Result<(), Error> {
        let mut val = self.pop()?;

        let out_val = match val {
            ValueLocation::Immediate(imm) => ValueLocation::Immediate(
                (f64::from_bits(imm.as_f64().unwrap().to_bits()) as u32).into(),
            ),
            _ => {
                let reg = self.into_temp_reg(F32, &mut val).unwrap();
                let temp = self.take_reg(I32).unwrap();

                let sign_mask = self.aligned_label(4, LabelValue::I32(SIGN_MASK_F32 as i32));
                let float_cmp_mask =
                    self.aligned_label(16, LabelValue::I64(0x41e0000000000000u64 as i64));
                let trap_label = self.trap_label();

                dynasm!(self.asm
                    ; ucomisd Rx(reg.rx().unwrap()), [=>float_cmp_mask.0]
                    ; jae >else_
                    ; jp =>trap_label.0
                    ; cvttsd2si Rd(temp.rq().unwrap()), Rx(reg.rx().unwrap())
                    ; test Rd(temp.rq().unwrap()), Rd(temp.rq().unwrap())
                    ; js =>trap_label.0
                    ; jmp >ret
                ; else_:
                    ; subsd Rx(reg.rx().unwrap()), [=>float_cmp_mask.0]
                    ; cvttsd2si Rd(temp.rq().unwrap()), Rx(reg.rx().unwrap())
                    ; test Rd(temp.rq().unwrap()), Rd(temp.rq().unwrap())
                    ; js =>trap_label.0
                    ; add Rq(temp.rq().unwrap()), [=>sign_mask.0]
                ; ret:
                );

                ValueLocation::Reg(temp)
            }
        };

        self.free_value(val)?;

        self.push(out_val);
        Ok(())
    }

    conversion!(
        f32_convert_from_i32_s,
        cvtsi2ss,
        Rd,
        rq,
        Rx,
        rx,
        i32,
        f32,
        as_i32,
        |a| Ieee32::from_bits((a as f32).to_bits())
    );
    conversion!(
        f64_convert_from_i32_s,
        cvtsi2sd,
        Rd,
        rq,
        Rx,
        rx,
        i32,
        f64,
        as_i32,
        |a| Ieee64::from_bits((a as f64).to_bits())
    );
    conversion!(
        f32_convert_from_i64_s,
        cvtsi2ss,
        Rq,
        rq,
        Rx,
        rx,
        i64,
        f32,
        as_i64,
        |a| Ieee32::from_bits((a as f32).to_bits())
    );
    conversion!(
        f64_convert_from_i64_s,
        cvtsi2sd,
        Rq,
        rq,
        Rx,
        rx,
        i64,
        f64,
        as_i64,
        |a| Ieee64::from_bits((a as f64).to_bits())
    );

    pub fn i64_truncate_f32_s(&mut self) -> Result<(), Error> {
        let mut val = self.pop()?;

        let out_val = match val {
            ValueLocation::Immediate(imm) => ValueLocation::Immediate(
                (f32::from_bits(imm.as_f32().unwrap().to_bits()) as i64).into(),
            ),
            _ => {
                let reg = self.into_temp_reg(F32, &mut val).unwrap();
                let temp = self.take_reg(I32).unwrap();

                let sign_mask = self.aligned_label(16, LabelValue::I64(SIGN_MASK_F64 as i64));
                let float_cmp_mask = self.aligned_label(16, LabelValue::I32(0xdf000000u32 as i32));
                let zero = self.aligned_label(16, LabelValue::I64(0));
                let trap_label = self.trap_label();

                dynasm!(self.asm
                    ; cvttss2si Rq(temp.rq().unwrap()), Rx(reg.rx().unwrap())
                    ; cmp Rq(temp.rq().unwrap()), [=>sign_mask.0]
                    ; jne >ret
                    ; ucomiss Rx(reg.rx().unwrap()), Rx(reg.rx().unwrap())
                    ; jp =>trap_label.0
                    ; ucomiss Rx(reg.rx().unwrap()), [=>float_cmp_mask.0]
                    ; jnae =>trap_label.0
                    ; ucomiss Rx(reg.rx().unwrap()), [=>zero.0]
                    ; jnb =>trap_label.0
                ; ret:
                );

                ValueLocation::Reg(temp)
            }
        };

        self.free_value(val)?;

        self.push(out_val);
        Ok(())
    }

    pub fn i64_truncate_f64_s(&mut self) -> Result<(), Error> {
        let mut val = self.pop()?;

        let out_val = match val {
            ValueLocation::Immediate(imm) => ValueLocation::Immediate(
                (f64::from_bits(imm.as_f64().unwrap().to_bits()) as i64).into(),
            ),
            _ => {
                let reg = self.into_reg(F32, &mut val).unwrap();
                let temp = self.take_reg(I32).unwrap();

                let sign_mask = self.aligned_label(8, LabelValue::I64(SIGN_MASK_F64 as i64));
                let float_cmp_mask =
                    self.aligned_label(16, LabelValue::I64(0xc3e0000000000000u64 as i64));
                let zero = self.aligned_label(16, LabelValue::I64(0));
                let trap_label = self.trap_label();

                dynasm!(self.asm
                    ; cvttsd2si Rq(temp.rq().unwrap()), Rx(reg.rx().unwrap())
                    ; cmp Rq(temp.rq().unwrap()), [=>sign_mask.0]
                    ; jne >ret
                    ; ucomisd Rx(reg.rx().unwrap()), Rx(reg.rx().unwrap())
                    ; jp =>trap_label.0
                    ; ucomisd Rx(reg.rx().unwrap()), [=>float_cmp_mask.0]
                    ; jnae =>trap_label.0
                    ; ucomisd Rx(reg.rx().unwrap()), [=>zero.0]
                    ; jnb =>trap_label.0
                ; ret:
                );

                ValueLocation::Reg(temp)
            }
        };

        self.free_value(val)?;

        self.push(out_val);
        Ok(())
    }

    pub fn i64_truncate_f32_u(&mut self) -> Result<(), Error> {
        let mut val = self.pop()?;

        let out_val = match val {
            ValueLocation::Immediate(imm) => ValueLocation::Immediate(
                (f32::from_bits(imm.as_f32().unwrap().to_bits()) as u64).into(),
            ),
            _ => {
                let reg = self.into_reg(F32, &mut val).unwrap();

                let temp = self.take_reg(I64).unwrap();
                let sign_mask = self.aligned_label(16, LabelValue::I64(SIGN_MASK_F64 as i64));
                let u64_trunc_f32_const = self.aligned_label(16, LabelValue::I32(0x5F000000));
                let trap_label = self.trap_label();

                dynasm!(self.asm
                    ; comiss Rx(reg.rx().unwrap()), [=>u64_trunc_f32_const.0]
                    ; jae >large
                    ; jp =>trap_label.0
                    ; cvttss2si Rq(temp.rq().unwrap()), Rx(reg.rx().unwrap())
                    ; test Rq(temp.rq().unwrap()), Rq(temp.rq().unwrap())
                    ; js =>trap_label.0
                    ; jmp >cont
                ; large:
                    ; subss Rx(reg.rx().unwrap()), [=>u64_trunc_f32_const.0]
                    ; cvttss2si Rq(temp.rq().unwrap()), Rx(reg.rx().unwrap())
                    ; test Rq(temp.rq().unwrap()), Rq(temp.rq().unwrap())
                    ; js =>trap_label.0
                    ; add Rq(temp.rq().unwrap()), [=>sign_mask.0]
                ; cont:
                );

                ValueLocation::Reg(temp)
            }
        };

        self.free_value(val)?;

        self.push(out_val);
        Ok(())
    }

    pub fn i64_truncate_f64_u(&mut self) -> Result<(), Error> {
        let mut val = self.pop()?;

        let out_val = match val {
            ValueLocation::Immediate(imm) => ValueLocation::Immediate(
                (f64::from_bits(imm.as_f64().unwrap().to_bits()) as u64).into(),
            ),
            _ => {
                let reg = self.into_reg(F64, &mut val).unwrap();
                let temp = self.take_reg(I64).unwrap();

                let sign_mask = self.aligned_label(16, LabelValue::I64(SIGN_MASK_F64 as i64));
                let u64_trunc_f64_const =
                    self.aligned_label(16, LabelValue::I64(0x43e0000000000000));
                let trap_label = self.trap_label();

                dynasm!(self.asm
                    ; comisd Rx(reg.rx().unwrap()), [=>u64_trunc_f64_const.0]
                    ; jnb >large
                    ; jp =>trap_label.0
                    ; cvttsd2si Rq(temp.rq().unwrap()), Rx(reg.rx().unwrap())
                    ; cmp Rq(temp.rq().unwrap()), 0
                    ; jge >cont
                    ; jmp =>trap_label.0
                ; large:
                    ; subsd Rx(reg.rx().unwrap()), [=>u64_trunc_f64_const.0]
                    ; cvttsd2si Rq(temp.rq().unwrap()), Rx(reg.rx().unwrap())
                    ; cmp Rq(temp.rq().unwrap()), 0
                    ; jnge =>trap_label.0
                    ; add Rq(temp.rq().unwrap()), [=>sign_mask.0]
                ; cont:
                );

                ValueLocation::Reg(temp)
            }
        };

        self.free_value(val)?;

        self.push(out_val);
        Ok(())
    }

    pub fn f32_convert_from_i32_u(&mut self) -> Result<(), Error> {
        let mut val = self.pop()?;

        let out_val = match val {
            ValueLocation::Immediate(imm) => ValueLocation::Immediate(
                Ieee32::from_bits((imm.as_i32().unwrap() as u32 as f32).to_bits()).into(),
            ),
            _ => {
                let reg = self.into_reg(I32, &mut val).unwrap();

                let temp = self.take_reg(F32).unwrap();

                dynasm!(self.asm
                    ; mov Rd(reg.rq().unwrap()), Rd(reg.rq().unwrap())
                    ; cvtsi2ss Rx(temp.rx().unwrap()), Rq(reg.rq().unwrap())
                );

                ValueLocation::Reg(temp)
            }
        };

        self.free_value(val)?;

        self.push(out_val);
        Ok(())
    }

    pub fn f64_convert_from_i32_u(&mut self) -> Result<(), Error> {
        let mut val = self.pop()?;

        let out_val = match val {
            ValueLocation::Immediate(imm) => ValueLocation::Immediate(
                Ieee64::from_bits((imm.as_i32().unwrap() as u32 as f64).to_bits()).into(),
            ),
            _ => {
                let reg = self.into_reg(I32, &mut val).unwrap();
                let temp = self.take_reg(F64).unwrap();

                dynasm!(self.asm
                    ; mov Rd(reg.rq().unwrap()), Rd(reg.rq().unwrap())
                    ; cvtsi2sd Rx(temp.rx().unwrap()), Rq(reg.rq().unwrap())
                );

                ValueLocation::Reg(temp)
            }
        };

        self.free_value(val)?;

        self.push(out_val);
        Ok(())
    }

    pub fn f32_convert_from_i64_u(&mut self) -> Result<(), Error> {
        let mut val = self.pop()?;

        let out_val = match val {
            ValueLocation::Immediate(imm) => ValueLocation::Immediate(
                Ieee32::from_bits((imm.as_i64().unwrap() as u64 as f32).to_bits()).into(),
            ),
            _ => {
                let reg = self.into_reg(I64, &mut val).unwrap();
                let out = self.take_reg(F32).unwrap();
                let temp = self.take_reg(I64).unwrap();

                dynasm!(self.asm
                    ; test Rq(reg.rq().unwrap()), Rq(reg.rq().unwrap())
                    ; js >negative
                    ; cvtsi2ss Rx(out.rx().unwrap()), Rq(reg.rq().unwrap())
                    ; jmp >ret
                ; negative:
                    ; mov Rq(temp.rq().unwrap()), Rq(reg.rq().unwrap())
                    ; shr Rq(temp.rq().unwrap()), 1
                    ; and Rq(reg.rq().unwrap()), 1
                    ; or Rq(reg.rq().unwrap()), Rq(temp.rq().unwrap())
                    ; cvtsi2ss Rx(out.rx().unwrap()), Rq(reg.rq().unwrap())
                    ; addss Rx(out.rx().unwrap()), Rx(out.rx().unwrap())
                ; ret:
                );

                self.free_value(ValueLocation::Reg(temp))?;

                ValueLocation::Reg(out)
            }
        };

        self.free_value(val)?;

        self.push(out_val);
        Ok(())
    }

    pub fn f64_convert_from_i64_u(&mut self) -> Result<(), Error> {
        let mut val = self.pop()?;

        let out_val = match val {
            ValueLocation::Immediate(imm) => ValueLocation::Immediate(
                Ieee64::from_bits((imm.as_i64().unwrap() as u64 as f64).to_bits()).into(),
            ),
            _ => {
                let reg = self.into_reg(I64, &mut val).unwrap();

                let out = self.take_reg(F32).unwrap();
                let temp = self.take_reg(I64).unwrap();

                dynasm!(self.asm
                    ; test Rq(reg.rq().unwrap()), Rq(reg.rq().unwrap())
                    ; js >negative
                    ; cvtsi2sd Rx(out.rx().unwrap()), Rq(reg.rq().unwrap())
                    ; jmp >ret
                ; negative:
                    ; mov Rq(temp.rq().unwrap()), Rq(reg.rq().unwrap())
                    ; shr Rq(temp.rq().unwrap()), 1
                    ; and Rq(reg.rq().unwrap()), 1
                    ; or Rq(reg.rq().unwrap()), Rq(temp.rq().unwrap())
                    ; cvtsi2sd Rx(out.rx().unwrap()), Rq(reg.rq().unwrap())
                    ; addsd Rx(out.rx().unwrap()), Rx(out.rx().unwrap())
                ; ret:
                );

                self.free_value(ValueLocation::Reg(temp))?;

                ValueLocation::Reg(out)
            }
        };

        self.free_value(val)?;

        self.push(out_val);
        Ok(())
    }

    pub fn i32_wrap_from_i64(&mut self) -> Result<(), Error> {
        let val = self.pop()?;

        let out = match val {
            ValueLocation::Immediate(imm) => {
                ValueLocation::Immediate((imm.as_i64().unwrap() as u64 as u32).into())
            }
            val => val,
        };

        self.push(out);
        Ok(())
    }

    pub fn i32_reinterpret_from_f32(&mut self) -> Result<(), Error> {
        let val = self.pop()?;

        let out = match val {
            ValueLocation::Immediate(imm) => {
                ValueLocation::Immediate(imm.as_f32().unwrap().to_bits().into())
            }
            val => val,
        };

        self.push(out);
        Ok(())
    }

    pub fn i64_reinterpret_from_f64(&mut self) -> Result<(), Error> {
        let val = self.pop()?;

        let out = match val {
            ValueLocation::Immediate(imm) => {
                ValueLocation::Immediate(imm.as_f64().unwrap().to_bits().into())
            }
            val => val,
        };

        self.push(out);
        Ok(())
    }

    pub fn f32_reinterpret_from_i32(&mut self) -> Result<(), Error> {
        let val = self.pop()?;

        let out = match val {
            ValueLocation::Immediate(imm) => {
                ValueLocation::Immediate(Ieee32::from_bits(imm.as_i32().unwrap() as _).into())
            }
            val => val,
        };

        self.push(out);
        Ok(())
    }

    pub fn f64_reinterpret_from_i64(&mut self) -> Result<(), Error> {
        let val = self.pop()?;

        let out = match val {
            ValueLocation::Immediate(imm) => {
                ValueLocation::Immediate(Ieee64::from_bits(imm.as_i64().unwrap() as _).into())
            }
            val => val,
        };

        self.push(out);
        Ok(())
    }

    unop!(i64_popcnt, popcnt, Rq, u64, |a: u64| a.count_ones() as u64);

    // TODO: Use `lea` when the LHS operand isn't a temporary but both of the operands
    //       are in registers.
    commutative_binop_i32!(i32_add, add, i32::wrapping_add);
    commutative_binop_i32!(i32_and, and, |a, b| a & b);
    commutative_binop_i32!(i32_or, or, |a, b| a | b);
    commutative_binop_i32!(i32_xor, xor, |a, b| a ^ b);
    binop_i32!(i32_sub, sub, i32::wrapping_sub);

    commutative_binop_i64!(i64_add, add, i64::wrapping_add);
    commutative_binop_i64!(i64_and, and, |a, b| a & b);
    commutative_binop_i64!(i64_or, or, |a, b| a | b);
    commutative_binop_i64!(i64_xor, xor, |a, b| a ^ b);
    binop_i64!(i64_sub, sub, i64::wrapping_sub);

    commutative_binop_f32!(f32_add, addss, |a, b| a + b);
    commutative_binop_f32!(f32_mul, mulss, |a, b| a * b);
    minmax_float!(
        f32_min,
        minss,
        ucomiss,
        addss,
        orps,
        as_f32,
        |a: Ieee32, b: Ieee32| Ieee32::from_bits(
            f32::from_bits(a.to_bits())
                .min(f32::from_bits(b.to_bits()))
                .to_bits()
        )
    );
    minmax_float!(
        f32_max,
        maxss,
        ucomiss,
        addss,
        andps,
        as_f32,
        |a: Ieee32, b: Ieee32| Ieee32::from_bits(
            f32::from_bits(a.to_bits())
                .max(f32::from_bits(b.to_bits()))
                .to_bits()
        )
    );
    binop_f32!(f32_sub, subss, |a, b| a - b);
    binop_f32!(f32_div, divss, |a, b| a / b);

    pub fn f32_ceil(&mut self) -> Result<(), Error> {
        self.relocated_function_call(
            &ir::ExternalName::LibCall(ir::LibCall::CeilF32),
            iter::once(F32),
            iter::once(F32),
            true,
        )?;
        Ok(())
    }

    pub fn f32_floor(&mut self) -> Result<(), Error> {
        self.relocated_function_call(
            &ir::ExternalName::LibCall(ir::LibCall::FloorF32),
            iter::once(F32),
            iter::once(F32),
            true,
        )?;
        Ok(())
    }

    pub fn f32_nearest(&mut self) -> Result<(), Error> {
        self.relocated_function_call(
            &ir::ExternalName::LibCall(ir::LibCall::NearestF32),
            iter::once(F32),
            iter::once(F32),
            true,
        )?;
        Ok(())
    }

    pub fn f32_trunc(&mut self) -> Result<(), Error> {
        self.relocated_function_call(
            &ir::ExternalName::LibCall(ir::LibCall::TruncF32),
            iter::once(F32),
            iter::once(F32),
            true,
        )?;
        Ok(())
    }

    commutative_binop_f64!(f64_add, addsd, |a, b| a + b);
    commutative_binop_f64!(f64_mul, mulsd, |a, b| a * b);
    minmax_float!(
        f64_min,
        minsd,
        ucomisd,
        addsd,
        orpd,
        as_f64,
        |a: Ieee64, b: Ieee64| Ieee64::from_bits(
            f64::from_bits(a.to_bits())
                .min(f64::from_bits(b.to_bits()))
                .to_bits()
        )
    );
    minmax_float!(
        f64_max,
        maxsd,
        ucomisd,
        addsd,
        andpd,
        as_f64,
        |a: Ieee64, b: Ieee64| Ieee64::from_bits(
            f64::from_bits(a.to_bits())
                .max(f64::from_bits(b.to_bits()))
                .to_bits()
        )
    );
    binop_f64!(f64_sub, subsd, |a, b| a - b);
    binop_f64!(f64_div, divsd, |a, b| a / b);

    pub fn f64_ceil(&mut self) -> Result<(), Error> {
        self.relocated_function_call(
            &ir::ExternalName::LibCall(ir::LibCall::CeilF64),
            iter::once(F64),
            iter::once(F64),
            true,
        )?;
        Ok(())
    }

    pub fn f64_floor(&mut self) -> Result<(), Error> {
        self.relocated_function_call(
            &ir::ExternalName::LibCall(ir::LibCall::FloorF64),
            iter::once(F64),
            iter::once(F64),
            true,
        )?;
        Ok(())
    }

    pub fn f64_nearest(&mut self) -> Result<(), Error> {
        self.relocated_function_call(
            &ir::ExternalName::LibCall(ir::LibCall::NearestF64),
            iter::once(F64),
            iter::once(F64),
            true,
        )?;
        Ok(())
    }

    pub fn f64_trunc(&mut self) -> Result<(), Error> {
        self.relocated_function_call(
            &ir::ExternalName::LibCall(ir::LibCall::TruncF64),
            iter::once(F64),
            iter::once(F64),
            true,
        )?;
        Ok(())
    }

    shift!(
        i32_shl,
        Rd,
        shl,
        |a, b| (a as i32).wrapping_shl(b as _),
        I32
    );
    shift!(
        i32_shr_s,
        Rd,
        sar,
        |a, b| (a as i32).wrapping_shr(b as _),
        I32
    );
    shift!(
        i32_shr_u,
        Rd,
        shr,
        |a, b| (a as u32).wrapping_shr(b as _),
        I32
    );
    shift!(
        i32_rotl,
        Rd,
        rol,
        |a, b| (a as u32).rotate_left(b as _),
        I32
    );
    shift!(
        i32_rotr,
        Rd,
        ror,
        |a, b| (a as u32).rotate_right(b as _),
        I32
    );

    shift!(
        i64_shl,
        Rq,
        shl,
        |a, b| (a as i64).wrapping_shl(b as _),
        I64
    );
    shift!(
        i64_shr_s,
        Rq,
        sar,
        |a, b| (a as i64).wrapping_shr(b as _),
        I64
    );
    shift!(
        i64_shr_u,
        Rq,
        shr,
        |a, b| (a as u64).wrapping_shr(b as _),
        I64
    );
    shift!(
        i64_rotl,
        Rq,
        rol,
        |a, b| (a as u64).rotate_left(b as _),
        I64
    );
    shift!(
        i64_rotr,
        Rq,
        ror,
        |a, b| (a as u64).rotate_right(b as _),
        I64
    );

    // TODO: Do this without emitting `mov`
    fn cleanup_gprs(&mut self, gprs: impl Iterator<Item = GPR>) {
        for gpr in gprs {
            dynasm!(self.asm
                ; pop Rq(gpr.rq().unwrap())
            );
            self.block_state.depth.free(1);
            // DON'T MARK IT USED HERE! See comment in `full_div`
        }
    }

    int_div!(
        i32_full_div_s,
        i32_full_div_u,
        i32_div_u,
        i32_div_s,
        i32_rem_u,
        i32_rem_s,
        imm_i32,
        i32,
        u32,
        Rd,
        DWORD
    );
    int_div!(
        i64_full_div_s,
        i64_full_div_u,
        i64_div_u,
        i64_div_s,
        i64_rem_u,
        i64_rem_s,
        imm_i64,
        i64,
        u64,
        Rq,
        QWORD
    );

    // TODO: With a proper SSE-like "Value" system we could do this way better (we wouldn't have
    //       to move `RAX`/`RDX` back afterwards).
    fn full_div(
        &mut self,
        mut divisor: ValueLocation,
        dividend: ValueLocation,
        do_div: impl FnOnce(&mut Self, &mut ValueLocation),
    ) -> Result<
        (
            ValueLocation,
            ValueLocation,
            impl Iterator<Item = GPR> + Clone + 'this,
        ),
        Error,
    > {
        // To stop `take_reg` from allocating either of these necessary registers
        self.block_state.regs.mark_used(RAX);
        self.block_state.regs.mark_used(RDX);
        if divisor == ValueLocation::Reg(RAX) || divisor == ValueLocation::Reg(RDX) {
            let new_reg = self.take_reg(GPRType::Rq).unwrap();
            self.copy_value(divisor, CCLoc::Reg(new_reg))?;
            self.free_value(divisor)?;

            divisor = ValueLocation::Reg(new_reg);
        }
        self.block_state.regs.release(RAX)?;
        self.block_state.regs.release(RDX)?;

        let saved_rax = if self.block_state.regs.is_free(RAX) {
            None
        } else {
            dynasm!(self.asm
                ; push rax
            );
            self.block_state.depth.reserve(1);
            // DON'T FREE THIS REGISTER HERE - since we don't
            // remove it from the stack freeing the register
            // here will cause `take_reg` to allocate it.
            Some(())
        };

        let saved_rdx = if self.block_state.regs.is_free(RDX) {
            None
        } else {
            dynasm!(self.asm
                ; push rdx
            );
            self.block_state.depth.reserve(1);
            // DON'T FREE THIS REGISTER HERE - since we don't
            // remove it from the stack freeing the register
            // here will cause `take_reg` to allocate it.
            Some(())
        };

        let saved = saved_rdx
            .map(|_| RDX)
            .into_iter()
            .chain(saved_rax.map(|_| RAX));

        self.copy_value(dividend, CCLoc::Reg(RAX))?;
        self.block_state.regs.mark_used(RAX);

        self.free_value(dividend)?;
        // To stop `take_reg` from allocating either of these necessary registers
        self.block_state.regs.mark_used(RDX);

        do_div(self, &mut divisor);
        self.free_value(divisor)?;

        assert!(!self.block_state.regs.is_free(RAX));
        assert!(!self.block_state.regs.is_free(RDX));

        Ok((ValueLocation::Reg(RAX), ValueLocation::Reg(RDX), saved))
    }

    fn i32_full_div_u(
        &mut self,
        divisor: ValueLocation,
        dividend: ValueLocation,
    ) -> Result<
        (
            ValueLocation,
            ValueLocation,
            impl Iterator<Item = GPR> + Clone + 'this,
        ),
        Error,
    > {
        self.full_div(divisor, dividend, |this, divisor| match divisor {
            ValueLocation::Stack(offset) => {
                let offset = this.adjusted_offset(*offset);
                dynasm!(this.asm
                    ; xor edx, edx
                    ; div DWORD [rsp + offset]
                );
            }
            ValueLocation::Immediate(_) | ValueLocation::Reg(_) | ValueLocation::Cond(_) => {
                let r = this.into_reg(I32, divisor).unwrap();
                dynasm!(this.asm
                    ; xor edx, edx
                    ; div Rd(r.rq().unwrap())
                );
            }
        })
    }

    fn i32_full_div_s(
        &mut self,
        divisor: ValueLocation,
        dividend: ValueLocation,
    ) -> Result<
        (
            ValueLocation,
            ValueLocation,
            impl Iterator<Item = GPR> + Clone + 'this,
        ),
        Error,
    > {
        self.full_div(divisor, dividend, |this, divisor| match divisor {
            ValueLocation::Stack(offset) => {
                let offset = this.adjusted_offset(*offset);
                dynasm!(this.asm
                    ; cdq
                    ; idiv DWORD [rsp + offset]
                );
            }
            ValueLocation::Immediate(_) | ValueLocation::Reg(_) | ValueLocation::Cond(_) => {
                let r = this.into_reg(I32, divisor).unwrap();
                dynasm!(this.asm
                    ; cdq
                    ; idiv Rd(r.rq().unwrap())
                );
            }
        })
    }

    fn i64_full_div_u(
        &mut self,
        divisor: ValueLocation,
        dividend: ValueLocation,
    ) -> Result<
        (
            ValueLocation,
            ValueLocation,
            impl Iterator<Item = GPR> + Clone + 'this,
        ),
        Error,
    > {
        self.full_div(divisor, dividend, |this, divisor| match divisor {
            ValueLocation::Stack(offset) => {
                let offset = this.adjusted_offset(*offset);
                dynasm!(this.asm
                    ; xor rdx, rdx
                    ; div QWORD [rsp + offset]
                );
            }
            ValueLocation::Immediate(_) | ValueLocation::Reg(_) | ValueLocation::Cond(_) => {
                let r = this.into_reg(I64, divisor).unwrap();
                dynasm!(this.asm
                    ; xor rdx, rdx
                    ; div Rq(r.rq().unwrap())
                );
            }
        })
    }

    fn i64_full_div_s(
        &mut self,
        divisor: ValueLocation,
        dividend: ValueLocation,
    ) -> Result<
        (
            ValueLocation,
            ValueLocation,
            impl Iterator<Item = GPR> + Clone + 'this,
        ),
        Error,
    > {
        self.full_div(divisor, dividend, |this, divisor| match divisor {
            ValueLocation::Stack(offset) => {
                let offset = this.adjusted_offset(*offset);
                dynasm!(this.asm
                    ; cqo
                    ; idiv QWORD [rsp + offset]
                );
            }
            ValueLocation::Immediate(_) | ValueLocation::Reg(_) | ValueLocation::Cond(_) => {
                let r = this.into_reg(I64, divisor).unwrap();
                dynasm!(this.asm
                    ; cqo
                    ; idiv Rq(r.rq().unwrap())
                );
            }
        })
    }

    // `i32_mul` needs to be separate because the immediate form of the instruction
    // has a different syntax to the immediate form of the other instructions.
    pub fn i32_mul(&mut self) -> Result<(), Error> {
        let right = self.pop()?;
        let left = self.pop()?;

        if let Some(right) = right.immediate() {
            if let Some(left) = left.immediate() {
                self.push(ValueLocation::Immediate(
                    i32::wrapping_mul(right.as_i32().unwrap(), left.as_i32().unwrap()).into(),
                ));
                return Ok(());
            }
        }

        let (mut left, mut right) = match left {
            ValueLocation::Reg(_) => (left, right),
            _ => {
                if right.immediate().is_some() {
                    (left, right)
                } else {
                    (right, left)
                }
            }
        };

        let out = match right {
            ValueLocation::Reg(_) | ValueLocation::Cond(_) => {
                let rreg = self.into_reg(I32, &mut right).unwrap();
                let lreg = self.into_temp_reg(I32, &mut left).unwrap();
                dynasm!(self.asm
                    ; imul Rd(lreg.rq().unwrap()), Rd(rreg.rq().unwrap())
                );
                left
            }
            ValueLocation::Stack(offset) => {
                let offset = self.adjusted_offset(offset);

                let lreg = self.into_temp_reg(I32, &mut left).unwrap();
                dynasm!(self.asm
                    ; imul Rd(lreg.rq().unwrap()), [rsp + offset]
                );
                left
            }
            ValueLocation::Immediate(i) => {
                let lreg = self.into_reg(I32, &mut left).unwrap();
                let new_reg = self.take_reg(I32).unwrap();
                dynasm!(self.asm
                    ; imul Rd(new_reg.rq().unwrap()), Rd(lreg.rq().unwrap()), i.as_i32().unwrap()
                );
                self.free_value(left)?;
                ValueLocation::Reg(new_reg)
            }
        };

        self.push(out);
        self.free_value(right)?;
        Ok(())
    }

    // `i64_mul` needs to be separate because the immediate form of the instruction
    // has a different syntax to the immediate form of the other instructions.
    pub fn i64_mul(&mut self) -> Result<(), Error> {
        let right = self.pop()?;
        let left = self.pop()?;

        if let Some(right) = right.immediate() {
            if let Some(left) = left.immediate() {
                self.push(ValueLocation::Immediate(
                    i64::wrapping_mul(right.as_i64().unwrap(), left.as_i64().unwrap()).into(),
                ));
                return Ok(());
            }
        }

        let (mut left, mut right) = match left {
            ValueLocation::Reg(_) => (left, right),
            _ => {
                if right.immediate().is_some() {
                    (left, right)
                } else {
                    (right, left)
                }
            }
        };

        let out = match right {
            ValueLocation::Reg(_) | ValueLocation::Cond(_) => {
                let rreg = self.into_reg(I64, &mut right).unwrap();
                let lreg = self.into_temp_reg(I64, &mut left).unwrap();
                dynasm!(self.asm
                    ; imul Rq(lreg.rq().unwrap()), Rq(rreg.rq().unwrap())
                );
                left
            }
            ValueLocation::Stack(offset) => {
                let offset = self.adjusted_offset(offset);

                let lreg = self.into_temp_reg(I64, &mut left).unwrap();
                dynasm!(self.asm
                    ; imul Rq(lreg.rq().unwrap()), [rsp + offset]
                );
                left
            }
            ValueLocation::Immediate(i) => {
                let i = i.as_i64().unwrap();
                if let Some(i) = i.try_into().ok() {
                    let new_reg = self.take_reg(I64).unwrap();
                    let lreg = self.into_reg(I64, &mut left).unwrap();

                    dynasm!(self.asm
                        ; imul Rq(new_reg.rq().unwrap()), Rq(lreg.rq().unwrap()), i
                    );

                    self.free_value(left)?;

                    ValueLocation::Reg(new_reg)
                } else {
                    let rreg = self.into_reg(I64, &mut right).unwrap();
                    let lreg = self.into_temp_reg(I64, &mut left).unwrap();
                    dynasm!(self.asm
                        ; imul Rq(lreg.rq().unwrap()), Rq(rreg.rq().unwrap())
                    );
                    left
                }
            }
        };

        self.push(out);
        self.free_value(right)?;
        Ok(())
    }

    fn cmov(&mut self, cond_code: CondCode, dst: GPR, src: CCLoc) {
        match src {
            CCLoc::Reg(reg) => match cond_code {
                cc::EQUAL => {
                    dynasm!(self.asm
                        ; cmove Rq(dst.rq().unwrap()), Rq(reg.rq().unwrap())
                    );
                }
                cc::NOT_EQUAL => {
                    dynasm!(self.asm
                        ; cmovne Rq(dst.rq().unwrap()), Rq(reg.rq().unwrap())
                    );
                }
                cc::GE_U => {
                    dynasm!(self.asm
                        ; cmovae Rq(dst.rq().unwrap()), Rq(reg.rq().unwrap())
                    );
                }
                cc::LT_U => {
                    dynasm!(self.asm
                        ; cmovb Rq(dst.rq().unwrap()), Rq(reg.rq().unwrap())
                    );
                }
                cc::GT_U => {
                    dynasm!(self.asm
                        ; cmova Rq(dst.rq().unwrap()), Rq(reg.rq().unwrap())
                    );
                }
                cc::LE_U => {
                    dynasm!(self.asm
                        ; cmovbe Rq(dst.rq().unwrap()), Rq(reg.rq().unwrap())
                    );
                }
                cc::GE_S => {
                    dynasm!(self.asm
                        ; cmovge Rq(dst.rq().unwrap()), Rq(reg.rq().unwrap())
                    );
                }
                cc::LT_S => {
                    dynasm!(self.asm
                        ; cmovl Rq(dst.rq().unwrap()), Rq(reg.rq().unwrap())
                    );
                }
                cc::GT_S => {
                    dynasm!(self.asm
                        ; cmovg Rq(dst.rq().unwrap()), Rq(reg.rq().unwrap())
                    );
                }
                cc::LE_S => {
                    dynasm!(self.asm
                        ; cmovle Rq(dst.rq().unwrap()), Rq(reg.rq().unwrap())
                    );
                }
            },
            CCLoc::Stack(offset) => {
                let offset = self.adjusted_offset(offset);

                match cond_code {
                    cc::EQUAL => {
                        dynasm!(self.asm
                            ; cmove Rq(dst.rq().unwrap()), [rsp + offset]
                        );
                    }
                    cc::NOT_EQUAL => {
                        dynasm!(self.asm
                            ; cmovne Rq(dst.rq().unwrap()), [rsp + offset]
                        );
                    }
                    cc::GE_U => {
                        dynasm!(self.asm
                            ; cmovae Rq(dst.rq().unwrap()), [rsp + offset]
                        );
                    }
                    cc::LT_U => {
                        dynasm!(self.asm
                            ; cmovb Rq(dst.rq().unwrap()), [rsp + offset]
                        );
                    }
                    cc::GT_U => {
                        dynasm!(self.asm
                            ; cmova Rq(dst.rq().unwrap()), [rsp + offset]
                        );
                    }
                    cc::LE_U => {
                        dynasm!(self.asm
                            ; cmovbe Rq(dst.rq().unwrap()), [rsp + offset]
                        );
                    }
                    cc::GE_S => {
                        dynasm!(self.asm
                            ; cmovge Rq(dst.rq().unwrap()), [rsp + offset]
                        );
                    }
                    cc::LT_S => {
                        dynasm!(self.asm
                            ; cmovl Rq(dst.rq().unwrap()), [rsp + offset]
                        );
                    }
                    cc::GT_S => {
                        dynasm!(self.asm
                            ; cmovg Rq(dst.rq().unwrap()), [rsp + offset]
                        );
                    }
                    cc::LE_S => {
                        dynasm!(self.asm
                            ; cmovle Rq(dst.rq().unwrap()), [rsp + offset]
                        );
                    }
                }
            }
        }
    }

    pub fn select(&mut self) -> Result<(), Error> {
        let mut cond = self.pop()?;
        let mut else_ = self.pop()?;
        let mut then = self.pop()?;

        if let ValueLocation::Immediate(i) = cond {
            if i.as_i32().unwrap() == 0 {
                self.free_value(then)?;
                self.push(else_);
            } else {
                self.free_value(else_)?;
                self.push(then);
            }

            return Ok(());
        }

        let cond_code = match cond {
            ValueLocation::Cond(cc) => cc,
            _ => {
                let cond_reg = self.into_reg(I32, &mut cond).unwrap();
                dynasm!(self.asm
                    ; test Rd(cond_reg.rq().unwrap()), Rd(cond_reg.rq().unwrap())
                );
                self.free_value(cond)?;

                cc::NOT_EQUAL
            }
        };

        let else_ = if let ValueLocation::Stack(offset) = else_ {
            CCLoc::Stack(offset)
        } else {
            CCLoc::Reg(self.into_reg(I32, &mut else_).unwrap())
        };

        let then = if let ValueLocation::Stack(offset) = then {
            CCLoc::Stack(offset)
        } else {
            CCLoc::Reg(self.into_reg(I32, &mut then).unwrap())
        };

        let out_gpr = match (then, else_) {
            (CCLoc::Reg(then_reg), else_) if self.block_state.regs.num_usages(then_reg) <= 1 => {
                self.cmov(!cond_code, then_reg, else_);
                self.free_value(else_.into())?;

                then_reg
            }
            (then, CCLoc::Reg(else_reg)) if self.block_state.regs.num_usages(else_reg) <= 1 => {
                self.cmov(cond_code, else_reg, then);
                self.free_value(then.into())?;

                else_reg
            }
            (then, else_) => {
                let out = self.take_reg(GPRType::Rq).unwrap();
                self.copy_value(else_.into(), CCLoc::Reg(out))?;
                self.cmov(cond_code, out, then);

                self.free_value(then.into())?;
                self.free_value(else_.into())?;

                out
            }
        };

        self.push(ValueLocation::Reg(out_gpr));
        Ok(())
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

    pub fn const_(&mut self, imm: Value) {
        self.push(ValueLocation::Immediate(imm));
    }

    fn relocated_function_call(
        &mut self,
        name: &cranelift_codegen::ir::ExternalName,
        args: impl IntoIterator<Item = SignlessType>,
        rets: impl IntoIterator<Item = SignlessType>,
        preserve_vmctx: bool,
    ) -> Result<(), Error> {
        let locs = arg_locs(args);

        self.save_volatile(..locs.len())?;

        if preserve_vmctx {
            dynasm!(self.asm
                ; push Rq(VMCTX)
            );
            self.block_state.depth.reserve(1);
        }

        let depth = self.block_state.depth;

        self.pass_outgoing_args(&locs)?;
        // 2 bytes for the 64-bit `mov` opcode + register ident, the rest is the immediate
        self.reloc_sink.reloc_external(
            (self.asm.offset().0
                - self.func_starts[self.current_function as usize]
                    .0
                    .unwrap()
                    .0) as u32
                + 2,
            binemit::Reloc::Abs8,
            name,
            0,
        );
        let temp = self.take_reg(I64).unwrap();
        dynasm!(self.asm
            ; mov Rq(temp.rq().unwrap()), QWORD 0xdeadbeefdeadbeefu64 as i64
            ; call Rq(temp.rq().unwrap())
        );
        self.block_state.regs.release(temp)?;

        for i in locs {
            self.free_value(i.into())?;
        }

        self.push_function_returns(rets);

        if preserve_vmctx {
            self.set_stack_depth(depth)?;

            dynasm!(self.asm
                ; pop Rq(VMCTX)
            );
            self.block_state.depth.free(1);
        }
        Ok(())
    }

    // TODO: Other memory indices
    pub fn memory_size(&mut self) -> Result<(), Error> {
        let memory_index = 0;
        if let Some(defined_memory_index) = self.module_context.defined_memory_index(memory_index) {
            self.push(ValueLocation::Immediate(defined_memory_index.into()));
            self.relocated_function_call(
                &magic::get_memory32_size_name(),
                iter::once(I32),
                iter::once(I32),
                true,
            )?;
        } else {
            self.push(ValueLocation::Immediate(memory_index.into()));
            self.relocated_function_call(
                &magic::get_imported_memory32_size_name(),
                iter::once(I32),
                iter::once(I32),
                true,
            )?;
        }
        Ok(())
    }

    // TODO: Other memory indices
    pub fn memory_grow(&mut self) -> Result<(), Error> {
        let memory_index = 0;
        if let Some(defined_memory_index) = self.module_context.defined_memory_index(memory_index) {
            self.push(ValueLocation::Immediate(defined_memory_index.into()));
            self.relocated_function_call(
                &magic::get_memory32_grow_name(),
                iter::once(I32).chain(iter::once(I32)),
                iter::once(I32),
                true,
            )?;
        } else {
            self.push(ValueLocation::Immediate(memory_index.into()));
            self.relocated_function_call(
                &magic::get_imported_memory32_grow_name(),
                iter::once(I32).chain(iter::once(I32)),
                iter::once(I32),
                true,
            )?;
        }
        Ok(())
    }

    // TODO: Use `ArrayVec`?
    // TODO: This inefficiently duplicates registers but it's not really possible
    //       to double up stack space right now.
    /// Saves volatile (i.e. caller-saved) registers before a function call, if they are used.
    fn save_volatile(&mut self, _bounds: impl std::ops::RangeBounds<usize>) -> Result<(), Error> {
        self.save_regs(SCRATCH_REGS, ..)?;
        Ok(())
    }

    fn save_regs<I>(
        &mut self,
        regs: &I,
        bounds: impl std::ops::RangeBounds<usize>,
    ) -> Result<(), Error>
    where
        for<'a> &'a I: IntoIterator<Item = &'a GPR>,
        I: ?Sized,
    {
        use std::ops::Bound::*;

        let mut stack = mem::replace(&mut self.block_state.stack, vec![]);
        let (start, end) = (
            match bounds.end_bound() {
                Unbounded => 0,
                Included(v) => stack.len().saturating_sub(1 + v),
                Excluded(v) => stack.len().saturating_sub(*v),
            },
            match bounds.start_bound() {
                Unbounded => stack.len(),
                Included(v) => stack.len().saturating_sub(*v),
                Excluded(v) => stack.len().saturating_sub(1 + v),
            },
        );

        let mut slice = &mut stack[start..end];

        while let Some((first, rest)) = slice.split_first_mut() {
            if let ValueLocation::Reg(vreg) = *first {
                if regs.into_iter().any(|r| *r == vreg) {
                    let old = *first;
                    *first = self.push_physical(old)?;
                    for val in &mut *rest {
                        if *val == old {
                            self.free_value(*val)?;
                            *val = *first;
                        }
                    }
                }
            }

            slice = rest;
        }

        mem::replace(&mut self.block_state.stack, stack);
        Ok(())
    }

    /// Write the arguments to the callee to the registers and the stack using the SystemV
    /// calling convention.
    fn pass_outgoing_args(&mut self, out_locs: &[CCLoc]) -> Result<(), Error> {
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
        let mut depth = self.block_state.depth.0 + total_stack_space;

        if depth & 1 != 0 {
            self.set_stack_depth(StackDepth(self.block_state.depth.0 + 1))?;
            depth += 1;
        }

        let mut pending = Vec::<(ValueLocation, CCLoc)>::with_capacity(out_locs.len());

        for &loc in out_locs.iter().rev() {
            let val = self.pop()?;

            pending.push((val, loc));
        }

        while !pending.is_empty() {
            let start_len = pending.len();

            for (src, dst) in mem::replace(&mut pending, vec![]) {
                if src != ValueLocation::from(dst) {
                    if let CCLoc::Reg(r) = dst {
                        if !self.block_state.regs.is_free(r) {
                            pending.push((src, dst));
                            continue;
                        }

                        self.block_state.regs.mark_used(r);
                    }

                    self.copy_value(src, dst)?;
                    self.free_value(src)?;
                }
            }

            if pending.len() == start_len {
                let src = *pending
                    .iter()
                    .filter_map(|(src, _)| {
                        if let ValueLocation::Reg(reg) = src {
                            Some(reg)
                        } else {
                            None
                        }
                    })
                    .next()
                    .expect(
                        "Programmer error: We shouldn't need to push \
                         intermediate args if we don't have any argument sources in registers",
                    );
                let new_src = self.push_physical(ValueLocation::Reg(src))?;
                for (old_src, _) in pending.iter_mut() {
                    if *old_src == ValueLocation::Reg(src) {
                        *old_src = new_src;
                    }
                }
            }
        }

        self.set_stack_depth(StackDepth(depth))?;
        Ok(())
    }

    fn push_function_returns(&mut self, returns: impl IntoIterator<Item = SignlessType>) {
        for loc in ret_locs(returns) {
            if let CCLoc::Reg(reg) = loc {
                self.block_state.regs.mark_used(reg);
            }

            self.push(loc.into());
        }
    }

    pub fn call_indirect(
        &mut self,
        type_id: u32,
        arg_types: impl IntoIterator<Item = SignlessType>,
        return_types: impl IntoIterator<Item = SignlessType>,
    ) -> Result<(), Error> {
        let locs = arg_locs(arg_types);

        for &loc in &locs {
            if let CCLoc::Reg(r) = loc {
                self.block_state.regs.mark_used(r);
            }
        }

        let mut callee = self.pop()?;
        let callee_reg = self.into_temp_reg(I32, &mut callee).unwrap();

        for &loc in &locs {
            if let CCLoc::Reg(r) = loc {
                self.block_state.regs.release(r)?;
            }
        }

        self.save_volatile(..locs.len())?;
        dynasm!(self.asm
            ; push Rq(VMCTX)
        );
        self.block_state.depth.reserve(1);
        let depth = self.block_state.depth;

        self.pass_outgoing_args(&locs)?;

        let fail = self.trap_label().0;
        let table_index = 0;
        let reg_offset = self
            .module_context
            .defined_table_index(table_index)
            .map(|index| {
                (
                    None,
                    self.module_context.vmctx_vmtable_definition(index) as i32,
                )
            });

        let vmctx = GPR::Rq(VMCTX);
        let (reg, offset) = reg_offset.unwrap_or_else(|| {
            let reg = self.take_reg(I64).unwrap();

            dynasm!(self.asm
                ; mov Rq(reg.rq().unwrap()), [
                    Rq(VMCTX) + self.module_context.vmctx_vmtable_import_from(table_index) as i32
                ]
            );

            (Some(reg), 0)
        });

        let temp0 = self.take_reg(I64).unwrap();

        dynasm!(self.asm
            ; cmp Rd(callee_reg.rq().unwrap()), [
                Rq(reg.unwrap_or(vmctx).rq().unwrap()) +
                    offset +
                    self.module_context.vmtable_definition_current_elements() as i32
            ]
            ; jae =>fail
            ; imul
                Rd(callee_reg.rq().unwrap()),
                Rd(callee_reg.rq().unwrap()),
                self.module_context.size_of_vmcaller_checked_anyfunc() as i32
            ; mov Rq(temp0.rq().unwrap()), [
                Rq(reg.unwrap_or(vmctx).rq().unwrap()) +
                    offset +
                    self.module_context.vmtable_definition_base() as i32
            ]
        );

        if let Some(reg) = reg {
            self.block_state.regs.release(reg)?;
        }

        let temp1 = self.take_reg(I64).unwrap();

        dynasm!(self.asm
            ; mov Rd(temp1.rq().unwrap()), [
                Rq(VMCTX) +
                    self.module_context
                        .vmctx_vmshared_signature_id(type_id) as i32
            ]
            ; cmp DWORD [
                Rq(temp0.rq().unwrap()) +
                    Rq(callee_reg.rq().unwrap()) +
                    self.module_context.vmcaller_checked_anyfunc_type_index() as i32
            ], Rd(temp1.rq().unwrap())
            ; jne =>fail
            ; mov Rq(VMCTX), [
                Rq(temp0.rq().unwrap()) +
                    Rq(callee_reg.rq().unwrap()) +
                    self.module_context.vmcaller_checked_anyfunc_vmctx() as i32
            ]
            ; call QWORD [
                Rq(temp0.rq().unwrap()) +
                    Rq(callee_reg.rq().unwrap()) +
                    self.module_context.vmcaller_checked_anyfunc_func_ptr() as i32
            ]
        );

        self.block_state.regs.release(temp0)?;
        self.block_state.regs.release(temp1)?;
        self.free_value(callee)?;

        for i in locs {
            self.free_value(i.into())?;
        }

        self.push_function_returns(return_types);

        self.set_stack_depth(depth)?;
        dynasm!(self.asm
            ; pop Rq(VMCTX)
        );
        self.block_state.depth.free(1);
        Ok(())
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
        return_types: impl IntoIterator<Item = SignlessType>,
    ) -> Result<(), Error> {
        self.relocated_function_call(
            &ir::ExternalName::user(0, index),
            arg_types,
            return_types,
            false,
        )?;
        Ok(())
    }

    /// Call a function with the given index
    pub fn call_direct_self(
        &mut self,
        defined_index: u32,
        arg_types: impl IntoIterator<Item = SignlessType>,
        return_types: impl IntoIterator<Item = SignlessType>,
    ) -> Result<(), Error> {
        let locs = arg_locs(arg_types);

        self.save_volatile(..locs.len())?;

        let (_, label) = self.func_starts[defined_index as usize];

        self.pass_outgoing_args(&locs)?;
        dynasm!(self.asm
            ; call =>label
        );

        for i in locs {
            self.free_value(i.into())?;
        }

        self.push_function_returns(return_types);
        Ok(())
    }

    /// Call a function with the given index
    pub fn call_direct_imported(
        &mut self,
        index: u32,
        arg_types: impl IntoIterator<Item = SignlessType>,
        return_types: impl IntoIterator<Item = SignlessType>,
    ) -> Result<(), Error> {
        let locs = arg_locs(arg_types);

        dynasm!(self.asm
            ; push Rq(VMCTX)
        );
        self.block_state.depth.reserve(1);
        let depth = self.block_state.depth;

        self.save_volatile(..locs.len())?;
        self.pass_outgoing_args(&locs)?;

        let callee = self.take_reg(I64).unwrap();

        dynasm!(self.asm
            ; mov Rq(callee.rq().unwrap()), [
                Rq(VMCTX) + self.module_context.vmctx_vmfunction_import_body(index) as i32
            ]
            ; mov Rq(VMCTX), [
                Rq(VMCTX) + self.module_context.vmctx_vmfunction_import_vmctx(index) as i32
            ]
            ; call Rq(callee.rq().unwrap())
        );

        self.block_state.regs.release(callee)?;

        for i in locs {
            self.free_value(i.into())?;
        }

        self.push_function_returns(return_types);

        self.set_stack_depth(depth)?;
        dynasm!(self.asm
            ; pop Rq(VMCTX)
        );
        self.block_state.depth.free(1);
        Ok(())
    }

    // TODO: Reserve space to store RBX, RBP, and R12..R15 so we can use them
    //       as scratch registers
    /// Writes the function prologue and stores the arguments as locals
    pub fn start_function(&mut self, params: impl IntoIterator<Item = SignlessType>) {
        let locs = Vec::from_iter(arg_locs(params));

        self.apply_cc(&BlockCallingConvention::function_start(locs));
    }

    pub fn ret(&mut self) {
        dynasm!(self.asm
            ; ret
        );
    }

    pub fn epilogue(&mut self) {}

    pub fn trap(&mut self) {
        let trap_label = self.trap_label();
        dynasm!(self.asm
            ; jmp =>trap_label.0
        );
    }

    pub fn trap_label(&mut self) -> Label {
        self.label(|asm: &mut Assembler| {
            dynasm!(asm
                ; ud2
            );
        })
    }

    pub fn ret_label(&mut self) -> Label {
        self.label(|asm: &mut Assembler| {
            dynasm!(asm
                ; ret
            );
        })
    }

    fn label<F>(&mut self, fun: F) -> Label
    where
        F: IntoLabel,
    {
        self.aligned_label(1, fun)
    }

    fn aligned_label<F>(&mut self, align: u32, fun: F) -> Label
    where
        F: IntoLabel,
    {
        let key = fun.key();
        if let Some((label, _, _)) = self.labels.get(&(align, key)) {
            return *label;
        }

        let label = self.create_label();
        self.labels
            .insert((align, key), (label, align, Some(fun.callback())));

        label
    }

    fn target_to_label(&mut self, target: BrTarget<Label>) -> Label {
        match target {
            BrTarget::Label(label) => label,
            BrTarget::Return => self.ret_label(),
        }
    }
}

trait IntoLabel {
    fn key(&self) -> Either<TypeId, (LabelValue, Option<LabelValue>)>;
    fn callback(self) -> Box<dyn FnMut(&mut Assembler)>;
}

impl<F> IntoLabel for F
where
    F: FnMut(&mut Assembler) + Any,
{
    fn key(&self) -> Either<TypeId, (LabelValue, Option<LabelValue>)> {
        Either::Left(TypeId::of::<Self>())
    }

    fn callback(self) -> Box<dyn FnMut(&mut Assembler)> {
        Box::new(self)
    }
}

fn const_value(val: LabelValue) -> impl FnMut(&mut Assembler) {
    move |asm| match val {
        LabelValue::I32(val) => dynasm!(asm
            ; .dword val
        ),
        LabelValue::I64(val) => dynasm!(asm
            ; .qword val
        ),
    }
}

fn const_values(a: LabelValue, b: LabelValue) -> impl FnMut(&mut Assembler) {
    move |asm| {
        match a {
            LabelValue::I32(val) => dynasm!(asm
                ; .dword val
            ),
            LabelValue::I64(val) => dynasm!(asm
                ; .qword val
            ),
        }

        match b {
            LabelValue::I32(val) => dynasm!(asm
                ; .dword val
            ),
            LabelValue::I64(val) => dynasm!(asm
                ; .qword val
            ),
        }
    }
}

impl IntoLabel for LabelValue {
    fn key(&self) -> Either<TypeId, (LabelValue, Option<LabelValue>)> {
        Either::Right((*self, None))
    }
    fn callback(self) -> Box<dyn FnMut(&mut Assembler)> {
        Box::new(const_value(self))
    }
}

impl IntoLabel for (LabelValue, LabelValue) {
    fn key(&self) -> Either<TypeId, (LabelValue, Option<LabelValue>)> {
        Either::Right((self.0, Some(self.1)))
    }
    fn callback(self) -> Box<dyn FnMut(&mut Assembler)> {
        Box::new(const_values(self.0, self.1))
    }
}
