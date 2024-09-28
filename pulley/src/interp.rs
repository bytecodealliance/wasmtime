//! Interpretation of pulley bytecode.

use crate::decode::*;
use crate::imms::*;
use crate::regs::*;
use crate::ExtendedOpcode;
use alloc::string::ToString;
use alloc::{vec, vec::Vec};
use core::fmt;
use core::mem;
use core::ops::{Index, IndexMut};
use core::ptr::{self, NonNull};
use sptr::Strict;

const DEFAULT_STACK_SIZE: usize = 1 << 20; // 1 MiB

/// A virtual machine for interpreting Pulley bytecode.
pub struct Vm {
    decoder: Decoder,
    state: MachineState,
}

impl Default for Vm {
    fn default() -> Self {
        Vm::new()
    }
}

impl Vm {
    /// Create a new virtual machine with the default stack size.
    pub fn new() -> Self {
        Self::with_stack(vec![0; DEFAULT_STACK_SIZE])
    }

    /// Create a new virtual machine with the given stack.
    pub fn with_stack(stack: Vec<u8>) -> Self {
        Self {
            decoder: Decoder::new(),
            state: MachineState::with_stack(stack),
        }
    }

    /// Get a shared reference to this VM's machine state.
    pub fn state(&self) -> &MachineState {
        &self.state
    }

    /// Get an exclusive reference to this VM's machine state.
    pub fn state_mut(&mut self) -> &mut MachineState {
        &mut self.state
    }

    /// Consumer this VM and return its stack storage.
    pub fn into_stack(self) -> Vec<u8> {
        self.state.stack
    }

    /// Call a bytecode function.
    ///
    /// The given `func` must point to the beginning of a valid Pulley bytecode
    /// function.
    ///
    /// The given `args` must match the number and type of arguments that
    /// function expects.
    ///
    /// The given `rets` must match the function's actual return types.
    ///
    /// Returns either the resulting values, or the PC at which a trap was
    /// raised.
    pub unsafe fn call<'a>(
        &'a mut self,
        func: NonNull<u8>,
        args: &[Val],
        rets: impl IntoIterator<Item = RegType> + 'a,
    ) -> Result<impl Iterator<Item = Val> + 'a, NonNull<u8>> {
        // NB: make sure this method stays in sync with
        // `PulleyMachineDeps::compute_arg_locs`!

        let mut x_args = (0..16).map(|x| XReg::new_unchecked(x));
        let mut f_args = (0..16).map(|f| FReg::new_unchecked(f));
        let mut v_args = (0..16).map(|v| VReg::new_unchecked(v));

        for arg in args {
            match arg {
                Val::XReg(val) => match x_args.next() {
                    Some(reg) => self.state[reg] = *val,
                    None => todo!("stack slots"),
                },
                Val::FReg(val) => match f_args.next() {
                    Some(reg) => self.state[reg] = *val,
                    None => todo!("stack slots"),
                },
                Val::VReg(val) => match v_args.next() {
                    Some(reg) => self.state[reg] = *val,
                    None => todo!("stack slots"),
                },
            }
        }

        self.run(func)?;

        let mut x_rets = (0..16).map(|x| XReg::new_unchecked(x));
        let mut f_rets = (0..16).map(|f| FReg::new_unchecked(f));
        let mut v_rets = (0..16).map(|v| VReg::new_unchecked(v));

        Ok(rets.into_iter().map(move |ty| match ty {
            RegType::XReg => match x_rets.next() {
                Some(reg) => Val::XReg(self.state[reg]),
                None => todo!("stack slots"),
            },
            RegType::FReg => match f_rets.next() {
                Some(reg) => Val::FReg(self.state[reg]),
                None => todo!("stack slots"),
            },
            RegType::VReg => match v_rets.next() {
                Some(reg) => Val::VReg(self.state[reg]),
                None => todo!("stack slots"),
            },
        }))
    }

    unsafe fn run(&mut self, pc: NonNull<u8>) -> Result<(), NonNull<u8>> {
        let mut visitor = InterpreterVisitor {
            state: &mut self.state,
            pc: UnsafeBytecodeStream::new(pc),
        };

        loop {
            let continuation = self.decoder.decode_one(&mut visitor).unwrap();

            // Really wish we had `feature(explicit_tail_calls)`...
            match continuation {
                Continuation::Continue => {
                    continue;
                }

                // Out-of-line slow paths marked `cold` and `inline(never)` to
                // improve codegen.
                Continuation::Trap => {
                    let pc = visitor.pc.as_ptr();
                    return self.trap(pc);
                }
                Continuation::ReturnToHost => return self.return_to_host(),
                Continuation::HostCall => return self.host_call(),
            }
        }
    }

    #[cold]
    #[inline(never)]
    fn return_to_host(&self) -> Result<(), NonNull<u8>> {
        Ok(())
    }

    #[cold]
    #[inline(never)]
    fn trap(&self, pc: NonNull<u8>) -> Result<(), NonNull<u8>> {
        // We are given the VM's PC upon having executed a trap instruction,
        // which is actually pointing to the next instruction after the
        // trap. Back the PC up to point exactly at the trap.
        let trap_pc = unsafe {
            NonNull::new_unchecked(pc.as_ptr().byte_sub(ExtendedOpcode::ENCODED_SIZE_OF_TRAP))
        };
        Err(trap_pc)
    }

    #[cold]
    #[inline(never)]
    fn host_call(&self) -> Result<(), NonNull<u8>> {
        todo!()
    }
}

