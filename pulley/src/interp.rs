//! Interpretation of pulley bytecode.

use crate::decode::*;
use crate::encode::Encode;
use crate::imms::*;
use crate::regs::*;
use alloc::string::ToString;
use alloc::{vec, vec::Vec};
use core::fmt;
use core::mem;
use core::ops::ControlFlow;
use core::ops::{Index, IndexMut};
use core::ptr::NonNull;
use sptr::Strict;
use wasmtime_math::WasmFloat;

mod debug;
#[cfg(all(not(pulley_tail_calls), not(pulley_assume_llvm_makes_tail_calls)))]
mod match_loop;
#[cfg(any(pulley_tail_calls, pulley_assume_llvm_makes_tail_calls))]
mod tail_loop;

const DEFAULT_STACK_SIZE: usize = 1 << 20; // 1 MiB

/// A virtual machine for interpreting Pulley bytecode.
pub struct Vm {
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
    ) -> DoneReason<impl Iterator<Item = Val> + 'a> {
        self.call_start(args);

        match self.call_run(func) {
            DoneReason::ReturnToHost(()) => DoneReason::ReturnToHost(self.call_end(rets)),
            DoneReason::Trap { pc, kind } => DoneReason::Trap { pc, kind },
            DoneReason::CallIndirectHost { id, resume } => {
                DoneReason::CallIndirectHost { id, resume }
            }
        }
    }

    /// Peforms the initial part of [`Vm::call`] in setting up the `args`
    /// provided in registers according to Pulley's ABI.
    ///
    /// # Unsafety
    ///
    /// All the same unsafety as `call` and additiionally, you must
    /// invoke `call_run` and then `call_end` after calling `call_start`.
    /// If you don't want to wrangle these invocations, use `call` instead
    /// of `call_{start,run,end}`.
    pub unsafe fn call_start<'a>(&'a mut self, args: &[Val]) {
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
    }

    /// Peforms the internal part of [`Vm::call`] where bytecode is actually
    /// executed.
    ///
    /// # Unsafety
    ///
    /// In addition to all the invariants documented for `call`, you
    /// may only invoke `call_run` after invoking `call_start` to
    /// initialize this call's arguments.
    pub unsafe fn call_run(&mut self, pc: NonNull<u8>) -> DoneReason<()> {
        self.state.debug_assert_done_reason_none();
        let interpreter = Interpreter {
            state: &mut self.state,
            pc: UnsafeBytecodeStream::new(pc),
        };
        let done = interpreter.run();
        self.state.done_decode(done)
    }

    /// Peforms the tail end of [`Vm::call`] by returning the values as
    /// determined by `rets` according to Pulley's ABI.
    ///
    /// # Unsafety
    ///
    /// In addition to the invariants documented for `call`, this may
    /// only be called after `call_run`.
    pub unsafe fn call_end<'a>(
        &'a mut self,
        rets: impl IntoIterator<Item = RegType> + 'a,
    ) -> impl Iterator<Item = Val> + 'a {
        // NB: make sure this method stays in sync with
        // `PulleyMachineDeps::compute_arg_locs`!

        let mut x_rets = (0..16).map(|x| XReg::new_unchecked(x));
        let mut f_rets = (0..16).map(|f| FReg::new_unchecked(f));
        let mut v_rets = (0..16).map(|v| VReg::new_unchecked(v));

        rets.into_iter().map(move |ty| match ty {
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
        })
    }

    /// Returns the current `fp` register value.
    pub fn fp(&self) -> *mut u8 {
        self.state.fp
    }

    /// Returns the current `lr` register value.
    pub fn lr(&self) -> *mut u8 {
        self.state.lr
    }

    /// Sets the current `fp` register value.
    pub unsafe fn set_fp(&mut self, fp: *mut u8) {
        self.state.fp = fp;
    }

    /// Sets the current `lr` register value.
    pub unsafe fn set_lr(&mut self, lr: *mut u8) {
        self.state.lr = lr;
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

impl PartialEq for XRegVal {
    fn eq(&self, other: &Self) -> bool {
        self.get_u64() == other.get_u64()
    }
}

impl Eq for XRegVal {}

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

/// Contents of an "x" register, or a general-purpose register.
///
/// This is represented as a Rust `union` to make it easier to access typed
/// views of this, notably the `ptr` field which enables preserving a bit of
/// provenance for Rust for values stored as a pointer and read as a pointer.
///
/// Note that the actual in-memory representation of this value is handled
/// carefully at this time. Pulley bytecode exposes the ability to store a
/// 32-bit result into a register and then read the 64-bit contents of the
/// register. This leaves us with the question of what to do with the upper bits
/// of the register when the 32-bit result is generated. Possibilities for
/// handling this are:
///
/// 1. Do nothing, just store the 32-bit value. The problem with this approach
///    means that the "upper bits" are now endianness-dependent. That means that
///    the state of the register is now platform-dependent.
/// 2. Sign or zero-extend. This restores platform-independent behavior but
///    requires an extra store on 32-bit platforms because they can probably
///    only store 32-bits at a time.
/// 3. Always store the values in this union as little-endian. This means that
///    big-endian platforms have to do a byte-swap but otherwise it has
///    platform-independent behavior.
///
/// This union chooses route (3) at this time where the values here are always
/// stored in little-endian form (even the `ptr` field). That guarantees
/// cross-platform behavior while also minimizing the amount of data stored on
/// writes.
///
/// In the future we may wish to benchmark this and possibly change this.
/// Technically Cranelift-generated bytecode should never rely on the upper bits
/// of a register if it didn't previously write them so this in theory doesn't
/// actually matter for Cranelift or wasm semantics. The only cost right now is
/// to big-endian platforms though and it's not certain how crucial performance
/// will be there.
///
/// One final note is that this notably contrasts with native CPUs where
/// native ISAs like RISC-V specifically define the entire register on every
/// instruction, even if only the low half contains a significant result. Pulley
/// is unlikely to become out-of-order within the CPU itself as it's interpreted
/// meaning that severing data-dependencies with previous operations is
/// hypothesized to not be too important. If this is ever a problem though it
/// could increase the likelihood we go for route (2) above instead (or maybe
/// even (1)).
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

// NB: like `XRegUnion` values here are always little-endian, see the
// documentation above for more details.
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

/// 128-bit vector registers.
///
/// This register is always stored in little-endian order and has different
/// constraints than `XRegVal` and `FRegVal` above. Notably all fields of this
/// union are the same width so all bits are always defined. Note that
/// little-endian is required though so bitcasts between different shapes of
/// vectors works. This union cannot be stored in big-endian.
#[derive(Copy, Clone)]
#[repr(align(16))]
union VRegUnion {
    u128: u128,
    i8x16: [i8; 16],
    i16x8: [i16; 8],
    i32x4: [i32; 4],
    i64x2: [i64; 2],
    u8x16: [u8; 16],
    u16x8: [u16; 8],
    u32x4: [u32; 4],
    u64x2: [u64; 2],
    // Note that these are `u32` and `u64`, not f32/f64. That's only because
    // f32/f64 don't have `.to_le()` and `::from_le()` so need to go through the
    // bits anyway.
    f32x4: [u32; 4],
    f64x2: [u64; 2],
}

impl Default for VRegVal {
    fn default() -> Self {
        Self(unsafe { mem::zeroed() })
    }
}

#[allow(missing_docs)]
impl VRegVal {
    pub fn new_u128(i: u128) -> Self {
        let mut val = Self::default();
        val.set_u128(i);
        val
    }

    pub fn get_u128(&self) -> u128 {
        let val = unsafe { self.0.u128 };
        u128::from_le(val)
    }

    pub fn set_u128(&mut self, val: u128) {
        self.0.u128 = val.to_le();
    }

    fn get_i8x16(&self) -> [i8; 16] {
        let val = unsafe { self.0.i8x16 };
        val.map(|e| i8::from_le(e))
    }

    fn set_i8x16(&mut self, val: [i8; 16]) {
        self.0.i8x16 = val.map(|e| e.to_le());
    }

    fn get_u8x16(&self) -> [u8; 16] {
        let val = unsafe { self.0.u8x16 };
        val.map(|e| u8::from_le(e))
    }

    fn set_u8x16(&mut self, val: [u8; 16]) {
        self.0.u8x16 = val.map(|e| e.to_le());
    }

    fn get_i16x8(&self) -> [i16; 8] {
        let val = unsafe { self.0.i16x8 };
        val.map(|e| i16::from_le(e))
    }

    fn set_i16x8(&mut self, val: [i16; 8]) {
        self.0.i16x8 = val.map(|e| e.to_le());
    }

    fn get_u16x8(&self) -> [u16; 8] {
        let val = unsafe { self.0.u16x8 };
        val.map(|e| u16::from_le(e))
    }

    fn set_u16x8(&mut self, val: [u16; 8]) {
        self.0.u16x8 = val.map(|e| e.to_le());
    }

    fn get_i32x4(&self) -> [i32; 4] {
        let val = unsafe { self.0.i32x4 };
        val.map(|e| i32::from_le(e))
    }

    fn set_i32x4(&mut self, val: [i32; 4]) {
        self.0.i32x4 = val.map(|e| e.to_le());
    }

    fn get_u32x4(&self) -> [u32; 4] {
        let val = unsafe { self.0.u32x4 };
        val.map(|e| u32::from_le(e))
    }

    fn set_u32x4(&mut self, val: [u32; 4]) {
        self.0.u32x4 = val.map(|e| e.to_le());
    }

    fn get_i64x2(&self) -> [i64; 2] {
        let val = unsafe { self.0.i64x2 };
        val.map(|e| i64::from_le(e))
    }

    fn set_i64x2(&mut self, val: [i64; 2]) {
        self.0.i64x2 = val.map(|e| e.to_le());
    }

    fn get_u64x2(&self) -> [u64; 2] {
        let val = unsafe { self.0.u64x2 };
        val.map(|e| u64::from_le(e))
    }

    fn set_u64x2(&mut self, val: [u64; 2]) {
        self.0.u64x2 = val.map(|e| e.to_le());
    }

    fn get_f64x2(&self) -> [f64; 2] {
        let val = unsafe { self.0.f64x2 };
        val.map(|e| f64::from_bits(u64::from_le(e)))
    }

    fn set_f64x2(&mut self, val: [f64; 2]) {
        self.0.f64x2 = val.map(|e| e.to_bits().to_le());
    }

    fn get_f32x4(&self) -> [f32; 4] {
        let val = unsafe { self.0.f32x4 };
        val.map(|e| f32::from_bits(u32::from_le(e)))
    }

    fn set_f32x4(&mut self, val: [f32; 4]) {
        self.0.f32x4 = val.map(|e| e.to_bits().to_le());
    }
}

/// The machine state for a Pulley virtual machine: the various registers and
/// stack.
pub struct MachineState {
    x_regs: [XRegVal; XReg::RANGE.end as usize],
    f_regs: [FRegVal; FReg::RANGE.end as usize],
    v_regs: [VRegVal; VReg::RANGE.end as usize],
    fp: *mut u8,
    lr: *mut u8,
    stack: Vec<u8>,
    done_reason: Option<DoneReason<()>>,
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
            done_reason: _,
            fp: _,
            lr: _,
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
        impl Index<$reg_ty> for Vm {
            type Output = $value_ty;

            fn index(&self, reg: $reg_ty) -> &Self::Output {
                &self.state[reg]
            }
        }

        impl IndexMut<$reg_ty> for Vm {
            fn index_mut(&mut self, reg: $reg_ty) -> &mut Self::Output {
                &mut self.state[reg]
            }
        }

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

/// Sentinel return address that signals the end of the call stack.
const HOST_RETURN_ADDR: *mut u8 = usize::MAX as *mut u8;

impl MachineState {
    fn with_stack(stack: Vec<u8>) -> Self {
        assert!(stack.len() > 0);
        let mut state = Self {
            x_regs: [Default::default(); XReg::RANGE.end as usize],
            f_regs: Default::default(),
            v_regs: Default::default(),
            stack,
            done_reason: None,
            fp: HOST_RETURN_ADDR,
            lr: HOST_RETURN_ADDR,
        };

        // Take care to construct SP such that we preserve pointer provenance
        // for the whole stack.
        let len = state.stack.len();
        let sp = &mut state.stack[..];
        let sp = sp.as_mut_ptr();
        let sp = unsafe { sp.add(len) };
        state[XReg::sp] = XRegVal::new_ptr(sp);

        state
    }
}

/// Inner private module to prevent creation of the `Done` structure outside of
/// this module.
mod done {
    use super::{Encode, Interpreter, MachineState};
    use core::ops::ControlFlow;
    use core::ptr::NonNull;

    /// Zero-sized sentinel indicating that pulley execution has halted.
    ///
    /// The reason for halting is stored in `MachineState`.
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct Done {
        _priv: (),
    }

    /// Reason that the pulley interpreter has ceased execution.
    pub enum DoneReason<T> {
        /// A trap happened at this bytecode instruction.
        Trap {
            /// Which instruction is raising this trap.
            pc: NonNull<u8>,
            /// The kind of trap being raised, if known.
            kind: Option<TrapKind>,
        },
        /// The `call_indirect_host` instruction was executed.
        CallIndirectHost {
            /// The payload of `call_indirect_host`.
            id: u8,
            /// Where to resume execution after the host has finished.
            resume: NonNull<u8>,
        },
        /// Pulley has finished and the provided value is being returned.
        ReturnToHost(T),
    }

    /// Stored within `DoneReason::Trap`.
    #[allow(missing_docs, reason = "self-describing variants")]
    pub enum TrapKind {
        DivideByZero,
        IntegerOverflow,
        BadConversionToInteger,
    }

    impl MachineState {
        pub(super) fn debug_assert_done_reason_none(&mut self) {
            debug_assert!(self.done_reason.is_none());
        }

        pub(super) fn done_decode(&mut self, Done { _priv }: Done) -> DoneReason<()> {
            self.done_reason.take().unwrap()
        }
    }

    impl Interpreter<'_> {
        /// Finishes execution by recording `DoneReason::Trap`.
        ///
        /// This method takes an `I` generic parameter indicating which
        /// instruction is executing this function and generating a trap. That's
        /// used to go backwards from the current `pc` which is just beyond the
        /// instruction to point to the instruction itself in the trap metadata
        /// returned from the interpreter.
        pub fn done_trap<I: Encode>(&mut self) -> ControlFlow<Done> {
            self.done_trap_kind::<I>(None)
        }

        /// Same as `done_trap` but with an explicit `TrapKind`.
        pub fn done_trap_kind<I: Encode>(&mut self, kind: Option<TrapKind>) -> ControlFlow<Done> {
            let pc = self.current_pc::<I>();
            self.state.done_reason = Some(DoneReason::Trap { pc, kind });
            ControlFlow::Break(Done { _priv: () })
        }

        /// Finishes execution by recording `DoneReason::CallIndirectHost`.
        pub fn done_call_indirect_host(&mut self, id: u8) -> ControlFlow<Done> {
            self.state.done_reason = Some(DoneReason::CallIndirectHost {
                id,
                resume: self.pc.as_ptr(),
            });
            ControlFlow::Break(Done { _priv: () })
        }

        /// Finishes execution by recording `DoneReason::ReturnToHost`.
        pub fn done_return_to_host(&mut self) -> ControlFlow<Done> {
            self.state.done_reason = Some(DoneReason::ReturnToHost(()));
            ControlFlow::Break(Done { _priv: () })
        }
    }
}

use done::Done;
pub use done::{DoneReason, TrapKind};

struct Interpreter<'a> {
    state: &'a mut MachineState,
    pc: UnsafeBytecodeStream,
}

