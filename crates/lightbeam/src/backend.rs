#![allow(clippy::float_cmp)]

use self::registers::*;
use crate::{
    alloc::{Alloc, Ptr, Size},
    error::{error, Error},
    microwasm::{BrTarget, Ieee32, Ieee64, SignlessType, Type, Value, F32, F64, I32, I64},
    module::{ModuleContext, Signature},
    Sinks,
};
use cranelift_codegen::{
    binemit,
    ir::{self, SourceLoc, TrapCode},
};
use dynasm::dynasm;
use dynasmrt::x64::Assembler;
use dynasmrt::{AssemblyOffset, DynamicLabel, DynasmApi, DynasmLabelApi, ExecutableBuffer};
use more_asserts::{assert_ge, assert_lt, debug_assert_ge, debug_assert_lt};
#[cfg(debug_assertions)]
use more_asserts::{assert_le, debug_assert_le};
use std::{
    collections::HashMap,
    convert::{TryFrom, TryInto},
    fmt::Display,
    hash::Hash,
    iter::{self, FromIterator},
    mem,
    ops::{Deref, RangeInclusive},
};
// use wasmtime_environ::BuiltinFunctionIndex;

mod magic {
    /// An index type for builtin functions.
    pub struct BuiltinFunctionIndex(u32);

    impl BuiltinFunctionIndex {
        /// Returns an index for wasm's `memory.grow` builtin function.
        pub const fn get_memory32_grow_index() -> Self {
            Self(0)
        }
        /// Returns an index for wasm's imported `memory.grow` builtin function.
        pub const fn get_imported_memory32_grow_index() -> Self {
            Self(1)
        }
        /// Returns an index for wasm's `memory.size` builtin function.
        pub const fn get_memory32_size_index() -> Self {
            Self(2)
        }
        /// Returns an index for wasm's imported `memory.size` builtin function.
        pub const fn get_imported_memory32_size_index() -> Self {
            Self(3)
        }

        /// Return the index as an u32 number.
        pub const fn index(&self) -> u32 {
            self.0
        }
    }
}

use magic::BuiltinFunctionIndex;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Locs<I> {
    pub max_depth: StackDepth,
    pub locs: I,
}

impl<T: Copy + 'static, I: Deref> Locs<I>
where
    for<'a> &'a I::Target: IntoIterator<Item = &'a T>,
{
    pub fn as_ref(&self) -> Locs<iter::Copied<<&'_ I::Target as IntoIterator>::IntoIter>> {
        Locs {
            // We start and return the function with stack depth 1 since we must
            // allow space for the saved return address.
            max_depth: self.max_depth.clone(),
            locs: self.locs.into_iter().copied(),
        }
    }
}

fn arg_locs<T: From<CCLoc>, I: IntoIterator<Item = SignlessType>>(types: I) -> Locs<Vec<T>>
where
    I::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
{
    // TODO: VmCtx is in the first register
    let mut int_gpr_iter = INTEGER_ARGS_IN_GPRS.iter();
    let mut float_gpr_iter = FLOAT_ARGS_IN_GPRS.iter();
    let mut stack_idx = 0u32;

    let iter = types
        .into_iter()
        .map(|ty| {
            match ty {
                I32 | I64 => int_gpr_iter.next(),
                F32 | F64 => float_gpr_iter.next(),
            }
            .map(|&r| CCLoc::Reg(r))
            .unwrap_or_else(|| {
                let out = CCLoc::Stack(stack_idx as i32);
                stack_idx += 1;
                out
            })
        })
        .map(From::from)
        // Since we only advance the iterators based on the values in `types`,
        // we can't do this lazily.
        .collect::<Vec<_>>();

    Locs {
        max_depth: StackDepth(stack_idx),
        locs: iter,
    }
}

fn arg_locs_skip_caller_vmctx<T: From<CCLoc>, I: IntoIterator<Item = SignlessType>>(
    types: I,
) -> Locs<Vec<T>>
where
    I::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
{
    #[derive(Debug, Clone)]
    struct WithInt<I> {
        caller_vmctx_ty: Option<SignlessType>,
        iter: I,
    }

    impl<I> Iterator for WithInt<I>
    where
        I: Iterator<Item = SignlessType>,
    {
        type Item = SignlessType;

        fn next(&mut self) -> Option<Self::Item> {
            self.caller_vmctx_ty.take().or_else(|| self.iter.next())
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            let ty_len = if self.caller_vmctx_ty.is_some() { 1 } else { 0 };
            let (lower, upper) = self.iter.size_hint();

            (lower + ty_len, upper.map(|u| u + ty_len))
        }
    }

    impl<I> DoubleEndedIterator for WithInt<I>
    where
        I: DoubleEndedIterator<Item = SignlessType>,
    {
        fn next_back(&mut self) -> Option<Self::Item> {
            self.iter
                .next_back()
                .or_else(|| self.caller_vmctx_ty.take())
        }
    }

    impl<I> ExactSizeIterator for WithInt<I> where I: ExactSizeIterator<Item = SignlessType> {}

    let Locs {
        mut locs,
        max_depth,
    } = arg_locs::<T, _>(WithInt {
        caller_vmctx_ty: Some(I32),
        iter: types.into_iter(),
    });

    locs.remove(0);

    Locs { locs, max_depth }
}