/// The type of a register in the Pulley machine state.
#[derive(Clone, Copy, Debug)]
pub enum RegType {
    /// An `x` register: integers.
    XReg,

    /// An `f` register: floats.
    FReg,

    /// A `v` register: vectors.
    VReg,
}

/// A value that can be stored in a register.
#[derive(Clone, Copy, Debug)]
pub enum Val {
    /// An `x` register value: integers.
    XReg(XRegVal),

    /// An `f` register value: floats.
    FReg(FRegVal),

    /// A `v` register value: vectors.
    VReg(VRegVal),
}

impl fmt::LowerHex for Val {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Val::XReg(v) => fmt::LowerHex::fmt(v, f),
            Val::FReg(v) => fmt::LowerHex::fmt(v, f),
            Val::VReg(v) => fmt::LowerHex::fmt(v, f),
        }
    }
}

impl From<XRegVal> for Val {
    fn from(value: XRegVal) -> Self {
        Val::XReg(value)
    }
}

impl From<u64> for Val {
    fn from(value: u64) -> Self {
        XRegVal::new_u64(value).into()
    }
}

impl From<u32> for Val {
    fn from(value: u32) -> Self {
        XRegVal::new_u32(value).into()
    }
}

impl From<i64> for Val {
    fn from(value: i64) -> Self {
        XRegVal::new_i64(value).into()
    }
}

impl From<i32> for Val {
    fn from(value: i32) -> Self {
        XRegVal::new_i32(value).into()
    }
}

impl<T> From<*mut T> for Val {
    fn from(value: *mut T) -> Self {
        XRegVal::new_ptr(value).into()
    }
}

impl From<FRegVal> for Val {
    fn from(value: FRegVal) -> Self {
        Val::FReg(value)
    }
}

impl From<f64> for Val {
    fn from(value: f64) -> Self {
        FRegVal::new_f64(value).into()
    }
}

impl From<f32> for Val {
    fn from(value: f32) -> Self {
        FRegVal::new_f32(value).into()
    }
}

impl From<VRegVal> for Val {
    fn from(value: VRegVal) -> Self {
        Val::VReg(value)
    }
}

/// An `x` register value: integers.
#[derive(Copy, Clone)]
pub struct XRegVal(XRegUnion);

impl fmt::Debug for XRegVal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("XRegVal")
            .field("as_u64", &self.get_u64())
            .finish()
    }
}

impl fmt::LowerHex for XRegVal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerHex::fmt(&self.get_u64(), f)
    }
}

// NB: we always store these in little endian, so we have to `from_le_bytes`
// whenever we read and `to_le_bytes` whenever we store.
#[derive(Copy, Clone)]
union XRegUnion {
    i32: i32,
    u32: u32,
    i64: i64,
    u64: u64,
    ptr: *mut u8,
}

impl Default for XRegVal {
    fn default() -> Self {
        Self(unsafe { mem::zeroed() })
    }
}

#[allow(missing_docs)]
impl XRegVal {
    pub fn new_i32(x: i32) -> Self {
        let mut val = XRegVal::default();
        val.set_i32(x);
        val
    }

    pub fn new_u32(x: u32) -> Self {
        let mut val = XRegVal::default();
        val.set_u32(x);
        val
    }

    pub fn new_i64(x: i64) -> Self {
        let mut val = XRegVal::default();
        val.set_i64(x);
        val
    }

    pub fn new_u64(x: u64) -> Self {
        let mut val = XRegVal::default();
        val.set_u64(x);
        val
    }

    pub fn new_ptr<T>(ptr: *mut T) -> Self {
        let mut val = XRegVal::default();
        val.set_ptr(ptr);
        val
    }