impl Interpreter<'_> {
    /// Performs a relative jump of `offset` bytes from the current instruction.
    ///
    /// This will jump from the start of the current instruction, identified by
    /// `I`, `offset` bytes away. Note that the `self.pc` at the start of this
    /// function actually points to the instruction after this one so `I` is
    /// necessary to go back to ourselves after which we then go `offset` away.
    #[inline]
    fn pc_rel_jump<I: Encode>(&mut self, offset: PcRelOffset) -> ControlFlow<Done> {
        let offset = isize::try_from(i32::from(offset)).unwrap();
        let my_pc = self.current_pc::<I>();
        self.pc = unsafe { UnsafeBytecodeStream::new(my_pc.offset(offset)) };
        ControlFlow::Continue(())
    }

    /// Returns the PC of the current instruction where `I` is the static type
    /// representing the current instruction.
    fn current_pc<I: Encode>(&self) -> NonNull<u8> {
        unsafe { self.pc.offset(-isize::from(I::WIDTH)).as_ptr() }
    }

    /// `sp -= size_of::<T>(); *sp = val;`
    ///
    /// Note that `I` is the instruction which is pushing data to use if a trap
    /// is generated.
    #[must_use]
    fn push<I: Encode, T>(&mut self, val: T) -> ControlFlow<Done> {
        let new_sp = self.state[XReg::sp].get_ptr::<T>().wrapping_sub(1);
        self.set_sp::<I>(new_sp.cast())?;
        unsafe {
            new_sp.write_unaligned(val);
        }
        ControlFlow::Continue(())
    }

    /// `ret = *sp; sp -= size_of::<T>()`
    fn pop<T>(&mut self) -> T {
        let sp = self.state[XReg::sp].get_ptr::<T>();
        let val = unsafe { sp.read_unaligned() };
        self.set_sp_unchecked(sp.wrapping_add(1));
        val
    }

    /// Sets the stack pointer to the `sp` provided.
    ///
    /// Returns a trap if this would result in stack overflow, or if `sp` is
    /// beneath the base pointer of `self.state.stack`.
    ///
    /// The `I` parameter here is the instruction that is setting the stack
    /// pointer and is used to calculate this instruction's own `pc` if this
    /// instruction traps.
    #[must_use]
    fn set_sp<I: Encode>(&mut self, sp: *mut u8) -> ControlFlow<Done> {
        let sp_raw = sp as usize;
        let base_raw = self.state.stack.as_ptr() as usize;
        if sp_raw < base_raw {
            return self.done_trap::<I>();
        }
        self.set_sp_unchecked(sp);
        ControlFlow::Continue(())
    }

    /// Same as `set_sp` but does not check to see if `sp` is in-bounds. Should
    /// only be used with stack increment operations such as `pop`.
    fn set_sp_unchecked<T>(&mut self, sp: *mut T) {
        if cfg!(debug_assertions) {
            let sp_raw = sp as usize;
            let base = self.state.stack.as_ptr() as usize;
            let end = base + self.state.stack.len();
            assert!(base <= sp_raw && sp_raw <= end);
        }
        self.state[XReg::sp].set_ptr(sp);
    }

    unsafe fn load<T>(&self, ptr: XReg, offset: i32) -> T {
        unsafe {
            self.state[ptr]
                .get_ptr::<T>()
                .byte_offset(offset as isize)
                .read_unaligned()
        }
    }

    unsafe fn store<T>(&self, ptr: XReg, offset: i32, val: T) {
        self.state[ptr]
            .get_ptr::<T>()
            .byte_offset(offset as isize)
            .write_unaligned(val)
    }

    fn check_xnn_from_fnn<I: Encode>(&mut self, val: f64, lo: f64, hi: f64) -> ControlFlow<Done> {
        if val != val {
            return self.done_trap_kind::<I>(Some(TrapKind::BadConversionToInteger));
        }
        let val = val.wasm_trunc();
        if val <= lo || val >= hi {
            return self.done_trap_kind::<I>(Some(TrapKind::IntegerOverflow));
        }
        ControlFlow::Continue(())
    }
}

#[test]
fn simple_push_pop() {
    let mut state = MachineState::with_stack(vec![0; 16]);
    unsafe {
        let mut bytecode = [0; 10];
        let mut i = Interpreter {
            state: &mut state,
            // this isn't actually read so just manufacture a dummy one
            pc: UnsafeBytecodeStream::new(NonNull::new(bytecode.as_mut_ptr().offset(4)).unwrap()),
        };
        assert!(i.push::<crate::Ret, _>(0_i32).is_continue());
        assert_eq!(i.pop::<i32>(), 0_i32);
        assert!(i.push::<crate::Ret, _>(1_i32).is_continue());
        assert!(i.push::<crate::Ret, _>(2_i32).is_continue());
        assert!(i.push::<crate::Ret, _>(3_i32).is_continue());
        assert!(i.push::<crate::Ret, _>(4_i32).is_continue());
        assert!(i.push::<crate::Ret, _>(5_i32).is_break());
        assert!(i.push::<crate::Ret, _>(6_i32).is_break());
        assert_eq!(i.pop::<i32>(), 4_i32);
        assert_eq!(i.pop::<i32>(), 3_i32);
        assert_eq!(i.pop::<i32>(), 2_i32);
        assert_eq!(i.pop::<i32>(), 1_i32);
    }
}