pub fn ret_locs(types: impl IntoIterator<Item = SignlessType>) -> Result<Vec<CCLoc>, Error> {
    let types = types.into_iter();
    let mut out = Vec::with_capacity(types.size_hint().0);
    // TODO: VmCtx is in the first register
    let mut int_gpr_iter = INTEGER_RETURN_GPRS.iter();
    let mut float_gpr_iter = FLOAT_RETURN_GPRS.iter();

    for ty in types {
        match ty {
            I32 | I64 => match int_gpr_iter.next() {
                None => return Err(error("We don't support stack returns yet".to_string())),
                Some(val) => out.push(CCLoc::Reg(*val)),
            },
            F32 | F64 => match float_gpr_iter.next() {
                None => return Err(error("We don't support stack returns yet".to_string())),
                Some(val) => out.push(CCLoc::Reg(*val)),
            },
        }
    }

    Ok(out)
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

const SIGN_MASK_F64: u64 = 0x8000_0000_0000_0000;
const REST_MASK_F64: u64 = !SIGN_MASK_F64;
const SIGN_MASK_F32: u32 = 0x8000_0000;
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
        Self {
            scratch_64: (GPRs::new(), [1; NUM_GPRS as _]),
            scratch_128: (GPRs::new(), [1; NUM_GPRS as _]),
        }
    }

    pub fn release_scratch_register(&mut self) -> Result<(), Error> {
        // Give ourselves a few scratch registers to work with, for now.
        for &scratch in SCRATCH_REGS {
            self.release(scratch)?;
        }
        Ok(())
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
            None => return Err(error(format!("Double-freed register: {}", gpr))),
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
pub struct CallingConvention<I = Stack> {
    pub arguments: Locs<I>,
    pub depth: Option<StackDepth>,
}

// The return address and the saved `rbp` each take up a slot.
pub(crate) const FUNCTION_START_DEPTH: StackDepth = StackDepth(2);

impl<I> CallingConvention<I> {
    pub fn function_start(arguments: I) -> Self {
        CallingConvention {
            depth: Some(FUNCTION_START_DEPTH),
            arguments: Locs {
                locs: arguments,
                max_depth: FUNCTION_START_DEPTH,
            },
        }
    }
}

impl<T: Copy + 'static, I: Deref> CallingConvention<I>
where
    for<'a> &'a I::Target: IntoIterator<Item = &'a T>,
{
    pub fn as_ref(
        &self,
    ) -> CallingConvention<iter::Copied<<&'_ I::Target as IntoIterator>::IntoIter>> {
        CallingConvention {
            // We start and return the function with stack depth 1 since we must
            // allow space for the saved return address.
            depth: self.depth.clone(),
            arguments: self.arguments.as_ref(),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum FunctionDefLocation {
    SameModule,
    PossiblyExternal,
}

// TODO: Combine this with `ValueLocation`?
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CCLoc {
    /// Value exists in a register.
    Reg(GPR),
    /// Value exists on the stack.
    Stack(i32),
}

impl TryFrom<ValueLocation> for CCLoc {
    type Error = ();

    fn try_from(other: ValueLocation) -> Result<CCLoc, ()> {
        match other {
            ValueLocation::Reg(reg) => Ok(CCLoc::Reg(reg)),
            ValueLocation::Stack(offset) => Ok(CCLoc::Stack(offset)),
            ValueLocation::Cond(_) | ValueLocation::Immediate(_) => Err(()),
        }
    }
}

impl CCLoc {
    pub fn stack(self) -> Option<i32> {
        match self {
            CCLoc::Stack(o) => Some(o),
            _ => None,
        }
    }

    pub fn reg(self) -> Option<GPR> {
        match self {
            CCLoc::Reg(r) => Some(r),
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

    pub const EQUAL: CondCode = CondCode::ZF1;
    pub const NOT_EQUAL: CondCode = CondCode::ZF0;
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
    pub fn stack(self) -> Option<i32> {
        match self {
            ValueLocation::Stack(o) => Some(o),
            _ => None,
        }
    }

    pub fn reg(self) -> Option<GPR> {
        match self {
            ValueLocation::Reg(r) => Some(r),
            _ => None,
        }
    }

    pub fn immediate(self) -> Option<Value> {
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
// Usually system-v uses rdi and rsi, but rdi is used for the vmctx and rsi is used for the _caller_ vmctx
const INTEGER_ARGS_IN_GPRS: &[GPR] = &[GPR::Rq(CALLER_VMCTX), RDX, RCX, R8, R9];
const INTEGER_RETURN_GPRS: &[GPR] = &[RAX, RDX];
const FLOAT_ARGS_IN_GPRS: &[GPR] = &[XMM0, XMM1, XMM2, XMM3, XMM4, XMM5, XMM6, XMM7];
const FLOAT_RETURN_GPRS: &[GPR] = &[XMM0, XMM1];
// List of scratch registers taken from https://wiki.osdev.org/System_V_ABI
const SCRATCH_REGS: &[GPR] = &[
    RSI, RDX, RCX, R8, R9, RAX, R10, R11, XMM0, XMM1, XMM2, XMM3, XMM4, XMM5, XMM6, XMM7, XMM8,
    XMM9, XMM10, XMM11, XMM12, XMM13, XMM14, XMM15,
];
const VMCTX: RegId = rq::RDI;
const CALLER_VMCTX: RegId = rq::RSI;

pub struct CodeGenSession<'module, M> {
    assembler: Assembler,
    pub module_context: &'module M,
    pub op_offset_map: Vec<(AssemblyOffset, Box<dyn Display + Send + Sync>)>,
    func_starts: Vec<(Option<AssemblyOffset>, DynamicLabel)>,
    pointer_type: SignlessType,
}

impl<'module, M> CodeGenSession<'module, M> {
    pub fn new(func_count: u32, module_context: &'module M, pointer_type: SignlessType) -> Self {
        let mut assembler = Assembler::new().unwrap();
        let func_starts = iter::repeat_with(|| (None, assembler.new_dynamic_label()))
            .take(func_count as usize)
            .collect::<Vec<_>>();

        CodeGenSession {
            assembler,
            op_offset_map: Default::default(),
            func_starts,
            module_context,
            pointer_type,
        }
    }

    pub fn offset(&self) -> usize {
        self.assembler.offset().0
    }

    pub fn pointer_type(&self) -> SignlessType {
        self.pointer_type
    }

    pub fn new_context<'this>(
        &'this mut self,
        defined_func_idx: u32,
        sinks: Sinks<'this>,
    ) -> Context<'this, M> {
        {
            let func_start = &mut self.func_starts[defined_func_idx as usize];

            // At this point we know the exact start address of this function. Save it
            // and define dynamic label at this location.
            func_start.0 = Some(self.assembler.offset());
            self.assembler.dynamic_label(func_start.1);
        }

        Context {
            asm: &mut self.assembler,
            current_function: defined_func_idx,
            sinks,
            pointer_type: self.pointer_type,
            source_loc: Default::default(),
            func_starts: &self.func_starts,
            stack: Default::default(),
            physical_stack_depth: StackDepth(0),
            stack_depth_locked: false,
            regs: Default::default(),
            allocated_stack: Default::default(),
            module_context: self.module_context,
            labels: Default::default(),
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

#[derive(Debug, Default, Clone, PartialEq)]
pub struct StackUsage {
    // This is so we can track usage of stack args
    offset: u32,
    alloc: Alloc,
    // TODO: Make this `NonZeroU32`
    num_usages: HashMap<Ptr, u32>,
}

const ELEMENT_SIZE: Size = Size(WORD_SIZE as usize);

fn real_offset(num_arguments: u32, offset: i32) -> Result<u32, Error> {
    (-(offset + 1))
        .checked_add(num_arguments as i32)
        .and_then(|o| u32::try_from(o).ok())
        .map(|o| o * WORD_SIZE)
        .ok_or_else(|| {
            error(format!(
                "Tried to access stack value outside of bounds: {}, {}",
                num_arguments, offset
            ))
        })
}

impl StackUsage {
    pub fn new(num_arguments: u32) -> Self {
        StackUsage {
            offset: num_arguments,
            alloc: Alloc::new(Size((num_arguments * WORD_SIZE) as usize)),
            num_usages: Default::default(),
        }
    }

    pub fn clear(&mut self) {
        *self = Self::new(self.offset);
    }

    fn real_offset(&self, offset: i32) -> Result<u32, Error> {
        real_offset(self.offset, offset)
    }

    pub fn alloc(&mut self) -> Option<i32> {
        let ptr = self.alloc.malloc(ELEMENT_SIZE)?;

        let _prev_val = self.num_usages.insert(ptr, 1);
        // debug_assert_eq!(prev_val, None);

        Some(-((ptr.0 as u32 / WORD_SIZE) as i32 - self.offset as i32) - 1)
    }

    pub fn set_depth(&mut self, depth: StackDepth) -> Result<(), Error> {
        let alloc_size = (depth.0 + self.offset) * WORD_SIZE;
        let cur_depth = self.stack_depth();
        self.alloc.set_size(Size(alloc_size as usize));

        for i in cur_depth.0..depth.0 {
            let ptr = Ptr(self.real_offset(-(i as i32 + 1))? as usize);
            let _prev_val = self.num_usages.insert(ptr, 1);
            // debug_assert_eq!(prev_val, None);
        }

        // for i in depth.0..cur_depth.0 {
        //     let ptr = Ptr(self.real_offset(-(i as i32 + 1))? as usize);
        //     let _prev_val = self.num_usages.remove(&ptr);
        //     // debug_assert_eq!(prev_val, None);
        // }

        Ok(())
    }

    pub fn set_depth_and_free(&mut self, depth: StackDepth) -> Result<(), Error> {
        let ptr = Ptr(self.alloc.size().0);
        let cur_depth = self.stack_depth();

        let alloc_size = (depth.0 + self.offset) * WORD_SIZE;
        self.alloc.set_size(Size(alloc_size as usize));

        if let Some(size) = depth
            .0
            .checked_sub(cur_depth.0)
            .map(|diff| diff * WORD_SIZE)
        {
            self.alloc.free(ptr, Size(size as usize));
        }

        // for i in depth.0..cur_depth.0 {
        //     let ptr = Ptr(self.real_offset(-(i as i32 + 1))? as usize);
        //     let _prev_val = self.num_usages.remove(&ptr);
        //     // debug_assert_eq!(prev_val, None);
        // }

        Ok(())
    }

    pub fn mark_used(&mut self, offset: i32) -> Result<(), Error> {
        let ptr = Ptr(self.real_offset(offset)? as usize);
        if self.alloc.is_free(ptr, ELEMENT_SIZE) {
            self.alloc.mark_allocated(ptr, ELEMENT_SIZE);
            let _prev_val = self.num_usages.insert(ptr, 1);
        // debug_assert_eq!(prev_val, None);
        } else {
            *self.num_usages.entry(ptr).or_insert(0) += 1;
        }

        Ok(())
    }

    pub fn release(&mut self, offset: i32) -> Result<(), Error> {
        let ptr = Ptr(self.real_offset(offset)? as usize);

        if self.alloc.is_free(ptr, ELEMENT_SIZE) {
            return Err(error(format!(
                "Tried to release stack element that was already free at {}\nState: {:#?}",
                offset, self,
            )));
        }

        let num_usages = self
            .num_usages
            .remove(&ptr)
            .ok_or_else(|| {
                error(format!(
                    "Tried to release stack element that was already free at {}",
                    offset
                ))
            })?
            .checked_sub(1)
            .ok_or_else(|| {
                error(format!(
                    "Tried to release stack element that was already free at {}",
                    offset
                ))
            })?;

        if num_usages == 0 {
            self.alloc.free(ptr, ELEMENT_SIZE);
        } else {
            self.num_usages.insert(ptr, num_usages);
        }

        Ok(())
    }

    pub fn num_usages(&self, offset: i32) -> Result<u32, Error> {
        let ptr = Ptr(self.real_offset(offset)? as usize);

        Ok(*self
            .num_usages
            .get(&ptr)
            .ok_or_else(|| error("Tried to get number of usages of element that doesn't exist"))?)
    }

    pub fn is_free(&self, offset: i32) -> Result<bool, Error> {
        let ptr = Ptr(self.real_offset(offset)? as usize);

        Ok(self.alloc.is_free(ptr, ELEMENT_SIZE))
    }

    pub fn stack_depth(&self) -> StackDepth {
        StackDepth((self.alloc.size().0 as u32 / WORD_SIZE) - self.offset)
    }

    pub fn used_stack_depth(&self) -> StackDepth {
        StackDepth((self.alloc.used_size().0 as u32 / WORD_SIZE) - self.offset)
    }
}

type Stack = Vec<ValueLocation>;

mod labels {
    use super::Label;
    use std::collections::HashMap;

    pub struct UndefinedLabel {
        pub label: Label,
        pub align: u32,
    }

    #[derive(Copy, Clone, PartialEq, Eq, Hash)]
    pub enum LabelValue {
        Ret,
        I32(i32),
        I64(i64),
    }

    #[derive(Default)]
    pub struct Labels {
        map: HashMap<LabelValue, Result<UndefinedLabel, Label>>,
    }

    impl Labels {
        pub fn drain(&mut self) -> impl Iterator<Item = (LabelValue, UndefinedLabel)> + '_ {
            self.map.iter_mut().filter_map(|(val, info)| {
                let label = match info {
                    Ok(info) => info.label,
                    Err(label) => *label,
                };

                std::mem::replace(info, Err(label))
                    .ok()
                    .map(|info| (*val, info))
            })
        }

        pub fn insert(
            &mut self,
            l: impl FnOnce() -> Label,
            align: u32,
            label: LabelValue,
        ) -> Label {
            let val = self
                .map
                .entry(label)
                .or_insert_with(move || Ok(UndefinedLabel { label: l(), align }));

            match val {
                Ok(info) => {
                    info.align = info.align.max(align);

                    info.label
                }
                Err(label) => *label,
            }
        }

        pub fn define(&mut self, label: &LabelValue) -> Option<Result<UndefinedLabel, Label>> {
            self.map.get_mut(label).map(|info| {
                let label = match info {
                    Ok(info) => info.label,
                    Err(label) => *label,
                };

                std::mem::replace(info, Err(label))
            })
        }
    }
}

use labels::{LabelValue, Labels, UndefinedLabel};

pub struct Context<'this, M> {
    pub asm: &'this mut Assembler,
    pointer_type: SignlessType,
    source_loc: SourceLoc,
    pub sinks: Sinks<'this>,
    module_context: &'this M,
    current_function: u32,
    func_starts: &'this Vec<(Option<AssemblyOffset>, DynamicLabel)>,
    pub stack: Stack,
    pub physical_stack_depth: StackDepth,
    stack_depth_locked: bool,
    pub regs: Registers,
    // TODO: Replace with `alloc`
    pub allocated_stack: StackUsage,
    labels: Labels,
}

/// Label in code.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Label(DynamicLabel);

/// Offset from starting value of SP counted in words.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct StackDepth(u32);

macro_rules! int_div {
    ($full_div_s:ident, $full_div_u:ident, $div_u:ident, $div_s:ident, $rem_u:ident, $rem_s:ident, $imm_fn:ident, $signed_ty:ty, $unsigned_ty:ty, $reg_ty:tt, $pointer_ty:tt) => {
        // TODO: Fast div using mul for constant divisor? It looks like LLVM doesn't do that for us when
        //       emitting Wasm.
        pub fn $div_u(&mut self) -> Result<(), Error>{
            let divisor = self.pop()?;
            let dividend = self.pop()?;

            if let (Some(dividend), Some(divisor)) = (dividend.$imm_fn(), divisor.$imm_fn()) {
                if divisor == 0 {
                    self.trap(TrapCode::IntegerDivisionByZero);
                    self.push(ValueLocation::Immediate((0 as $unsigned_ty).into()))?;
                } else {
                    self.push(ValueLocation::Immediate(
                        <$unsigned_ty>::wrapping_div(dividend as _, divisor as _).into(),
                    ))?;
                }

                return Ok(())
            }

            let (mut div, rem, saved) = self.$full_div_u(divisor, dividend)?;

            self.free(rem)?;

            match div {
                ValueLocation::Reg(div_reg)
                    if saved.clone().any(|(_, dst)| ValueLocation::from(dst) == div) =>
                {
                    let new = self.take_or_free_reg(I32).ok_or_else(|| error("Ran out of free registers"))?;
                    dynasm!(self.asm
                        ; mov Rq(new.rq().unwrap()), Rq(div_reg.rq().unwrap())
                    );
                    self.free(ValueLocation::Reg(div_reg))?;
                    div = ValueLocation::Reg(new);
                },
                ValueLocation::Reg(_) |
                ValueLocation::Stack(_) |
                ValueLocation::Cond(_) |
                ValueLocation::Immediate(_) => {},
            }

            for (src, dst) in saved {
                self.copy_value(src, dst)?;
                self.free(src)?;
            }

            debug_assert!(self.stack.iter().all(|v| {
                if let ValueLocation::Stack(o) = v {
                    debug_assert_ge!(self.adjusted_offset(*o), 0);
                }

                true
            }));

            self.push(div)?;
            Ok(())
        }

        // TODO: Fast div using mul for constant divisor? It looks like LLVM doesn't do that for us when
        //       emitting Wasm.
        pub fn $div_s(&mut self) -> Result<(), Error>{
            let divisor = self.pop()?;
            let dividend = self.pop()?;

            if let (Some(dividend), Some(divisor)) = (dividend.$imm_fn(), divisor.$imm_fn()) {
                if divisor == 0 {
                    self.trap(TrapCode::IntegerDivisionByZero);
                    self.push(ValueLocation::Immediate((0 as $signed_ty).into()))?;
                } else {
                    self.push(ValueLocation::Immediate(
                        <$signed_ty>::wrapping_div(dividend, divisor).into(),
                    ))?;
                }

                return Ok(())
            }

            let (mut div, rem, saved) = self.$full_div_s(divisor, dividend)?;

            self.free(rem)?;

            match div {
                ValueLocation::Reg(div_reg)
                    if saved.clone().any(|(_, dst)| ValueLocation::from(dst) == div) =>
                {
                    let new = self.take_or_free_reg(I32).ok_or_else(|| error("Ran out of free registers"))?;
                    dynasm!(self.asm
                        ; mov Rq(new.rq().unwrap()), Rq(div_reg.rq().unwrap())
                    );
                    self.free(div)?;
                    div = ValueLocation::Reg(new);
                },
                ValueLocation::Reg(_) |
                ValueLocation::Stack(_) |
                ValueLocation::Cond(_) |
                ValueLocation::Immediate(_) => {},
            }

            for (src, dst) in saved {
                self.copy_value(src, dst)?;
                self.free(src)?;
            }

            self.push(div)?;
            Ok(())
        }

        pub fn $rem_u(&mut self) -> Result<(), Error>{
            let divisor = self.pop()?;
            let dividend = self.pop()?;

            if let (Some(dividend), Some(divisor)) = (dividend.$imm_fn(), divisor.$imm_fn()) {
                if divisor == 0 {
                    self.trap(TrapCode::IntegerDivisionByZero);
                    self.push(ValueLocation::Immediate((0 as $unsigned_ty).into()))?;
                } else {
                    self.push(ValueLocation::Immediate(
                        (dividend as $unsigned_ty % divisor as $unsigned_ty).into(),
                    ))?;
                }
                return Ok(());
            }

            let (div, mut rem, saved) = self.$full_div_u(divisor, dividend)?;

            self.free(div)?;

            match rem {
                ValueLocation::Reg(rem_reg)
                    if saved.clone().any(|(_, dst)| ValueLocation::from(dst) == rem) =>
                {
                    let new = self.take_or_free_reg(I32).ok_or_else(|| error("Ran out of free registers"))?;
                    dynasm!(self.asm
                        ; mov Rq(new.rq().unwrap()), Rq(rem_reg.rq().unwrap())
                    );
                    self.free(rem)?;
                    rem = ValueLocation::Reg(new);
                },
                ValueLocation::Reg(_) |
                ValueLocation::Stack(_) |
                ValueLocation::Cond(_) |
                ValueLocation::Immediate(_) => {},
            }

            for (src, dst) in saved {
                self.copy_value(src, dst)?;
                self.free(src)?;
            }

            self.push(rem)?;
            Ok(())
        }

        pub fn $rem_s(&mut self) -> Result<(), Error>{
            let mut divisor = self.pop()?;
            let dividend = self.pop()?;

            if let (Some(dividend), Some(divisor)) = (dividend.$imm_fn(), divisor.$imm_fn()) {
                if divisor == 0 {
                    self.trap(TrapCode::IntegerDivisionByZero);
                    self.push(ValueLocation::Immediate((0 as $signed_ty).into()))?;
                } else {
                    self.push(ValueLocation::Immediate((dividend % divisor).into()))?;
                }
                return Ok(());
            }

            let is_neg1 = self.create_label();

            let current_depth = self.physical_stack_depth.clone();

            // TODO: This could cause segfaults because of implicit push/pop
            let gen_neg1_case = match divisor {
                ValueLocation::Immediate(_) => {
                    if divisor.$imm_fn().unwrap() == -1 {
                        self.push(ValueLocation::Immediate((-1 as $signed_ty).into()))?;
                        self.free(dividend)?;
                        return Ok(());
                    }

                    false
                }
                ValueLocation::Reg(_) => {
                    let reg = self.put_into_register(GPRType::Rq, &mut divisor)?.ok_or_else(|| error("Ran out of free registers".to_string()))?;

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

            let (div, mut rem, saved) = self.$full_div_s(divisor, dividend)?;

            self.free(div)?;

            match rem {
                ValueLocation::Reg(rem_reg)
                    if saved.clone().any(|(_, dst)| ValueLocation::from(dst) == rem) =>
                {
                    let new = self.take_or_free_reg(I32).ok_or_else(|| error("Ran out of free registers"))?;
                    dynasm!(self.asm
                        ; mov Rq(new.rq().unwrap()), Rq(rem_reg.rq().unwrap())
                    );
                    self.free(rem)?;
                    rem = ValueLocation::Reg(new);
                },
                ValueLocation::Reg(_) |
                ValueLocation::Stack(_) |
                ValueLocation::Cond(_) |
                ValueLocation::Immediate(_) => {},
            }

            for (src, dst) in saved {
                self.copy_value(src, dst)?;
                self.free(src)?;
            }

            if gen_neg1_case {
                let ret = self.create_label();
                self.set_stack_depth(current_depth.clone())?;
                dynasm!(self.asm
                    ; jmp =>ret.0
                );
                self.define_label(is_neg1);

                let dst_ccloc = match CCLoc::try_from(rem).ok() {
                    None => {
                        return Err(error(
                            "$rem_s Programmer error".to_string()
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

            self.push(rem)?;
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
                    let temp = self.take_or_free_reg(Type::for_::<$typ>()).unwrap();
                    dynasm!(self.asm
                        ; $instr $reg_ty(temp.rq().unwrap()), [rsp + offset]
                    );
                    ValueLocation::Reg(temp)
                }
                ValueLocation::Reg(_) | ValueLocation::Cond(_) => {
                    let reg = self.put_into_register(GPRType::Rq, &mut val)?.ok_or_else(|| error("Ran out of free registers".to_string()))?;
                    let temp = self.take_or_free_reg(Type::for_::<$typ>()).unwrap();
                    dynasm!(self.asm
                        ; $instr $reg_ty(temp.rq().unwrap()), $reg_ty(reg.rq().unwrap())
                    );
                    ValueLocation::Reg(temp)
                }
            };

            self.free(val)?;
            self.push(out_val)?;
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
                    let temp = self.take_or_free_reg(Type::for_::<$out_typ>()).unwrap();
                    dynasm!(self.asm
                        ; $instr $out_reg_ty(temp.$out_reg_fn().unwrap()), [rsp + offset]
                    );

                    ValueLocation::Reg(temp)
                }
                ValueLocation::Reg(_) | ValueLocation::Cond(_) => {
                    let reg = self.put_into_register(Type::for_::<$in_typ>(), &mut val)? .ok_or_else(|| error("Ran out of free registers".to_string()))?;
                    let temp = self.take_or_free_reg(Type::for_::<$out_typ>()).unwrap();

                    dynasm!(self.asm
                        ; $instr $out_reg_ty(temp.$out_reg_fn().unwrap()), $in_reg_ty(reg.$in_reg_fn().unwrap())
                    );

                    ValueLocation::Reg(temp)
                }
            };

            self.free(val)?;

            self.push(out_val)?;
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
                        let reg = self.put_into_temp_register($ty, &mut val)?.ok_or_else(|| error("Ran out of free registers".to_string()))?;

                        dynasm!(self.asm
                            ; $instr $reg_ty(reg.rq().unwrap()), imm
                        );
                        self.push(ValueLocation::Reg(reg))?;
                        return Ok(());
                    }
                }
            }

            if val == ValueLocation::Reg(RCX) {
                let new = self.take_or_free_reg($ty).ok_or_else(|| error("Ran out of free registers"))?;
                self.copy_value(val, CCLoc::Reg(new))?;
                self.free(val)?;
                val = ValueLocation::Reg(new);
            }

            // TODO: Maybe allocate `RCX`, write `count` to it and then free `count`.
            //       Once we've implemented refcounting this will do the right thing
            //       for free.
            let temp_rcx = match count {
                ValueLocation::Reg(RCX) => {None}
                other => {
                    let out = if self.is_free(CCLoc::Reg(RCX))? {
                        None
                    } else {
                        let new_reg = self.take_or_free_reg(I32).ok_or_else(|| error("Ran out of free registers"))?;
                        dynasm!(self.asm
                            ; mov Rq(new_reg.rq().unwrap()), rcx
                        );
                        Some(new_reg)
                    };

                    self.copy_value(other, CCLoc::Reg(RCX))?;

                    out
                }
            };

            self.free(count)?;
            self.mark_used(CCLoc::Reg(RCX))?;
            count = ValueLocation::Reg(RCX);

            let reg = self.put_into_temp_register($ty, &mut val)?.ok_or_else(|| error("Ran out of free registers".to_string()))?;

            dynasm!(self.asm
                ; $instr $reg_ty(reg.rq().unwrap()), cl
            );

            self.free(count)?;

            if let Some(gpr) = temp_rcx {
                dynasm!(self.asm
                    ; mov rcx, Rq(gpr.rq().unwrap())
                );
                self.free(ValueLocation::Reg(gpr))?;
            }

            self.push(val)?;
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
                        let rreg = self.put_into_register(I32, &mut right)?.ok_or_else(|| error("Ran out of free registers".to_string()))?;
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
                let lreg = self.put_into_register(I32, &mut left)?.ok_or_else(|| error("Ran out of free registers".to_string()))?;

                match right {
                    ValueLocation::Stack(offset) => {
                        let offset = self.adjusted_offset(offset);
                        dynasm!(self.asm
                            ; cmp Rd(lreg.rq().unwrap()), [rsp + offset]
                        );
                    }
                    ValueLocation::Reg(_) | ValueLocation::Cond(_) => {
                        let rreg = self.put_into_register(I32, &mut right)?.ok_or_else(|| error("Ran out of free registers".to_string()))?;

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

            self.free(left)?;
            self.free(right)?;

            self.push(out)?;
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
                            let lreg = self.put_into_register(I32, &mut left)?.ok_or_else(|| error("Ran out of free registers".to_string()))?;

                            dynasm!(self.asm
                                ; cmp QWORD [rsp + offset], Rq(lreg.rq().unwrap())
                            );
                        }
                        ValueLocation::Cond($reverse_flags)
                    }
                    ValueLocation::Reg(_) | ValueLocation::Cond(_) => {
                        let rreg = self.put_into_register(I32, &mut right)?.ok_or_else(|| error("Ran out of free registers".to_string()))?;

                        if let Some(i) = i.try_into().ok() {
                            dynasm!(self.asm
                                ; cmp Rq(rreg.rq().unwrap()), i
                            );
                        } else {
                            let lreg = self.put_into_register(I32, &mut left)?.ok_or_else(|| error("Ran out of free registers".to_string()))?;

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
                let lreg = self.put_into_register(I64, &mut left)?.ok_or_else(|| error("Ran out of free registers".to_string()))?;

                match right {
                    ValueLocation::Stack(offset) => {
                        let offset = self.adjusted_offset(offset);
                        dynasm!(self.asm
                            ; cmp Rq(lreg.rq().unwrap()), [rsp + offset]
                        );
                    }
                    ValueLocation::Reg(_) | ValueLocation::Cond(_) => {
                        let rreg = self.put_into_register(I32, &mut right)?.ok_or_else(|| error("Ran out of free registers".to_string()))?;

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
                            let rreg = self.put_into_register(I32, &mut right)?.ok_or_else(|| error("Ran out of free registers".to_string()))?;

                            dynasm!(self.asm
                                ; cmp Rq(lreg.rq().unwrap()), Rq(rreg.rq().unwrap())
                            );
                        }
                    }
                }

                ValueLocation::Cond($flags)
            };

            self.free(left)?;
            self.free(right)?;
            self.push(out)?;
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
                    ))?;
                    return Ok(());
                }
            }

            let (mut left, mut right) = match left {
                ValueLocation::Reg(r) if self.num_usages(CCLoc::Reg(r))? <= 1 => (left, right),
                _ =>  (right, left)
            };

            let lreg = self.put_into_temp_register(GPRType::Rx, &mut left)?.ok_or_else(|| error("Ran out of free registers".to_string()))?;
            let rreg = self.put_into_register(GPRType::Rx, &mut right)?.ok_or_else(|| error("Ran out of free registers".to_string()))?;

            let out = self.take_or_free_reg(I32).ok_or_else(|| error("Ran out of free registers"))?;

            dynasm!(self.asm
                ; $instr Rx(lreg.rx().unwrap()), Rx(rreg.rx().unwrap())
                ; movd Rd(out.rq().unwrap()), Rx(lreg.rx().unwrap())
                ; and Rd(out.rq().unwrap()), 1
            );

            self.push(ValueLocation::Reg(out))?;
            self.free(left)?;
            self.free(right)?;
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
                    ))?;
                    return Ok(());
                }
            }

            let (mut left, mut right) = match left {
                ValueLocation::Reg(r) if self.num_usages(CCLoc::Reg(r))? <= 1 => (left, right),
                _ =>  (right, left)
            };

            let lreg = self.put_into_temp_register(GPRType::Rx, &mut left)?.ok_or_else(|| error("Ran out of free registers".to_string()))?;
            let rreg = self.put_into_register(GPRType::Rx, &mut right)?.ok_or_else(|| error("Ran out of free registers".to_string()))?;

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

            self.push(left)?;
            self.free(right)?;
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
            let lreg = this.put_into_register(GPRType::Rx, left)?.ok_or_else(|| error("Ran out of free registers".to_string()))?;

            let result = this.take_or_free_reg(I32).ok_or_else(|| error("Ran out of free registers"))?;

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
                    let rreg = this.put_into_register(GPRType::Rx, right)?.ok_or_else(|| error("Ran out of free registers".to_string()))?;

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

            self.free(left)?;
            self.free(right)?;

            self.push(out)?;
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

            self.free(left)?;
            self.free(right)?;

            self.push(out)?;
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
                    self.push(ValueLocation::Immediate($const_fallback(i1, i0).into()))?;
                    return Ok(());
                }
            }

            let (mut left, mut right) = $map_op(left, right);
            let lreg = self.put_into_temp_register($ty, &mut left)?.ok_or_else(|| error("Ran out of free registers".to_string()))?;

            match right {
                ValueLocation::Reg(_) | ValueLocation::Cond(_) => {
                    // This handles the case where we (for example) have a float in an `Rq` reg
                    let right_reg = self.put_into_register($ty, &mut right)?.ok_or_else(|| error("Ran out of free registers".to_string()))?;

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
                        let scratch = self.take_or_free_reg($ty).ok_or_else(|| error("Ran out of free registers"))?;
                        self.immediate_to_reg(scratch, i)?;

                        dynasm!(self.asm
                            ; $instr $reg_ty(lreg.$reg_fn().unwrap()), $reg_ty(scratch.$reg_fn().unwrap())
                        );

                        self.free(ValueLocation::Reg(scratch))?;
                    }
                }
            }

            self.free(right)?;
            self.push(left)?;
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
                let (reg, mem_offset) = reg_offset.ok_or(()).or_else::<Error, _>(|_| {
                    let reg = ctx
                        .take_or_free_reg(I64)
                        .ok_or_else(|| error("Ran out of free registers"))?;

                    dynasm!(ctx.asm
                        ; mov Rq(reg.rq().unwrap()), [
                            Rq(VMCTX) + ctx.module_context.vmctx_vmmemory_import_from(mem_index) as i32
                        ]
                    );

                    Ok((Some(reg), 0))
                })?;

                let vmctx = GPR::Rq(VMCTX);

                if ctx.module_context.emit_memory_bounds_check() {
                    let addr_reg = match runtime_offset {
                        Ok(imm) => {
                            let addr_reg = ctx.take_or_free_reg(I64).ok_or_else(|| error("Ran out of free registers"))?;
                            dynasm!(ctx.asm
                                ; mov Rq(addr_reg.rq().unwrap()), QWORD imm as i64 + offset as i64
                            );
                            addr_reg
                        }
                        Err(gpr) => {
                            if offset == 0 {
                                ctx.clone_to_register(I32, ValueLocation::Reg(gpr))?.ok_or_else(|| error("Ran out of free registers".to_string()))?
                            } else if offset > 0 {
                                let addr_reg = ctx.take_or_free_reg(I64).ok_or_else(|| error("Ran out of free registers"))?;
                                dynasm!(ctx.asm
                                    ; lea Rq(addr_reg.rq().unwrap()), [Rq(gpr.rq().unwrap()) + offset]
                                );
                                addr_reg
                            } else {
                                let addr_reg = ctx.take_or_free_reg(I64).ok_or_else(|| error("Ran out of free registers"))?;
                                let offset_reg = ctx.take_or_free_reg(I64).ok_or_else(|| error("Ran out of free registers"))?;
                                dynasm!(ctx.asm
                                    ; mov Rd(offset_reg.rq().unwrap()), offset
                                    ; mov Rq(addr_reg.rq().unwrap()), Rq(gpr.rq().unwrap())
                                    ; add Rq(addr_reg.rq().unwrap()), Rq(offset_reg.rq().unwrap())
                                );
                                ctx.free(ValueLocation::Reg(offset_reg))?;
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
                        ;; ctx.trap_if(cc::GE_U, TrapCode::HeapOutOfBounds)
                    );
                    ctx.free(ValueLocation::Reg(addr_reg))?;
                }

                let mem_ptr_reg = ctx.take_or_free_reg(I64).ok_or_else(|| error("Ran out of free registers"))?;
                dynasm!(ctx.asm
                    ; mov Rq(mem_ptr_reg.rq().unwrap()), [
                        Rq(reg.unwrap_or(vmctx).rq().unwrap()) +
                            mem_offset +
                            ctx.module_context.vmmemory_definition_base() as i32
                    ]
                );
                if let Some(reg) = reg {
                    ctx.free(ValueLocation::Reg(reg))?;
                }
                $emit_fn(ctx, dst, mem_ptr_reg, runtime_offset, offset)?;
                ctx.free(ValueLocation::Reg(mem_ptr_reg))?;
                Ok(())
            }

            let base = self.pop()?;

            let temp = self.take_or_free_reg($rtype)
                .ok_or_else(|| error("Ran out of free registers"))?;

            match base {
                ValueLocation::Immediate(i) => {
                    load_to_reg(self, temp, (offset as _, Ok(i.as_i32().unwrap())))?;
                }
                mut base => {
                    let gpr = self.put_into_register(I32, &mut base)?.ok_or_else(|| error("Ran out of free registers".to_string()))?;
                    load_to_reg(self, temp, (offset as _, Err(gpr)))?;
                    self.free(base)?;
                }
            }

            self.push(ValueLocation::Reg(temp))?;
            Ok(())
        }
    };
    ($name:ident, $rtype:expr, $reg_ty:tt, NONE, $rq_instr:ident, $ty:ident) => {
        load!(@inner
            $name,
            $rtype,
            $reg_ty,
            |ctx: &mut Context<_>, dst: GPR, mem_ptr_reg: GPR, runtime_offset: Result<i32, GPR>, offset: i32| -> Result<(), Error>  {
                match runtime_offset {
                    Ok(imm) => {
                        dynasm!(ctx.asm
                            ; $rq_instr $reg_ty(dst.rq().unwrap()), $ty [Rq(mem_ptr_reg.rq().unwrap()) + offset + imm]
                        );
                        Ok(())
                    }
                    Err(offset_reg) => {
                        dynasm!(ctx.asm
                            ; $rq_instr $reg_ty(dst.rq().unwrap()), $ty [Rq(mem_ptr_reg.rq().unwrap()) + Rq(offset_reg.rq().unwrap()) + offset]
                        );
                        Ok(())
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
            |ctx: &mut Context<_>, dst: GPR, mem_ptr_reg: GPR, runtime_offset: Result<i32, GPR>, offset: i32| -> Result<(), Error>  {
                match (dst, runtime_offset) {
                    (GPR::Rq(r), Ok(imm)) => {
                        dynasm!(ctx.asm
                            ; $rq_instr $reg_ty(r), $ty [Rq(mem_ptr_reg.rq().unwrap()) + offset + imm]
                        );
                        Ok(())
                    }
                    (GPR::Rx(r), Ok(imm)) => {
                        if let Some(combined) = offset.checked_add(imm) {
                            dynasm!(ctx.asm
                                ; $xmm_instr Rx(r), $ty [Rq(mem_ptr_reg.rq().unwrap()) + combined]
                            );
                            Ok(())
                        } else {
                            let offset_reg = ctx.take_or_free_reg(GPRType::Rq).ok_or_else(|| error("Ran out of free registers"))?;
                            dynasm!(ctx.asm
                                ; mov Rq(offset_reg.rq().unwrap()), offset
                                ; $xmm_instr Rx(r), $ty [
                                    Rq(mem_ptr_reg.rq().unwrap()) +
                                    Rq(offset_reg.rq().unwrap()) +
                                    imm
                                ]
                            );
                            ctx.free(ValueLocation::Reg(offset_reg))?;
                            Ok(())
                        }
                    }
                    (GPR::Rq(r), Err(offset_reg)) => {
                        dynasm!(ctx.asm
                            ; $rq_instr $reg_ty(r), $ty [Rq(mem_ptr_reg.rq().unwrap()) + Rq(offset_reg.rq().unwrap()) + offset]
                        );
                        Ok(())
                    }
                    (GPR::Rx(r), Err(offset_reg)) => {
                        dynasm!(ctx.asm
                            ; $xmm_instr Rx(r), $ty [Rq(mem_ptr_reg.rq().unwrap()) + Rq(offset_reg.rq().unwrap()) + offset]
                        );
                        Ok(())
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
                let (reg, mem_offset) = reg_offset.ok_or(()).or_else::<Error, _>(|_| {
                    let reg = ctx
                        .take_or_free_reg(I64)
                        .ok_or_else(|| error("Ran out of free registers"))?;

                    dynasm!(ctx.asm
                        ; mov Rq(reg.rq().unwrap()), [
                            Rq(VMCTX) + ctx.module_context.vmctx_vmmemory_import_from(mem_index) as i32
                        ]
                    );

                    Ok((Some(reg), 0))
                })?;

                let vmctx = GPR::Rq(VMCTX);

                if ctx.module_context.emit_memory_bounds_check() {
                    let addr_reg = match runtime_offset {
                        Ok(imm) => {
                            let addr_reg = ctx.take_or_free_reg(I64).ok_or_else(|| error("Ran out of free registers"))?;
                            dynasm!(ctx.asm
                                ; mov Rq(addr_reg.rq().unwrap()), QWORD imm as i64 + offset as i64
                            );
                            addr_reg
                        }
                        Err(gpr) => {
                            if offset == 0 {
                                ctx.clone_to_register(I32, ValueLocation::Reg(gpr))?.ok_or_else(|| error("Ran out of free registers".to_string()))?

                            } else if offset > 0 {
                                let addr_reg = ctx.take_or_free_reg(I64).ok_or_else(|| error("Ran out of free registers"))?;
                                dynasm!(ctx.asm
                                    ; lea Rq(addr_reg.rq().unwrap()), [Rq(gpr.rq().unwrap()) + offset]
                                );
                                addr_reg
                            } else {
                                let addr_reg = ctx.take_or_free_reg(I64).ok_or_else(|| error("Ran out of free registers"))?;
                                let offset_reg = ctx.take_or_free_reg(I64).ok_or_else(|| error("Ran out of free registers"))?;
                                dynasm!(ctx.asm
                                    ; mov Rd(offset_reg.rq().unwrap()), offset
                                    ; mov Rq(addr_reg.rq().unwrap()), Rq(gpr.rq().unwrap())
                                    ; add Rq(addr_reg.rq().unwrap()), Rq(offset_reg.rq().unwrap())
                                );
                                ctx.free(ValueLocation::Reg(offset_reg))?;
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
                        ;; ctx.trap_if(cc::GE_U, TrapCode::HeapOutOfBounds)
                    );
                    ctx.free(ValueLocation::Reg(addr_reg))?;
                }

                let mem_ptr_reg = ctx.take_or_free_reg(I64).ok_or_else(|| error("Ran out of free registers"))?;
                dynasm!(ctx.asm
                    ; mov Rq(mem_ptr_reg.rq().unwrap()), [
                        Rq(reg.unwrap_or(vmctx).rq().unwrap()) +
                            mem_offset +
                            ctx.module_context.vmmemory_definition_base() as i32
                    ]
                );
                if let Some(reg) = reg {
                    ctx.free(ValueLocation::Reg(reg))?;
                }
                let src = $match_offset(ctx, mem_ptr_reg, runtime_offset, offset, src)?;
                ctx.free(ValueLocation::Reg(mem_ptr_reg))?;
                ctx.free(ValueLocation::Reg(src))?;
                Ok(())
            }

            if !(offset <= i32::max_value() as u32) {
                return Err(error(format!("store: offset value too big {}", offset)))
            }

            let mut src = self.pop()?;
            let base = self.pop()?;

            // `store_from_reg` frees `src`
            // TODO: Would it be better to free it outside `store_from_reg`?
            let src_reg = self.put_into_register(None, &mut src)?.ok_or_else(|| error("Ran out of free registers".to_string()))?;


            match base {
                ValueLocation::Immediate(i) => {
                    store_from_reg(self, src_reg, (offset as i32, Ok(i.as_i32().unwrap())))?
                }
                mut base => {
                    let gpr = self.put_into_register(I32, &mut base)?.ok_or_else(|| error("Ran out of free registers".to_string()))?;
                    store_from_reg(self, src_reg, (offset as i32, Err(gpr)))?;
                    self.free(base)?;
                }
            }
            Ok(())
        }
    };
    ($name:ident, $int_reg_ty:tt, NONE, $size:ident) => {
        store!(@inner
            $name,
            $int_reg_ty,
            |ctx: &mut Context<_>, mem_ptr_reg: GPR, runtime_offset: Result<i32, GPR>, offset: i32, src| -> Result<GPR, Error> {
                let src_reg = ctx.put_into_temp_register(GPRType::Rq, &mut ValueLocation::Reg(src))?.ok_or_else(|| error("Ran out of free registers".to_string()))?;

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

                Ok(src_reg)
            },
            $size
        );
    };
    ($name:ident, $int_reg_ty:tt, $xmm_instr:ident, $size:ident) => {
        store!(@inner
            $name,
            $int_reg_ty,
            |ctx: &mut Context<_>, mem_ptr_reg: GPR, runtime_offset: Result<i32, GPR>, offset: i32, src| -> Result<GPR, Error> {
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

                Ok(src)
            },
            $size
        );
    };
}

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum BrAction {
    Jump,
    Continue,
}

pub struct Target {
    pub target: BrTarget<Label>,
    pub action: BrAction,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MaybeCCLoc {
    Concrete(CCLoc),
    Serialize,
    NoSerialize,
}

impl<'this, M: ModuleContext> Context<'this, M> {
    fn free_reg(&mut self, type_: GPRType) -> Result<bool, Error> {
        let pos = if let Some(pos) = self
            .stack
            .iter()
            .position(|r| r.reg().map(|reg| reg.type_() == type_).unwrap_or(false))
        {
            pos
        } else {
            return Ok(false);
        };

        let old_loc = self.stack[pos];
        let new_loc = self.push_physical(old_loc)?.into();
        self.stack[pos] = new_loc;

        let mut stack = mem::take(&mut self.stack);

        for elem in &mut stack[pos + 1..] {
            if *elem == old_loc {
                *elem = new_loc;
                self.mark_used(new_loc)?;
                self.free(old_loc)?;
            }
        }

        mem::replace(&mut self.stack, stack);

        Ok(true)
    }

    fn take_reg(&mut self, r: impl Into<GPRType>) -> Option<GPR> {
        self.regs.take(r.into())
    }

    fn take_or_free_reg(&mut self, r: impl Into<GPRType>) -> Option<GPR> {
        let r = r.into();
        loop {
            if let Some(gpr) = self.take_reg(r) {
                break Some(gpr);
            }

            if self.free_reg(r) != Ok(true) {
                break None;
            }
        }
    }

    pub fn set_source_loc(&mut self, loc: SourceLoc) {
        self.source_loc = loc;
    }

    pub fn virtual_calling_convention(&self) -> CallingConvention {
        CallingConvention {
            arguments: Locs {
                locs: self.stack.clone(),
                // max_depth: self.allocated_stack.used_stack_depth(),
                max_depth: self.physical_stack_depth.clone(),
            },
            // depth: Some(self.allocated_stack.used_stack_depth()),
            depth: Some(self.physical_stack_depth.clone()),
        }
    }

    /// Create a new undefined label.
    pub fn create_label(&mut self) -> Label {
        Label(self.asm.new_dynamic_label())
    }

    fn adjusted_offset(&self, offset: i32) -> i32 {
        let out = (self.physical_stack_depth.0 as i32 + offset) * WORD_SIZE as i32;
        debug_assert_ge!(
            out,
            0,
            "Trying to access stack value at offset less than allocated space"
        );
        debug_assert_ne!(
            out,
            ((self.physical_stack_depth.0 - 1) * WORD_SIZE) as i32,
            "Trying to access saved return address",
        );
        debug_assert_ne!(
            out,
            ((self.physical_stack_depth.0 - 2) * WORD_SIZE) as i32,
            "Trying to access saved `rbp`",
        );
        debug_assert_lt!(
            out,
            ((self.physical_stack_depth.0
                + self
                    .module_context
                    .defined_func_type(self.current_function)
                    .params()
                    .len() as u32)
                * WORD_SIZE) as i32,
            "Trying to access stack value at offset greater than allocated space"
        );
        out
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
            ))?;
            return Ok(());
        }

        if let ValueLocation::Cond(loc) = val {
            self.push(ValueLocation::Cond(!loc))?;
            return Ok(());
        }

        let reg = self
            .put_into_register(I32, &mut val)?
            .ok_or_else(|| error("Ran out of free registers".to_string()))?;

        dynasm!(self.asm
            ; test Rd(reg.rq().unwrap()), Rd(reg.rq().unwrap())
        );

        self.free(val)?;

        self.push(ValueLocation::Cond(cc::EQUAL))?;
        Ok(())
    }

    pub fn i64_eqz(&mut self) -> Result<(), Error> {
        let mut val = self.pop()?;

        if let ValueLocation::Immediate(Value::I64(i)) = val {
            self.push(ValueLocation::Immediate(
                (if i == 0 { 1i32 } else { 0 }).into(),
            ))?;
            return Ok(());
        }

        if let ValueLocation::Cond(loc) = val {
            self.push(ValueLocation::Cond(!loc))?;
            return Ok(());
        }

        let reg = self
            .put_into_register(I64, &mut val)?
            .ok_or_else(|| error("Ran out of free registers".to_string()))?;

        dynasm!(self.asm
            ; test Rq(reg.rq().unwrap()), Rq(reg.rq().unwrap())
        );

        self.free(val)?;

        self.push(ValueLocation::Cond(cc::EQUAL))?;
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

    pub fn select_stack_depth(
        &mut self,
        cond: CondCode,
        stack_depth_if_true: StackDepth,
        stack_depth_if_false: StackDepth,
    ) -> Result<(), Error> {
        if stack_depth_if_true == stack_depth_if_false {
            self.set_stack_depth(stack_depth_if_true)?;
        } else if let Some(reg) = self.take_reg(GPRType::Rq) {
            self.set_stack_depth(stack_depth_if_false)?;
            let diff = self.physical_stack_depth.0 as i32 - stack_depth_if_true.0 as i32;
            dynasm!(self.asm
                ; lea Rq(reg.rq().unwrap()), [rsp + diff * WORD_SIZE as i32]
            );
            let reg = CCLoc::Reg(reg);
            self.cmov(cond, RSP, reg);
            self.free(reg)?;
        } else {
            self.set_stack_depth(stack_depth_if_true)?;
            let ret = self.create_label();
            self.br_on_cond_code(ret, cond);
            self.set_stack_depth(stack_depth_if_false)?;
            self.define_label(ret);
        }

        Ok(())
    }

    fn br_if(
        &mut self,
        val: ValueLocation,
        if_true: Target,
        if_false: Target,
        depth: Option<StackDepth>,
    ) -> Result<(), Error> {
        let cond = match val {
            ValueLocation::Immediate(imm) => {
                let target = if imm.as_bytes() == 0 {
                    if_false
                } else {
                    if_true
                };

                if target.action == BrAction::Jump {
                    self.br(target.target);
                }

                return Ok(());
            }
            ValueLocation::Cond(cc) => cc,
            ValueLocation::Reg(GPR::Rq(rq)) => {
                dynasm!(self.asm
                    ; test Rd(rq), Rd(rq)
                );

                CondCode::ZF0
            }
            ValueLocation::Reg(GPR::Rx(rx)) => {
                dynasm!(self.asm
                    ; ptest Rx(rx), Rx(rx)
                );

                CondCode::ZF0
            }
            ValueLocation::Stack(o) => {
                let offset = self.adjusted_offset(o);

                dynasm!(self.asm
                    ; cmp DWORD [rsp + offset], 0
                );

                cc::NOT_EQUAL
            }
        };

        if let Some(depth) = depth {
            self.set_stack_depth(depth)?;
        }

        match (if_true.action, if_false.action) {
            (BrAction::Jump, BrAction::Jump) => {
                let if_true = self.target_to_label(if_true.target);
                self.br_on_cond_code(if_true, cond);
                self.br(if_false.target);
            }
            (BrAction::Continue, BrAction::Jump) => {
                let if_false = self.target_to_label(if_false.target);
                self.br_on_cond_code(if_false, !cond);
            }
            (BrAction::Jump, BrAction::Continue) => {
                let if_true = self.target_to_label(if_true.target);
                self.br_on_cond_code(if_true, cond);
            }
            (BrAction::Continue, BrAction::Continue) => {}
        }

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
    pub fn end_block<I>(
        &mut self,
        targets: I,
        default: Target,
        depth: Option<StackDepth>,
        mut selector: ValueLocation,
    ) -> Result<(), Error>
    where
        I: IntoIterator<Item = Target>,
        I::IntoIter: ExactSizeIterator + DoubleEndedIterator,
    {
        let mut targets = targets.into_iter();
        let count = targets.len();

        match (count, selector) {
            (0, _) => {
                debug_assert!(targets.next().is_none(), "Invalid `ExactSizeIterator` impl");

                if let Some(depth) = depth {
                    self.set_stack_depth(depth)?;
                }

                if default.action == BrAction::Jump {
                    self.br(default.target);
                }
            }
            (_, ValueLocation::Immediate(imm)) => {
                let target = usize::try_from(imm.as_bytes())
                    .ok()
                    .and_then(|i| targets.nth(i))
                    .unwrap_or(default);

                if let Some(depth) = depth {
                    self.set_stack_depth(depth)?;
                }

                if target.action == BrAction::Jump {
                    self.br(target.target);
                }
            }
            (1, _) | (_, ValueLocation::Cond(_)) => {
                // We know that `count` is > 0 because the `0, _` case is before this one.
                let if_false = targets.next().expect("Invalid `ExactSizeIterator` impl");
                let if_true = targets.next().unwrap_or(default);

                self.br_if(selector, if_true, if_false, depth)?;
            }
            (_, ValueLocation::Reg(GPR::Rq(rq))) => {
                if let Some(depth) = depth {
                    self.set_stack_depth(depth)?;
                }

                for (i, target) in targets.enumerate() {
                    let i: u32 = i
                        .try_into()
                        .map_err(|_| error("Number of `br_table` targets overflowed `u32`"))?;
                    let label = self.target_to_label(target.target);
                    dynasm!(self.asm
                        ; cmp Rd(rq), i as i32
                        ; je =>label.0
                    );
                }

                if default.action == BrAction::Jump {
                    self.br(default.target);
                }
            }
            // Arbitrary choice of maximum of 16 targets before switching to jump table
            (count, ValueLocation::Stack(_)) | (count, ValueLocation::Reg(GPR::Rx(_)))
                if count < 16 =>
            {
                if let Some(reg) = self.take_reg(GPRType::Rq) {
                    self.copy_value(selector, CCLoc::Reg(reg))?;

                    if let Some(depth) = depth {
                        self.set_stack_depth(depth)?;
                    }

                    for (i, target) in targets.enumerate() {
                        let i: u32 = i
                            .try_into()
                            .map_err(|_| error("Number of `br_table` targets overflowed `u32`"))?;
                        let label = self.target_to_label(target.target);
                        dynasm!(self.asm
                            ; cmp Rd(reg.rq().unwrap()), i as i32
                            ; je =>label.0
                        );
                    }

                    if default.action == BrAction::Jump {
                        self.br(default.target);
                    }

                    self.free(ValueLocation::Reg(reg))?;
                } else {
                    if let Some(depth) = depth {
                        self.set_stack_depth(depth)?;
                    }

                    let o = if let ValueLocation::Stack(o) = selector {
                        o
                    } else {
                        unimplemented!()
                    };

                    let offset = self.adjusted_offset(o);

                    for (i, target) in targets.enumerate() {
                        let i: u32 = i
                            .try_into()
                            .map_err(|_| error("Number of `br_table` targets overflowed `u32`"))?;
                        let label = self.target_to_label(target.target);
                        dynasm!(self.asm
                            ; cmp DWORD [rsp + offset], i as i32
                            ; je =>label.0
                        );
                    }

                    if default.action == BrAction::Jump {
                        self.br(default.target);
                    }
                }
            }
            (count, _) => {
                let (selector_reg, saved_selector) =
                    match self.put_into_temp_register(GPRType::Rq, &mut selector)? {
                        Some(r) => (r, None),
                        None => (
                            RAX,
                            Some(ValueLocation::from(
                                self.push_copy_physical(ValueLocation::Reg(RAX))?,
                            )),
                        ),
                    };

                let (tmp, saved_tmp) = if let Some(reg) = self.take_or_free_reg(I64) {
                    (reg, None)
                } else {
                    let out_reg = if selector_reg == RAX { RCX } else { RAX };

                    (
                        out_reg,
                        Some(ValueLocation::from(
                            self.push_copy_physical(ValueLocation::Reg(out_reg))?,
                        )),
                    )
                };

                self.immediate_to_reg(tmp, (count as u32).into())?;
                dynasm!(self.asm
                    ; cmp Rq(selector_reg.rq().unwrap()), Rq(tmp.rq().unwrap())
                    ; cmova Rq(selector_reg.rq().unwrap()), Rq(tmp.rq().unwrap())
                    ; lea Rq(tmp.rq().unwrap()), [>start_label]
                    ; lea Rq(selector_reg.rq().unwrap()), [
                        Rq(selector_reg.rq().unwrap()) * 5
                    ]
                    ; add Rq(selector_reg.rq().unwrap()), Rq(tmp.rq().unwrap())
                );

                if let Some(saved) = saved_tmp {
                    self.copy_value(saved, CCLoc::Reg(tmp))?;
                } else {
                    self.free(ValueLocation::Reg(tmp))?;
                }

                if let Some(saved) = saved_selector {
                    self.copy_value(saved, CCLoc::Reg(selector_reg))?;
                }

                if let Some(depth) = depth {
                    self.set_stack_depth(depth)?;
                }

                dynasm!(self.asm
                    ; jmp Rq(selector_reg.rq().unwrap())
                ; start_label:
                );

                for target in targets {
                    let label = self.target_to_label(target.target);
                    dynasm!(self.asm
                        ; jmp =>label.0
                    );
                }

                if default.action == BrAction::Jump {
                    self.br(default.target);
                }
            }
        }

        self.free(selector)?;

        Ok(())
    }

    #[cfg(debug_assertions)]
    fn check_block_depth_inbounds(&mut self, depth: StackDepth) -> Result<(), Error> {
        let old_depth = mem::replace(&mut self.physical_stack_depth, depth);

        let block_depth_is_inbounds = self.stack.iter().all(|v| {
            if let ValueLocation::Stack(o) = v {
                self.adjusted_offset(*o) >= 0
            } else {
                true
            }
        });

        let out = if block_depth_is_inbounds {
            Ok(())
        } else {
            Err(error(format!(
                "Some elements on the stack were out of range of the new \
                        stack depth: {:#?}",
                self.stack
            )))
        };

        self.physical_stack_depth = old_depth;

        out
    }

    #[cfg(not(debug_assertions))]
    fn check_block_depth_inbounds(&self, _: StackDepth) -> Result<(), Error> {
        Ok(())
    }

    fn set_stack_depth(&mut self, depth: StackDepth) -> Result<(), Error> {
        if self.physical_stack_depth != depth {
            if depth == FUNCTION_START_DEPTH {
                dynasm!(self.asm
                    ; mov rsp, rbp
                );
            } else {
                let diff = self.physical_stack_depth.0 as i32 - depth.0 as i32;

                dynasm!(self.asm
                    ; lea rsp, [rsp + diff * WORD_SIZE as i32]
                );
            }
        }

        self.allocated_stack.set_depth_and_free(depth.clone())?;
        self.physical_stack_depth = depth;

        self.check_block_depth_inbounds(self.physical_stack_depth.clone())
    }

    fn do_pass_block_args<I>(
        &mut self,
        params: I,
        max_stack: Option<StackDepth>,
    ) -> Result<Locs<Stack>, Error>
    where
        I: ExactSizeIterator<Item = MaybeCCLoc> + DoubleEndedIterator,
    {
        #[derive(Debug, Copy, Clone)]
        enum Dest {
            Concrete(CCLoc),
            Serialize(ValueLocation),
            NoSerialize(ValueLocation),
        }

        let mut pending = Vec::<(ValueLocation, CCLoc)>::with_capacity(params.len());
        let mut out = Vec::<Dest>::with_capacity(params.len());

        let mut max_dst_or_src_depth = max_stack
            .clone()
            .unwrap_or(FUNCTION_START_DEPTH)
            .0
            .max(self.allocated_stack.used_stack_depth().0);

        for loc in params.rev() {
            match loc {
                MaybeCCLoc::Concrete(loc) => {
                    if let Some(depth) = loc.stack().and_then(|o| (-o).try_into().ok()) {
                        max_dst_or_src_depth = max_dst_or_src_depth.max(depth);
                    }

                    if self.is_free(loc)? {
                        self.mark_used(loc)?;
                        self.pop_into(loc)?;
                    } else {
                        let src = self.pop()?;
                        if let Some(depth) = src.stack().and_then(|o| (-o).try_into().ok()) {
                            max_dst_or_src_depth = max_dst_or_src_depth.max(depth);
                        }

                        if src != ValueLocation::from(loc) {
                            pending.push((src, loc));
                        }
                    }

                    out.push(Dest::Concrete(loc));
                }
                MaybeCCLoc::Serialize => {
                    out.push(Dest::Serialize(self.pop()?));
                }
                MaybeCCLoc::NoSerialize => {
                    out.push(Dest::NoSerialize(self.pop()?));
                }
            }
        }

        self.set_stack_depth(StackDepth(max_dst_or_src_depth))?;

        while let Some(val) = self.try_pop() {
            self.free(val)?;
        }

        for loc in out.iter_mut().rev() {
            match loc {
                Dest::Serialize(vloc) | Dest::NoSerialize(vloc) => {
                    if CCLoc::try_from(*vloc).is_ok() {
                        self.put_into_temp_location(None, vloc)?;
                    }

                    self.push(*vloc)?;
                }
                _ => {}
            }
        }

        let mut last_len = None;

        while !pending.is_empty() {
            let start_len = pending.len();

            for (src, dst) in mem::take(&mut pending) {
                if src != ValueLocation::from(dst) {
                    if self.is_free(dst)? {
                        self.mark_used(dst)?;
                        self.copy_value(src, dst)?;
                        self.free(src)?;
                    } else {
                        pending.push((src, dst));
                        continue;
                    }
                }
            }

            if last_len == Some(pending.len()) {
                return Err(error(
                    "BUG: Making no progress allocating locations for block calling convention",
                ));
            }

            if pending.len() == start_len {
                let mut stack = mem::take(&mut self.stack);
                let mut stack_borrow = &mut stack[..];

                while let Some((cur, rest)) = stack_borrow.split_first_mut() {
                    if pending
                        .iter()
                        .any(|(_, dst)| ValueLocation::from(*dst) == *cur)
                    {
                        let old = *cur;

                        self.mark_used(old)?;
                        let new = self.put_into_temp_location(None, cur)?.into();
                        self.free(old)?;

                        for cur in &mut *rest {
                            if *cur == old {
                                self.free(old)?;
                                *cur = new;
                                self.mark_used(new)?;
                            }
                        }

                        for (src, _) in &mut pending {
                            if *src == old {
                                self.free(old)?;
                                *src = new;
                                self.mark_used(new)?;
                            }
                        }
                    }

                    stack_borrow = rest;
                }

                let mut pending_borrow = &mut pending[..];

                while let Some((cur, rest)) = pending_borrow.split_first_mut() {
                    let (src, _) = cur;

                    if let Some(loc) = CCLoc::try_from(*src).ok() {
                        if rest.iter().any(|(_, dst)| *dst == loc) {
                            let old = *src;

                            self.mark_used(old)?;
                            let new = self.put_into_temp_location(None, src)?.into();
                            self.free(old)?;

                            for (src, _) in &mut *rest {
                                if *src == old {
                                    self.free(old)?;
                                    *src = new;
                                    self.mark_used(new)?;
                                }
                            }
                        }
                    }

                    pending_borrow = rest;
                }

                self.stack = stack;
            }

            last_len = Some(start_len);
        }

        if let Some(depth) = max_stack.clone() {
            self.set_stack_depth(StackDepth(self.physical_stack_depth.0.max(depth.0)))?;
            self.allocated_stack.set_depth_and_free(depth)?;
            self.stack_depth_locked = true;
        }

        let mut out = out
            .into_iter()
            .map(|loc| {
                Ok(match loc {
                    Dest::Concrete(ccloc) => ccloc.into(),
                    Dest::Serialize(_) => {
                        let mut loc = self.pop()?;
                        self.put_into_temp_location(None, &mut loc)?;

                        if let Some(max_stack) = &max_stack {
                            match loc {
                                ValueLocation::Stack(offset) if offset < -(max_stack.0 as i32) => {
                                    if self.put_into_register(None, &mut loc)?.is_none() {
                                        loc = self.push_physical(loc)?.into();
                                    }
                                }
                                _ => {}
                            }
                        }

                        loc
                    }
                    Dest::NoSerialize(_) => {
                        let mut loc = self.pop()?;

                        if let Some(max_stack) = &max_stack {
                            match loc {
                                ValueLocation::Stack(offset) if offset < -(max_stack.0 as i32) => {
                                    if self.put_into_register(None, &mut loc)?.is_none() {
                                        let old_loc = loc;
                                        loc = self.push_copy_physical(loc)?.into();
                                        self.free(old_loc)?;
                                    }
                                }
                                _ => {}
                            }
                        }

                        loc
                    }
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;

        self.stack_depth_locked = false;

        out.reverse();

        let max_stack_depth = out
            .iter()
            .filter_map(|l| l.stack().map(|offset| -offset))
            .max()
            .and_then(|max| u32::try_from(max).ok())
            .map(|max| max.max(FUNCTION_START_DEPTH.0))
            .map(StackDepth)
            .unwrap_or(FUNCTION_START_DEPTH);

        Ok(Locs {
            locs: out,
            max_depth: max_stack_depth,
        })
    }

    pub fn serialize_block_args<I>(
        &mut self,
        params: I,
        depth: Option<StackDepth>,
    ) -> Result<CallingConvention, Error>
    where
        I: IntoIterator<Item = MaybeCCLoc>,
        I::IntoIter: ExactSizeIterator + DoubleEndedIterator,
    {
        let locs = self.do_pass_block_args(params.into_iter(), depth.clone())?;

        #[cfg(debug_assertions)]
        if let Some(depth) = &depth {
            debug_assert_le!(locs.max_depth.0, depth.0);
        }

        Ok(CallingConvention {
            arguments: locs,
            depth,
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
            .ok_or(())
            .or_else::<Error, _>(|_| {
                let reg = self
                    .take_or_free_reg(I64)
                    .ok_or_else(|| error("Ran out of free registers"))?;

                dynasm!(self.asm
                    ; mov Rq(reg.rq().unwrap()), [
                        Rq(VMCTX) +
                            self.module_context.vmctx_vmglobal_import_from(global_idx) as i32
                    ]
                );

                Ok((Some(reg), 0))
            })?;

        let out = self
            .take_or_free_reg(GPRType::Rq)
            .ok_or_else(|| error("Ran out of free registers"))?;
        let vmctx = GPR::Rq(VMCTX);

        // TODO: Are globals necessarily aligned to 128 bits? We can load directly to an XMM reg if so
        dynasm!(self.asm
            ; mov Rq(out.rq().unwrap()), [Rq(reg.unwrap_or(vmctx).rq().unwrap()) + offset as i32]
        );

        if let Some(reg) = reg {
            self.free(ValueLocation::Reg(reg))?;
        }

        self.push(ValueLocation::Reg(out))?;
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
            .ok_or(())
            .or_else::<Error, _>(|_| {
                let reg = self
                    .take_or_free_reg(I64)
                    .ok_or_else(|| error("Ran out of free registers"))?;

                dynasm!(self.asm
                    ; mov Rq(reg.rq().unwrap()), [
                        Rq(VMCTX) +
                            self.module_context.vmctx_vmglobal_import_from(global_idx) as i32
                    ]
                );

                Ok((Some(reg), 0))
            })?;

        let val_reg = self
            .put_into_register(GPRType::Rq, &mut val)?
            .ok_or_else(|| error("Ran out of free registers".to_string()))?;
        let vmctx = GPR::Rq(VMCTX);

        // We always use `Rq` (even for floats) since the globals are not necessarily aligned to 128 bits
        dynasm!(self.asm
            ; mov [
                Rq(reg.unwrap_or(vmctx).rq().unwrap()) + offset as i32
            ], Rq(val_reg.rq().unwrap())
        );

        if let Some(reg) = reg {
            self.free(ValueLocation::Reg(reg))?;
        }

        self.free(val)?;
        Ok(())
    }

    fn immediate_to_reg(&mut self, reg: GPR, val: Value) -> Result<(), Error> {
        fn immediate_to_rq(asm: &mut Assembler, reg: RegId, val: Value) {
            let val = val.as_bytes();
            if (val as u64) <= u32::max_value() as u64 {
                dynasm!(asm
                    ; mov Rd(reg), val as i32
                );
            } else {
                dynasm!(asm
                    ; mov Rq(reg), QWORD val
                );
            }
        }

        match reg {
            GPR::Rq(rq) => immediate_to_rq(&mut self.asm, rq, val),
            GPR::Rx(rx) => {
                if let Some(tmp) = self.take_or_free_reg(GPRType::Rq) {
                    dynasm!(self.asm
                        ;; immediate_to_rq(&mut self.asm, tmp.rq().unwrap(), val)
                        ; movq Rx(rx), Rq(tmp.rq().unwrap())
                    );
                    self.free(ValueLocation::Reg(tmp))?;
                } else {
                    dynasm!(self.asm
                        ; push rax
                        ;; immediate_to_rq(&mut self.asm, rq::RAX, val)
                        ; movq Rx(rx), rax
                        ; pop rax
                    );
                };
            }
        }

        Ok(())
    }

    // The `&` and `&mut` aren't necessary (`ValueLocation` is copy) but it ensures that we don't get
    // the arguments the wrong way around. In the future we want to have a `ReadLocation` and `WriteLocation`
    // so we statically can't write to a literal so this will become a non-issue.
    fn copy_value(&mut self, src: ValueLocation, dst: CCLoc) -> Result<(), Error> {
        match (src, dst) {
            (ValueLocation::Cond(cond), CCLoc::Stack(o)) => {
                let offset = self.adjusted_offset(o);

                self.copy_value(ValueLocation::Immediate(0u64.into()), dst)?;

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
                    self.copy_value(ValueLocation::Immediate(0u64.into()), dst)?;

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
                    let temp = CCLoc::Reg(
                        self.take_or_free_reg(I32)
                            .ok_or_else(|| error("Ran out of free registers"))?,
                    );
                    self.copy_value(src, temp)?;
                    let temp = temp.into();
                    self.copy_value(temp, dst)?;
                    self.free(temp)?;
                }
            },
            (ValueLocation::Stack(in_offset), CCLoc::Stack(out_offset)) => {
                if in_offset != out_offset {
                    if let Some(gpr) = self.take_or_free_reg(I64) {
                        let in_offset = self.adjusted_offset(in_offset);
                        let out_offset = self.adjusted_offset(out_offset);

                        dynasm!(self.asm
                            ; mov Rq(gpr.rq().unwrap()), [rsp + in_offset]
                            ; mov [rsp + out_offset], Rq(gpr.rq().unwrap())
                        );
                        self.free(ValueLocation::Reg(gpr))?;
                    } else {
                        dynasm!(self.asm
                            ; push rax
                            ;; self.reserve_stack_space(1)?
                        );

                        let in_offset = self.adjusted_offset(in_offset);
                        let out_offset = self.adjusted_offset(out_offset);
                        dynasm!(self.asm
                            ; mov rax, [rsp + in_offset]
                            ; mov [rsp + out_offset], rax
                        );

                        dynasm!(self.asm
                            ; pop rax
                            ;; self.free_stack_space(1)?
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
                if let Some(scratch) = self.take_or_free_reg(I64) {
                    let out_offset = self.adjusted_offset(out_offset);

                    dynasm!(self.asm
                        ; mov Rq(scratch.rq().unwrap()), QWORD i
                        ; mov [rsp + out_offset], Rq(scratch.rq().unwrap())
                    );

                    self.free(ValueLocation::Reg(scratch))?;
                } else {
                    dynasm!(self.asm
                        ; push rax
                        ;; self.reserve_stack_space(1)?
                    );

                    let out_offset = self.adjusted_offset(out_offset);

                    dynasm!(self.asm
                        ; mov rax, QWORD i
                        ; mov [rsp + out_offset], rax
                    );

                    dynasm!(self.asm
                        ; pop rax
                        ;; self.free_stack_space(1)?
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
                self.immediate_to_reg(out_reg, i)?;
            }
        }
        Ok(())
    }

    /// Define the given label at the current position.
    ///
    /// Multiple labels can be defined at the same position. However, a label
    /// can be defined only once.
    pub fn define_label(&mut self, label: Label) {
        // TODO: Use `lea rsp, [rbp + depth]`-style construct to ensure that the stack depth is
        //       always correct here.
        self.asm.dynamic_label(label.0);
    }

    pub fn set_state<I>(&mut self, state: CallingConvention<I>) -> Result<(), Error>
    where
        I: IntoIterator,
        Stack: FromIterator<I::Item>,
    {
        self.regs = Registers::new();
        self.regs.release_scratch_register()?;
        let stack: Stack = state.arguments.locs.into_iter().collect();

        self.allocated_stack.clear();
        self.allocated_stack.set_depth(FUNCTION_START_DEPTH)?;

        let depth = match state.depth {
            Some(depth) => depth,
            None => {
                let required_depth = state.arguments.max_depth.0.max(FUNCTION_START_DEPTH.0);
                let diff = required_depth - FUNCTION_START_DEPTH.0;

                dynasm!(self.asm
                    ; lea rsp, [rbp - ((diff * WORD_SIZE) as i32)]
                );

                StackDepth(required_depth)
            }
        };

        self.physical_stack_depth = depth.clone();
        self.allocated_stack.set_depth_and_free(depth)?;

        for elem in &stack {
            if let Ok(ccloc) = CCLoc::try_from(*elem) {
                self.mark_used(ccloc)?;
            }
        }

        self.stack = stack;

        Ok(())
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

    fn reserve_stack_space(&mut self, amount: u32) -> Result<(), Error> {
        self.physical_stack_depth.0 += amount;
        self.allocated_stack
            .set_depth(self.physical_stack_depth.clone())?;

        Ok(())
    }

    fn free_stack_space(&mut self, amount: u32) -> Result<(), Error> {
        self.physical_stack_depth.0 -= amount;
        self.allocated_stack
            .set_depth(self.physical_stack_depth.clone())?;

        Ok(())
    }

    fn push_copy_physical(&mut self, value: ValueLocation) -> Result<CCLoc, Error> {
        if let Some(offset) = self.allocated_stack.alloc() {
            let out = CCLoc::Stack(offset);

            self.copy_value(value, out)?;

            Ok(out)
        } else if !self.stack_depth_locked {
            let out_offset = -(self.physical_stack_depth.0 as i32 + 1);
            match value {
                ValueLocation::Reg(GPR::Rq(gpr)) => {
                    dynasm!(self.asm
                        ; push Rq(gpr)
                    );
                    self.reserve_stack_space(1)?;
                }
                ValueLocation::Stack(o) => {
                    // If `rsp` is used in `push` it uses its value _before_ `push`.
                    let offset = self.adjusted_offset(o);
                    dynasm!(self.asm
                        ; push QWORD [rsp + offset]
                    );
                    self.reserve_stack_space(1)?;
                }
                _ => {
                    if let Some(gpr) = self.take_reg(GPRType::Rq) {
                        self.copy_value(value, CCLoc::Reg(gpr))?;
                        dynasm!(self.asm
                            ; push Rq(gpr.rq().unwrap())
                        );
                        self.reserve_stack_space(1)?;

                        self.free(ValueLocation::Reg(gpr))?;
                    } else {
                        dynasm!(self.asm
                            ; push rax
                        );
                        self.reserve_stack_space(1)?;

                        self.copy_value(value, CCLoc::Stack(out_offset))?;
                    }
                }
            }

            debug_assert_eq!(self.adjusted_offset(out_offset), 0);

            let out = CCLoc::Stack(out_offset);

            Ok(out)
        } else {
            Err(error("Cannot allocate stack space"))
        }
    }

    pub fn push_physical(&mut self, value: ValueLocation) -> Result<CCLoc, Error> {
        self.free(value)?;
        self.push_copy_physical(value)
    }

    fn push(&mut self, value: ValueLocation) -> Result<(), Error> {
        if let ValueLocation::Stack(o) = value {
            debug_assert_ge!(self.adjusted_offset(o), 0);
        }

        if let Some(mut top) = self.stack.pop() {
            if let ValueLocation::Cond(_) = top {
                self.put_into_temp_location(I32, &mut top)?;
            }

            self.stack.push(top);
        }

        self.stack.push(value);
        Ok(())
    }

    pub fn pop(&mut self) -> Result<ValueLocation, Error> {
        match self.stack.pop() {
            Some(v) => Ok(v),
            None => Err(error("Stack is empty - pop impossible".to_string())),
        }
    }

    fn try_pop(&mut self) -> Option<ValueLocation> {
        self.stack.pop()
    }

    pub fn drop(&mut self, range: RangeInclusive<u32>) -> Result<(), Error> {
        let mut repush = Vec::with_capacity(*range.start() as _);

        for _ in 0..*range.start() {
            let v = self.pop()?;
            repush.push(v);
        }

        for _ in range {
            let val = self.pop()?;
            self.free(val)?;
        }

        for v in repush.into_iter().rev() {
            self.push(v)?;
        }
        Ok(())
    }

    fn pop_into(&mut self, dst: CCLoc) -> Result<(), Error> {
        let val = self.pop()?;
        self.copy_value(val, dst)?;
        self.free(val)?;
        Ok(())
    }

    fn num_usages(&self, val: CCLoc) -> Result<u32, Error> {
        match val {
            CCLoc::Reg(r) => Ok(self.regs.num_usages(r) as u32),
            CCLoc::Stack(o) => self.allocated_stack.num_usages(o),
        }
    }

    fn is_free(&self, val: CCLoc) -> Result<bool, Error> {
        match val {
            CCLoc::Reg(r) if SCRATCH_REGS.contains(&r) => Ok(self.regs.is_free(r)),
            CCLoc::Reg(_) => Ok(true),
            CCLoc::Stack(o) => self.allocated_stack.is_free(o),
        }
    }

    fn mark_used<C: TryInto<CCLoc>>(&mut self, val: C) -> Result<(), Error> {
        match val.try_into() {
            Ok(CCLoc::Reg(r)) if SCRATCH_REGS.contains(&r) => self.regs.mark_used(r),
            Ok(CCLoc::Stack(o)) => self.allocated_stack.mark_used(o)?,
            _ => {}
        }

        Ok(())
    }

    fn free<C: TryInto<CCLoc>>(&mut self, val: C) -> Result<(), Error> {
        match val.try_into() {
            Ok(CCLoc::Reg(r)) if SCRATCH_REGS.contains(&r) => self.regs.release(r)?,
            Ok(CCLoc::Stack(o)) => self.allocated_stack.release(o)?,
            _ => {}
        }

        Ok(())
    }

    /// Puts this value into a register so that it can be efficiently read
    fn put_into_register(
        &mut self,
        ty: impl Into<Option<GPRType>>,
        val: &mut ValueLocation,
    ) -> Result<Option<GPR>, Error> {
        if let Some(out) = self.clone_to_register(ty, *val)? {
            self.free(*val)?;
            *val = ValueLocation::Reg(out);
            Ok(Some(out))
        } else {
            Ok(None)
        }
    }

    /// Clones this value into a register so that it can be efficiently read
    fn clone_to_register(
        &mut self,
        ty: impl Into<Option<GPRType>>,
        val: ValueLocation,
    ) -> Result<Option<GPR>, Error> {
        let ty = ty.into();
        match val {
            ValueLocation::Reg(r) if ty.map(|t| t == r.type_()).unwrap_or(true) => {
                self.mark_used(CCLoc::Reg(r))?;
                Ok(Some(r))
            }
            val => match self.take_or_free_reg(ty.unwrap_or(GPRType::Rq)) {
                Some(scratch) => {
                    self.copy_value(val, CCLoc::Reg(scratch))?;
                    Ok(Some(scratch))
                }
                None => Ok(None),
            },
        }
    }

    /// Puts this value into a temporary register so that operations
    /// on that register don't write to a local.
    fn put_into_temp_register(
        &mut self,
        ty: impl Into<Option<GPRType>>,
        val: &mut ValueLocation,
    ) -> Result<Option<GPR>, Error> {
        let out = self.clone_to_temp_register(ty, *val)?;
        if let Some(o) = out {
            self.free(*val)?;
            *val = ValueLocation::Reg(o);
            Ok(Some(o))
        } else {
            Ok(None)
        }
    }

    fn put_into_temp_location(
        &mut self,
        ty: impl Into<Option<GPRType>>,
        val: &mut ValueLocation,
    ) -> Result<CCLoc, Error> {
        let out = if let Some(gpr) = self.put_into_temp_register(ty, val)? {
            CCLoc::Reg(gpr)
        } else if let Some(ccloc) = CCLoc::try_from(*val).ok() {
            if self.num_usages(ccloc)? <= 1 {
                ccloc
            } else {
                self.push_physical(*val)?
            }
        } else {
            self.push_physical(*val)?
        };

        *val = out.into();

        Ok(out)
    }

    /// Clones this value into a temporary register so that operations
    /// on that register don't write to a value that will be read later.
    fn clone_to_temp_register(
        &mut self,
        ty: impl Into<Option<GPRType>>,
        val: ValueLocation,
    ) -> Result<Option<GPR>, Error> {
        // If we have `None` as the type then it always matches (`.unwrap_or(true)`)
        match val {
            ValueLocation::Reg(r) => {
                let ty = ty.into();
                let type_matches = ty.map(|t| t == r.type_()).unwrap_or(true);

                if self.num_usages(CCLoc::Reg(r))? <= 1 && type_matches {
                    self.mark_used(CCLoc::Reg(r))?;
                    Ok(Some(r))
                } else if let Some(scratch) = self.take_or_free_reg(ty.unwrap_or(GPRType::Rq)) {
                    self.copy_value(val, CCLoc::Reg(scratch))?;
                    Ok(Some(scratch))
                } else {
                    Ok(None)
                }
            }
            val => self.clone_to_register(ty, val),
        }
    }

    pub fn f32_neg(&mut self) -> Result<(), Error> {
        let mut val = self.pop()?;

        let out = if let Some(i) = val.imm_f32() {
            ValueLocation::Immediate(
                Ieee32::from_bits((-f32::from_bits(i.to_bits())).to_bits()).into(),
            )
        } else {
            let reg = match self.put_into_temp_register(GPRType::Rx, &mut val) {
                Err(e) => return Err(e),
                Ok(o) => o.ok_or_else(|| error("Ran out of free registers".to_string()))?,
            };
            let const_label = self.aligned_label(16, LabelValue::I32(SIGN_MASK_F32 as i32));

            dynasm!(self.asm
                ; xorps Rx(reg.rx().unwrap()), [=>const_label.0]
            );

            val
        };

        self.push(out)?;
        Ok(())
    }

    pub fn f64_neg(&mut self) -> Result<(), Error> {
        let mut val = self.pop()?;

        let out = if let Some(i) = val.imm_f64() {
            ValueLocation::Immediate(
                Ieee64::from_bits((-f64::from_bits(i.to_bits())).to_bits()).into(),
            )
        } else {
            let reg = match self.put_into_temp_register(GPRType::Rx, &mut val) {
                Err(e) => return Err(e),
                Ok(o) => o.ok_or_else(|| error("Ran out of free registers".to_string()))?,
            };
            let const_label = self.aligned_label(16, LabelValue::I64(SIGN_MASK_F64 as i64));

            dynasm!(self.asm
                ; xorpd Rx(reg.rx().unwrap()), [=>const_label.0]
            );

            val
        };

        self.push(out)?;
        Ok(())
    }

    pub fn f32_abs(&mut self) -> Result<(), Error> {
        let mut val = self.pop()?;

        let out = if let Some(i) = val.imm_f32() {
            ValueLocation::Immediate(
                Ieee32::from_bits(f32::from_bits(i.to_bits()).abs().to_bits()).into(),
            )
        } else {
            let reg = match self.put_into_temp_register(GPRType::Rx, &mut val) {
                Err(e) => return Err(e),
                Ok(o) => o.ok_or_else(|| error("Ran out of free registers".to_string()))?,
            };
            let const_label = self.aligned_label(16, LabelValue::I32(REST_MASK_F32 as i32));

            dynasm!(self.asm
                ; andps Rx(reg.rx().unwrap()), [=>const_label.0]
            );

            val
        };

        self.push(out)?;
        Ok(())
    }

    pub fn f64_abs(&mut self) -> Result<(), Error> {
        let mut val = self.pop()?;

        let out = if let Some(i) = val.imm_f64() {
            ValueLocation::Immediate(
                Ieee64::from_bits(f64::from_bits(i.to_bits()).abs().to_bits()).into(),
            )
        } else {
            let reg = match self.put_into_temp_register(GPRType::Rx, &mut val) {
                Err(e) => return Err(e),
                Ok(o) => o.ok_or_else(|| error("Ran out of free registers".to_string()))?,
            };

            let const_label = self.aligned_label(16, LabelValue::I64(REST_MASK_F64 as i64));

            dynasm!(self.asm
                ; andps Rx(reg.rx().unwrap()), [=>const_label.0]
            );

            val
        };

        self.push(out)?;
        Ok(())
    }

    pub fn f32_sqrt(&mut self) -> Result<(), Error> {
        let mut val = self.pop()?;

        let out = if let Some(i) = val.imm_f32() {
            ValueLocation::Immediate(
                Ieee32::from_bits(f32::from_bits(i.to_bits()).sqrt().to_bits()).into(),
            )
        } else {
            let reg = match self.put_into_temp_register(GPRType::Rx, &mut val) {
                Err(e) => return Err(e),
                Ok(o) => o.ok_or_else(|| error("Ran out of free registers".to_string()))?,
            };

            dynasm!(self.asm
                ; sqrtss Rx(reg.rx().unwrap()), Rx(reg.rx().unwrap())
            );

            val
        };

        self.push(out)?;
        Ok(())
    }

    pub fn f64_sqrt(&mut self) -> Result<(), Error> {
        let mut val = self.pop()?;

        let out = if let Some(i) = val.imm_f64() {
            ValueLocation::Immediate(
                Ieee64::from_bits(f64::from_bits(i.to_bits()).sqrt().to_bits()).into(),
            )
        } else {
            let reg = match self.put_into_temp_register(GPRType::Rx, &mut val) {
                Err(e) => return Err(e),
                Ok(o) => o.ok_or_else(|| error("Ran out of free registers".to_string()))?,
            };

            dynasm!(self.asm
                ; sqrtsd Rx(reg.rx().unwrap()), Rx(reg.rx().unwrap())
            );

            ValueLocation::Reg(reg)
        };

        self.push(out)?;
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
            let lreg = match self.put_into_temp_register(GPRType::Rx, &mut left) {
                Err(e) => return Err(e),
                Ok(o) => o.ok_or_else(|| error("Ran out of free registers".to_string()))?,
            };
            let rreg = match self.put_into_register(GPRType::Rx, &mut right) {
                Err(e) => return Err(e),
                Ok(o) => o.ok_or_else(|| error("Ran out of free registers".to_string()))?,
            };

            let sign_mask = self.aligned_label(16, LabelValue::I32(SIGN_MASK_F32 as i32));
            let rest_mask = self.aligned_label(16, LabelValue::I32(REST_MASK_F32 as i32));

            dynasm!(self.asm
                ; andps Rx(rreg.rx().unwrap()), [=>sign_mask.0]
                ; andps Rx(lreg.rx().unwrap()), [=>rest_mask.0]
                ; orps  Rx(lreg.rx().unwrap()), Rx(rreg.rx().unwrap())
            );

            self.free(right)?;

            left
        };

        self.push(out)?;
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
            let lreg = match self.put_into_temp_register(GPRType::Rx, &mut left) {
                Err(e) => return Err(e),
                Ok(o) => o.ok_or_else(|| error("Ran out of free registers".to_string()))?,
            };
            let rreg = match self.put_into_register(GPRType::Rx, &mut right) {
                Err(e) => return Err(e),
                Ok(o) => o.ok_or_else(|| error("Ran out of free registers".to_string()))?,
            };

            let sign_mask = self.aligned_label(16, LabelValue::I64(SIGN_MASK_F64 as i64));
            let rest_mask = self.aligned_label(16, LabelValue::I64(REST_MASK_F64 as i64));

            dynasm!(self.asm
                ; andpd Rx(rreg.rx().unwrap()), [=>sign_mask.0]
                ; andpd Rx(lreg.rx().unwrap()), [=>rest_mask.0]
                ; orpd  Rx(lreg.rx().unwrap()), Rx(rreg.rx().unwrap())
            );

            self.free(right)?;

            left
        };

        self.push(out)?;
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
                let temp = self
                    .take_or_free_reg(I32)
                    .ok_or_else(|| error("Ran out of free registers"))?;

                if is_x86_feature_detected!("lzcnt") {
                    dynasm!(self.asm
                        ; lzcnt Rd(temp.rq().unwrap()), [rsp + offset]
                    );
                    ValueLocation::Reg(temp)
                } else {
                    let temp_2 = self
                        .take_or_free_reg(I32)
                        .ok_or_else(|| error("Ran out of free registers"))?;

                    dynasm!(self.asm
                        ; bsr Rd(temp.rq().unwrap()), [rsp + offset]
                        ; mov Rd(temp_2.rq().unwrap()), DWORD 0x3fu64 as _
                        ; cmove Rd(temp.rq().unwrap()), Rd(temp_2.rq().unwrap())
                        ; mov Rd(temp_2.rq().unwrap()), DWORD 0x1fu64 as _
                        ; xor Rd(temp.rq().unwrap()), Rd(temp_2.rq().unwrap())
                    );
                    self.free(ValueLocation::Reg(temp_2))?;
                    ValueLocation::Reg(temp)
                }
            }
            ValueLocation::Reg(_) | ValueLocation::Cond(_) => {
                let reg = match self.put_into_register(GPRType::Rq, &mut val) {
                    Err(e) => return Err(e),
                    Ok(o) => o.ok_or_else(|| error("Ran out of free registers".to_string()))?,
                };

                let temp = self
                    .take_or_free_reg(I32)
                    .ok_or_else(|| error("Ran out of free registers"))?;

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

        self.free(val)?;
        self.push(out_val)?;
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
                let temp = self
                    .take_or_free_reg(I64)
                    .ok_or_else(|| error("Ran out of free registers"))?;

                if is_x86_feature_detected!("lzcnt") {
                    dynasm!(self.asm
                        ; lzcnt Rq(temp.rq().unwrap()), [rsp + offset]
                    );
                    ValueLocation::Reg(temp)
                } else {
                    let temp_2 = self
                        .take_or_free_reg(I64)
                        .ok_or_else(|| error("Ran out of free registers"))?;

                    dynasm!(self.asm
                        ; bsr Rq(temp.rq().unwrap()), [rsp + offset]
                        ; mov Rq(temp_2.rq().unwrap()), QWORD 0x7fu64 as _
                        ; cmove Rq(temp.rq().unwrap()), Rq(temp_2.rq().unwrap())
                        ; mov Rq(temp_2.rq().unwrap()), QWORD 0x3fu64 as _
                        ; xor Rq(temp.rq().unwrap()), Rq(temp_2.rq().unwrap())
                    );
                    self.free(ValueLocation::Reg(temp_2))?;
                    ValueLocation::Reg(temp)
                }
            }
            ValueLocation::Reg(_) | ValueLocation::Cond(_) => {
                let reg = match self.put_into_register(GPRType::Rq, &mut val) {
                    Err(e) => return Err(e),
                    Ok(o) => o.ok_or_else(|| error("Ran out of free registers".to_string()))?,
                };
                let temp = self
                    .take_or_free_reg(I64)
                    .ok_or_else(|| error("Ran out of free registers"))?;

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

        self.free(val)?;
        self.push(out_val)?;
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
                let temp = self
                    .take_or_free_reg(I32)
                    .ok_or_else(|| error("Ran out of free registers"))?;

                if is_x86_feature_detected!("lzcnt") {
                    dynasm!(self.asm
                        ; tzcnt Rd(temp.rq().unwrap()), [rsp + offset]
                    );
                    ValueLocation::Reg(temp)
                } else {
                    let temp_zero_val = self
                        .take_or_free_reg(I32)
                        .ok_or_else(|| error("Ran out of free registers"))?;

                    dynasm!(self.asm
                        ; bsf Rd(temp.rq().unwrap()), [rsp + offset]
                        ; mov Rd(temp_zero_val.rq().unwrap()), DWORD 0x20u32 as _
                        ; cmove Rd(temp.rq().unwrap()), Rd(temp_zero_val.rq().unwrap())
                    );
                    self.free(ValueLocation::Reg(temp_zero_val))?;
                    ValueLocation::Reg(temp)
                }
            }
            ValueLocation::Reg(_) | ValueLocation::Cond(_) => {
                let reg = match self.put_into_register(GPRType::Rq, &mut val) {
                    Err(e) => return Err(e),
                    Ok(o) => o.ok_or_else(|| error("Ran out of free registers".to_string()))?,
                };
                let temp = self
                    .take_or_free_reg(I32)
                    .ok_or_else(|| error("Ran out of free registers"))?;

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

        self.free(val)?;
        self.push(out_val)?;
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
                let temp = self
                    .take_or_free_reg(I64)
                    .ok_or_else(|| error("Ran out of free registers"))?;

                if is_x86_feature_detected!("lzcnt") {
                    dynasm!(self.asm
                        ; tzcnt Rq(temp.rq().unwrap()), [rsp + offset]
                    );
                    ValueLocation::Reg(temp)
                } else {
                    let temp_zero_val = self
                        .take_or_free_reg(I64)
                        .ok_or_else(|| error("Ran out of free registers"))?;

                    dynasm!(self.asm
                        ; bsf Rq(temp.rq().unwrap()), [rsp + offset]
                        ; mov Rq(temp_zero_val.rq().unwrap()), QWORD 0x40u64 as _
                        ; cmove Rq(temp.rq().unwrap()), Rq(temp_zero_val.rq().unwrap())
                    );
                    self.free(ValueLocation::Reg(temp_zero_val))?;
                    ValueLocation::Reg(temp)
                }
            }
            ValueLocation::Reg(_) | ValueLocation::Cond(_) => {
                let reg = match self.put_into_register(GPRType::Rq, &mut val) {
                    Err(e) => return Err(e),
                    Ok(o) => o.ok_or_else(|| error("Ran out of free registers".to_string()))?,
                };
                let temp = self
                    .take_or_free_reg(I64)
                    .ok_or_else(|| error("Ran out of free registers"))?;

                dynasm!(self.asm
                    ; bsf Rq(temp.rq().unwrap()), Rq(reg.rq().unwrap())
                    ; mov Rq(reg.rq().unwrap()), QWORD 0x40u64 as _
                    ; cmove Rq(temp.rq().unwrap()), Rq(reg.rq().unwrap())
                );
                ValueLocation::Reg(temp)
            }
        };

        self.free(val)?;
        self.push(out_val)?;
        Ok(())
    }

    pub fn i32_extend_u(&mut self) -> Result<(), Error> {
        let val = self.pop()?;

        let out = if let ValueLocation::Immediate(imm) = val {
            ValueLocation::Immediate((imm.as_i32().unwrap() as u32 as u64).into())
        } else {
            let new_reg = self
                .take_or_free_reg(I64)
                .ok_or_else(|| error("Ran out of free registers"))?;

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
                ValueLocation::Immediate(_) => {
                    return Err(error("i32_extend_u unreachable code".to_string()))
                }
            }

            ValueLocation::Reg(new_reg)
        };

        self.free(val)?;

        self.push(out)?;
        Ok(())
    }

    pub fn i32_extend_s(&mut self) -> Result<(), Error> {
        let val = self.pop()?;

        self.free(val)?;
        let new_reg = self
            .take_or_free_reg(I64)
            .ok_or_else(|| error("Ran out of free registers"))?;

        let out = match val {
            ValueLocation::Reg(GPR::Rx(rxreg)) => {
                dynasm!(self.asm
                    ; movd Rd(new_reg.rq().unwrap()), Rx(rxreg)
                    ; movsxd Rq(new_reg.rq().unwrap()), Rd(new_reg.rq().unwrap())
                );

                ValueLocation::Reg(new_reg)
            }
            ValueLocation::Reg(GPR::Rq(rqreg)) => {
                dynasm!(self.asm
                    ; movsxd Rq(new_reg.rq().unwrap()), Rd(rqreg)
                );

                ValueLocation::Reg(new_reg)
            }
            ValueLocation::Stack(offset) => {
                let offset = self.adjusted_offset(offset);

                dynasm!(self.asm
                    ; movsxd Rq(new_reg.rq().unwrap()), DWORD [rsp + offset]
                );

                ValueLocation::Reg(new_reg)
            }
            // `CondCode` can only be 0 or 1, so sign-extension is always the same as
            // zero-extension
            val @ ValueLocation::Cond(_) => {
                self.copy_value(val, CCLoc::Reg(new_reg))?;

                ValueLocation::Reg(new_reg)
            }
            ValueLocation::Immediate(imm) => {
                self.free(ValueLocation::Reg(new_reg))?;

                ValueLocation::Immediate((imm.as_i32().unwrap() as i64).into())
            }
        };

        self.push(out)?;
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
                let reg = match self.put_into_register(F32, &mut val) {
                    Err(e) => return Err(e),
                    Ok(o) => o.ok_or_else(|| error("Ran out of free registers".to_string()))?,
                };
                let temp = self
                    .take_or_free_reg(I32)
                    .ok_or_else(|| error("Ran out of free registers"))?;

                let sign_mask = self.aligned_label(4, LabelValue::I32(SIGN_MASK_F32 as i32));
                let float_cmp_mask =
                    self.aligned_label(16, LabelValue::I32(0xCF00_0000_u32 as i32));
                let zero = self.aligned_label(16, LabelValue::I32(0));

                dynasm!(self.asm
                    ; cvttss2si Rd(temp.rq().unwrap()), Rx(reg.rx().unwrap())
                    ; cmp Rd(temp.rq().unwrap()), [=>sign_mask.0]
                    ; jne >ret
                    ; ucomiss Rx(reg.rx().unwrap()), Rx(reg.rx().unwrap())
                    ; jp >trap
                    ; ucomiss Rx(reg.rx().unwrap()), [=>float_cmp_mask.0]
                    ; jnae >trap
                    ; ucomiss Rx(reg.rx().unwrap()), [=>zero.0]
                    ; jb >ret
                ; trap:
                    ;; self.trap(TrapCode::BadConversionToInteger)
                ; ret:
                );

                ValueLocation::Reg(temp)
            }
        };

        self.free(val)?;

        self.push(out_val)?;
        Ok(())
    }

    pub fn i32_truncate_f32_u(&mut self) -> Result<(), Error> {
        let mut val = self.pop()?;

        let out_val = match val {
            ValueLocation::Immediate(imm) => ValueLocation::Immediate(
                (f32::from_bits(imm.as_f32().unwrap().to_bits()) as i32).into(),
            ),
            _ => {
                let reg = match self.put_into_temp_register(F32, &mut val) {
                    Err(e) => return Err(e),
                    Ok(o) => o.ok_or_else(|| error("Ran out of free registers".to_string()))?,
                };

                let temp = self
                    .take_or_free_reg(I32)
                    .ok_or_else(|| error("Ran out of free registers"))?;

                let sign_mask = self.aligned_label(4, LabelValue::I32(SIGN_MASK_F32 as i32));
                let float_cmp_mask =
                    self.aligned_label(16, LabelValue::I32(0x4F00_0000_u32 as i32));

                dynasm!(self.asm
                    ; ucomiss Rx(reg.rx().unwrap()), [=>float_cmp_mask.0]
                    ; jae >else_
                    ; jp >trap
                    ; cvttss2si Rd(temp.rq().unwrap()), Rx(reg.rx().unwrap())
                    ; test Rd(temp.rq().unwrap()), Rd(temp.rq().unwrap())
                    ; js >trap
                    ; jmp >ret
                ; else_:
                    ; subss Rx(reg.rx().unwrap()), [=>float_cmp_mask.0]
                    ; cvttss2si Rd(temp.rq().unwrap()), Rx(reg.rx().unwrap())
                    ; test Rd(temp.rq().unwrap()), Rd(temp.rq().unwrap())
                    ; js >trap
                    ; add Rq(temp.rq().unwrap()), [=>sign_mask.0]
                    ; jmp >ret
                ; trap:
                    ;; self.trap(TrapCode::BadConversionToInteger)
                ; ret:
                );

                ValueLocation::Reg(temp)
            }
        };

        self.free(val)?;

        self.push(out_val)?;
        Ok(())
    }

    pub fn i32_truncate_f64_s(&mut self) -> Result<(), Error> {
        let mut val = self.pop()?;

        let out_val = match val {
            ValueLocation::Immediate(imm) => ValueLocation::Immediate(
                (f64::from_bits(imm.as_f64().unwrap().to_bits()) as i32).into(),
            ),
            _ => {
                let reg = match self.put_into_register(F32, &mut val) {
                    Err(e) => return Err(e),
                    Ok(o) => o.ok_or_else(|| error("Ran out of free registers".to_string()))?,
                };

                let temp = self
                    .take_or_free_reg(I32)
                    .ok_or_else(|| error("Ran out of free registers"))?;

                let sign_mask = self.aligned_label(4, LabelValue::I32(SIGN_MASK_F32 as i32));
                let float_cmp_mask =
                    self.aligned_label(16, LabelValue::I64(0xC1E0_0000_0020_0000_u64 as i64));
                let zero = self.aligned_label(16, LabelValue::I64(0));

                dynasm!(self.asm
                    ; cvttsd2si Rd(temp.rq().unwrap()), Rx(reg.rx().unwrap())
                    ; cmp Rd(temp.rq().unwrap()), [=>sign_mask.0]
                    ; jne >ret
                    ; ucomisd Rx(reg.rx().unwrap()), Rx(reg.rx().unwrap())
                    ; jp >trap
                    ; ucomisd Rx(reg.rx().unwrap()), [=>float_cmp_mask.0]
                    ; jna >trap
                    ; ucomisd Rx(reg.rx().unwrap()), [=>zero.0]
                    ; jb >ret
                ; trap:
                    ;; self.trap(TrapCode::BadConversionToInteger)
                ; ret:
                );

                ValueLocation::Reg(temp)
            }
        };

        self.free(val)?;

        self.push(out_val)?;
        Ok(())
    }

    pub fn i32_truncate_f64_u(&mut self) -> Result<(), Error> {
        let mut val = self.pop()?;

        let out_val = match val {
            ValueLocation::Immediate(imm) => ValueLocation::Immediate(
                (f64::from_bits(imm.as_f64().unwrap().to_bits()) as u32).into(),
            ),
            _ => {
                let reg = match self.put_into_temp_register(F32, &mut val) {
                    Err(e) => return Err(e),
                    Ok(o) => o.ok_or_else(|| error("Ran out of free registers".to_string()))?,
                };

                let temp = self
                    .take_or_free_reg(I32)
                    .ok_or_else(|| error("Ran out of free registers"))?;

                let sign_mask = self.aligned_label(4, LabelValue::I32(SIGN_MASK_F32 as i32));
                let float_cmp_mask =
                    self.aligned_label(16, LabelValue::I64(0x41E0_0000_0000_0000_u64 as i64));

                dynasm!(self.asm
                    ; ucomisd Rx(reg.rx().unwrap()), [=>float_cmp_mask.0]
                    ; jae >else_
                    ; jp >trap
                    ; cvttsd2si Rd(temp.rq().unwrap()), Rx(reg.rx().unwrap())
                    ; test Rd(temp.rq().unwrap()), Rd(temp.rq().unwrap())
                    ; js >trap
                    ; jmp >ret
                ; else_:
                    ; subsd Rx(reg.rx().unwrap()), [=>float_cmp_mask.0]
                    ; cvttsd2si Rd(temp.rq().unwrap()), Rx(reg.rx().unwrap())
                    ; test Rd(temp.rq().unwrap()), Rd(temp.rq().unwrap())
                    ; js >trap
                    ; add Rq(temp.rq().unwrap()), [=>sign_mask.0]
                    ; jmp >ret
                ; trap:
                    ;; self.trap(TrapCode::BadConversionToInteger)
                ; ret:
                );

                ValueLocation::Reg(temp)
            }
        };

        self.free(val)?;

        self.push(out_val)?;
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
                let reg = match self.put_into_temp_register(F32, &mut val) {
                    Err(e) => return Err(e),
                    Ok(o) => o.ok_or_else(|| error("Ran out of free registers".to_string()))?,
                };

                let temp = self
                    .take_or_free_reg(I32)
                    .ok_or_else(|| error("Ran out of free registers"))?;

                let sign_mask = self.aligned_label(16, LabelValue::I64(SIGN_MASK_F64 as i64));
                let float_cmp_mask =
                    self.aligned_label(16, LabelValue::I32(0xDF00_0000_u32 as i32));
                let zero = self.aligned_label(16, LabelValue::I64(0));

                dynasm!(self.asm
                    ; cvttss2si Rq(temp.rq().unwrap()), Rx(reg.rx().unwrap())
                    ; cmp Rq(temp.rq().unwrap()), [=>sign_mask.0]
                    ; jne >ret
                    ; ucomiss Rx(reg.rx().unwrap()), Rx(reg.rx().unwrap())
                    ; jp >trap
                    ; ucomiss Rx(reg.rx().unwrap()), [=>float_cmp_mask.0]
                    ; jnae >trap
                    ; ucomiss Rx(reg.rx().unwrap()), [=>zero.0]
                    ; jb >ret
                ; trap:
                    ;; self.trap(TrapCode::BadConversionToInteger)
                ; ret:
                );

                ValueLocation::Reg(temp)
            }
        };

        self.free(val)?;

        self.push(out_val)?;
        Ok(())
    }

    pub fn i64_truncate_f64_s(&mut self) -> Result<(), Error> {
        let mut val = self.pop()?;

        let out_val = match val {
            ValueLocation::Immediate(imm) => ValueLocation::Immediate(
                (f64::from_bits(imm.as_f64().unwrap().to_bits()) as i64).into(),
            ),
            _ => {
                let reg = match self.put_into_register(F32, &mut val) {
                    Err(e) => return Err(e),
                    Ok(o) => o.ok_or_else(|| error("Ran out of free registers".to_string()))?,
                };

                let temp = self
                    .take_or_free_reg(I32)
                    .ok_or_else(|| error("Ran out of free registers"))?;

                let sign_mask = self.aligned_label(8, LabelValue::I64(SIGN_MASK_F64 as i64));
                let float_cmp_mask =
                    self.aligned_label(16, LabelValue::I64(0xC3E0_0000_0000_0000_u64 as i64));
                let zero = self.aligned_label(16, LabelValue::I64(0));

                dynasm!(self.asm
                    ; cvttsd2si Rq(temp.rq().unwrap()), Rx(reg.rx().unwrap())
                    ; cmp Rq(temp.rq().unwrap()), [=>sign_mask.0]
                    ; jne >ret
                    ; ucomisd Rx(reg.rx().unwrap()), Rx(reg.rx().unwrap())
                    ; jp >trap
                    ; ucomisd Rx(reg.rx().unwrap()), [=>float_cmp_mask.0]
                    ; jnae >trap
                    ; ucomisd Rx(reg.rx().unwrap()), [=>zero.0]
                    ; jb >ret
                ; trap:
                    ;; self.trap(TrapCode::BadConversionToInteger)
                ; ret:
                );

                ValueLocation::Reg(temp)
            }
        };

        self.free(val)?;

        self.push(out_val)?;
        Ok(())
    }

    pub fn i64_truncate_f32_u(&mut self) -> Result<(), Error> {
        let mut val = self.pop()?;

        let out_val = match val {
            ValueLocation::Immediate(imm) => ValueLocation::Immediate(
                (f32::from_bits(imm.as_f32().unwrap().to_bits()) as u64).into(),
            ),
            _ => {
                let reg = match self.put_into_register(F32, &mut val) {
                    Err(e) => return Err(e),
                    Ok(o) => o.ok_or_else(|| error("Ran out of free registers".to_string()))?,
                };

                let temp = self
                    .take_or_free_reg(I64)
                    .ok_or_else(|| error("Ran out of free registers"))?;
                let sign_mask = self.aligned_label(16, LabelValue::I64(SIGN_MASK_F64 as i64));
                let u64_trunc_f32_const = self.aligned_label(16, LabelValue::I32(0x5F00_0000_i32));

                dynasm!(self.asm
                    ; comiss Rx(reg.rx().unwrap()), [=>u64_trunc_f32_const.0]
                    ; jae >large
                    ; jp >trap
                    ; cvttss2si Rq(temp.rq().unwrap()), Rx(reg.rx().unwrap())
                    ; test Rq(temp.rq().unwrap()), Rq(temp.rq().unwrap())
                    ; js >trap
                    ; jmp >cont
                ; large:
                    ; subss Rx(reg.rx().unwrap()), [=>u64_trunc_f32_const.0]
                    ; cvttss2si Rq(temp.rq().unwrap()), Rx(reg.rx().unwrap())
                    ; test Rq(temp.rq().unwrap()), Rq(temp.rq().unwrap())
                    ; js >trap
                    ; add Rq(temp.rq().unwrap()), [=>sign_mask.0]
                    ; jmp >cont
                ; trap:
                    ;; self.trap(TrapCode::BadConversionToInteger)
                ; cont:
                );

                ValueLocation::Reg(temp)
            }
        };

        self.free(val)?;

        self.push(out_val)?;
        Ok(())
    }

    pub fn i64_truncate_f64_u(&mut self) -> Result<(), Error> {
        let mut val = self.pop()?;

        let out_val = match val {
            ValueLocation::Immediate(imm) => ValueLocation::Immediate(
                (f64::from_bits(imm.as_f64().unwrap().to_bits()) as u64).into(),
            ),
            _ => {
                let reg = match self.put_into_register(F64, &mut val) {
                    Err(e) => return Err(e),
                    Ok(o) => o.ok_or_else(|| error("Ran out of free registers".to_string()))?,
                };

                let temp = self
                    .take_or_free_reg(I64)
                    .ok_or_else(|| error("Ran out of free registers"))?;

                let sign_mask = self.aligned_label(16, LabelValue::I64(SIGN_MASK_F64 as i64));
                let u64_trunc_f64_const =
                    self.aligned_label(16, LabelValue::I64(0x43E0_0000_0000_0000_i64));

                dynasm!(self.asm
                    ; comisd Rx(reg.rx().unwrap()), [=>u64_trunc_f64_const.0]
                    ; jnb >large
                    ; jp >trap
                    ; cvttsd2si Rq(temp.rq().unwrap()), Rx(reg.rx().unwrap())
                    ; cmp Rq(temp.rq().unwrap()), 0
                    ; jl >trap
                    ; jmp >cont
                ; large:
                    ; subsd Rx(reg.rx().unwrap()), [=>u64_trunc_f64_const.0]
                    ; cvttsd2si Rq(temp.rq().unwrap()), Rx(reg.rx().unwrap())
                    ; cmp Rq(temp.rq().unwrap()), 0
                    ; jnge >trap
                    ; add Rq(temp.rq().unwrap()), [=>sign_mask.0]
                    ; jmp >cont
                ; trap:
                    ;; self.trap(TrapCode::BadConversionToInteger)
                ; cont:
                );

                ValueLocation::Reg(temp)
            }
        };

        self.free(val)?;

        self.push(out_val)?;
        Ok(())
    }

    pub fn f32_convert_from_i32_u(&mut self) -> Result<(), Error> {
        let mut val = self.pop()?;

        let out_val = match val {
            ValueLocation::Immediate(imm) => ValueLocation::Immediate(
                Ieee32::from_bits((imm.as_i32().unwrap() as u32 as f32).to_bits()).into(),
            ),
            _ => {
                let reg = match self.put_into_register(I32, &mut val) {
                    Err(e) => return Err(e),
                    Ok(o) => o.ok_or_else(|| error("Ran out of free registers".to_string()))?,
                };

                let temp = self
                    .take_or_free_reg(F32)
                    .ok_or_else(|| error("Ran out of free registers"))?;

                dynasm!(self.asm
                    ; mov Rd(reg.rq().unwrap()), Rd(reg.rq().unwrap())
                    ; cvtsi2ss Rx(temp.rx().unwrap()), Rq(reg.rq().unwrap())
                );

                ValueLocation::Reg(temp)
            }
        };

        self.free(val)?;

        self.push(out_val)?;
        Ok(())
    }

    pub fn f64_convert_from_i32_u(&mut self) -> Result<(), Error> {
        let mut val = self.pop()?;

        let out_val = match val {
            ValueLocation::Immediate(imm) => ValueLocation::Immediate(
                Ieee64::from_bits((imm.as_i32().unwrap() as u32 as f64).to_bits()).into(),
            ),
            _ => {
                let reg = match self.put_into_register(I32, &mut val) {
                    Err(e) => return Err(e),
                    Ok(o) => o.ok_or_else(|| error("Ran out of free registers".to_string()))?,
                };

                let temp = self
                    .take_or_free_reg(F64)
                    .ok_or_else(|| error("Ran out of free registers"))?;

                dynasm!(self.asm
                    ; mov Rd(reg.rq().unwrap()), Rd(reg.rq().unwrap())
                    ; cvtsi2sd Rx(temp.rx().unwrap()), Rq(reg.rq().unwrap())
                );

                ValueLocation::Reg(temp)
            }
        };

        self.free(val)?;

        self.push(out_val)?;
        Ok(())
    }

    pub fn f32_convert_from_i64_u(&mut self) -> Result<(), Error> {
        let mut val = self.pop()?;

        let out_val = match val {
            ValueLocation::Immediate(imm) => ValueLocation::Immediate(
                Ieee32::from_bits((imm.as_i64().unwrap() as u64 as f32).to_bits()).into(),
            ),
            _ => {
                let reg = match self.put_into_register(I64, &mut val) {
                    Err(e) => return Err(e),
                    Ok(o) => o.ok_or_else(|| error("Ran out of free registers".to_string()))?,
                };

                let out = self
                    .take_or_free_reg(F32)
                    .ok_or_else(|| error("Ran out of free registers"))?;
                let temp = self
                    .take_or_free_reg(I64)
                    .ok_or_else(|| error("Ran out of free registers"))?;

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

                self.free(ValueLocation::Reg(temp))?;

                ValueLocation::Reg(out)
            }
        };

        self.free(val)?;

        self.push(out_val)?;
        Ok(())
    }

    pub fn f64_convert_from_i64_u(&mut self) -> Result<(), Error> {
        let mut val = self.pop()?;

        let out_val = match val {
            ValueLocation::Immediate(imm) => ValueLocation::Immediate(
                Ieee64::from_bits((imm.as_i64().unwrap() as u64 as f64).to_bits()).into(),
            ),
            _ => {
                let reg = match self.put_into_register(I64, &mut val) {
                    Err(e) => return Err(e),
                    Ok(o) => o.ok_or_else(|| error("Ran out of free registers".to_string()))?,
                };

                let out = self
                    .take_or_free_reg(F32)
                    .ok_or_else(|| error("Ran out of free registers"))?;
                let temp = self
                    .take_or_free_reg(I64)
                    .ok_or_else(|| error("Ran out of free registers"))?;

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

                self.free(ValueLocation::Reg(temp))?;

                ValueLocation::Reg(out)
            }
        };

        self.free(val)?;

        self.push(out_val)?;
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

        self.push(out)?;
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

        self.push(out)?;
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

        self.push(out)?;
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

        self.push(out)?;
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

        self.push(out)?;
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
            FunctionDefLocation::PossiblyExternal,
        )?;
        Ok(())
    }

    pub fn f32_floor(&mut self) -> Result<(), Error> {
        self.relocated_function_call(
            &ir::ExternalName::LibCall(ir::LibCall::FloorF32),
            iter::once(F32),
            iter::once(F32),
            FunctionDefLocation::PossiblyExternal,
        )?;
        Ok(())
    }

    pub fn f32_nearest(&mut self) -> Result<(), Error> {
        self.relocated_function_call(
            &ir::ExternalName::LibCall(ir::LibCall::NearestF32),
            iter::once(F32),
            iter::once(F32),
            FunctionDefLocation::PossiblyExternal,
        )?;
        Ok(())
    }

    pub fn f32_trunc(&mut self) -> Result<(), Error> {
        self.relocated_function_call(
            &ir::ExternalName::LibCall(ir::LibCall::TruncF32),
            iter::once(F32),
            iter::once(F32),
            FunctionDefLocation::PossiblyExternal,
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
            FunctionDefLocation::PossiblyExternal,
        )?;
        Ok(())
    }

    pub fn f64_floor(&mut self) -> Result<(), Error> {
        self.relocated_function_call(
            &ir::ExternalName::LibCall(ir::LibCall::FloorF64),
            iter::once(F64),
            iter::once(F64),
            FunctionDefLocation::PossiblyExternal,
        )?;
        Ok(())
    }

    pub fn f64_nearest(&mut self) -> Result<(), Error> {
        self.relocated_function_call(
            &ir::ExternalName::LibCall(ir::LibCall::NearestF64),
            iter::once(F64),
            iter::once(F64),
            FunctionDefLocation::PossiblyExternal,
        )?;
        Ok(())
    }

    pub fn f64_trunc(&mut self) -> Result<(), Error> {
        self.relocated_function_call(
            &ir::ExternalName::LibCall(ir::LibCall::TruncF64),
            iter::once(F64),
            iter::once(F64),
            FunctionDefLocation::PossiblyExternal,
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
        do_div: impl FnOnce(&mut Self, &mut ValueLocation) -> Result<(), Error>,
    ) -> Result<
        (
            ValueLocation,
            ValueLocation,
            impl Iterator<Item = (ValueLocation, CCLoc)> + Clone + 'this,
        ),
        Error,
    > {
        // To stop `take_or_free_reg` from allocating either of these necessary registers
        self.mark_used(CCLoc::Reg(RAX))?;
        self.mark_used(CCLoc::Reg(RDX))?;
        if divisor == ValueLocation::Reg(RAX) || divisor == ValueLocation::Reg(RDX) {
            let new_reg = self
                .take_or_free_reg(GPRType::Rq)
                .ok_or_else(|| error("Ran out of free registers"))?;
            self.copy_value(divisor, CCLoc::Reg(new_reg))?;
            self.free(divisor)?;

            divisor = ValueLocation::Reg(new_reg);
        }
        self.free(ValueLocation::Reg(RDX))?;
        self.free(ValueLocation::Reg(RAX))?;

        let saved_rax = if self.is_free(CCLoc::Reg(RAX))? {
            None
        } else {
            // DON'T FREE THIS REGISTER HERE - since we don't
            // remove it from the stack freeing the register
            // here will cause `take_or_free_reg` to allocate it.
            Some(self.push_copy_physical(ValueLocation::Reg(RAX))?)
        };

        let saved_rdx = if self.is_free(CCLoc::Reg(RDX))? {
            None
        } else {
            // DON'T FREE THIS REGISTER HERE - since we don't
            // remove it from the stack freeing the register
            // here will cause `take_or_free_reg` to allocate it.
            Some(self.push_copy_physical(ValueLocation::Reg(RDX))?)
        };

        let saved = saved_rax
            .map(|saved| (ValueLocation::from(saved), CCLoc::Reg(RAX)))
            .into_iter()
            .chain(saved_rdx.map(|saved| (ValueLocation::from(saved), CCLoc::Reg(RDX))));

        self.copy_value(dividend, CCLoc::Reg(RAX))?;
        self.mark_used(CCLoc::Reg(RAX))?;

        self.free(dividend)?;
        // To stop `take_or_free_reg` from allocating either of these necessary registers
        self.mark_used(CCLoc::Reg(RDX))?;

        do_div(self, &mut divisor)?;
        self.free(divisor)?;

        if self.is_free(CCLoc::Reg(RAX))? {
            return Err(error("full_div: RAX is not free".to_string()));
        }
        if self.is_free(CCLoc::Reg(RDX))? {
            return Err(error("full_div: RDX is not free".to_string()));
        }

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
            impl Iterator<Item = (ValueLocation, CCLoc)> + Clone + 'this,
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
                Ok(())
            }
            ValueLocation::Immediate(_) | ValueLocation::Reg(_) | ValueLocation::Cond(_) => {
                let r = match this.put_into_register(I32, divisor) {
                    Err(e) => return Err(e),
                    Ok(o) => o.ok_or_else(|| error("Ran out of free registers".to_string()))?,
                };

                dynasm!(this.asm
                    ; xor edx, edx
                    ; div Rd(r.rq().unwrap())
                );
                Ok(())
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
            impl Iterator<Item = (ValueLocation, CCLoc)> + Clone + 'this,
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
                Ok(())
            }
            ValueLocation::Immediate(_) | ValueLocation::Reg(_) | ValueLocation::Cond(_) => {
                let r = match this.put_into_register(I32, divisor) {
                    Err(e) => return Err(e),
                    Ok(o) => o.ok_or_else(|| error("Ran out of free registers".to_string()))?,
                };

                dynasm!(this.asm
                    ; cdq
                    ; idiv Rd(r.rq().unwrap())
                );
                Ok(())
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
            impl Iterator<Item = (ValueLocation, CCLoc)> + Clone + 'this,
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
                Ok(())
            }
            ValueLocation::Immediate(_) | ValueLocation::Reg(_) | ValueLocation::Cond(_) => {
                let r = match this.put_into_register(I64, divisor) {
                    Err(e) => return Err(e),
                    Ok(o) => o.ok_or_else(|| error("Ran out of free registers".to_string()))?,
                };
                dynasm!(this.asm
                    ; xor rdx, rdx
                    ; div Rq(r.rq().unwrap())
                );
                Ok(())
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
            impl Iterator<Item = (ValueLocation, CCLoc)> + Clone + 'this,
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
                Ok(())
            }
            ValueLocation::Immediate(_) | ValueLocation::Reg(_) | ValueLocation::Cond(_) => {
                let r = match this.put_into_register(I64, divisor) {
                    Err(e) => return Err(e),
                    Ok(o) => o.ok_or_else(|| error("Ran out of free registers".to_string()))?,
                };

                dynasm!(this.asm
                    ; cqo
                    ; idiv Rq(r.rq().unwrap())
                );
                Ok(())
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
                ))?;
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
                let rreg = match self.put_into_register(I32, &mut right) {
                    Err(e) => return Err(e),
                    Ok(o) => o.ok_or_else(|| error("Ran out of free registers".to_string()))?,
                };
                let lreg = match self.put_into_temp_register(I32, &mut left) {
                    Err(e) => return Err(e),
                    Ok(o) => o.ok_or_else(|| error("Ran out of free registers".to_string()))?,
                };

                dynasm!(self.asm
                    ; imul Rd(lreg.rq().unwrap()), Rd(rreg.rq().unwrap())
                );
                left
            }
            ValueLocation::Stack(offset) => {
                let offset = self.adjusted_offset(offset);

                let lreg = match self.put_into_temp_register(I32, &mut left) {
                    Err(e) => return Err(e),
                    Ok(o) => o.ok_or_else(|| error("Ran out of free registers".to_string()))?,
                };

                dynasm!(self.asm
                    ; imul Rd(lreg.rq().unwrap()), [rsp + offset]
                );
                left
            }
            ValueLocation::Immediate(i) => {
                let lreg = match self.put_into_register(I32, &mut left) {
                    Err(e) => return Err(e),
                    Ok(o) => o.ok_or_else(|| error("Ran out of free registers".to_string()))?,
                };

                let new_reg = self
                    .take_or_free_reg(I32)
                    .ok_or_else(|| error("Ran out of free registers"))?;
                dynasm!(self.asm
                    ; imul Rd(new_reg.rq().unwrap()), Rd(lreg.rq().unwrap()), i.as_i32().unwrap()
                );
                self.free(left)?;
                ValueLocation::Reg(new_reg)
            }
        };

        self.push(out)?;
        self.free(right)?;
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
                ))?;
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
                let rreg = match self.put_into_register(I64, &mut right) {
                    Err(e) => return Err(e),
                    Ok(o) => o.ok_or_else(|| error("Ran out of free registers".to_string()))?,
                };
                let lreg = match self.put_into_temp_register(I64, &mut left) {
                    Err(e) => return Err(e),
                    Ok(o) => o.ok_or_else(|| error("Ran out of free registers".to_string()))?,
                };

                dynasm!(self.asm
                    ; imul Rq(lreg.rq().unwrap()), Rq(rreg.rq().unwrap())
                );
                left
            }
            ValueLocation::Stack(offset) => {
                let offset = self.adjusted_offset(offset);

                let lreg = match self.put_into_temp_register(I64, &mut left) {
                    Err(e) => return Err(e),
                    Ok(o) => o.ok_or_else(|| error("Ran out of free registers".to_string()))?,
                };

                dynasm!(self.asm
                    ; imul Rq(lreg.rq().unwrap()), [rsp + offset]
                );
                left
            }
            ValueLocation::Immediate(i) => {
                let i = i.as_i64().unwrap();
                if let Ok(i) = i.try_into() {
                    let new_reg = self
                        .take_or_free_reg(I64)
                        .ok_or_else(|| error("Ran out of free registers"))?;

                    let lreg = self
                        .put_into_register(I64, &mut left)?
                        .ok_or_else(|| error("Ran out of free registers".to_string()))?;

                    dynasm!(self.asm
                        ; imul Rq(new_reg.rq().unwrap()), Rq(lreg.rq().unwrap()), i
                    );

                    self.free(left)?;

                    ValueLocation::Reg(new_reg)
                } else {
                    let rreg = self
                        .put_into_register(I64, &mut right)?
                        .ok_or_else(|| error("Ran out of free registers".to_string()))?;
                    let lreg = self
                        .put_into_temp_register(I64, &mut left)?
                        .ok_or_else(|| error("Ran out of free registers".to_string()))?;

                    dynasm!(self.asm
                        ; imul Rq(lreg.rq().unwrap()), Rq(rreg.rq().unwrap())
                    );
                    left
                }
            }
        };

        self.push(out)?;
        self.free(right)?;
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
                self.free(then)?;
                self.push(else_)?;
            } else {
                self.free(else_)?;
                self.push(then)?;
            }

            return Ok(());
        }

        let cond_code = match cond {
            ValueLocation::Cond(cc) => cc,
            _ => {
                let cond_reg = match self.put_into_register(I32, &mut cond) {
                    Err(e) => return Err(e),
                    Ok(o) => o.ok_or_else(|| error("Ran out of free registers".to_string()))?,
                };
                dynasm!(self.asm
                    ; test Rd(cond_reg.rq().unwrap()), Rd(cond_reg.rq().unwrap())
                );
                self.free(cond)?;

                cc::NOT_EQUAL
            }
        };

        let else_ = if let ValueLocation::Stack(offset) = else_ {
            CCLoc::Stack(offset)
        } else {
            let gpr = match self.put_into_register(I32, &mut else_) {
                Err(e) => return Err(e),
                Ok(o) => o.ok_or_else(|| error("Ran out of free registers".to_string()))?,
            };
            CCLoc::Reg(gpr)
        };

        let then = if let ValueLocation::Stack(offset) = then {
            CCLoc::Stack(offset)
        } else {
            let gpr = match self.put_into_register(I32, &mut then) {
                Err(e) => return Err(e),
                Ok(o) => o.ok_or_else(|| error("Ran out of free registers".to_string()))?,
            };
            CCLoc::Reg(gpr)
        };

        let out_gpr = match (then, else_) {
            (CCLoc::Reg(then_reg), else_) if self.num_usages(then)? <= 1 => {
                self.cmov(!cond_code, then_reg, else_);
                self.free(else_)?;

                then_reg
            }
            (then, CCLoc::Reg(else_reg)) if self.num_usages(else_)? <= 1 => {
                self.cmov(cond_code, else_reg, then);
                self.free(then)?;

                else_reg
            }
            (then, else_) => {
                let out = self
                    .take_or_free_reg(GPRType::Rq)
                    .ok_or_else(|| error("Ran out of free registers"))?;
                self.copy_value(else_.into(), CCLoc::Reg(out))?;
                self.cmov(cond_code, out, then);

                self.free(then)?;
                self.free(else_)?;

                out
            }
        };

        self.push(ValueLocation::Reg(out_gpr))?;
        Ok(())
    }

    pub fn pick(&mut self, depth: u32) -> Result<(), Error> {
        let idx = self.stack.len() - 1 - depth as usize;
        let v = self.stack[idx];
        self.mark_used(v)?;
        self.push(v)
    }

    pub fn const_(&mut self, imm: Value) -> Result<(), Error> {
        self.push(ValueLocation::Immediate(imm))?;
        Ok(())
    }

    fn relocated_function_call<
        A: IntoIterator<Item = SignlessType>,
        R: IntoIterator<Item = SignlessType>,
    >(
        &mut self,
        name: &cranelift_codegen::ir::ExternalName,
        args: A,
        rets: R,
        func_def_loc: FunctionDefLocation,
    ) -> Result<(), Error>
    where
        A::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
        R::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        let locs = arg_locs_skip_caller_vmctx::<CCLoc, _>(args);

        let saved_vmctx = if func_def_loc == FunctionDefLocation::PossiblyExternal {
            self.save_regs(iter::once(GPR::Rq(CALLER_VMCTX)))?;
            dynasm!(self.asm
                ; mov Rq(CALLER_VMCTX), Rq(VMCTX)
            );
            self.mark_used(CCLoc::Reg(GPR::Rq(CALLER_VMCTX)))?;
            Some(
                self.push_copy_physical(ValueLocation::Reg(GPR::Rq(VMCTX)))?
                    .into(),
            )
        } else {
            None
        };

        self.save_volatile()?;

        let locs = self.pass_outgoing_args(locs.as_ref())?;

        let needed_depth = self.physical_stack_depth.clone();

        // 2 bytes for the 64-bit `mov` opcode + register ident, the rest is the immediate
        self.sinks.relocs.reloc_external(
            (self.asm.offset().0
                - self.func_starts[self.current_function as usize]
                    .0
                    .unwrap()
                    .0) as u32
                + 2,
            // Passing a default location here, since until proven otherwise, it's not used.
            ir::SourceLoc::default(),
            binemit::Reloc::Abs8,
            name,
            0,
        );
        let temp = self
            .take_reg(I64)
            .ok_or_else(|| error("Ran out of free registers"))?;

        dynasm!(self.asm
            ; mov Rq(temp.rq().unwrap()), QWORD 0xDEAD_BEEF_DEAD_BEEF_u64 as i64
            ;; assert_eq!(self.physical_stack_depth, needed_depth)
            ; call Rq(temp.rq().unwrap())
        );
        self.free(ValueLocation::Reg(temp))?;

        for i in locs {
            self.free(i)?;
        }

        self.push_function_returns(rets)?;

        if func_def_loc == FunctionDefLocation::PossiblyExternal {
            let saved_vmctx = saved_vmctx.unwrap();
            self.copy_value(saved_vmctx, CCLoc::Reg(GPR::Rq(VMCTX)))?;
            self.free(saved_vmctx)?;
            self.free(ValueLocation::Reg(GPR::Rq(CALLER_VMCTX)))?;
        }

        Ok(())
    }

    fn builtin_function_call<
        A: IntoIterator<Item = SignlessType>,
        R: IntoIterator<Item = SignlessType>,
    >(
        &mut self,
        i: BuiltinFunctionIndex,
        args: A,
        rets: R,
    ) -> Result<(), Error>
    where
        A::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
        R::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        let locs = arg_locs::<CCLoc, _>(args);

        let saved_vmctx =
            ValueLocation::from(self.push_copy_physical(ValueLocation::Reg(GPR::Rq(VMCTX)))?);

        self.save_volatile()?;

        let locs = self.pass_outgoing_args(locs.as_ref())?;
        let needed_depth = self.physical_stack_depth.clone();

        let temp = self
            .take_reg(I64)
            .ok_or_else(|| error("Ran out of free registers"))?;
        dynasm!(self.asm
            ; mov Rq(temp.rq().unwrap()), [
                Rq(VMCTX) + self.module_context.vmctx_builtin_function(i.index()) as i32
            ]
            ;; assert_eq!(self.physical_stack_depth, needed_depth)
            ; call Rq(temp.rq().unwrap())
        );

        self.free(ValueLocation::Reg(temp))?;

        for i in locs {
            self.free(i)?;
        }

        self.push_function_returns(rets)?;

        self.copy_value(saved_vmctx, CCLoc::Reg(GPR::Rq(VMCTX)))?;
        self.free(saved_vmctx)?;

        Ok(())
    }

    // TODO: Other memory indices
    pub fn memory_size(&mut self) -> Result<(), Error> {
        let memory_index = 0;
        if let Some(defined_memory_index) = self.module_context.defined_memory_index(memory_index) {
            self.push(ValueLocation::Immediate(defined_memory_index.into()))?;
            self.builtin_function_call(
                BuiltinFunctionIndex::get_memory32_size_index(),
                [self.pointer_type].iter().copied(),
                [self.pointer_type].iter().copied(),
            )?;
        } else {
            self.push(ValueLocation::Immediate(memory_index.into()))?;
            self.builtin_function_call(
                BuiltinFunctionIndex::get_imported_memory32_size_index(),
                [self.pointer_type].iter().copied(),
                [self.pointer_type].iter().copied(),
            )?;
        }
        Ok(())
    }

    // TODO: Other memory indices
    pub fn memory_grow(&mut self) -> Result<(), Error> {
        let memory_index = 0;
        if let Some(defined_memory_index) = self.module_context.defined_memory_index(memory_index) {
            self.push(ValueLocation::Immediate(defined_memory_index.into()))?;
            self.builtin_function_call(
                BuiltinFunctionIndex::get_memory32_grow_index(),
                [self.pointer_type, self.pointer_type].iter().copied(),
                [self.pointer_type].iter().copied(),
            )?;
        } else {
            self.push(ValueLocation::Immediate(memory_index.into()))?;
            self.builtin_function_call(
                BuiltinFunctionIndex::get_imported_memory32_grow_index(),
                [self.pointer_type, self.pointer_type].iter().copied(),
                [self.pointer_type].iter().copied(),
            )?;
        }
        Ok(())
    }

    // TODO: Use `ArrayVec`?
    // TODO: This inefficiently duplicates registers but it's not really possible
    //       to double up stack space right now.
    /// Saves volatile (i.e. caller-saved) registers before a function call, if they are used.
    fn save_volatile(&mut self) -> Result<(), Error> {
        self.save_regs(SCRATCH_REGS.iter().copied())?;
        Ok(())
    }

    fn save_regs<I>(&mut self, to_save: I) -> Result<(), Error>
    where
        I: IntoIterator<Item = GPR>,
        I::IntoIter: Clone,
    {
        // TODO: We can filter out registers that are already marked free, but just to ensure
        //       that this doesn't fail when confronted with the `memory_grow`/`memory_size`
        //       weirdness.
        let to_save = to_save.into_iter();
        if to_save.clone().count() == 0 {
            return Ok(());
        }

        for i in 0..self.stack.len() {
            let old = self.stack[i];
            if let ValueLocation::Reg(vreg) = old {
                if to_save.clone().any(|r| r == vreg) {
                    let new = self.push_physical(old)?.into();

                    self.stack[i] = new;

                    for j in i + 1..self.stack.len() {
                        let cur = self.stack[j];
                        if cur == old {
                            self.free(cur)?;
                            self.mark_used(new)?;
                            self.stack[j] = new;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Write the arguments to the callee to the registers and the stack using the SystemV
    /// calling convention.
    fn pass_outgoing_args(
        &mut self,
        out_locs: Locs<impl ExactSizeIterator<Item = CCLoc> + DoubleEndedIterator + Clone>,
    ) -> Result<impl ExactSizeIterator<Item = CCLoc>, Error> {
        let mut needed_depth =
            StackDepth(self.physical_stack_depth.clone().0 + out_locs.max_depth.0);
        if needed_depth.0 & 1 != 0 {
            needed_depth.0 += 1;
        }

        self.set_stack_depth(needed_depth.clone())?;

        let mut pending = Vec::<(ValueLocation, CCLoc)>::with_capacity(out_locs.locs.len());

        for loc in out_locs.locs.clone().rev() {
            pending.push((
                self.pop()?,
                match loc {
                    CCLoc::Stack(offset) => CCLoc::Stack(offset - needed_depth.0 as i32),
                    CCLoc::Reg(_) => loc,
                },
            ));
        }

        let mut last_len = None;

        while !pending.is_empty() {
            let start_len = pending.len();

            for (src, dst) in mem::take(&mut pending) {
                if src != ValueLocation::from(dst) {
                    let dst = if self.is_free(dst)? {
                        self.mark_used(dst)?;
                        dst
                    } else {
                        pending.push((src, dst));
                        continue;
                    };

                    self.copy_value(src, dst)?;
                    self.free(src)?;
                }
            }

            if last_len == Some(pending.len()) {
                return Err(error(
                    "BUG: Making no progress allocating locations for function calling convention",
                ));
            }

            if pending.len() == start_len {
                let src = pending
                    .iter()
                    .filter_map(|(src, _)| CCLoc::try_from(*src).ok())
                    .next()
                    .ok_or_else(|| {
                        error(format!(
                            "Programmer error: We shouldn't need to push \
                             intermediate args if we don't have any argument sources in reusable \
                             locations ({:?}",
                            pending
                        ))
                    })?;

                let new_src = self.push_copy_physical(src.into())?.into();
                self.free(new_src)?;
                for (old_src, _) in pending.iter_mut() {
                    if *old_src == ValueLocation::from(src) {
                        *old_src = new_src;
                        self.free(*old_src)?;
                        self.mark_used(new_src)?;
                    }
                }
            }

            last_len = Some(start_len);
        }

        // We do this a second time just in case we had to use `push_physical` to resolve cycles in
        // `pending`
        self.set_stack_depth(needed_depth.clone())?;

        Ok(out_locs.locs.map(move |loc| match loc {
            CCLoc::Stack(offset) => CCLoc::Stack(offset - needed_depth.0 as i32),
            CCLoc::Reg(_) => loc,
        }))
    }

    fn push_function_returns(
        &mut self,
        returns: impl IntoIterator<Item = SignlessType>,
    ) -> Result<(), Error> {
        for loc in ret_locs(returns)? {
            self.mark_used(loc)?;

            self.push(loc.into())?;
        }
        Ok(())
    }

    fn trap_if(&mut self, ccode: CondCode, trap_code: TrapCode) {
        let label = self.create_label();
        self.br_on_cond_code(label, !ccode);
        self.trap(trap_code);
        self.define_label(label);
    }

    pub fn call_indirect<
        A: IntoIterator<Item = SignlessType>,
        R: IntoIterator<Item = SignlessType>,
    >(
        &mut self,
        type_id: u32,
        arg_types: A,
        return_types: R,
    ) -> Result<(), Error>
    where
        A::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
        R::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        let saved_vmctx = self.push_copy_physical(ValueLocation::Reg(GPR::Rq(VMCTX)))?;
        let locs = arg_locs_skip_caller_vmctx::<CCLoc, _>(arg_types);

        self.save_regs(iter::once(GPR::Rq(CALLER_VMCTX)))?;
        dynasm!(self.asm
            ; mov Rq(CALLER_VMCTX), Rq(VMCTX)
        );
        self.mark_used(CCLoc::Reg(GPR::Rq(CALLER_VMCTX)))?;

        for loc in locs.as_ref().locs {
            if loc.reg().is_some() {
                self.mark_used(loc)?;
            }
        }

        let mut callee = self.pop()?;
        let callee_reg = self
            .put_into_temp_register(I32, &mut callee)?
            .ok_or_else(|| error("Ran out of free registers".to_string()))?;

        for loc in locs.as_ref().locs {
            if loc.reg().is_some() {
                self.free(loc)?;
            }
        }

        self.save_volatile()?;

        let locs = self.pass_outgoing_args(locs.as_ref())?;
        let needed_depth = self.physical_stack_depth.clone();

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
        let (reg, offset) = reg_offset.ok_or(()).or_else::<Error, _>(|_| {
            let reg = self
                .take_reg(I64)
                .ok_or_else(|| error("Ran out of free registers"))?;

            dynasm!(self.asm
                ; mov Rq(reg.rq().unwrap()), [
                    Rq(VMCTX) + self.module_context.vmctx_vmtable_import_from(table_index) as i32
                ]
            );

            Ok((Some(reg), 0))
        })?;

        let temp0 = self
            .take_reg(I64)
            .ok_or_else(|| error("Ran out of free registers"))?;
        dynasm!(self.asm
            ; cmp Rd(callee_reg.rq().unwrap()), [
                Rq(reg.unwrap_or(vmctx).rq().unwrap()) +
                    offset +
                    self.module_context.vmtable_definition_current_elements() as i32
            ]
            ;; self.trap_if(cc::GE_U, TrapCode::TableOutOfBounds)
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
            self.free(ValueLocation::Reg(reg))?;
        }

        let temp1 = self
            .take_reg(I64)
            .ok_or_else(|| error("Ran out of free registers"))?;

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
            ;; self.trap_if(cc::NOT_EQUAL, TrapCode::BadSignature)
            ; mov Rq(VMCTX), [
                Rq(temp0.rq().unwrap()) +
                    Rq(callee_reg.rq().unwrap()) +
                    self.module_context.vmcaller_checked_anyfunc_vmctx() as i32
            ]
            ;; assert_eq!(self.physical_stack_depth, needed_depth)
            ; call QWORD [
                Rq(temp0.rq().unwrap()) +
                    Rq(callee_reg.rq().unwrap()) +
                    self.module_context.vmcaller_checked_anyfunc_func_ptr() as i32
            ]
        );

        self.free(ValueLocation::Reg(temp0))?;
        self.free(ValueLocation::Reg(temp1))?;
        self.free(ValueLocation::Reg(GPR::Rq(CALLER_VMCTX)))?;
        self.free(callee)?;

        for i in locs {
            self.free(i)?;
        }

        self.push_function_returns(return_types)?;

        self.copy_value(saved_vmctx.into(), CCLoc::Reg(GPR::Rq(VMCTX)))?;
        self.free(saved_vmctx)?;

        Ok(())
    }

    pub fn swap(&mut self, depth: u32) -> Result<(), Error> {
        let last = self.stack.len() - 1;

        if let Some(mut top) = self.stack.pop() {
            if let ValueLocation::Cond(_) = top {
                self.put_into_temp_location(I32, &mut top)?;
            }

            self.stack.push(top);
        }

        self.stack.swap(last, last - depth as usize);

        Ok(())
    }

    /// Call a function with the given index
    pub fn call_direct<A: IntoIterator<Item = SignlessType>, R: IntoIterator<Item = SignlessType>>(
        &mut self,
        index: u32,
        arg_types: A,
        return_types: R,
    ) -> Result<(), Error>
    where
        A::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
        R::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        self.relocated_function_call(
            &ir::ExternalName::user(0, index),
            arg_types,
            return_types,
            FunctionDefLocation::SameModule,
        )?;
        Ok(())
    }

    /// Recursively call the same function again
    pub fn call_direct_self<
        A: IntoIterator<Item = SignlessType>,
        R: IntoIterator<Item = SignlessType>,
    >(
        &mut self,
        arg_types: A,
        return_types: R,
    ) -> Result<(), Error>
    where
        A::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
        R::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        let locs = arg_locs_skip_caller_vmctx::<CCLoc, _>(arg_types);

        self.save_volatile()?;

        let (_, label) = self.func_starts[self.current_function as usize];

        let locs = self.pass_outgoing_args(locs.as_ref())?;

        dynasm!(self.asm
            ; call =>label
        );

        for i in locs {
            self.free(i)?;
        }

        self.push_function_returns(return_types)?;
        Ok(())
    }

    /// Call a function with the given index
    pub fn call_direct_imported<
        A: IntoIterator<Item = SignlessType>,
        R: IntoIterator<Item = SignlessType>,
    >(
        &mut self,
        index: u32,
        arg_types: A,
        return_types: R,
    ) -> Result<(), Error>
    where
        A::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
        R::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        let locs = arg_locs_skip_caller_vmctx::<CCLoc, _>(arg_types);

        self.save_regs(iter::once(GPR::Rq(CALLER_VMCTX)))?;
        dynasm!(self.asm
            ; mov Rq(CALLER_VMCTX), Rq(VMCTX)
        );
        self.mark_used(CCLoc::Reg(GPR::Rq(CALLER_VMCTX)))?;
        let saved_vmctx =
            ValueLocation::from(self.push_copy_physical(ValueLocation::Reg(GPR::Rq(VMCTX)))?);

        self.save_volatile()?;
        let locs = self.pass_outgoing_args(locs.as_ref())?;

        let callee = self
            .take_reg(I64)
            .ok_or_else(|| error("Ran out of free registers"))?;

        dynasm!(self.asm
            ; mov Rq(callee.rq().unwrap()), [
                Rq(VMCTX) + self.module_context.vmctx_vmfunction_import_body(index) as i32
            ]
            ; mov Rq(VMCTX), [
                Rq(VMCTX) + self.module_context.vmctx_vmfunction_import_vmctx(index) as i32
            ]
            ; call Rq(callee.rq().unwrap())
        );

        self.free(ValueLocation::Reg(callee))?;
        self.free(ValueLocation::Reg(GPR::Rq(CALLER_VMCTX)))?;

        for i in locs {
            self.free(i)?;
        }

        self.push_function_returns(return_types)?;

        self.copy_value(saved_vmctx, CCLoc::Reg(GPR::Rq(VMCTX)))?;
        self.free(saved_vmctx)?;

        Ok(())
    }

    // TODO: Reserve space to store RBX, RBP, and R12..R15 so we can use them
    //       as scratch registers
    /// Writes the function prologue and stores the arguments as locals
    pub fn start_function<P: IntoIterator<Item = SignlessType>>(
        &mut self,
        params: P,
    ) -> Result<(), Error>
    where
        P::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        let Locs { locs, max_depth } = arg_locs_skip_caller_vmctx(params);

        self.allocated_stack = StackUsage::new(max_depth.0);
        self.set_state(CallingConvention::function_start(locs))?;

        dynasm!(self.asm
            ; push rbp
            ; mov rbp, rsp
        );

        Ok(())
    }

    pub fn ret(&mut self) {
        self.label(LabelValue::Ret);
        match self
            .labels
            .define(&LabelValue::Ret)
            .expect("BUG: Could not get label directly after defining it")
        {
            Ok(UndefinedLabel { label, align }) => {
                dynasm!(self.asm
                    ; .align align as usize
                    ;; self.asm.dynamic_label(label.0)
                    ; pop rbp
                    ; ret
                );
            }
            Err(_) => {
                dynasm!(self.asm
                    ; pop rbp
                    ; ret
                );
            }
        }
    }

    pub fn epilogue(&mut self) {
        for (value, UndefinedLabel { label, align }) in mem::take(&mut self.labels).drain() {
            match value {
                LabelValue::I32(val) => {
                    dynasm!(self.asm
                        ; .align align as usize
                        ;; self.asm.dynamic_label(label.0)
                        ; .dword val
                    );
                }
                LabelValue::I64(val) => {
                    dynasm!(self.asm
                        ; .align align as usize
                        ;; self.asm.dynamic_label(label.0)
                        ; .qword val
                    );
                }
                LabelValue::Ret => {
                    dynasm!(self.asm
                        ; .align align as usize
                        ;; self.asm.dynamic_label(label.0)
                        ;; self.ret()
                    );
                }
            }
        }
    }

    pub fn trap(&mut self, trap_id: TrapCode) {
        self.sinks.traps.trap(
            u32::try_from(self.asm.offset().0).expect("Assembly offset overflowed u32"),
            self.source_loc,
            trap_id,
        );
        dynasm!(self.asm
            ; ud2
        );
    }

    pub fn ret_label(&mut self) -> Label {
        #[derive(Copy, Clone, Hash)]
        struct RetLabel;

        self.label(LabelValue::Ret)
    }

    fn label(&mut self, label: LabelValue) -> Label {
        self.aligned_label(1, label)
    }

    fn aligned_label(&mut self, align: u32, label: LabelValue) -> Label {
        let asm = &mut self.asm;
        self.labels
            .insert(|| Label(asm.new_dynamic_label()), align, label)
    }

    fn target_to_label(&mut self, target: BrTarget<Label>) -> Label {
        match target {
            BrTarget::Label(label) => label,
            BrTarget::Return => self.ret_label(),
        }
    }
}