    pub fn get_i32(&self) -> i32 {
        let x = unsafe { self.0.i32 };
        i32::from_le(x)
    }

    pub fn get_u32(&self) -> u32 {
        let x = unsafe { self.0.u32 };
        u32::from_le(x)
    }

    pub fn get_i64(&self) -> i64 {
        let x = unsafe { self.0.i64 };
        i64::from_le(x)
    }

    pub fn get_u64(&self) -> u64 {
        let x = unsafe { self.0.u64 };
        u64::from_le(x)
    }

    pub fn get_ptr<T>(&self) -> *mut T {
        let ptr = unsafe { self.0.ptr };
        Strict::map_addr(ptr, |p| usize::from_le(p)).cast()
    }

    pub fn set_i32(&mut self, x: i32) {
        self.0.i32 = x.to_le();
    }

    pub fn set_u32(&mut self, x: u32) {
        self.0.u32 = x.to_le();
    }

    pub fn set_i64(&mut self, x: i64) {
        self.0.i64 = x.to_le();
    }

    pub fn set_u64(&mut self, x: u64) {
        self.0.u64 = x.to_le();
    }

    pub fn set_ptr<T>(&mut self, ptr: *mut T) {
        self.0.ptr = Strict::map_addr(ptr, |p| p.to_le()).cast();
    }
}

/// An `f` register value: floats.
#[derive(Copy, Clone)]
pub struct FRegVal(FRegUnion);

impl fmt::Debug for FRegVal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FRegVal")
            .field("as_f32", &self.get_f32())
            .field("as_f64", &self.get_f64())
            .finish()
    }
}

impl fmt::LowerHex for FRegVal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerHex::fmt(&self.get_f64().to_bits(), f)
    }
}

// NB: we always store these in little endian, so we have to `from_le_bytes`
// whenever we read and `to_le_bytes` whenever we store.
#[derive(Copy, Clone)]
union FRegUnion {
    f32: u32,
    f64: u64,
}

impl Default for FRegVal {
    fn default() -> Self {
        Self(unsafe { mem::zeroed() })
    }
}

#[allow(missing_docs)]
impl FRegVal {
    pub fn new_f32(f: f32) -> Self {
        let mut val = Self::default();
        val.set_f32(f);
        val
    }

    pub fn new_f64(f: f64) -> Self {
        let mut val = Self::default();
        val.set_f64(f);
        val
    }

    pub fn get_f32(&self) -> f32 {
        let val = unsafe { self.0.f32 };
        f32::from_le_bytes(val.to_ne_bytes())
    }

    pub fn get_f64(&self) -> f64 {
        let val = unsafe { self.0.f64 };
        f64::from_le_bytes(val.to_ne_bytes())
    }

    pub fn set_f32(&mut self, val: f32) {
        self.0.f32 = u32::from_ne_bytes(val.to_le_bytes());
    }

    pub fn set_f64(&mut self, val: f64) {
        self.0.f64 = u64::from_ne_bytes(val.to_le_bytes());
    }
}

/// A `v` register value: vectors.
#[derive(Copy, Clone)]
pub struct VRegVal(VRegUnion);

impl fmt::Debug for VRegVal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VRegVal")
            .field("as_u128", &unsafe { self.0.u128 })
            .finish()
    }
}

impl fmt::LowerHex for VRegVal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerHex::fmt(unsafe { &self.0.u128 }, f)
    }
}

#[derive(Copy, Clone)]
union VRegUnion {
    // TODO: need to figure out how we are going to handle portability of lane
    // ordering on top of each lane's endianness.
    u128: u128,
}

impl Default for VRegVal {
    fn default() -> Self {
        Self(unsafe { mem::zeroed() })
    }
}

/// The machine state for a Pulley virtual machine: the various registers and
/// stack.
pub struct MachineState {
    x_regs: [XRegVal; XReg::RANGE.end as usize],
    f_regs: [FRegVal; FReg::RANGE.end as usize],
    v_regs: [VRegVal; VReg::RANGE.end as usize],
    stack: Vec<u8>,
}

unsafe impl Send for MachineState {}
unsafe impl Sync for MachineState {}