impl OpVisitor for Interpreter<'_> {
    type BytecodeStream = UnsafeBytecodeStream;
    type Return = ControlFlow<Done>;

    fn bytecode(&mut self) -> &mut UnsafeBytecodeStream {
        &mut self.pc
    }

    fn ret(&mut self) -> ControlFlow<Done> {
        let lr = self.state.lr;
        if lr == HOST_RETURN_ADDR {
            self.done_return_to_host()
        } else {
            self.pc = unsafe { UnsafeBytecodeStream::new(NonNull::new_unchecked(lr)) };
            ControlFlow::Continue(())
        }
    }

    fn call(&mut self, offset: PcRelOffset) -> ControlFlow<Done> {
        let return_addr = self.pc.as_ptr();
        self.state.lr = return_addr.as_ptr();
        self.pc_rel_jump::<crate::Call>(offset);
        ControlFlow::Continue(())
    }

    fn call_indirect(&mut self, dst: XReg) -> ControlFlow<Done> {
        let return_addr = self.pc.as_ptr();
        self.state.lr = return_addr.as_ptr();
        // SAFETY: part of the unsafe contract of the interpreter is only valid
        // bytecode is interpreted, so the jump destination is part of the validity
        // of the bytecode itself.
        unsafe {
            self.pc = UnsafeBytecodeStream::new(NonNull::new_unchecked(self.state[dst].get_ptr()));
        }
        ControlFlow::Continue(())
    }

    fn jump(&mut self, offset: PcRelOffset) -> ControlFlow<Done> {
        self.pc_rel_jump::<crate::Jump>(offset);
        ControlFlow::Continue(())
    }

    fn xjump(&mut self, reg: XReg) -> ControlFlow<Done> {
        unsafe {
            self.pc = UnsafeBytecodeStream::new(NonNull::new_unchecked(self.state[reg].get_ptr()));
        }
        ControlFlow::Continue(())
    }

    fn br_if32(&mut self, cond: XReg, offset: PcRelOffset) -> ControlFlow<Done> {
        let cond = self.state[cond].get_u32();
        if cond != 0 {
            self.pc_rel_jump::<crate::BrIf>(offset)
        } else {
            ControlFlow::Continue(())
        }
    }

    fn br_if_not32(&mut self, cond: XReg, offset: PcRelOffset) -> ControlFlow<Done> {
        let cond = self.state[cond].get_u32();
        if cond == 0 {
            self.pc_rel_jump::<crate::BrIfNot>(offset)
        } else {
            ControlFlow::Continue(())
        }
    }

    fn br_if_xeq32(&mut self, a: XReg, b: XReg, offset: PcRelOffset) -> ControlFlow<Done> {
        let a = self.state[a].get_u32();
        let b = self.state[b].get_u32();
        if a == b {
            self.pc_rel_jump::<crate::BrIfXeq32>(offset)
        } else {
            ControlFlow::Continue(())
        }
    }

    fn br_if_xneq32(&mut self, a: XReg, b: XReg, offset: PcRelOffset) -> ControlFlow<Done> {
        let a = self.state[a].get_u32();
        let b = self.state[b].get_u32();
        if a != b {
            self.pc_rel_jump::<crate::BrIfXneq32>(offset)
        } else {
            ControlFlow::Continue(())
        }
    }

    fn br_if_xslt32(&mut self, a: XReg, b: XReg, offset: PcRelOffset) -> ControlFlow<Done> {
        let a = self.state[a].get_i32();
        let b = self.state[b].get_i32();
        if a < b {
            self.pc_rel_jump::<crate::BrIfXslt32>(offset)
        } else {
            ControlFlow::Continue(())
        }
    }

    fn br_if_xslteq32(&mut self, a: XReg, b: XReg, offset: PcRelOffset) -> ControlFlow<Done> {
        let a = self.state[a].get_i32();
        let b = self.state[b].get_i32();
        if a <= b {
            self.pc_rel_jump::<crate::BrIfXslteq32>(offset)
        } else {
            ControlFlow::Continue(())
        }
    }

    fn br_if_xult32(&mut self, a: XReg, b: XReg, offset: PcRelOffset) -> ControlFlow<Done> {
        let a = self.state[a].get_u32();
        let b = self.state[b].get_u32();
        if a < b {
            self.pc_rel_jump::<crate::BrIfXult32>(offset)
        } else {
            ControlFlow::Continue(())
        }
    }

    fn br_if_xulteq32(&mut self, a: XReg, b: XReg, offset: PcRelOffset) -> ControlFlow<Done> {
        let a = self.state[a].get_u32();
        let b = self.state[b].get_u32();
        if a <= b {
            self.pc_rel_jump::<crate::BrIfXulteq32>(offset)
        } else {
            ControlFlow::Continue(())
        }
    }

    fn br_if_xeq64(&mut self, a: XReg, b: XReg, offset: PcRelOffset) -> ControlFlow<Done> {
        let a = self.state[a].get_u64();
        let b = self.state[b].get_u64();
        if a == b {
            self.pc_rel_jump::<crate::BrIfXeq64>(offset)
        } else {
            ControlFlow::Continue(())
        }
    }

    fn br_if_xneq64(&mut self, a: XReg, b: XReg, offset: PcRelOffset) -> ControlFlow<Done> {
        let a = self.state[a].get_u64();
        let b = self.state[b].get_u64();
        if a != b {
            self.pc_rel_jump::<crate::BrIfXneq64>(offset)
        } else {
            ControlFlow::Continue(())
        }
    }

    fn br_if_xslt64(&mut self, a: XReg, b: XReg, offset: PcRelOffset) -> ControlFlow<Done> {
        let a = self.state[a].get_i64();
        let b = self.state[b].get_i64();
        if a < b {
            self.pc_rel_jump::<crate::BrIfXslt64>(offset)
        } else {
            ControlFlow::Continue(())
        }
    }

    fn br_if_xslteq64(&mut self, a: XReg, b: XReg, offset: PcRelOffset) -> ControlFlow<Done> {
        let a = self.state[a].get_i64();
        let b = self.state[b].get_i64();
        if a <= b {
            self.pc_rel_jump::<crate::BrIfXslteq64>(offset)
        } else {
            ControlFlow::Continue(())
        }
    }

    fn br_if_xult64(&mut self, a: XReg, b: XReg, offset: PcRelOffset) -> ControlFlow<Done> {
        let a = self.state[a].get_u64();
        let b = self.state[b].get_u64();
        if a < b {
            self.pc_rel_jump::<crate::BrIfXult64>(offset)
        } else {
            ControlFlow::Continue(())
        }
    }

    fn br_if_xulteq64(&mut self, a: XReg, b: XReg, offset: PcRelOffset) -> ControlFlow<Done> {
        let a = self.state[a].get_u64();
        let b = self.state[b].get_u64();
        if a <= b {
            self.pc_rel_jump::<crate::BrIfXulteq64>(offset)
        } else {
            ControlFlow::Continue(())
        }
    }

    fn xmov(&mut self, dst: XReg, src: XReg) -> ControlFlow<Done> {
        let val = self.state[src];
        self.state[dst] = val;
        ControlFlow::Continue(())
    }

    fn fmov(&mut self, dst: FReg, src: FReg) -> ControlFlow<Done> {
        let val = self.state[src];
        self.state[dst] = val;
        ControlFlow::Continue(())
    }

    fn vmov(&mut self, dst: VReg, src: VReg) -> ControlFlow<Done> {
        let val = self.state[src];
        self.state[dst] = val;
        ControlFlow::Continue(())
    }

    fn xconst8(&mut self, dst: XReg, imm: i8) -> ControlFlow<Done> {
        self.state[dst].set_i64(i64::from(imm));
        ControlFlow::Continue(())
    }

    fn xconst16(&mut self, dst: XReg, imm: i16) -> ControlFlow<Done> {
        self.state[dst].set_i64(i64::from(imm));
        ControlFlow::Continue(())
    }

    fn xconst32(&mut self, dst: XReg, imm: i32) -> ControlFlow<Done> {
        self.state[dst].set_i64(i64::from(imm));
        ControlFlow::Continue(())
    }

    fn xconst64(&mut self, dst: XReg, imm: i64) -> ControlFlow<Done> {
        self.state[dst].set_i64(imm);
        ControlFlow::Continue(())
    }

    fn xadd32(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u32();
        let b = self.state[operands.src2].get_u32();
        self.state[operands.dst].set_u32(a.wrapping_add(b));
        ControlFlow::Continue(())
    }

    fn xadd64(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u64();
        let b = self.state[operands.src2].get_u64();
        self.state[operands.dst].set_u64(a.wrapping_add(b));
        ControlFlow::Continue(())
    }

    fn xadd32_uoverflow_trap(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u32();
        let b = self.state[operands.src2].get_u32();
        match a.checked_add(b) {
            Some(c) => {
                self.state[operands.dst].set_u32(c);
                ControlFlow::Continue(())
            }
            None => self.done_trap::<crate::Xadd32UoverflowTrap>(),
        }
    }

    fn xadd64_uoverflow_trap(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u64();
        let b = self.state[operands.src2].get_u64();
        match a.checked_add(b) {
            Some(c) => {
                self.state[operands.dst].set_u64(c);
                ControlFlow::Continue(())
            }
            None => self.done_trap::<crate::Xadd64UoverflowTrap>(),
        }
    }

    fn xsub32(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u32();
        let b = self.state[operands.src2].get_u32();
        self.state[operands.dst].set_u32(a.wrapping_sub(b));
        ControlFlow::Continue(())
    }

    fn xsub64(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u64();
        let b = self.state[operands.src2].get_u64();
        self.state[operands.dst].set_u64(a.wrapping_sub(b));
        ControlFlow::Continue(())
    }

    fn xmul32(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u32();
        let b = self.state[operands.src2].get_u32();
        self.state[operands.dst].set_u32(a.wrapping_mul(b));
        ControlFlow::Continue(())
    }

    fn xmul64(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u64();
        let b = self.state[operands.src2].get_u64();
        self.state[operands.dst].set_u64(a.wrapping_mul(b));
        ControlFlow::Continue(())
    }

    fn xmulhi64_s(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_i64();
        let b = self.state[operands.src2].get_i64();
        let result = ((i128::from(a) * i128::from(b)) >> 64) as i64;
        self.state[operands.dst].set_i64(result);
        ControlFlow::Continue(())
    }

    fn xmulhi64_u(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u64();
        let b = self.state[operands.src2].get_u64();
        let result = ((u128::from(a) * u128::from(b)) >> 64) as u64;
        self.state[operands.dst].set_u64(result);
        ControlFlow::Continue(())
    }

    fn xshl32(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u32();
        let b = self.state[operands.src2].get_u32();
        self.state[operands.dst].set_u32(a.wrapping_shl(b));
        ControlFlow::Continue(())
    }

    fn xshr32_u(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u32();
        let b = self.state[operands.src2].get_u32();
        self.state[operands.dst].set_u32(a.wrapping_shr(b));
        ControlFlow::Continue(())
    }

    fn xshr32_s(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_i32();
        let b = self.state[operands.src2].get_u32();
        self.state[operands.dst].set_i32(a.wrapping_shr(b));
        ControlFlow::Continue(())
    }

    fn xshl64(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u64();
        let b = self.state[operands.src2].get_u32();
        self.state[operands.dst].set_u64(a.wrapping_shl(b));
        ControlFlow::Continue(())
    }

    fn xshr64_u(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u64();
        let b = self.state[operands.src2].get_u32();
        self.state[operands.dst].set_u64(a.wrapping_shr(b));
        ControlFlow::Continue(())
    }

    fn xshr64_s(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_i64();
        let b = self.state[operands.src2].get_u32();
        self.state[operands.dst].set_i64(a.wrapping_shr(b));
        ControlFlow::Continue(())
    }

    fn xneg32(&mut self, dst: XReg, src: XReg) -> ControlFlow<Done> {
        let a = self.state[src].get_i32();
        self.state[dst].set_i32(a.wrapping_neg());
        ControlFlow::Continue(())
    }

    fn xneg64(&mut self, dst: XReg, src: XReg) -> ControlFlow<Done> {
        let a = self.state[src].get_i64();
        self.state[dst].set_i64(a.wrapping_neg());
        ControlFlow::Continue(())
    }

    fn xeq64(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u64();
        let b = self.state[operands.src2].get_u64();
        self.state[operands.dst].set_u32(u32::from(a == b));
        ControlFlow::Continue(())
    }

    fn xneq64(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u64();
        let b = self.state[operands.src2].get_u64();
        self.state[operands.dst].set_u32(u32::from(a != b));
        ControlFlow::Continue(())
    }

    fn xslt64(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_i64();
        let b = self.state[operands.src2].get_i64();
        self.state[operands.dst].set_u32(u32::from(a < b));
        ControlFlow::Continue(())
    }

    fn xslteq64(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_i64();
        let b = self.state[operands.src2].get_i64();
        self.state[operands.dst].set_u32(u32::from(a <= b));
        ControlFlow::Continue(())
    }

    fn xult64(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u64();
        let b = self.state[operands.src2].get_u64();
        self.state[operands.dst].set_u32(u32::from(a < b));
        ControlFlow::Continue(())
    }

    fn xulteq64(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u64();
        let b = self.state[operands.src2].get_u64();
        self.state[operands.dst].set_u32(u32::from(a <= b));
        ControlFlow::Continue(())
    }

    fn xeq32(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u32();
        let b = self.state[operands.src2].get_u32();
        self.state[operands.dst].set_u32(u32::from(a == b));
        ControlFlow::Continue(())
    }

    fn xneq32(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u32();
        let b = self.state[operands.src2].get_u32();
        self.state[operands.dst].set_u32(u32::from(a != b));
        ControlFlow::Continue(())
    }

    fn xslt32(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_i32();
        let b = self.state[operands.src2].get_i32();
        self.state[operands.dst].set_u32(u32::from(a < b));
        ControlFlow::Continue(())
    }

    fn xslteq32(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_i32();
        let b = self.state[operands.src2].get_i32();
        self.state[operands.dst].set_u32(u32::from(a <= b));
        ControlFlow::Continue(())
    }

    fn xult32(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u32();
        let b = self.state[operands.src2].get_u32();
        self.state[operands.dst].set_u32(u32::from(a < b));
        ControlFlow::Continue(())
    }

    fn xulteq32(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u32();
        let b = self.state[operands.src2].get_u32();
        self.state[operands.dst].set_u32(u32::from(a <= b));
        ControlFlow::Continue(())
    }

    fn xload8_u32_offset32(&mut self, dst: XReg, ptr: XReg, offset: i32) -> ControlFlow<Done> {
        let val = unsafe { self.load::<u8>(ptr, offset) };
        self.state[dst].set_u32(val.into());
        ControlFlow::Continue(())
    }

    fn xload8_s32_offset32(&mut self, dst: XReg, ptr: XReg, offset: i32) -> ControlFlow<Done> {
        let val = unsafe { self.load::<i8>(ptr, offset) };
        self.state[dst].set_i32(val.into());
        ControlFlow::Continue(())
    }

    fn xload16le_u32_offset32(&mut self, dst: XReg, ptr: XReg, offset: i32) -> ControlFlow<Done> {
        let val = unsafe { self.load::<u16>(ptr, offset) };
        self.state[dst].set_u32(u16::from_le(val).into());
        ControlFlow::Continue(())
    }

    fn xload16le_s32_offset32(&mut self, dst: XReg, ptr: XReg, offset: i32) -> ControlFlow<Done> {
        let val = unsafe { self.load::<i16>(ptr, offset) };
        self.state[dst].set_i32(i16::from_le(val).into());
        ControlFlow::Continue(())
    }

    fn xload32le_offset32(&mut self, dst: XReg, ptr: XReg, offset: i32) -> ControlFlow<Done> {
        let val = unsafe { self.load::<i32>(ptr, offset) };
        self.state[dst].set_i32(i32::from_le(val));
        ControlFlow::Continue(())
    }

    fn xload8_u64_offset32(&mut self, dst: XReg, ptr: XReg, offset: i32) -> ControlFlow<Done> {
        let val = unsafe { self.load::<u8>(ptr, offset) };
        self.state[dst].set_u64(val.into());
        ControlFlow::Continue(())
    }

    fn xload8_s64_offset32(&mut self, dst: XReg, ptr: XReg, offset: i32) -> ControlFlow<Done> {
        let val = unsafe { self.load::<i8>(ptr, offset) };
        self.state[dst].set_i64(val.into());
        ControlFlow::Continue(())
    }

    fn xload16le_u64_offset32(&mut self, dst: XReg, ptr: XReg, offset: i32) -> ControlFlow<Done> {
        let val = unsafe { self.load::<u16>(ptr, offset) };
        self.state[dst].set_u64(u16::from_le(val).into());
        ControlFlow::Continue(())
    }

    fn xload16le_s64_offset32(&mut self, dst: XReg, ptr: XReg, offset: i32) -> ControlFlow<Done> {
        let val = unsafe { self.load::<i16>(ptr, offset) };
        self.state[dst].set_i64(i16::from_le(val).into());
        ControlFlow::Continue(())
    }

    fn xload32le_u64_offset32(&mut self, dst: XReg, ptr: XReg, offset: i32) -> ControlFlow<Done> {
        let val = unsafe { self.load::<u32>(ptr, offset) };
        self.state[dst].set_u64(u32::from_le(val).into());
        ControlFlow::Continue(())
    }

    fn xload32le_s64_offset32(&mut self, dst: XReg, ptr: XReg, offset: i32) -> ControlFlow<Done> {
        let val = unsafe { self.load::<i32>(ptr, offset) };
        self.state[dst].set_i64(i32::from_le(val).into());
        ControlFlow::Continue(())
    }

    fn xload64le_offset32(&mut self, dst: XReg, ptr: XReg, offset: i32) -> ControlFlow<Done> {
        let val = unsafe { self.load::<i64>(ptr, offset) };
        self.state[dst].set_i64(i64::from_le(val));
        ControlFlow::Continue(())
    }

    fn xstore8_offset32(&mut self, ptr: XReg, offset: i32, src: XReg) -> ControlFlow<Done> {
        let val = self.state[src].get_u32() as u8;
        unsafe {
            self.store(ptr, offset, val);
        }
        ControlFlow::Continue(())
    }

    fn xstore16le_offset32(&mut self, ptr: XReg, offset: i32, src: XReg) -> ControlFlow<Done> {
        let val = self.state[src].get_u32() as u16;
        unsafe {
            self.store(ptr, offset, val.to_le());
        }
        ControlFlow::Continue(())
    }

    fn xstore32le_offset32(&mut self, ptr: XReg, offset: i32, src: XReg) -> ControlFlow<Done> {
        let val = self.state[src].get_u32();
        unsafe {
            self.store(ptr, offset, val.to_le());
        }
        ControlFlow::Continue(())
    }

    fn xstore64le_offset32(&mut self, ptr: XReg, offset: i32, src: XReg) -> ControlFlow<Done> {
        let val = self.state[src].get_u64();
        unsafe {
            self.store(ptr, offset, val.to_le());
        }
        ControlFlow::Continue(())
    }

    fn fload32le_offset32(&mut self, dst: FReg, ptr: XReg, offset: i32) -> ControlFlow<Done> {
        let val = unsafe { self.load::<u32>(ptr, offset) };
        self.state[dst].set_f32(f32::from_bits(u32::from_le(val)));
        ControlFlow::Continue(())
    }

    fn fload64le_offset32(&mut self, dst: FReg, ptr: XReg, offset: i32) -> ControlFlow<Done> {
        let val = unsafe { self.load::<u64>(ptr, offset) };
        self.state[dst].set_f64(f64::from_bits(u64::from_le(val)));
        ControlFlow::Continue(())
    }

    fn fstore32le_offset32(&mut self, ptr: XReg, offset: i32, src: FReg) -> ControlFlow<Done> {
        let val = self.state[src].get_f32();
        unsafe {
            self.store(ptr, offset, val.to_bits().to_le());
        }
        ControlFlow::Continue(())
    }

    fn fstore64le_offset32(&mut self, ptr: XReg, offset: i32, src: FReg) -> ControlFlow<Done> {
        let val = self.state[src].get_f64();
        unsafe {
            self.store(ptr, offset, val.to_bits().to_le());
        }
        ControlFlow::Continue(())
    }

    fn vload128le_offset32(&mut self, dst: VReg, ptr: XReg, offset: i32) -> ControlFlow<Done> {
        let val = unsafe { self.load::<u128>(ptr, offset) };
        self.state[dst].set_u128(u128::from_le(val));
        ControlFlow::Continue(())
    }

    fn vstore128le_offset32(&mut self, ptr: XReg, offset: i32, src: VReg) -> ControlFlow<Done> {
        let val = self.state[src].get_u128();
        unsafe {
            self.store(ptr, offset, val.to_le());
        }
        ControlFlow::Continue(())
    }

    fn xpush32(&mut self, src: XReg) -> ControlFlow<Done> {
        self.push::<crate::XPush32, _>(self.state[src].get_u32())?;
        ControlFlow::Continue(())
    }

    fn xpush32_many(&mut self, srcs: RegSet<XReg>) -> ControlFlow<Done> {
        for src in srcs {
            self.push::<crate::XPush32Many, _>(self.state[src].get_u32())?;
        }
        ControlFlow::Continue(())
    }

    fn xpush64(&mut self, src: XReg) -> ControlFlow<Done> {
        self.push::<crate::XPush64, _>(self.state[src].get_u64())?;
        ControlFlow::Continue(())
    }

    fn xpush64_many(&mut self, srcs: RegSet<XReg>) -> ControlFlow<Done> {
        for src in srcs {
            self.push::<crate::XPush64Many, _>(self.state[src].get_u64())?;
        }
        ControlFlow::Continue(())
    }

    fn xpop32(&mut self, dst: XReg) -> ControlFlow<Done> {
        let val = self.pop();
        self.state[dst].set_u32(val);
        ControlFlow::Continue(())
    }

    fn xpop32_many(&mut self, dsts: RegSet<XReg>) -> ControlFlow<Done> {
        for dst in dsts.into_iter().rev() {
            let val = self.pop();
            self.state[dst].set_u32(val);
        }
        ControlFlow::Continue(())
    }

    fn xpop64(&mut self, dst: XReg) -> ControlFlow<Done> {
        let val = self.pop();
        self.state[dst].set_u64(val);
        ControlFlow::Continue(())
    }

    fn xpop64_many(&mut self, dsts: RegSet<XReg>) -> ControlFlow<Done> {
        for dst in dsts.into_iter().rev() {
            let val = self.pop();
            self.state[dst].set_u64(val);
        }
        ControlFlow::Continue(())
    }

    fn push_frame(&mut self) -> ControlFlow<Done> {
        self.push::<crate::PushFrame, _>(self.state.lr)?;
        self.push::<crate::PushFrame, _>(self.state.fp)?;
        self.state.fp = self.state[XReg::sp].get_ptr();
        ControlFlow::Continue(())
    }

    fn pop_frame(&mut self) -> ControlFlow<Done> {
        self.set_sp_unchecked(self.state.fp);
        let fp = self.pop();
        let lr = self.pop();
        self.state.fp = fp;
        self.state.lr = lr;
        ControlFlow::Continue(())
    }

    fn bitcast_int_from_float_32(&mut self, dst: XReg, src: FReg) -> ControlFlow<Done> {
        let val = self.state[src].get_f32();
        self.state[dst].set_u32(val.to_bits());
        ControlFlow::Continue(())
    }

    fn bitcast_int_from_float_64(&mut self, dst: XReg, src: FReg) -> ControlFlow<Done> {
        let val = self.state[src].get_f64();
        self.state[dst].set_u64(val.to_bits());
        ControlFlow::Continue(())
    }

    fn bitcast_float_from_int_32(&mut self, dst: FReg, src: XReg) -> ControlFlow<Done> {
        let val = self.state[src].get_u32();
        self.state[dst].set_f32(f32::from_bits(val));
        ControlFlow::Continue(())
    }

    fn bitcast_float_from_int_64(&mut self, dst: FReg, src: XReg) -> ControlFlow<Done> {
        let val = self.state[src].get_u64();
        self.state[dst].set_f64(f64::from_bits(val));
        ControlFlow::Continue(())
    }

    fn br_table32(&mut self, idx: XReg, amt: u32) -> ControlFlow<Done> {
        let idx = self.state[idx].get_u32().min(amt - 1) as isize;
        // SAFETY: part of the contract of the interpreter is only dealing with
        // valid bytecode, so this offset should be safe.
        self.pc = unsafe { self.pc.offset(idx * 4) };

        // Decode the `PcRelOffset` without tampering with `self.pc` as the
        // jump is relative to `self.pc`.
        let mut tmp = self.pc;
        let rel = unwrap_uninhabited(PcRelOffset::decode(&mut tmp));
        let offset = isize::try_from(i32::from(rel)).unwrap();
        self.pc = unsafe { self.pc.offset(offset) };
        ControlFlow::Continue(())
    }

    fn stack_alloc32(&mut self, amt: u32) -> ControlFlow<Done> {
        let amt = usize::try_from(amt).unwrap();
        let new_sp = self.state[XReg::sp].get_ptr::<u8>().wrapping_sub(amt);
        self.set_sp::<crate::StackAlloc32>(new_sp)?;
        ControlFlow::Continue(())
    }

    fn stack_free32(&mut self, amt: u32) -> ControlFlow<Done> {
        let amt = usize::try_from(amt).unwrap();
        let new_sp = self.state[XReg::sp].get_ptr::<u8>().wrapping_add(amt);
        self.set_sp_unchecked(new_sp);
        ControlFlow::Continue(())
    }

    fn zext8(&mut self, dst: XReg, src: XReg) -> ControlFlow<Done> {
        let src = self.state[src].get_u64() as u8;
        self.state[dst].set_u64(src.into());
        ControlFlow::Continue(())
    }

    fn zext16(&mut self, dst: XReg, src: XReg) -> ControlFlow<Done> {
        let src = self.state[src].get_u64() as u16;
        self.state[dst].set_u64(src.into());
        ControlFlow::Continue(())
    }

    fn zext32(&mut self, dst: XReg, src: XReg) -> ControlFlow<Done> {
        let src = self.state[src].get_u64() as u32;
        self.state[dst].set_u64(src.into());
        ControlFlow::Continue(())
    }

    fn sext8(&mut self, dst: XReg, src: XReg) -> ControlFlow<Done> {
        let src = self.state[src].get_i64() as i8;
        self.state[dst].set_i64(src.into());
        ControlFlow::Continue(())
    }

    fn sext16(&mut self, dst: XReg, src: XReg) -> ControlFlow<Done> {
        let src = self.state[src].get_i64() as i16;
        self.state[dst].set_i64(src.into());
        ControlFlow::Continue(())
    }

    fn sext32(&mut self, dst: XReg, src: XReg) -> ControlFlow<Done> {
        let src = self.state[src].get_i64() as i32;
        self.state[dst].set_i64(src.into());
        ControlFlow::Continue(())
    }

    fn xdiv32_s(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_i32();
        let b = self.state[operands.src2].get_i32();
        match a.checked_div(b) {
            Some(result) => {
                self.state[operands.dst].set_i32(result);
                ControlFlow::Continue(())
            }
            None => {
                let kind = if b == 0 {
                    TrapKind::DivideByZero
                } else {
                    TrapKind::IntegerOverflow
                };
                self.done_trap_kind::<crate::XDiv32S>(Some(kind))
            }
        }
    }

    fn xdiv64_s(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_i64();
        let b = self.state[operands.src2].get_i64();
        match a.checked_div(b) {
            Some(result) => {
                self.state[operands.dst].set_i64(result);
                ControlFlow::Continue(())
            }
            None => {
                let kind = if b == 0 {
                    TrapKind::DivideByZero
                } else {
                    TrapKind::IntegerOverflow
                };
                self.done_trap_kind::<crate::XDiv64S>(Some(kind))
            }
        }
    }

    fn xdiv32_u(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u32();
        let b = self.state[operands.src2].get_u32();
        match a.checked_div(b) {
            Some(result) => {
                self.state[operands.dst].set_u32(result);
                ControlFlow::Continue(())
            }
            None => self.done_trap_kind::<crate::XDiv64U>(Some(TrapKind::DivideByZero)),
        }
    }

    fn xdiv64_u(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u64();
        let b = self.state[operands.src2].get_u64();
        match a.checked_div(b) {
            Some(result) => {
                self.state[operands.dst].set_u64(result);
                ControlFlow::Continue(())
            }
            None => self.done_trap_kind::<crate::XDiv64U>(Some(TrapKind::DivideByZero)),
        }
    }

    fn xrem32_s(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_i32();
        let b = self.state[operands.src2].get_i32();
        let result = if a == i32::MIN && b == -1 {
            Some(0)
        } else {
            a.checked_rem(b)
        };
        match result {
            Some(result) => {
                self.state[operands.dst].set_i32(result);
                ControlFlow::Continue(())
            }
            None => self.done_trap_kind::<crate::XRem32S>(Some(TrapKind::DivideByZero)),
        }
    }

    fn xrem64_s(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_i64();
        let b = self.state[operands.src2].get_i64();
        let result = if a == i64::MIN && b == -1 {
            Some(0)
        } else {
            a.checked_rem(b)
        };
        match result {
            Some(result) => {
                self.state[operands.dst].set_i64(result);
                ControlFlow::Continue(())
            }
            None => self.done_trap_kind::<crate::XRem64S>(Some(TrapKind::DivideByZero)),
        }
    }

    fn xrem32_u(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u32();
        let b = self.state[operands.src2].get_u32();
        match a.checked_rem(b) {
            Some(result) => {
                self.state[operands.dst].set_u32(result);
                ControlFlow::Continue(())
            }
            None => self.done_trap_kind::<crate::XRem32U>(Some(TrapKind::DivideByZero)),
        }
    }

    fn xrem64_u(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u64();
        let b = self.state[operands.src2].get_u64();
        match a.checked_rem(b) {
            Some(result) => {
                self.state[operands.dst].set_u64(result);
                ControlFlow::Continue(())
            }
            None => self.done_trap_kind::<crate::XRem64U>(Some(TrapKind::DivideByZero)),
        }
    }

    fn xband32(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u32();
        let b = self.state[operands.src2].get_u32();
        self.state[operands.dst].set_u32(a & b);
        ControlFlow::Continue(())
    }

    fn xband64(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u64();
        let b = self.state[operands.src2].get_u64();
        self.state[operands.dst].set_u64(a & b);
        ControlFlow::Continue(())
    }

    fn xbor32(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u32();
        let b = self.state[operands.src2].get_u32();
        self.state[operands.dst].set_u32(a | b);
        ControlFlow::Continue(())
    }

    fn xbor64(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u64();
        let b = self.state[operands.src2].get_u64();
        self.state[operands.dst].set_u64(a | b);
        ControlFlow::Continue(())
    }

    fn xbxor32(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u32();
        let b = self.state[operands.src2].get_u32();
        self.state[operands.dst].set_u32(a ^ b);
        ControlFlow::Continue(())
    }

    fn xbxor64(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u64();
        let b = self.state[operands.src2].get_u64();
        self.state[operands.dst].set_u64(a ^ b);
        ControlFlow::Continue(())
    }

    fn xbnot32(&mut self, dst: XReg, src: XReg) -> ControlFlow<Done> {
        let a = self.state[src].get_u32();
        self.state[dst].set_u32(!a);
        ControlFlow::Continue(())
    }

    fn xbnot64(&mut self, dst: XReg, src: XReg) -> ControlFlow<Done> {
        let a = self.state[src].get_u64();
        self.state[dst].set_u64(!a);
        ControlFlow::Continue(())
    }

    fn xmin32_u(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u32();
        let b = self.state[operands.src2].get_u32();
        self.state[operands.dst].set_u32(a.min(b));
        ControlFlow::Continue(())
    }

    fn xmin32_s(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_i32();
        let b = self.state[operands.src2].get_i32();
        self.state[operands.dst].set_i32(a.min(b));
        ControlFlow::Continue(())
    }

    fn xmax32_u(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u32();
        let b = self.state[operands.src2].get_u32();
        self.state[operands.dst].set_u32(a.max(b));
        ControlFlow::Continue(())
    }

    fn xmax32_s(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_i32();
        let b = self.state[operands.src2].get_i32();
        self.state[operands.dst].set_i32(a.max(b));
        ControlFlow::Continue(())
    }

    fn xmin64_u(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u64();
        let b = self.state[operands.src2].get_u64();
        self.state[operands.dst].set_u64(a.min(b));
        ControlFlow::Continue(())
    }

    fn xmin64_s(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_i64();
        let b = self.state[operands.src2].get_i64();
        self.state[operands.dst].set_i64(a.min(b));
        ControlFlow::Continue(())
    }

    fn xmax64_u(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u64();
        let b = self.state[operands.src2].get_u64();
        self.state[operands.dst].set_u64(a.max(b));
        ControlFlow::Continue(())
    }

    fn xmax64_s(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_i64();
        let b = self.state[operands.src2].get_i64();
        self.state[operands.dst].set_i64(a.max(b));
        ControlFlow::Continue(())
    }

    fn xbmask32(&mut self, dst: XReg, src: XReg) -> Self::Return {
        let a = self.state[src].get_u32();
        if a == 0 {
            self.state[dst].set_u32(0);
        } else {
            self.state[dst].set_i32(-1);
        }
        ControlFlow::Continue(())
    }

    fn xbmask64(&mut self, dst: XReg, src: XReg) -> Self::Return {
        let a = self.state[src].get_u64();
        if a == 0 {
            self.state[dst].set_u64(0);
        } else {
            self.state[dst].set_i64(-1);
        }
        ControlFlow::Continue(())
    }

    fn fconst32(&mut self, dst: FReg, bits: u32) -> ControlFlow<Done> {
        self.state[dst].set_f32(f32::from_bits(bits));
        ControlFlow::Continue(())
    }

    fn fconst64(&mut self, dst: FReg, bits: u64) -> ControlFlow<Done> {
        self.state[dst].set_f64(f64::from_bits(bits));
        ControlFlow::Continue(())
    }

    fn feq32(&mut self, dst: XReg, src1: FReg, src2: FReg) -> ControlFlow<Done> {
        let a = self.state[src1].get_f32();
        let b = self.state[src2].get_f32();
        self.state[dst].set_u32(u32::from(a == b));
        ControlFlow::Continue(())
    }

    fn fneq32(&mut self, dst: XReg, src1: FReg, src2: FReg) -> ControlFlow<Done> {
        let a = self.state[src1].get_f32();
        let b = self.state[src2].get_f32();
        self.state[dst].set_u32(u32::from(a != b));
        ControlFlow::Continue(())
    }

    fn flt32(&mut self, dst: XReg, src1: FReg, src2: FReg) -> ControlFlow<Done> {
        let a = self.state[src1].get_f32();
        let b = self.state[src2].get_f32();
        self.state[dst].set_u32(u32::from(a < b));
        ControlFlow::Continue(())
    }

    fn flteq32(&mut self, dst: XReg, src1: FReg, src2: FReg) -> ControlFlow<Done> {
        let a = self.state[src1].get_f32();
        let b = self.state[src2].get_f32();
        self.state[dst].set_u32(u32::from(a <= b));
        ControlFlow::Continue(())
    }

    fn feq64(&mut self, dst: XReg, src1: FReg, src2: FReg) -> ControlFlow<Done> {
        let a = self.state[src1].get_f64();
        let b = self.state[src2].get_f64();
        self.state[dst].set_u32(u32::from(a == b));
        ControlFlow::Continue(())
    }

    fn fneq64(&mut self, dst: XReg, src1: FReg, src2: FReg) -> ControlFlow<Done> {
        let a = self.state[src1].get_f64();
        let b = self.state[src2].get_f64();
        self.state[dst].set_u32(u32::from(a != b));
        ControlFlow::Continue(())
    }

    fn flt64(&mut self, dst: XReg, src1: FReg, src2: FReg) -> ControlFlow<Done> {
        let a = self.state[src1].get_f64();
        let b = self.state[src2].get_f64();
        self.state[dst].set_u32(u32::from(a < b));
        ControlFlow::Continue(())
    }

    fn flteq64(&mut self, dst: XReg, src1: FReg, src2: FReg) -> ControlFlow<Done> {
        let a = self.state[src1].get_f64();
        let b = self.state[src2].get_f64();
        self.state[dst].set_u32(u32::from(a <= b));
        ControlFlow::Continue(())
    }

    fn xctz32(&mut self, dst: XReg, src: XReg) -> ControlFlow<Done> {
        let a = self.state[src].get_u32();
        self.state[dst].set_u32(a.trailing_zeros());
        ControlFlow::Continue(())
    }

    fn xctz64(&mut self, dst: XReg, src: XReg) -> ControlFlow<Done> {
        let a = self.state[src].get_u64();
        self.state[dst].set_u64(a.trailing_zeros().into());
        ControlFlow::Continue(())
    }

    fn xclz32(&mut self, dst: XReg, src: XReg) -> ControlFlow<Done> {
        let a = self.state[src].get_u32();
        self.state[dst].set_u32(a.leading_zeros());
        ControlFlow::Continue(())
    }

    fn xclz64(&mut self, dst: XReg, src: XReg) -> ControlFlow<Done> {
        let a = self.state[src].get_u64();
        self.state[dst].set_u64(a.leading_zeros().into());
        ControlFlow::Continue(())
    }

    fn xpopcnt32(&mut self, dst: XReg, src: XReg) -> ControlFlow<Done> {
        let a = self.state[src].get_u32();
        self.state[dst].set_u32(a.count_ones());
        ControlFlow::Continue(())
    }

    fn xpopcnt64(&mut self, dst: XReg, src: XReg) -> ControlFlow<Done> {
        let a = self.state[src].get_u64();
        self.state[dst].set_u64(a.count_ones().into());
        ControlFlow::Continue(())
    }

    fn xrotl32(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u32();
        let b = self.state[operands.src2].get_u32();
        self.state[operands.dst].set_u32(a.rotate_left(b));
        ControlFlow::Continue(())
    }

    fn xrotl64(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u64();
        let b = self.state[operands.src2].get_u32();
        self.state[operands.dst].set_u64(a.rotate_left(b));
        ControlFlow::Continue(())
    }

    fn xrotr32(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u32();
        let b = self.state[operands.src2].get_u32();
        self.state[operands.dst].set_u32(a.rotate_right(b));
        ControlFlow::Continue(())
    }

    fn xrotr64(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u64();
        let b = self.state[operands.src2].get_u32();
        self.state[operands.dst].set_u64(a.rotate_right(b));
        ControlFlow::Continue(())
    }

    fn xselect32(
        &mut self,
        dst: XReg,
        cond: XReg,
        if_nonzero: XReg,
        if_zero: XReg,
    ) -> ControlFlow<Done> {
        let result = if self.state[cond].get_u32() != 0 {
            self.state[if_nonzero].get_u32()
        } else {
            self.state[if_zero].get_u32()
        };
        self.state[dst].set_u32(result);
        ControlFlow::Continue(())
    }

    fn xselect64(
        &mut self,
        dst: XReg,
        cond: XReg,
        if_nonzero: XReg,
        if_zero: XReg,
    ) -> ControlFlow<Done> {
        let result = if self.state[cond].get_u32() != 0 {
            self.state[if_nonzero].get_u64()
        } else {
            self.state[if_zero].get_u64()
        };
        self.state[dst].set_u64(result);
        ControlFlow::Continue(())
    }

    fn fselect32(
        &mut self,
        dst: FReg,
        cond: XReg,
        if_nonzero: FReg,
        if_zero: FReg,
    ) -> ControlFlow<Done> {
        let result = if self.state[cond].get_u32() != 0 {
            self.state[if_nonzero].get_f32()
        } else {
            self.state[if_zero].get_f32()
        };
        self.state[dst].set_f32(result);
        ControlFlow::Continue(())
    }

    fn fselect64(
        &mut self,
        dst: FReg,
        cond: XReg,
        if_nonzero: FReg,
        if_zero: FReg,
    ) -> ControlFlow<Done> {
        let result = if self.state[cond].get_u32() != 0 {
            self.state[if_nonzero].get_f64()
        } else {
            self.state[if_zero].get_f64()
        };
        self.state[dst].set_f64(result);
        ControlFlow::Continue(())
    }

    fn f32_from_x32_s(&mut self, dst: FReg, src: XReg) -> ControlFlow<Done> {
        let a = self.state[src].get_i32();
        self.state[dst].set_f32(a as f32);
        ControlFlow::Continue(())
    }

    fn f32_from_x32_u(&mut self, dst: FReg, src: XReg) -> ControlFlow<Done> {
        let a = self.state[src].get_u32();
        self.state[dst].set_f32(a as f32);
        ControlFlow::Continue(())
    }

    fn f32_from_x64_s(&mut self, dst: FReg, src: XReg) -> ControlFlow<Done> {
        let a = self.state[src].get_i64();
        self.state[dst].set_f32(a as f32);
        ControlFlow::Continue(())
    }

    fn f32_from_x64_u(&mut self, dst: FReg, src: XReg) -> ControlFlow<Done> {
        let a = self.state[src].get_u64();
        self.state[dst].set_f32(a as f32);
        ControlFlow::Continue(())
    }

    fn f64_from_x32_s(&mut self, dst: FReg, src: XReg) -> ControlFlow<Done> {
        let a = self.state[src].get_i32();
        self.state[dst].set_f64(a as f64);
        ControlFlow::Continue(())
    }

    fn f64_from_x32_u(&mut self, dst: FReg, src: XReg) -> ControlFlow<Done> {
        let a = self.state[src].get_u32();
        self.state[dst].set_f64(a as f64);
        ControlFlow::Continue(())
    }

    fn f64_from_x64_s(&mut self, dst: FReg, src: XReg) -> ControlFlow<Done> {
        let a = self.state[src].get_i64();
        self.state[dst].set_f64(a as f64);
        ControlFlow::Continue(())
    }

    fn f64_from_x64_u(&mut self, dst: FReg, src: XReg) -> ControlFlow<Done> {
        let a = self.state[src].get_u64();
        self.state[dst].set_f64(a as f64);
        ControlFlow::Continue(())
    }

    fn x32_from_f32_s(&mut self, dst: XReg, src: FReg) -> ControlFlow<Done> {
        let a = self.state[src].get_f32();
        self.check_xnn_from_fnn::<crate::X32FromF32S>(a.into(), -2147483649.0, 2147483648.0)?;
        self.state[dst].set_i32(a as i32);
        ControlFlow::Continue(())
    }

    fn x32_from_f32_u(&mut self, dst: XReg, src: FReg) -> ControlFlow<Done> {
        let a = self.state[src].get_f32();
        self.check_xnn_from_fnn::<crate::X32FromF32U>(a.into(), -1.0, 4294967296.0)?;
        self.state[dst].set_u32(a as u32);
        ControlFlow::Continue(())
    }

    fn x64_from_f32_s(&mut self, dst: XReg, src: FReg) -> ControlFlow<Done> {
        let a = self.state[src].get_f32();
        self.check_xnn_from_fnn::<crate::X64FromF32S>(
            a.into(),
            -9223372036854777856.0,
            9223372036854775808.0,
        )?;
        self.state[dst].set_i64(a as i64);
        ControlFlow::Continue(())
    }

    fn x64_from_f32_u(&mut self, dst: XReg, src: FReg) -> ControlFlow<Done> {
        let a = self.state[src].get_f32();
        self.check_xnn_from_fnn::<crate::X64FromF32U>(a.into(), -1.0, 18446744073709551616.0)?;
        self.state[dst].set_u64(a as u64);
        ControlFlow::Continue(())
    }

    fn x32_from_f64_s(&mut self, dst: XReg, src: FReg) -> ControlFlow<Done> {
        let a = self.state[src].get_f64();
        self.check_xnn_from_fnn::<crate::X32FromF64S>(a, -2147483649.0, 2147483648.0)?;
        self.state[dst].set_i32(a as i32);
        ControlFlow::Continue(())
    }

    fn x32_from_f64_u(&mut self, dst: XReg, src: FReg) -> ControlFlow<Done> {
        let a = self.state[src].get_f64();
        self.check_xnn_from_fnn::<crate::X32FromF64U>(a, -1.0, 4294967296.0)?;
        self.state[dst].set_u32(a as u32);
        ControlFlow::Continue(())
    }

    fn x64_from_f64_s(&mut self, dst: XReg, src: FReg) -> ControlFlow<Done> {
        let a = self.state[src].get_f64();
        self.check_xnn_from_fnn::<crate::X64FromF64S>(
            a,
            -9223372036854777856.0,
            9223372036854775808.0,
        )?;
        self.state[dst].set_i64(a as i64);
        ControlFlow::Continue(())
    }

    fn x64_from_f64_u(&mut self, dst: XReg, src: FReg) -> ControlFlow<Done> {
        let a = self.state[src].get_f64();
        self.check_xnn_from_fnn::<crate::X64FromF64U>(a, -1.0, 18446744073709551616.0)?;
        self.state[dst].set_u64(a as u64);
        ControlFlow::Continue(())
    }

    fn x32_from_f32_s_sat(&mut self, dst: XReg, src: FReg) -> ControlFlow<Done> {
        let a = self.state[src].get_f32();
        self.state[dst].set_i32(a as i32);
        ControlFlow::Continue(())
    }

    fn x32_from_f32_u_sat(&mut self, dst: XReg, src: FReg) -> ControlFlow<Done> {
        let a = self.state[src].get_f32();
        self.state[dst].set_u32(a as u32);
        ControlFlow::Continue(())
    }

    fn x64_from_f32_s_sat(&mut self, dst: XReg, src: FReg) -> ControlFlow<Done> {
        let a = self.state[src].get_f32();
        self.state[dst].set_i64(a as i64);
        ControlFlow::Continue(())
    }

    fn x64_from_f32_u_sat(&mut self, dst: XReg, src: FReg) -> ControlFlow<Done> {
        let a = self.state[src].get_f32();
        self.state[dst].set_u64(a as u64);
        ControlFlow::Continue(())
    }

    fn x32_from_f64_s_sat(&mut self, dst: XReg, src: FReg) -> ControlFlow<Done> {
        let a = self.state[src].get_f64();
        self.state[dst].set_i32(a as i32);
        ControlFlow::Continue(())
    }

    fn x32_from_f64_u_sat(&mut self, dst: XReg, src: FReg) -> ControlFlow<Done> {
        let a = self.state[src].get_f64();
        self.state[dst].set_u32(a as u32);
        ControlFlow::Continue(())
    }

    fn x64_from_f64_s_sat(&mut self, dst: XReg, src: FReg) -> ControlFlow<Done> {
        let a = self.state[src].get_f64();
        self.state[dst].set_i64(a as i64);
        ControlFlow::Continue(())
    }

    fn x64_from_f64_u_sat(&mut self, dst: XReg, src: FReg) -> ControlFlow<Done> {
        let a = self.state[src].get_f64();
        self.state[dst].set_u64(a as u64);
        ControlFlow::Continue(())
    }

    fn f32_from_f64(&mut self, dst: FReg, src: FReg) -> ControlFlow<Done> {
        let a = self.state[src].get_f64();
        self.state[dst].set_f32(a as f32);
        ControlFlow::Continue(())
    }

    fn f64_from_f32(&mut self, dst: FReg, src: FReg) -> ControlFlow<Done> {
        let a = self.state[src].get_f32();
        self.state[dst].set_f64(a.into());
        ControlFlow::Continue(())
    }

    fn fcopysign32(&mut self, operands: BinaryOperands<FReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_f32();
        let b = self.state[operands.src2].get_f32();
        self.state[operands.dst].set_f32(a.wasm_copysign(b));
        ControlFlow::Continue(())
    }

    fn fcopysign64(&mut self, operands: BinaryOperands<FReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_f64();
        let b = self.state[operands.src2].get_f64();
        self.state[operands.dst].set_f64(a.wasm_copysign(b));
        ControlFlow::Continue(())
    }

    fn fadd32(&mut self, operands: BinaryOperands<FReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_f32();
        let b = self.state[operands.src2].get_f32();
        self.state[operands.dst].set_f32(a + b);
        ControlFlow::Continue(())
    }

    fn fsub32(&mut self, operands: BinaryOperands<FReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_f32();
        let b = self.state[operands.src2].get_f32();
        self.state[operands.dst].set_f32(a - b);
        ControlFlow::Continue(())
    }

    fn fmul32(&mut self, operands: BinaryOperands<FReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_f32();
        let b = self.state[operands.src2].get_f32();
        self.state[operands.dst].set_f32(a * b);
        ControlFlow::Continue(())
    }

    fn fdiv32(&mut self, operands: BinaryOperands<FReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_f32();
        let b = self.state[operands.src2].get_f32();
        self.state[operands.dst].set_f32(a / b);
        ControlFlow::Continue(())
    }

    fn fmaximum32(&mut self, operands: BinaryOperands<FReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_f32();
        let b = self.state[operands.src2].get_f32();
        self.state[operands.dst].set_f32(a.wasm_maximum(b));
        ControlFlow::Continue(())
    }

    fn fminimum32(&mut self, operands: BinaryOperands<FReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_f32();
        let b = self.state[operands.src2].get_f32();
        self.state[operands.dst].set_f32(a.wasm_minimum(b));
        ControlFlow::Continue(())
    }

    fn ftrunc32(&mut self, dst: FReg, src: FReg) -> ControlFlow<Done> {
        let a = self.state[src].get_f32();
        self.state[dst].set_f32(a.wasm_trunc());
        ControlFlow::Continue(())
    }

    fn ffloor32(&mut self, dst: FReg, src: FReg) -> ControlFlow<Done> {
        let a = self.state[src].get_f32();
        self.state[dst].set_f32(a.wasm_floor());
        ControlFlow::Continue(())
    }

    fn fceil32(&mut self, dst: FReg, src: FReg) -> ControlFlow<Done> {
        let a = self.state[src].get_f32();
        self.state[dst].set_f32(a.wasm_ceil());
        ControlFlow::Continue(())
    }

    fn fnearest32(&mut self, dst: FReg, src: FReg) -> ControlFlow<Done> {
        let a = self.state[src].get_f32();
        self.state[dst].set_f32(a.wasm_nearest());
        ControlFlow::Continue(())
    }

    fn fsqrt32(&mut self, dst: FReg, src: FReg) -> ControlFlow<Done> {
        let a = self.state[src].get_f32();
        self.state[dst].set_f32(a.wasm_sqrt());
        ControlFlow::Continue(())
    }

    fn fneg32(&mut self, dst: FReg, src: FReg) -> ControlFlow<Done> {
        let a = self.state[src].get_f32();
        self.state[dst].set_f32(-a);
        ControlFlow::Continue(())
    }

    fn fabs32(&mut self, dst: FReg, src: FReg) -> ControlFlow<Done> {
        let a = self.state[src].get_f32();
        self.state[dst].set_f32(a.wasm_abs());
        ControlFlow::Continue(())
    }

    fn fadd64(&mut self, operands: BinaryOperands<FReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_f64();
        let b = self.state[operands.src2].get_f64();
        self.state[operands.dst].set_f64(a + b);
        ControlFlow::Continue(())
    }

    fn fsub64(&mut self, operands: BinaryOperands<FReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_f64();
        let b = self.state[operands.src2].get_f64();
        self.state[operands.dst].set_f64(a - b);
        ControlFlow::Continue(())
    }

    fn fmul64(&mut self, operands: BinaryOperands<FReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_f64();
        let b = self.state[operands.src2].get_f64();
        self.state[operands.dst].set_f64(a * b);
        ControlFlow::Continue(())
    }

    fn fdiv64(&mut self, operands: BinaryOperands<FReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_f64();
        let b = self.state[operands.src2].get_f64();
        self.state[operands.dst].set_f64(a / b);
        ControlFlow::Continue(())
    }

    fn fmaximum64(&mut self, operands: BinaryOperands<FReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_f64();
        let b = self.state[operands.src2].get_f64();
        self.state[operands.dst].set_f64(a.wasm_maximum(b));
        ControlFlow::Continue(())
    }

    fn fminimum64(&mut self, operands: BinaryOperands<FReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_f64();
        let b = self.state[operands.src2].get_f64();
        self.state[operands.dst].set_f64(a.wasm_minimum(b));
        ControlFlow::Continue(())
    }

    fn ftrunc64(&mut self, dst: FReg, src: FReg) -> ControlFlow<Done> {
        let a = self.state[src].get_f64();
        self.state[dst].set_f64(a.wasm_trunc());
        ControlFlow::Continue(())
    }

    fn ffloor64(&mut self, dst: FReg, src: FReg) -> ControlFlow<Done> {
        let a = self.state[src].get_f64();
        self.state[dst].set_f64(a.wasm_floor());
        ControlFlow::Continue(())
    }

    fn fceil64(&mut self, dst: FReg, src: FReg) -> ControlFlow<Done> {
        let a = self.state[src].get_f64();
        self.state[dst].set_f64(a.wasm_ceil());
        ControlFlow::Continue(())
    }

    fn fnearest64(&mut self, dst: FReg, src: FReg) -> ControlFlow<Done> {
        let a = self.state[src].get_f64();
        self.state[dst].set_f64(a.wasm_nearest());
        ControlFlow::Continue(())
    }

    fn fsqrt64(&mut self, dst: FReg, src: FReg) -> ControlFlow<Done> {
        let a = self.state[src].get_f64();
        self.state[dst].set_f64(a.wasm_sqrt());
        ControlFlow::Continue(())
    }

    fn fneg64(&mut self, dst: FReg, src: FReg) -> ControlFlow<Done> {
        let a = self.state[src].get_f64();
        self.state[dst].set_f64(-a);
        ControlFlow::Continue(())
    }

    fn fabs64(&mut self, dst: FReg, src: FReg) -> ControlFlow<Done> {
        let a = self.state[src].get_f64();
        self.state[dst].set_f64(a.wasm_abs());
        ControlFlow::Continue(())
    }

    fn vaddi8x16(&mut self, operands: BinaryOperands<VReg>) -> ControlFlow<Done> {
        let mut a = self.state[operands.src1].get_i8x16();
        let b = self.state[operands.src2].get_i8x16();
        for (a, b) in a.iter_mut().zip(b) {
            *a = a.wrapping_add(b);
        }
        self.state[operands.dst].set_i8x16(a);
        ControlFlow::Continue(())
    }

    fn vaddi16x8(&mut self, operands: BinaryOperands<VReg>) -> ControlFlow<Done> {
        let mut a = self.state[operands.src1].get_i16x8();
        let b = self.state[operands.src2].get_i16x8();
        for (a, b) in a.iter_mut().zip(b) {
            *a = a.wrapping_add(b);
        }
        self.state[operands.dst].set_i16x8(a);
        ControlFlow::Continue(())
    }

    fn vaddi32x4(&mut self, operands: BinaryOperands<VReg>) -> ControlFlow<Done> {
        let mut a = self.state[operands.src1].get_i32x4();
        let b = self.state[operands.src2].get_i32x4();
        for (a, b) in a.iter_mut().zip(b) {
            *a = a.wrapping_add(b);
        }
        self.state[operands.dst].set_i32x4(a);
        ControlFlow::Continue(())
    }

    fn vaddi64x2(&mut self, operands: BinaryOperands<VReg>) -> ControlFlow<Done> {
        let mut a = self.state[operands.src1].get_i64x2();
        let b = self.state[operands.src2].get_i64x2();
        for (a, b) in a.iter_mut().zip(b) {
            *a = a.wrapping_add(b);
        }
        self.state[operands.dst].set_i64x2(a);
        ControlFlow::Continue(())
    }

    fn vaddf32x4(&mut self, operands: BinaryOperands<VReg>) -> ControlFlow<Done> {
        let mut a = self.state[operands.src1].get_f32x4();
        let b = self.state[operands.src2].get_f32x4();
        for (a, b) in a.iter_mut().zip(b) {
            *a += b;
        }
        self.state[operands.dst].set_f32x4(a);
        ControlFlow::Continue(())
    }

    fn vaddf64x2(&mut self, operands: BinaryOperands<VReg>) -> ControlFlow<Done> {
        let mut a = self.state[operands.src1].get_f64x2();
        let b = self.state[operands.src2].get_f64x2();
        for (a, b) in a.iter_mut().zip(b) {
            *a += b;
        }
        self.state[operands.dst].set_f64x2(a);
        ControlFlow::Continue(())
    }

    fn vshli8x16(&mut self, operands: BinaryOperands<VReg, VReg, XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_i8x16();
        let b = self.state[operands.src2].get_u32();
        self.state[operands.dst].set_i8x16(a.map(|a| a.wrapping_shl(b)));
        ControlFlow::Continue(())
    }

    fn vshli16x8(&mut self, operands: BinaryOperands<VReg, VReg, XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_i16x8();
        let b = self.state[operands.src2].get_u32();
        self.state[operands.dst].set_i16x8(a.map(|a| a.wrapping_shl(b)));
        ControlFlow::Continue(())
    }

    fn vshli32x4(&mut self, operands: BinaryOperands<VReg, VReg, XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_i32x4();
        let b = self.state[operands.src2].get_u32();
        self.state[operands.dst].set_i32x4(a.map(|a| a.wrapping_shl(b)));
        ControlFlow::Continue(())
    }

    fn vshli64x2(&mut self, operands: BinaryOperands<VReg, VReg, XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_i64x2();
        let b = self.state[operands.src2].get_u32();
        self.state[operands.dst].set_i64x2(a.map(|a| a.wrapping_shl(b)));
        ControlFlow::Continue(())
    }

    fn vshri8x16_s(&mut self, operands: BinaryOperands<VReg, VReg, XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_i8x16();
        let b = self.state[operands.src2].get_u32();
        self.state[operands.dst].set_i8x16(a.map(|a| a.wrapping_shr(b)));
        ControlFlow::Continue(())
    }

    fn vshri16x8_s(&mut self, operands: BinaryOperands<VReg, VReg, XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_i16x8();
        let b = self.state[operands.src2].get_u32();
        self.state[operands.dst].set_i16x8(a.map(|a| a.wrapping_shr(b)));
        ControlFlow::Continue(())
    }

    fn vshri32x4_s(&mut self, operands: BinaryOperands<VReg, VReg, XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_i32x4();
        let b = self.state[operands.src2].get_u32();
        self.state[operands.dst].set_i32x4(a.map(|a| a.wrapping_shr(b)));
        ControlFlow::Continue(())
    }

    fn vshri64x2_s(&mut self, operands: BinaryOperands<VReg, VReg, XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_i64x2();
        let b = self.state[operands.src2].get_u32();
        self.state[operands.dst].set_i64x2(a.map(|a| a.wrapping_shr(b)));
        ControlFlow::Continue(())
    }

    fn vshri8x16_u(&mut self, operands: BinaryOperands<VReg, VReg, XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u8x16();
        let b = self.state[operands.src2].get_u32();
        self.state[operands.dst].set_u8x16(a.map(|a| a.wrapping_shr(b)));
        ControlFlow::Continue(())
    }

    fn vshri16x8_u(&mut self, operands: BinaryOperands<VReg, VReg, XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u16x8();
        let b = self.state[operands.src2].get_u32();
        self.state[operands.dst].set_u16x8(a.map(|a| a.wrapping_shr(b)));
        ControlFlow::Continue(())
    }

    fn vshri32x4_u(&mut self, operands: BinaryOperands<VReg, VReg, XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u32x4();
        let b = self.state[operands.src2].get_u32();
        self.state[operands.dst].set_u32x4(a.map(|a| a.wrapping_shr(b)));
        ControlFlow::Continue(())
    }

    fn vshri64x2_u(&mut self, operands: BinaryOperands<VReg, VReg, XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u64x2();
        let b = self.state[operands.src2].get_u32();
        self.state[operands.dst].set_u64x2(a.map(|a| a.wrapping_shr(b)));
        ControlFlow::Continue(())
    }

    fn vconst128(&mut self, dst: VReg, val: u128) -> ControlFlow<Done> {
        self.state[dst].set_u128(val);
        ControlFlow::Continue(())
    }

    fn vsplatx8(&mut self, dst: VReg, src: XReg) -> ControlFlow<Done> {
        let val = self.state[src].get_u32() as u8;
        self.state[dst].set_u8x16([val; 16]);
        ControlFlow::Continue(())
    }

    fn vsplatx16(&mut self, dst: VReg, src: XReg) -> ControlFlow<Done> {
        let val = self.state[src].get_u32() as u16;
        self.state[dst].set_u16x8([val; 8]);
        ControlFlow::Continue(())
    }

    fn vsplatx32(&mut self, dst: VReg, src: XReg) -> ControlFlow<Done> {
        let val = self.state[src].get_u32();
        self.state[dst].set_u32x4([val; 4]);
        ControlFlow::Continue(())
    }

    fn vsplatx64(&mut self, dst: VReg, src: XReg) -> ControlFlow<Done> {
        let val = self.state[src].get_u64();
        self.state[dst].set_u64x2([val; 2]);
        ControlFlow::Continue(())
    }

    fn vsplatf32(&mut self, dst: VReg, src: FReg) -> ControlFlow<Done> {
        let val = self.state[src].get_f32();
        self.state[dst].set_f32x4([val; 4]);
        ControlFlow::Continue(())
    }

    fn vsplatf64(&mut self, dst: VReg, src: FReg) -> ControlFlow<Done> {
        let val = self.state[src].get_f64();
        self.state[dst].set_f64x2([val; 2]);
        ControlFlow::Continue(())
    }

    fn vload8x8_s_offset32(&mut self, dst: VReg, ptr: XReg, offset: i32) -> ControlFlow<Done> {
        let val = unsafe { self.load::<[i8; 8]>(ptr, offset) };
        self.state[dst].set_i16x8(val.map(|i| i.into()));
        ControlFlow::Continue(())
    }

    fn vload8x8_u_offset32(&mut self, dst: VReg, ptr: XReg, offset: i32) -> ControlFlow<Done> {
        let val = unsafe { self.load::<[u8; 8]>(ptr, offset) };
        self.state[dst].set_u16x8(val.map(|i| i.into()));
        ControlFlow::Continue(())
    }

    fn vload16x4le_s_offset32(&mut self, dst: VReg, ptr: XReg, offset: i32) -> ControlFlow<Done> {
        let val = unsafe { self.load::<[i16; 4]>(ptr, offset) };
        self.state[dst].set_i32x4(val.map(|i| i16::from_le(i).into()));
        ControlFlow::Continue(())
    }

    fn vload16x4le_u_offset32(&mut self, dst: VReg, ptr: XReg, offset: i32) -> ControlFlow<Done> {
        let val = unsafe { self.load::<[u16; 4]>(ptr, offset) };
        self.state[dst].set_u32x4(val.map(|i| u16::from_le(i).into()));
        ControlFlow::Continue(())
    }

    fn vload32x2le_s_offset32(&mut self, dst: VReg, ptr: XReg, offset: i32) -> ControlFlow<Done> {
        let val = unsafe { self.load::<[i32; 2]>(ptr, offset) };
        self.state[dst].set_i64x2(val.map(|i| i32::from_le(i).into()));
        ControlFlow::Continue(())
    }

    fn vload32x2le_u_offset32(&mut self, dst: VReg, ptr: XReg, offset: i32) -> ControlFlow<Done> {
        let val = unsafe { self.load::<[u32; 2]>(ptr, offset) };
        self.state[dst].set_u64x2(val.map(|i| u32::from_le(i).into()));
        ControlFlow::Continue(())
    }

    fn vband128(&mut self, operands: BinaryOperands<VReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u128();
        let b = self.state[operands.src2].get_u128();
        self.state[operands.dst].set_u128(a & b);
        ControlFlow::Continue(())
    }

    fn vbor128(&mut self, operands: BinaryOperands<VReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u128();
        let b = self.state[operands.src2].get_u128();
        self.state[operands.dst].set_u128(a | b);
        ControlFlow::Continue(())
    }

    fn vbxor128(&mut self, operands: BinaryOperands<VReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u128();
        let b = self.state[operands.src2].get_u128();
        self.state[operands.dst].set_u128(a ^ b);
        ControlFlow::Continue(())
    }

    fn vbnot128(&mut self, dst: VReg, src: VReg) -> ControlFlow<Done> {
        let a = self.state[src].get_u128();
        self.state[dst].set_u128(!a);
        ControlFlow::Continue(())
    }

    fn vbitselect128(&mut self, dst: VReg, c: VReg, x: VReg, y: VReg) -> ControlFlow<Done> {
        let c = self.state[c].get_u128();
        let x = self.state[x].get_u128();
        let y = self.state[y].get_u128();
        self.state[dst].set_u128((c & x) | (!c & y));
        ControlFlow::Continue(())
    }

    fn vbitmask8x16(&mut self, dst: XReg, src: VReg) -> ControlFlow<Done> {
        let a = self.state[src].get_u8x16();
        let mut result = 0;
        for item in a.iter().rev() {
            result <<= 1;
            result |= (*item >> 7) as u32;
        }
        self.state[dst].set_u32(result);
        ControlFlow::Continue(())
    }

    fn vbitmask16x8(&mut self, dst: XReg, src: VReg) -> ControlFlow<Done> {
        let a = self.state[src].get_u16x8();
        let mut result = 0;
        for item in a.iter().rev() {
            result <<= 1;
            result |= (*item >> 15) as u32;
        }
        self.state[dst].set_u32(result);
        ControlFlow::Continue(())
    }

    fn vbitmask32x4(&mut self, dst: XReg, src: VReg) -> ControlFlow<Done> {
        let a = self.state[src].get_u32x4();
        let mut result = 0;
        for item in a.iter().rev() {
            result <<= 1;
            result |= *item >> 31;
        }
        self.state[dst].set_u32(result);
        ControlFlow::Continue(())
    }

    fn vbitmask64x2(&mut self, dst: XReg, src: VReg) -> ControlFlow<Done> {
        let a = self.state[src].get_u64x2();
        let mut result = 0;
        for item in a.iter().rev() {
            result <<= 1;
            result |= (*item >> 63) as u32;
        }
        self.state[dst].set_u32(result);
        ControlFlow::Continue(())
    }

    fn valltrue8x16(&mut self, dst: XReg, src: VReg) -> ControlFlow<Done> {
        let a = self.state[src].get_u8x16();
        let result = a.iter().all(|a| *a != 0);
        self.state[dst].set_u32(u32::from(result));
        ControlFlow::Continue(())
    }

    fn valltrue16x8(&mut self, dst: XReg, src: VReg) -> ControlFlow<Done> {
        let a = self.state[src].get_u16x8();
        let result = a.iter().all(|a| *a != 0);
        self.state[dst].set_u32(u32::from(result));
        ControlFlow::Continue(())
    }

    fn valltrue32x4(&mut self, dst: XReg, src: VReg) -> ControlFlow<Done> {
        let a = self.state[src].get_u32x4();
        let result = a.iter().all(|a| *a != 0);
        self.state[dst].set_u32(u32::from(result));
        ControlFlow::Continue(())
    }

    fn valltrue64x2(&mut self, dst: XReg, src: VReg) -> ControlFlow<Done> {
        let a = self.state[src].get_u64x2();
        let result = a.iter().all(|a| *a != 0);
        self.state[dst].set_u32(u32::from(result));
        ControlFlow::Continue(())
    }

    fn vanytrue8x16(&mut self, dst: XReg, src: VReg) -> ControlFlow<Done> {
        let a = self.state[src].get_u8x16();
        let result = a.iter().any(|a| *a != 0);
        self.state[dst].set_u32(u32::from(result));
        ControlFlow::Continue(())
    }

    fn vanytrue16x8(&mut self, dst: XReg, src: VReg) -> ControlFlow<Done> {
        let a = self.state[src].get_u16x8();
        let result = a.iter().any(|a| *a != 0);
        self.state[dst].set_u32(u32::from(result));
        ControlFlow::Continue(())
    }

    fn vanytrue32x4(&mut self, dst: XReg, src: VReg) -> ControlFlow<Done> {
        let a = self.state[src].get_u32x4();
        let result = a.iter().any(|a| *a != 0);
        self.state[dst].set_u32(u32::from(result));
        ControlFlow::Continue(())
    }

    fn vanytrue64x2(&mut self, dst: XReg, src: VReg) -> ControlFlow<Done> {
        let a = self.state[src].get_u64x2();
        let result = a.iter().any(|a| *a != 0);
        self.state[dst].set_u32(u32::from(result));
        ControlFlow::Continue(())
    }
}