impl fmt::Debug for MachineState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let MachineState {
            x_regs,
            f_regs,
            v_regs,
            stack: _,
        } = self;

        struct RegMap<'a, R>(&'a [R], fn(u8) -> alloc::string::String);

        impl<R: fmt::Debug> fmt::Debug for RegMap<'_, R> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let mut f = f.debug_map();
                for (i, r) in self.0.iter().enumerate() {
                    f.entry(&(self.1)(i as u8), r);
                }
                f.finish()
            }
        }

        f.debug_struct("MachineState")
            .field(
                "x_regs",
                &RegMap(x_regs, |i| XReg::new(i).unwrap().to_string()),
            )
            .field(
                "f_regs",
                &RegMap(f_regs, |i| FReg::new(i).unwrap().to_string()),
            )
            .field(
                "v_regs",
                &RegMap(v_regs, |i| VReg::new(i).unwrap().to_string()),
            )
            .finish_non_exhaustive()
    }
}

macro_rules! index_reg {
    ($reg_ty:ty,$value_ty:ty,$field:ident) => {
        impl Index<$reg_ty> for MachineState {
            type Output = $value_ty;

            fn index(&self, reg: $reg_ty) -> &Self::Output {
                &self.$field[reg.index()]
            }
        }

        impl IndexMut<$reg_ty> for MachineState {
            fn index_mut(&mut self, reg: $reg_ty) -> &mut Self::Output {
                &mut self.$field[reg.index()]
            }
        }
    };
}

index_reg!(XReg, XRegVal, x_regs);
index_reg!(FReg, FRegVal, f_regs);
index_reg!(VReg, VRegVal, v_regs);

impl MachineState {
    fn with_stack(stack: Vec<u8>) -> Self {
        assert!(stack.len() > 0);
        let mut state = Self {
            x_regs: [Default::default(); XReg::RANGE.end as usize],
            f_regs: Default::default(),
            v_regs: Default::default(),
            stack,
        };

        // Take care to construct SP such that we preserve pointer provenance
        // for the whole stack.
        let len = state.stack.len();
        let sp = &mut state.stack[..];
        let sp = sp.as_mut_ptr();
        let sp = unsafe { sp.add(len) };
        state[XReg::sp] = XRegVal::new_ptr(sp);
        state[XReg::fp] = XRegVal::new_i64(-1);
        state[XReg::lr] = XRegVal::new_i64(-1);

        state
    }

    /// `*sp = val; sp += size_of::<T>()`
    fn push<T>(&mut self, val: T) {
        let sp = self[XReg::sp].get_ptr::<T>();
        unsafe { sp.write_unaligned(val) }
        self[XReg::sp].set_ptr(sp.wrapping_add(1));
    }

    /// `ret = *sp; sp -= size_of::<T>()`
    fn pop<T>(&mut self) -> T {
        let sp = self[XReg::sp].get_ptr::<T>();
        let val = unsafe { sp.read_unaligned() };
        self[XReg::sp].set_ptr(sp.wrapping_sub(1));
        val
    }
}

enum Continuation {
    Continue,
    ReturnToHost,
    Trap,

    #[allow(dead_code)]
    HostCall,
}

struct InterpreterVisitor<'a> {
    state: &'a mut MachineState,
    pc: UnsafeBytecodeStream,
}

impl InterpreterVisitor<'_> {
    #[inline(always)]
    fn pc_rel_jump(&mut self, offset: PcRelOffset, inst_size: isize) -> Continuation {
        let offset = isize::try_from(i32::from(offset)).unwrap();
        self.pc = unsafe { self.pc.offset(offset - inst_size) };
        Continuation::Continue
    }
}

#[doc(hidden)]
impl OpVisitor for InterpreterVisitor<'_> {
    type BytecodeStream = UnsafeBytecodeStream;

    fn bytecode(&mut self) -> &mut Self::BytecodeStream {
        &mut self.pc
    }

    type Return = Continuation;

    fn ret(&mut self) -> Self::Return {
        if self.state[XReg::lr].get_u64() == u64::MAX {
            Continuation::ReturnToHost
        } else {
            let return_addr = self.state[XReg::lr].get_ptr();
            self.pc = unsafe { UnsafeBytecodeStream::new(NonNull::new_unchecked(return_addr)) };
            // log::trace!("returning to {return_addr:#p}");
            Continuation::Continue
        }
    }

    fn call(&mut self, offset: PcRelOffset) -> Self::Return {
        let return_addr = self.pc.as_ptr();
        self.state[XReg::lr].set_ptr(return_addr.as_ptr());
        self.pc_rel_jump(offset, 5)
    }

    fn jump(&mut self, offset: PcRelOffset) -> Self::Return {
        self.pc_rel_jump(offset, 5)
    }

    fn br_if(&mut self, cond: XReg, offset: PcRelOffset) -> Self::Return {
        let cond = self.state[cond].get_u64();
        if cond != 0 {
            self.pc_rel_jump(offset, 6)
        } else {
            Continuation::Continue
        }
    }

    fn br_if_not(&mut self, cond: XReg, offset: PcRelOffset) -> Self::Return {
        let cond = self.state[cond].get_u64();
        if cond == 0 {
            self.pc_rel_jump(offset, 6)
        } else {
            Continuation::Continue
        }
    }

    fn br_if_xeq32(&mut self, a: XReg, b: XReg, offset: PcRelOffset) -> Self::Return {
        let a = self.state[a].get_u32();
        let b = self.state[b].get_u32();
        if a == b {
            self.pc_rel_jump(offset, 7)
        } else {
            Continuation::Continue
        }
    }

    fn br_if_xneq32(&mut self, a: XReg, b: XReg, offset: PcRelOffset) -> Self::Return {
        let a = self.state[a].get_u32();
        let b = self.state[b].get_u32();
        if a != b {
            self.pc_rel_jump(offset, 7)
        } else {
            Continuation::Continue
        }
    }

    fn br_if_xslt32(&mut self, a: XReg, b: XReg, offset: PcRelOffset) -> Self::Return {
        let a = self.state[a].get_i32();
        let b = self.state[b].get_i32();
        if a < b {
            self.pc_rel_jump(offset, 7)
        } else {
            Continuation::Continue
        }
    }

    fn br_if_xslteq32(&mut self, a: XReg, b: XReg, offset: PcRelOffset) -> Self::Return {
        let a = self.state[a].get_i32();
        let b = self.state[b].get_i32();
        if a <= b {
            self.pc_rel_jump(offset, 7)
        } else {
            Continuation::Continue
        }
    }

    fn br_if_xult32(&mut self, a: XReg, b: XReg, offset: PcRelOffset) -> Self::Return {
        let a = self.state[a].get_u32();
        let b = self.state[b].get_u32();
        if a < b {
            self.pc_rel_jump(offset, 7)
        } else {
            Continuation::Continue
        }
    }

    fn br_if_xulteq32(&mut self, a: XReg, b: XReg, offset: PcRelOffset) -> Self::Return {
        let a = self.state[a].get_u32();
        let b = self.state[b].get_u32();
        if a <= b {
            self.pc_rel_jump(offset, 7)
        } else {
            Continuation::Continue
        }
    }

    fn br_if_xeq64(&mut self, a: XReg, b: XReg, offset: PcRelOffset) -> Self::Return {
        let a = self.state[a].get_u64();
        let b = self.state[b].get_u64();
        if a == b {
            self.pc_rel_jump(offset, 7)
        } else {
            Continuation::Continue
        }
    }

    fn br_if_xneq64(&mut self, a: XReg, b: XReg, offset: PcRelOffset) -> Self::Return {
        let a = self.state[a].get_u64();
        let b = self.state[b].get_u64();
        if a != b {
            self.pc_rel_jump(offset, 7)
        } else {
            Continuation::Continue
        }
    }

    fn br_if_xslt64(&mut self, a: XReg, b: XReg, offset: PcRelOffset) -> Self::Return {
        let a = self.state[a].get_i64();
        let b = self.state[b].get_i64();
        if a < b {
            self.pc_rel_jump(offset, 7)
        } else {
            Continuation::Continue
        }
    }

    fn br_if_xslteq64(&mut self, a: XReg, b: XReg, offset: PcRelOffset) -> Self::Return {
        let a = self.state[a].get_i64();
        let b = self.state[b].get_i64();
        if a <= b {
            self.pc_rel_jump(offset, 7)
        } else {
            Continuation::Continue
        }
    }

    fn br_if_xult64(&mut self, a: XReg, b: XReg, offset: PcRelOffset) -> Self::Return {
        let a = self.state[a].get_u64();
        let b = self.state[b].get_u64();
        if a < b {
            self.pc_rel_jump(offset, 7)
        } else {
            Continuation::Continue
        }
    }

    fn br_if_xulteq64(&mut self, a: XReg, b: XReg, offset: PcRelOffset) -> Self::Return {
        let a = self.state[a].get_u64();
        let b = self.state[b].get_u64();
        if a <= b {
            self.pc_rel_jump(offset, 7)
        } else {
            Continuation::Continue
        }
    }

    fn xmov(&mut self, dst: XReg, src: XReg) -> Self::Return {
        let val = self.state[src];
        self.state[dst] = val;
        Continuation::Continue
    }

    fn fmov(&mut self, dst: FReg, src: FReg) -> Self::Return {
        let val = self.state[src];
        self.state[dst] = val;
        Continuation::Continue
    }

    fn vmov(&mut self, dst: VReg, src: VReg) -> Self::Return {
        let val = self.state[src];
        self.state[dst] = val;
        Continuation::Continue
    }

    fn xconst8(&mut self, dst: XReg, imm: i8) -> Self::Return {
        self.state[dst].set_i64(i64::from(imm));
        Continuation::Continue
    }

    fn xconst16(&mut self, dst: XReg, imm: i16) -> Self::Return {
        self.state[dst].set_i64(i64::from(imm));
        Continuation::Continue
    }

    fn xconst32(&mut self, dst: XReg, imm: i32) -> Self::Return {
        self.state[dst].set_i64(i64::from(imm));
        Continuation::Continue
    }

    fn xconst64(&mut self, dst: XReg, imm: i64) -> Self::Return {
        self.state[dst].set_i64(imm);
        Continuation::Continue
    }

    fn xadd32(&mut self, operands: BinaryOperands<XReg>) -> Self::Return {
        let a = self.state[operands.src1].get_u32();
        let b = self.state[operands.src2].get_u32();
        self.state[operands.dst].set_u32(a.wrapping_add(b));
        Continuation::Continue
    }

    fn xadd64(&mut self, operands: BinaryOperands<XReg>) -> Self::Return {
        let a = self.state[operands.src1].get_u64();
        let b = self.state[operands.src2].get_u64();
        self.state[operands.dst].set_u64(a.wrapping_add(b));
        Continuation::Continue
    }

    fn xeq64(&mut self, operands: BinaryOperands<XReg>) -> Self::Return {
        let a = self.state[operands.src1].get_u64();
        let b = self.state[operands.src2].get_u64();
        self.state[operands.dst].set_u64(u64::from(a == b));
        Continuation::Continue
    }

    fn xneq64(&mut self, operands: BinaryOperands<XReg>) -> Self::Return {
        let a = self.state[operands.src1].get_u64();
        let b = self.state[operands.src2].get_u64();
        self.state[operands.dst].set_u64(u64::from(a != b));
        Continuation::Continue
    }

    fn xslt64(&mut self, operands: BinaryOperands<XReg>) -> Self::Return {
        let a = self.state[operands.src1].get_i64();
        let b = self.state[operands.src2].get_i64();
        self.state[operands.dst].set_u64(u64::from(a < b));
        Continuation::Continue
    }

    fn xslteq64(&mut self, operands: BinaryOperands<XReg>) -> Self::Return {
        let a = self.state[operands.src1].get_i64();
        let b = self.state[operands.src2].get_i64();
        self.state[operands.dst].set_u64(u64::from(a <= b));
        Continuation::Continue
    }

    fn xult64(&mut self, operands: BinaryOperands<XReg>) -> Self::Return {
        let a = self.state[operands.src1].get_u64();
        let b = self.state[operands.src2].get_u64();
        self.state[operands.dst].set_u64(u64::from(a < b));
        Continuation::Continue
    }

    fn xulteq64(&mut self, operands: BinaryOperands<XReg>) -> Self::Return {
        let a = self.state[operands.src1].get_u64();
        let b = self.state[operands.src2].get_u64();
        self.state[operands.dst].set_u64(u64::from(a <= b));
        Continuation::Continue
    }

    fn xeq32(&mut self, operands: BinaryOperands<XReg>) -> Self::Return {
        let a = self.state[operands.src1].get_u32();
        let b = self.state[operands.src2].get_u32();
        self.state[operands.dst].set_u64(u64::from(a == b));
        Continuation::Continue
    }

    fn xneq32(&mut self, operands: BinaryOperands<XReg>) -> Self::Return {
        let a = self.state[operands.src1].get_u32();
        let b = self.state[operands.src2].get_u32();
        self.state[operands.dst].set_u64(u64::from(a != b));
        Continuation::Continue
    }

    fn xslt32(&mut self, operands: BinaryOperands<XReg>) -> Self::Return {
        let a = self.state[operands.src1].get_i32();
        let b = self.state[operands.src2].get_i32();
        self.state[operands.dst].set_u64(u64::from(a < b));
        Continuation::Continue
    }

    fn xslteq32(&mut self, operands: BinaryOperands<XReg>) -> Self::Return {
        let a = self.state[operands.src1].get_i32();
        let b = self.state[operands.src2].get_i32();
        self.state[operands.dst].set_u64(u64::from(a <= b));
        Continuation::Continue
    }

    fn xult32(&mut self, operands: BinaryOperands<XReg>) -> Self::Return {
        let a = self.state[operands.src1].get_u32();
        let b = self.state[operands.src2].get_u32();
        self.state[operands.dst].set_u64(u64::from(a < b));
        Continuation::Continue
    }

    fn xulteq32(&mut self, operands: BinaryOperands<XReg>) -> Self::Return {
        let a = self.state[operands.src1].get_u32();
        let b = self.state[operands.src2].get_u32();
        self.state[operands.dst].set_u64(u64::from(a <= b));
        Continuation::Continue
    }

    fn load32_u(&mut self, dst: XReg, ptr: XReg) -> Self::Return {
        let ptr = self.state[ptr].get_ptr::<u32>();
        let val = unsafe { ptr::read_unaligned(ptr) };
        self.state[dst].set_u64(u64::from(val));
        Continuation::Continue
    }

    fn load32_s(&mut self, dst: XReg, ptr: XReg) -> Self::Return {
        let ptr = self.state[ptr].get_ptr::<i32>();
        let val = unsafe { ptr::read_unaligned(ptr) };
        self.state[dst].set_i64(i64::from(val));
        Continuation::Continue
    }

    fn load64(&mut self, dst: XReg, ptr: XReg) -> Self::Return {
        let ptr = self.state[ptr].get_ptr::<u64>();
        let val = unsafe { ptr::read_unaligned(ptr) };
        self.state[dst].set_u64(val);
        Continuation::Continue
    }

    fn load32_u_offset8(&mut self, dst: XReg, ptr: XReg, offset: i8) -> Self::Return {
        let val = unsafe {
            self.state[ptr]
                .get_ptr::<u32>()
                .byte_offset(offset.into())
                .read_unaligned()
        };
        self.state[dst].set_u64(u64::from(val));
        Continuation::Continue
    }

    fn load32_s_offset8(&mut self, dst: XReg, ptr: XReg, offset: i8) -> Self::Return {
        let val = unsafe {
            self.state[ptr]
                .get_ptr::<i32>()
                .byte_offset(offset.into())
                .read_unaligned()
        };
        self.state[dst].set_i64(i64::from(val));
        Continuation::Continue
    }

    fn load32_u_offset64(&mut self, dst: XReg, ptr: XReg, offset: i64) -> Self::Return {
        let val = unsafe {
            self.state[ptr]
                .get_ptr::<u32>()
                .byte_offset(offset as isize)
                .read_unaligned()
        };
        self.state[dst].set_u64(u64::from(val));
        Continuation::Continue
    }

    fn load32_s_offset64(&mut self, dst: XReg, ptr: XReg, offset: i64) -> Self::Return {
        let val = unsafe {
            self.state[ptr]
                .get_ptr::<i32>()
                .byte_offset(offset as isize)
                .read_unaligned()
        };
        self.state[dst].set_i64(i64::from(val));
        Continuation::Continue
    }

    fn load64_offset8(&mut self, dst: XReg, ptr: XReg, offset: i8) -> Self::Return {
        let val = unsafe {
            self.state[ptr]
                .get_ptr::<u64>()
                .byte_offset(offset.into())
                .read_unaligned()
        };
        self.state[dst].set_u64(val);
        Continuation::Continue
    }

    fn load64_offset64(&mut self, dst: XReg, ptr: XReg, offset: i64) -> Self::Return {
        let val = unsafe {
            self.state[ptr]
                .get_ptr::<u64>()
                .byte_offset(offset as isize)
                .read_unaligned()
        };
        self.state[dst].set_u64(val);
        Continuation::Continue
    }

    fn store32(&mut self, ptr: XReg, src: XReg) -> Self::Return {
        let ptr = self.state[ptr].get_ptr::<u32>();
        let val = self.state[src].get_u32();
        unsafe {
            ptr::write_unaligned(ptr, val);
        }
        Continuation::Continue
    }

    fn store64(&mut self, ptr: XReg, src: XReg) -> Self::Return {
        let ptr = self.state[ptr].get_ptr::<u64>();
        let val = self.state[src].get_u64();
        unsafe {
            ptr::write_unaligned(ptr, val);
        }
        Continuation::Continue
    }

    fn store32_offset8(&mut self, ptr: XReg, offset: i8, src: XReg) -> Self::Return {
        let val = self.state[src].get_u32();
        unsafe {
            self.state[ptr]
                .get_ptr::<u32>()
                .byte_offset(offset.into())
                .write_unaligned(val);
        }
        Continuation::Continue
    }

    fn store64_offset8(&mut self, ptr: XReg, offset: i8, src: XReg) -> Self::Return {
        let val = self.state[src].get_u64();
        unsafe {
            self.state[ptr]
                .get_ptr::<u64>()
                .byte_offset(offset.into())
                .write_unaligned(val);
        }
        Continuation::Continue
    }

    fn store32_offset64(&mut self, ptr: XReg, offset: i64, src: XReg) -> Self::Return {
        let val = self.state[src].get_u32();
        unsafe {
            self.state[ptr]
                .get_ptr::<u32>()
                .byte_offset(offset as isize)
                .write_unaligned(val);
        }
        Continuation::Continue
    }

    fn store64_offset64(&mut self, ptr: XReg, offset: i64, src: XReg) -> Self::Return {
        let val = self.state[src].get_u64();
        unsafe {
            self.state[ptr]
                .get_ptr::<u64>()
                .byte_offset(offset as isize)
                .write_unaligned(val);
        }
        Continuation::Continue
    }

    fn xpush32(&mut self, src: XReg) -> Self::Return {
        self.state.push(self.state[src].get_u32());
        Continuation::Continue
    }

    fn xpush32_many(&mut self, srcs: RegSet<XReg>) -> Self::Return {
        for src in srcs {
            self.xpush32(src);
        }
        Continuation::Continue
    }

    fn xpush64(&mut self, src: XReg) -> Self::Return {
        self.state.push(self.state[src].get_u64());
        Continuation::Continue
    }

    fn xpush64_many(&mut self, srcs: RegSet<XReg>) -> Self::Return {
        for src in srcs {
            self.xpush64(src);
        }
        Continuation::Continue
    }

    fn xpop32(&mut self, dst: XReg) -> Self::Return {
        let val = self.state.pop();
        self.state[dst].set_u32(val);
        Continuation::Continue
    }

    fn xpop32_many(&mut self, dsts: RegSet<XReg>) -> Self::Return {
        for dst in dsts.into_iter().rev() {
            self.xpop32(dst);
        }
        Continuation::Continue
    }

    fn xpop64(&mut self, dst: XReg) -> Self::Return {
        let val = self.state.pop();
        self.state[dst].set_u64(val);
        Continuation::Continue
    }

    fn xpop64_many(&mut self, dsts: RegSet<XReg>) -> Self::Return {
        for dst in dsts.into_iter().rev() {
            self.xpop64(dst);
        }
        Continuation::Continue
    }

    /// `push lr; push fp; fp = sp`
    fn push_frame(&mut self) -> Self::Return {
        self.state.push(self.state[XReg::lr].get_ptr::<u8>());
        self.state.push(self.state[XReg::fp].get_ptr::<u8>());
        self.state[XReg::fp] = self.state[XReg::sp];
        Continuation::Continue
    }

    /// `sp = fp; pop fp; pop lr`
    fn pop_frame(&mut self) -> Self::Return {
        self.state[XReg::sp] = self.state[XReg::fp];
        let fp = self.state.pop();
        let lr = self.state.pop();
        self.state[XReg::fp].set_ptr::<u8>(fp);
        self.state[XReg::lr].set_ptr::<u8>(lr);
        Continuation::Continue
    }

    fn bitcast_int_from_float_32(&mut self, dst: XReg, src: FReg) -> Self::Return {
        let val = self.state[src].get_f32();
        self.state[dst].set_u64(u32::from_ne_bytes(val.to_ne_bytes()).into());
        Continuation::Continue
    }

    fn bitcast_int_from_float_64(&mut self, dst: XReg, src: FReg) -> Self::Return {
        let val = self.state[src].get_f64();
        self.state[dst].set_u64(u64::from_ne_bytes(val.to_ne_bytes()));
        Continuation::Continue
    }

    fn bitcast_float_from_int_32(&mut self, dst: FReg, src: XReg) -> Self::Return {
        let val = self.state[src].get_u32();
        self.state[dst].set_f32(f32::from_ne_bytes(val.to_ne_bytes()));
        Continuation::Continue
    }

    fn bitcast_float_from_int_64(&mut self, dst: FReg, src: XReg) -> Self::Return {
        let val = self.state[src].get_u64();
        self.state[dst].set_f64(f64::from_ne_bytes(val.to_ne_bytes()));
        Continuation::Continue
    }
}

impl ExtendedOpVisitor for InterpreterVisitor<'_> {
    fn nop(&mut self) -> Self::Return {
        Continuation::Continue
    }

    fn trap(&mut self) -> Self::Return {
        Continuation::Trap
    }

    fn get_sp(&mut self, dst: XReg) -> Self::Return {
        let sp = self.state[XReg::sp].get_u64();
        self.state[dst].set_u64(sp);
        Continuation::Continue
    }
}