impl ExtendedOpVisitor for Interpreter<'_> {
    fn nop(&mut self) -> ControlFlow<Done> {
        ControlFlow::Continue(())
    }

    fn trap(&mut self) -> ControlFlow<Done> {
        self.done_trap::<crate::Trap>()
    }

    fn call_indirect_host(&mut self, id: u8) -> ControlFlow<Done> {
        self.done_call_indirect_host(id)
    }

    fn bswap32(&mut self, dst: XReg, src: XReg) -> ControlFlow<Done> {
        let src = self.state[src].get_u32();
        self.state[dst].set_u32(src.swap_bytes());
        ControlFlow::Continue(())
    }

    fn bswap64(&mut self, dst: XReg, src: XReg) -> ControlFlow<Done> {
        let src = self.state[src].get_u64();
        self.state[dst].set_u64(src.swap_bytes());
        ControlFlow::Continue(())
    }

    fn xload16be_u64_offset32(&mut self, dst: XReg, ptr: XReg, offset: i32) -> ControlFlow<Done> {
        let val = unsafe { self.load::<u16>(ptr, offset) };
        self.state[dst].set_u64(u16::from_be(val).into());
        ControlFlow::Continue(())
    }

    fn xload16be_s64_offset32(&mut self, dst: XReg, ptr: XReg, offset: i32) -> ControlFlow<Done> {
        let val = unsafe { self.load::<i16>(ptr, offset) };
        self.state[dst].set_i64(i16::from_be(val).into());
        ControlFlow::Continue(())
    }

    fn xload32be_u64_offset32(&mut self, dst: XReg, ptr: XReg, offset: i32) -> ControlFlow<Done> {
        let val = unsafe { self.load::<u32>(ptr, offset) };
        self.state[dst].set_u64(u32::from_be(val).into());
        ControlFlow::Continue(())
    }

    fn xload32be_s64_offset32(&mut self, dst: XReg, ptr: XReg, offset: i32) -> ControlFlow<Done> {
        let val = unsafe { self.load::<i32>(ptr, offset) };
        self.state[dst].set_i64(i32::from_be(val).into());
        ControlFlow::Continue(())
    }

    fn xload64be_offset32(&mut self, dst: XReg, ptr: XReg, offset: i32) -> ControlFlow<Done> {
        let val = unsafe { self.load::<i64>(ptr, offset) };
        self.state[dst].set_i64(i64::from_be(val));
        ControlFlow::Continue(())
    }

    fn xstore16be_offset32(&mut self, ptr: XReg, offset: i32, src: XReg) -> ControlFlow<Done> {
        let val = self.state[src].get_u32() as u16;
        unsafe {
            self.store(ptr, offset, val.to_be());
        }
        ControlFlow::Continue(())
    }

    fn xstore32be_offset32(&mut self, ptr: XReg, offset: i32, src: XReg) -> ControlFlow<Done> {
        let val = self.state[src].get_u32();
        unsafe {
            self.store(ptr, offset, val.to_be());
        }
        ControlFlow::Continue(())
    }

    fn xstore64be_offset32(&mut self, ptr: XReg, offset: i32, src: XReg) -> ControlFlow<Done> {
        let val = self.state[src].get_u64();
        unsafe {
            self.store(ptr, offset, val.to_be());
        }
        ControlFlow::Continue(())
    }

    fn fload32be_offset32(&mut self, dst: FReg, ptr: XReg, offset: i32) -> ControlFlow<Done> {
        let val = unsafe { self.load::<u32>(ptr, offset) };
        self.state[dst].set_f32(f32::from_bits(u32::from_be(val)));
        ControlFlow::Continue(())
    }

    fn fload64be_offset32(&mut self, dst: FReg, ptr: XReg, offset: i32) -> ControlFlow<Done> {
        let val = unsafe { self.load::<u64>(ptr, offset) };
        self.state[dst].set_f64(f64::from_bits(u64::from_be(val)));
        ControlFlow::Continue(())
    }

    fn fstore32be_offset32(&mut self, ptr: XReg, offset: i32, src: FReg) -> ControlFlow<Done> {
        let val = self.state[src].get_f32();
        unsafe {
            self.store(ptr, offset, val.to_bits().to_be());
        }
        ControlFlow::Continue(())
    }

    fn fstore64be_offset32(&mut self, ptr: XReg, offset: i32, src: FReg) -> ControlFlow<Done> {
        let val = self.state[src].get_f64();
        unsafe {
            self.store(ptr, offset, val.to_bits().to_be());
        }
        ControlFlow::Continue(())
    }

    fn xmov_fp(&mut self, dst: XReg) -> ControlFlow<Done> {
        let fp = self.state.fp;
        self.state[dst].set_ptr(fp);
        ControlFlow::Continue(())
    }

    fn xmov_lr(&mut self, dst: XReg) -> ControlFlow<Done> {
        let lr = self.state.lr;
        self.state[dst].set_ptr(lr);
        ControlFlow::Continue(())
    }

    fn xabs32(&mut self, dst: XReg, src: XReg) -> ControlFlow<Done> {
        let a = self.state[src].get_i32();
        self.state[dst].set_i32(a.wrapping_abs());
        ControlFlow::Continue(())
    }

    fn xabs64(&mut self, dst: XReg, src: XReg) -> ControlFlow<Done> {
        let a = self.state[src].get_i64();
        self.state[dst].set_i64(a.wrapping_abs());
        ControlFlow::Continue(())
    }

    fn vf32x4_from_i32x4_s(&mut self, dst: VReg, src: VReg) -> ControlFlow<Done> {
        let a = self.state[src].get_i32x4();
        self.state[dst].set_f32x4(a.map(|i| i as f32));
        ControlFlow::Continue(())
    }

    fn vf32x4_from_i32x4_u(&mut self, dst: VReg, src: VReg) -> ControlFlow<Done> {
        let a = self.state[src].get_u32x4();
        self.state[dst].set_f32x4(a.map(|i| i as f32));
        ControlFlow::Continue(())
    }

    fn vf64x2_from_i64x2_s(&mut self, dst: VReg, src: VReg) -> ControlFlow<Done> {
        let a = self.state[src].get_i64x2();
        self.state[dst].set_f64x2(a.map(|i| i as f64));
        ControlFlow::Continue(())
    }

    fn vf64x2_from_i64x2_u(&mut self, dst: VReg, src: VReg) -> ControlFlow<Done> {
        let a = self.state[src].get_u64x2();
        self.state[dst].set_f64x2(a.map(|i| i as f64));
        ControlFlow::Continue(())
    }

    fn vwidenlow8x16_s(&mut self, dst: VReg, src: VReg) -> ControlFlow<Done> {
        let a = *self.state[src].get_i8x16().first_chunk().unwrap();
        self.state[dst].set_i16x8(a.map(|i| i.into()));
        ControlFlow::Continue(())
    }

    fn vwidenlow8x16_u(&mut self, dst: VReg, src: VReg) -> ControlFlow<Done> {
        let a = *self.state[src].get_u8x16().first_chunk().unwrap();
        self.state[dst].set_u16x8(a.map(|i| i.into()));
        ControlFlow::Continue(())
    }

    fn vwidenlow16x8_s(&mut self, dst: VReg, src: VReg) -> ControlFlow<Done> {
        let a = *self.state[src].get_i16x8().first_chunk().unwrap();
        self.state[dst].set_i32x4(a.map(|i| i.into()));
        ControlFlow::Continue(())
    }

    fn vwidenlow16x8_u(&mut self, dst: VReg, src: VReg) -> ControlFlow<Done> {
        let a = *self.state[src].get_u16x8().first_chunk().unwrap();
        self.state[dst].set_u32x4(a.map(|i| i.into()));
        ControlFlow::Continue(())
    }

    fn vwidenlow32x4_s(&mut self, dst: VReg, src: VReg) -> ControlFlow<Done> {
        let a = *self.state[src].get_i32x4().first_chunk().unwrap();
        self.state[dst].set_i64x2(a.map(|i| i.into()));
        ControlFlow::Continue(())
    }

    fn vwidenlow32x4_u(&mut self, dst: VReg, src: VReg) -> ControlFlow<Done> {
        let a = *self.state[src].get_u32x4().first_chunk().unwrap();
        self.state[dst].set_u64x2(a.map(|i| i.into()));
        ControlFlow::Continue(())
    }

    fn vwidenhigh8x16_s(&mut self, dst: VReg, src: VReg) -> ControlFlow<Done> {
        let a = *self.state[src].get_i8x16().last_chunk().unwrap();
        self.state[dst].set_i16x8(a.map(|i| i.into()));
        ControlFlow::Continue(())
    }

    fn vwidenhigh8x16_u(&mut self, dst: VReg, src: VReg) -> ControlFlow<Done> {
        let a = *self.state[src].get_u8x16().last_chunk().unwrap();
        self.state[dst].set_u16x8(a.map(|i| i.into()));
        ControlFlow::Continue(())
    }

    fn vwidenhigh16x8_s(&mut self, dst: VReg, src: VReg) -> ControlFlow<Done> {
        let a = *self.state[src].get_i16x8().last_chunk().unwrap();
        self.state[dst].set_i32x4(a.map(|i| i.into()));
        ControlFlow::Continue(())
    }

    fn vwidenhigh16x8_u(&mut self, dst: VReg, src: VReg) -> ControlFlow<Done> {
        let a = *self.state[src].get_u16x8().last_chunk().unwrap();
        self.state[dst].set_u32x4(a.map(|i| i.into()));
        ControlFlow::Continue(())
    }

    fn vwidenhigh32x4_s(&mut self, dst: VReg, src: VReg) -> ControlFlow<Done> {
        let a = *self.state[src].get_i32x4().last_chunk().unwrap();
        self.state[dst].set_i64x2(a.map(|i| i.into()));
        ControlFlow::Continue(())
    }

    fn vwidenhigh32x4_u(&mut self, dst: VReg, src: VReg) -> ControlFlow<Done> {
        let a = *self.state[src].get_u32x4().last_chunk().unwrap();
        self.state[dst].set_u64x2(a.map(|i| i.into()));
        ControlFlow::Continue(())
    }

    fn vnarrow16x8_s(&mut self, operands: BinaryOperands<VReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_i16x8();
        let b = self.state[operands.src2].get_i16x8();
        let mut result = [0; 16];
        for (i, d) in a.iter().chain(&b).zip(&mut result) {
            *d = (*i)
                .try_into()
                .unwrap_or(if *i < 0 { i8::MIN } else { i8::MAX });
        }
        self.state[operands.dst].set_i8x16(result);
        ControlFlow::Continue(())
    }

    fn vnarrow16x8_u(&mut self, operands: BinaryOperands<VReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_i16x8();
        let b = self.state[operands.src2].get_i16x8();
        let mut result = [0; 16];
        for (i, d) in a.iter().chain(&b).zip(&mut result) {
            *d = (*i)
                .try_into()
                .unwrap_or(if *i < 0 { u8::MIN } else { u8::MAX });
        }
        self.state[operands.dst].set_u8x16(result);
        ControlFlow::Continue(())
    }

    fn vnarrow32x4_s(&mut self, operands: BinaryOperands<VReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_i32x4();
        let b = self.state[operands.src2].get_i32x4();
        let mut result = [0; 8];
        for (i, d) in a.iter().chain(&b).zip(&mut result) {
            *d = (*i)
                .try_into()
                .unwrap_or(if *i < 0 { i16::MIN } else { i16::MAX });
        }
        self.state[operands.dst].set_i16x8(result);
        ControlFlow::Continue(())
    }

    fn vnarrow32x4_u(&mut self, operands: BinaryOperands<VReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_i32x4();
        let b = self.state[operands.src2].get_i32x4();
        let mut result = [0; 8];
        for (i, d) in a.iter().chain(&b).zip(&mut result) {
            *d = (*i)
                .try_into()
                .unwrap_or(if *i < 0 { u16::MIN } else { u16::MAX });
        }
        self.state[operands.dst].set_u16x8(result);
        ControlFlow::Continue(())
    }

    fn vfpromotelow(&mut self, dst: VReg, src: VReg) -> ControlFlow<Done> {
        let a = self.state[src].get_f32x4();
        self.state[dst].set_f64x2([a[0].into(), a[1].into()]);
        ControlFlow::Continue(())
    }

    fn vfdemote(&mut self, dst: VReg, src: VReg) -> ControlFlow<Done> {
        let a = self.state[src].get_f64x2();
        self.state[dst].set_f32x4([a[0] as f32, a[1] as f32, 0.0, 0.0]);
        ControlFlow::Continue(())
    }

    fn vsubi8x16(&mut self, operands: BinaryOperands<VReg>) -> ControlFlow<Done> {
        let mut a = self.state[operands.src1].get_i8x16();
        let b = self.state[operands.src2].get_i8x16();
        for (a, b) in a.iter_mut().zip(b) {
            *a = a.wrapping_sub(b);
        }
        self.state[operands.dst].set_i8x16(a);
        ControlFlow::Continue(())
    }

    fn vsubi16x8(&mut self, operands: BinaryOperands<VReg>) -> ControlFlow<Done> {
        let mut a = self.state[operands.src1].get_i16x8();
        let b = self.state[operands.src2].get_i16x8();
        for (a, b) in a.iter_mut().zip(b) {
            *a = a.wrapping_sub(b);
        }
        self.state[operands.dst].set_i16x8(a);
        ControlFlow::Continue(())
    }

    fn vsubi32x4(&mut self, operands: BinaryOperands<VReg>) -> ControlFlow<Done> {
        let mut a = self.state[operands.src1].get_i32x4();
        let b = self.state[operands.src2].get_i32x4();
        for (a, b) in a.iter_mut().zip(b) {
            *a = a.wrapping_sub(b);
        }
        self.state[operands.dst].set_i32x4(a);
        ControlFlow::Continue(())
    }

    fn vsubi64x2(&mut self, operands: BinaryOperands<VReg>) -> ControlFlow<Done> {
        let mut a = self.state[operands.src1].get_i64x2();
        let b = self.state[operands.src2].get_i64x2();
        for (a, b) in a.iter_mut().zip(b) {
            *a = a.wrapping_sub(b);
        }
        self.state[operands.dst].set_i64x2(a);
        ControlFlow::Continue(())
    }

    fn vmuli8x16(&mut self, operands: BinaryOperands<VReg>) -> ControlFlow<Done> {
        let mut a = self.state[operands.src1].get_i8x16();
        let b = self.state[operands.src2].get_i8x16();
        for (a, b) in a.iter_mut().zip(b) {
            *a = a.wrapping_mul(b);
        }
        self.state[operands.dst].set_i8x16(a);
        ControlFlow::Continue(())
    }

    fn vmuli16x8(&mut self, operands: BinaryOperands<VReg>) -> ControlFlow<Done> {
        let mut a = self.state[operands.src1].get_i16x8();
        let b = self.state[operands.src2].get_i16x8();
        for (a, b) in a.iter_mut().zip(b) {
            *a = a.wrapping_mul(b);
        }
        self.state[operands.dst].set_i16x8(a);
        ControlFlow::Continue(())
    }

    fn vmuli32x4(&mut self, operands: BinaryOperands<VReg>) -> ControlFlow<Done> {
        let mut a = self.state[operands.src1].get_i32x4();
        let b = self.state[operands.src2].get_i32x4();
        for (a, b) in a.iter_mut().zip(b) {
            *a = a.wrapping_mul(b);
        }
        self.state[operands.dst].set_i32x4(a);
        ControlFlow::Continue(())
    }

    fn vmuli64x2(&mut self, operands: BinaryOperands<VReg>) -> ControlFlow<Done> {
        let mut a = self.state[operands.src1].get_i64x2();
        let b = self.state[operands.src2].get_i64x2();
        for (a, b) in a.iter_mut().zip(b) {
            *a = a.wrapping_mul(b);
        }
        self.state[operands.dst].set_i64x2(a);
        ControlFlow::Continue(())
    }

    fn xextractv8x16(&mut self, dst: XReg, src: VReg, lane: u8) -> ControlFlow<Done> {
        let a = unsafe { *self.state[src].get_u8x16().get_unchecked(usize::from(lane)) };
        self.state[dst].set_u32(u32::from(a));
        ControlFlow::Continue(())
    }

    fn xextractv16x8(&mut self, dst: XReg, src: VReg, lane: u8) -> ControlFlow<Done> {
        let a = unsafe { *self.state[src].get_u16x8().get_unchecked(usize::from(lane)) };
        self.state[dst].set_u32(u32::from(a));
        ControlFlow::Continue(())
    }

    fn xextractv32x4(&mut self, dst: XReg, src: VReg, lane: u8) -> ControlFlow<Done> {
        let a = unsafe { *self.state[src].get_u32x4().get_unchecked(usize::from(lane)) };
        self.state[dst].set_u32(a);
        ControlFlow::Continue(())
    }

    fn xextractv64x2(&mut self, dst: XReg, src: VReg, lane: u8) -> ControlFlow<Done> {
        let a = unsafe { *self.state[src].get_u64x2().get_unchecked(usize::from(lane)) };
        self.state[dst].set_u64(a);
        ControlFlow::Continue(())
    }

    fn fextractv32x4(&mut self, dst: FReg, src: VReg, lane: u8) -> ControlFlow<Done> {
        let a = unsafe { *self.state[src].get_f32x4().get_unchecked(usize::from(lane)) };
        self.state[dst].set_f32(a);
        ControlFlow::Continue(())
    }

    fn fextractv64x2(&mut self, dst: FReg, src: VReg, lane: u8) -> ControlFlow<Done> {
        let a = unsafe { *self.state[src].get_f64x2().get_unchecked(usize::from(lane)) };
        self.state[dst].set_f64(a);
        ControlFlow::Continue(())
    }

    fn vinsertx8(
        &mut self,
        operands: BinaryOperands<VReg, VReg, XReg>,
        lane: u8,
    ) -> ControlFlow<Done> {
        let mut a = self.state[operands.src1].get_u8x16();
        let b = self.state[operands.src2].get_u32() as u8;
        unsafe {
            *a.get_unchecked_mut(usize::from(lane)) = b;
        }
        self.state[operands.dst].set_u8x16(a);
        ControlFlow::Continue(())
    }

    fn vinsertx16(
        &mut self,
        operands: BinaryOperands<VReg, VReg, XReg>,
        lane: u8,
    ) -> ControlFlow<Done> {
        let mut a = self.state[operands.src1].get_u16x8();
        let b = self.state[operands.src2].get_u32() as u16;
        unsafe {
            *a.get_unchecked_mut(usize::from(lane)) = b;
        }
        self.state[operands.dst].set_u16x8(a);
        ControlFlow::Continue(())
    }

    fn vinsertx32(
        &mut self,
        operands: BinaryOperands<VReg, VReg, XReg>,
        lane: u8,
    ) -> ControlFlow<Done> {
        let mut a = self.state[operands.src1].get_u32x4();
        let b = self.state[operands.src2].get_u32();
        unsafe {
            *a.get_unchecked_mut(usize::from(lane)) = b;
        }
        self.state[operands.dst].set_u32x4(a);
        ControlFlow::Continue(())
    }

    fn vinsertx64(
        &mut self,
        operands: BinaryOperands<VReg, VReg, XReg>,
        lane: u8,
    ) -> ControlFlow<Done> {
        let mut a = self.state[operands.src1].get_u64x2();
        let b = self.state[operands.src2].get_u64();
        unsafe {
            *a.get_unchecked_mut(usize::from(lane)) = b;
        }
        self.state[operands.dst].set_u64x2(a);
        ControlFlow::Continue(())
    }

    fn vinsertf32(
        &mut self,
        operands: BinaryOperands<VReg, VReg, FReg>,
        lane: u8,
    ) -> ControlFlow<Done> {
        let mut a = self.state[operands.src1].get_f32x4();
        let b = self.state[operands.src2].get_f32();
        unsafe {
            *a.get_unchecked_mut(usize::from(lane)) = b;
        }
        self.state[operands.dst].set_f32x4(a);
        ControlFlow::Continue(())
    }

    fn vinsertf64(
        &mut self,
        operands: BinaryOperands<VReg, VReg, FReg>,
        lane: u8,
    ) -> ControlFlow<Done> {
        let mut a = self.state[operands.src1].get_f64x2();
        let b = self.state[operands.src2].get_f64();
        unsafe {
            *a.get_unchecked_mut(usize::from(lane)) = b;
        }
        self.state[operands.dst].set_f64x2(a);
        ControlFlow::Continue(())
    }
}
