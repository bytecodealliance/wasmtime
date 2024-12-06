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
use core::ptr::{self, NonNull};
use sptr::Strict;

#[cfg(not(pulley_tail_calls))]
mod match_loop;
#[cfg(pulley_tail_calls)]
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
            DoneReason::Trap(pc) => DoneReason::Trap(pc),
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
    /// Sentinel return address that signals the end of the call stack.
    pub const HOST_RETURN_ADDR: Self = Self(XRegUnion { i64: -1 });

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

impl MachineState {
    fn with_stack(stack: Vec<u8>) -> Self {
        assert!(stack.len() > 0);
        let mut state = Self {
            x_regs: [Default::default(); XReg::RANGE.end as usize],
            f_regs: Default::default(),
            v_regs: Default::default(),
            stack,
            done_reason: None,
        };

        // Take care to construct SP such that we preserve pointer provenance
        // for the whole stack.
        let len = state.stack.len();
        let sp = &mut state.stack[..];
        let sp = sp.as_mut_ptr();
        let sp = unsafe { sp.add(len) };
        state[XReg::sp] = XRegVal::new_ptr(sp);
        state[XReg::fp] = XRegVal::HOST_RETURN_ADDR;
        state[XReg::lr] = XRegVal::HOST_RETURN_ADDR;

        state
    }
}

/// Inner private module to prevent creation of the `Done` structure outside of
/// this module.
mod done {
    use super::{Interpreter, MachineState};
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
        Trap(NonNull<u8>),
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
        pub fn done_trap(&mut self, pc: NonNull<u8>) -> Done {
            self.state.done_reason = Some(DoneReason::Trap(pc));
            Done { _priv: () }
        }

        /// Finishes execution by recording `DoneReason::CallIndirectHost`.
        pub fn done_call_indirect_host(&mut self, id: u8) -> Done {
            self.state.done_reason = Some(DoneReason::CallIndirectHost {
                id,
                resume: self.pc.as_ptr(),
            });
            Done { _priv: () }
        }

        /// Finishes execution by recording `DoneReason::ReturnToHost`.
        pub fn done_return_to_host(&mut self) -> Done {
            self.state.done_reason = Some(DoneReason::ReturnToHost(()));
            Done { _priv: () }
        }
    }
}

use done::Done;
pub use done::DoneReason;

struct Interpreter<'a> {
    state: &'a mut MachineState,
    pc: UnsafeBytecodeStream,
}

impl Interpreter<'_> {
    #[inline]
    fn pc_rel_jump(&mut self, offset: PcRelOffset, inst_size: isize) -> ControlFlow<Done> {
        let offset = isize::try_from(i32::from(offset)).unwrap();
        self.pc = unsafe { self.pc.offset(offset - inst_size) };
        ControlFlow::Continue(())
    }

    /// Returns the PC of the current instruction where `I` is the static type
    /// representing the current instruction.
    fn current_pc<I: Encode>(&self) -> NonNull<u8> {
        unsafe { self.pc.offset(-isize::from(I::WIDTH)).as_ptr() }
    }

    /// `sp -= size_of::<T>(); *sp = val;`
    #[must_use]
    fn push<T>(&mut self, val: T, pc: NonNull<u8>) -> ControlFlow<Done> {
        let new_sp = self.state[XReg::sp].get_ptr::<T>().wrapping_sub(1);
        self.set_sp(new_sp, pc)?;
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
    #[must_use]
    fn set_sp<T>(&mut self, sp: *mut T, pc: NonNull<u8>) -> ControlFlow<Done> {
        let sp_raw = sp as usize;
        let base_raw = self.state.stack.as_ptr() as usize;
        if sp_raw < base_raw {
            return ControlFlow::Break(self.done_trap(pc));
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
}

#[test]
fn simple_push_pop() {
    let mut state = MachineState::with_stack(vec![0; 16]);
    unsafe {
        let mut i = Interpreter {
            state: &mut state,
            // this isn't actually read so just manufacture a dummy one
            pc: UnsafeBytecodeStream::new((&mut 0).into()),
        };
        let pc = NonNull::from(&0);
        assert!(i.push(0_i32, pc).is_continue());
        assert_eq!(i.pop::<i32>(), 0_i32);
        assert!(i.push(1_i32, pc).is_continue());
        assert!(i.push(2_i32, pc).is_continue());
        assert!(i.push(3_i32, pc).is_continue());
        assert!(i.push(4_i32, pc).is_continue());
        assert!(i.push(5_i32, pc).is_break());
        assert!(i.push(6_i32, pc).is_break());
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
        let lr = self.state[XReg::lr];
        if lr == XRegVal::HOST_RETURN_ADDR {
            ControlFlow::Break(self.done_return_to_host())
        } else {
            let return_addr = lr.get_ptr();
            self.pc = unsafe { UnsafeBytecodeStream::new(NonNull::new_unchecked(return_addr)) };
            ControlFlow::Continue(())
        }
    }

    fn call(&mut self, offset: PcRelOffset) -> ControlFlow<Done> {
        let return_addr = self.pc.as_ptr();
        self.state[XReg::lr].set_ptr(return_addr.as_ptr());
        self.pc_rel_jump(offset, 5);
        ControlFlow::Continue(())
    }

    fn call_indirect(&mut self, dst: XReg) -> ControlFlow<Done> {
        let return_addr = self.pc.as_ptr();
        self.state[XReg::lr].set_ptr(return_addr.as_ptr());
        // SAFETY: part of the unsafe contract of the interpreter is only valid
        // bytecode is interpreted, so the jump destination is part of the validity
        // of the bytecode itself.
        unsafe {
            self.pc = UnsafeBytecodeStream::new(NonNull::new_unchecked(self.state[dst].get_ptr()));
        }
        ControlFlow::Continue(())
    }

    fn jump(&mut self, offset: PcRelOffset) -> ControlFlow<Done> {
        self.pc_rel_jump(offset, 5);
        ControlFlow::Continue(())
    }

    fn br_if(&mut self, cond: XReg, offset: PcRelOffset) -> ControlFlow<Done> {
        let cond = self.state[cond].get_u64();
        if cond != 0 {
            self.pc_rel_jump(offset, 6)
        } else {
            ControlFlow::Continue(())
        }
    }

    fn br_if_not(&mut self, cond: XReg, offset: PcRelOffset) -> ControlFlow<Done> {
        let cond = self.state[cond].get_u64();
        if cond == 0 {
            self.pc_rel_jump(offset, 6)
        } else {
            ControlFlow::Continue(())
        }
    }

    fn br_if_xeq32(&mut self, a: XReg, b: XReg, offset: PcRelOffset) -> ControlFlow<Done> {
        let a = self.state[a].get_u32();
        let b = self.state[b].get_u32();
        if a == b {
            self.pc_rel_jump(offset, 7)
        } else {
            ControlFlow::Continue(())
        }
    }

    fn br_if_xneq32(&mut self, a: XReg, b: XReg, offset: PcRelOffset) -> ControlFlow<Done> {
        let a = self.state[a].get_u32();
        let b = self.state[b].get_u32();
        if a != b {
            self.pc_rel_jump(offset, 7)
        } else {
            ControlFlow::Continue(())
        }
    }

    fn br_if_xslt32(&mut self, a: XReg, b: XReg, offset: PcRelOffset) -> ControlFlow<Done> {
        let a = self.state[a].get_i32();
        let b = self.state[b].get_i32();
        if a < b {
            self.pc_rel_jump(offset, 7)
        } else {
            ControlFlow::Continue(())
        }
    }

    fn br_if_xslteq32(&mut self, a: XReg, b: XReg, offset: PcRelOffset) -> ControlFlow<Done> {
        let a = self.state[a].get_i32();
        let b = self.state[b].get_i32();
        if a <= b {
            self.pc_rel_jump(offset, 7)
        } else {
            ControlFlow::Continue(())
        }
    }

    fn br_if_xult32(&mut self, a: XReg, b: XReg, offset: PcRelOffset) -> ControlFlow<Done> {
        let a = self.state[a].get_u32();
        let b = self.state[b].get_u32();
        if a < b {
            self.pc_rel_jump(offset, 7)
        } else {
            ControlFlow::Continue(())
        }
    }

    fn br_if_xulteq32(&mut self, a: XReg, b: XReg, offset: PcRelOffset) -> ControlFlow<Done> {
        let a = self.state[a].get_u32();
        let b = self.state[b].get_u32();
        if a <= b {
            self.pc_rel_jump(offset, 7)
        } else {
            ControlFlow::Continue(())
        }
    }

    fn br_if_xeq64(&mut self, a: XReg, b: XReg, offset: PcRelOffset) -> ControlFlow<Done> {
        let a = self.state[a].get_u64();
        let b = self.state[b].get_u64();
        if a == b {
            self.pc_rel_jump(offset, 7)
        } else {
            ControlFlow::Continue(())
        }
    }

    fn br_if_xneq64(&mut self, a: XReg, b: XReg, offset: PcRelOffset) -> ControlFlow<Done> {
        let a = self.state[a].get_u64();
        let b = self.state[b].get_u64();
        if a != b {
            self.pc_rel_jump(offset, 7)
        } else {
            ControlFlow::Continue(())
        }
    }

    fn br_if_xslt64(&mut self, a: XReg, b: XReg, offset: PcRelOffset) -> ControlFlow<Done> {
        let a = self.state[a].get_i64();
        let b = self.state[b].get_i64();
        if a < b {
            self.pc_rel_jump(offset, 7)
        } else {
            ControlFlow::Continue(())
        }
    }

    fn br_if_xslteq64(&mut self, a: XReg, b: XReg, offset: PcRelOffset) -> ControlFlow<Done> {
        let a = self.state[a].get_i64();
        let b = self.state[b].get_i64();
        if a <= b {
            self.pc_rel_jump(offset, 7)
        } else {
            ControlFlow::Continue(())
        }
    }

    fn br_if_xult64(&mut self, a: XReg, b: XReg, offset: PcRelOffset) -> ControlFlow<Done> {
        let a = self.state[a].get_u64();
        let b = self.state[b].get_u64();
        if a < b {
            self.pc_rel_jump(offset, 7)
        } else {
            ControlFlow::Continue(())
        }
    }

    fn br_if_xulteq64(&mut self, a: XReg, b: XReg, offset: PcRelOffset) -> ControlFlow<Done> {
        let a = self.state[a].get_u64();
        let b = self.state[b].get_u64();
        if a <= b {
            self.pc_rel_jump(offset, 7)
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

    fn xeq64(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u64();
        let b = self.state[operands.src2].get_u64();
        self.state[operands.dst].set_u64(u64::from(a == b));
        ControlFlow::Continue(())
    }

    fn xneq64(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u64();
        let b = self.state[operands.src2].get_u64();
        self.state[operands.dst].set_u64(u64::from(a != b));
        ControlFlow::Continue(())
    }

    fn xslt64(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_i64();
        let b = self.state[operands.src2].get_i64();
        self.state[operands.dst].set_u64(u64::from(a < b));
        ControlFlow::Continue(())
    }

    fn xslteq64(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_i64();
        let b = self.state[operands.src2].get_i64();
        self.state[operands.dst].set_u64(u64::from(a <= b));
        ControlFlow::Continue(())
    }

    fn xult64(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u64();
        let b = self.state[operands.src2].get_u64();
        self.state[operands.dst].set_u64(u64::from(a < b));
        ControlFlow::Continue(())
    }

    fn xulteq64(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u64();
        let b = self.state[operands.src2].get_u64();
        self.state[operands.dst].set_u64(u64::from(a <= b));
        ControlFlow::Continue(())
    }

    fn xeq32(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u32();
        let b = self.state[operands.src2].get_u32();
        self.state[operands.dst].set_u64(u64::from(a == b));
        ControlFlow::Continue(())
    }

    fn xneq32(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u32();
        let b = self.state[operands.src2].get_u32();
        self.state[operands.dst].set_u64(u64::from(a != b));
        ControlFlow::Continue(())
    }

    fn xslt32(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_i32();
        let b = self.state[operands.src2].get_i32();
        self.state[operands.dst].set_u64(u64::from(a < b));
        ControlFlow::Continue(())
    }

    fn xslteq32(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_i32();
        let b = self.state[operands.src2].get_i32();
        self.state[operands.dst].set_u64(u64::from(a <= b));
        ControlFlow::Continue(())
    }

    fn xult32(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u32();
        let b = self.state[operands.src2].get_u32();
        self.state[operands.dst].set_u64(u64::from(a < b));
        ControlFlow::Continue(())
    }

    fn xulteq32(&mut self, operands: BinaryOperands<XReg>) -> ControlFlow<Done> {
        let a = self.state[operands.src1].get_u32();
        let b = self.state[operands.src2].get_u32();
        self.state[operands.dst].set_u64(u64::from(a <= b));
        ControlFlow::Continue(())
    }

    fn load32_u(&mut self, dst: XReg, ptr: XReg) -> ControlFlow<Done> {
        let ptr = self.state[ptr].get_ptr::<u32>();
        let val = unsafe { u32::from_le(ptr::read_unaligned(ptr)) };
        self.state[dst].set_u64(u64::from(val));
        ControlFlow::Continue(())
    }

    fn load32_s(&mut self, dst: XReg, ptr: XReg) -> ControlFlow<Done> {
        let ptr = self.state[ptr].get_ptr::<i32>();
        let val = unsafe { i32::from_le(ptr::read_unaligned(ptr)) };
        self.state[dst].set_i64(i64::from(val));
        ControlFlow::Continue(())
    }

    fn load64(&mut self, dst: XReg, ptr: XReg) -> ControlFlow<Done> {
        let ptr = self.state[ptr].get_ptr::<u64>();
        let val = unsafe { u64::from_le(ptr::read_unaligned(ptr)) };
        self.state[dst].set_u64(val);
        ControlFlow::Continue(())
    }

    fn load32_u_offset8(&mut self, dst: XReg, ptr: XReg, offset: i8) -> ControlFlow<Done> {
        let val = unsafe {
            u32::from_le(
                self.state[ptr]
                    .get_ptr::<u32>()
                    .byte_offset(offset.into())
                    .read_unaligned(),
            )
        };
        self.state[dst].set_u64(u64::from(val));
        ControlFlow::Continue(())
    }

    fn load32_s_offset8(&mut self, dst: XReg, ptr: XReg, offset: i8) -> ControlFlow<Done> {
        let val = unsafe {
            i32::from_le(
                self.state[ptr]
                    .get_ptr::<i32>()
                    .byte_offset(offset.into())
                    .read_unaligned(),
            )
        };
        self.state[dst].set_i64(i64::from(val));
        ControlFlow::Continue(())
    }

    fn load32_u_offset64(&mut self, dst: XReg, ptr: XReg, offset: i64) -> ControlFlow<Done> {
        let val = unsafe {
            u32::from_le(
                self.state[ptr]
                    .get_ptr::<u32>()
                    .byte_offset(offset as isize)
                    .read_unaligned(),
            )
        };
        self.state[dst].set_u64(u64::from(val));
        ControlFlow::Continue(())
    }

    fn load32_s_offset64(&mut self, dst: XReg, ptr: XReg, offset: i64) -> ControlFlow<Done> {
        let val = unsafe {
            i32::from_le(
                self.state[ptr]
                    .get_ptr::<i32>()
                    .byte_offset(offset as isize)
                    .read_unaligned(),
            )
        };
        self.state[dst].set_i64(i64::from(val));
        ControlFlow::Continue(())
    }

    fn load64_offset8(&mut self, dst: XReg, ptr: XReg, offset: i8) -> ControlFlow<Done> {
        let val = unsafe {
            u64::from_le(
                self.state[ptr]
                    .get_ptr::<u64>()
                    .byte_offset(offset.into())
                    .read_unaligned(),
            )
        };
        self.state[dst].set_u64(val);
        ControlFlow::Continue(())
    }

    fn load64_offset64(&mut self, dst: XReg, ptr: XReg, offset: i64) -> ControlFlow<Done> {
        let val = unsafe {
            u64::from_le(
                self.state[ptr]
                    .get_ptr::<u64>()
                    .byte_offset(offset as isize)
                    .read_unaligned(),
            )
        };
        self.state[dst].set_u64(val);
        ControlFlow::Continue(())
    }

    fn store32(&mut self, ptr: XReg, src: XReg) -> ControlFlow<Done> {
        let ptr = self.state[ptr].get_ptr::<u32>();
        let val = self.state[src].get_u32();
        unsafe {
            ptr::write_unaligned(ptr, val.to_le());
        }
        ControlFlow::Continue(())
    }

    fn store64(&mut self, ptr: XReg, src: XReg) -> ControlFlow<Done> {
        let ptr = self.state[ptr].get_ptr::<u64>();
        let val = self.state[src].get_u64();
        unsafe {
            ptr::write_unaligned(ptr, val.to_le());
        }
        ControlFlow::Continue(())
    }

    fn store32_offset8(&mut self, ptr: XReg, offset: i8, src: XReg) -> ControlFlow<Done> {
        let val = self.state[src].get_u32();
        unsafe {
            self.state[ptr]
                .get_ptr::<u32>()
                .byte_offset(offset.into())
                .write_unaligned(val.to_le());
        }
        ControlFlow::Continue(())
    }

    fn store64_offset8(&mut self, ptr: XReg, offset: i8, src: XReg) -> ControlFlow<Done> {
        let val = self.state[src].get_u64();
        unsafe {
            self.state[ptr]
                .get_ptr::<u64>()
                .byte_offset(offset.into())
                .write_unaligned(val.to_le());
        }
        ControlFlow::Continue(())
    }

    fn store32_offset64(&mut self, ptr: XReg, offset: i64, src: XReg) -> ControlFlow<Done> {
        let val = self.state[src].get_u32();
        unsafe {
            self.state[ptr]
                .get_ptr::<u32>()
                .byte_offset(offset as isize)
                .write_unaligned(val.to_le());
        }
        ControlFlow::Continue(())
    }

    fn store64_offset64(&mut self, ptr: XReg, offset: i64, src: XReg) -> ControlFlow<Done> {
        let val = self.state[src].get_u64();
        unsafe {
            self.state[ptr]
                .get_ptr::<u64>()
                .byte_offset(offset as isize)
                .write_unaligned(val.to_le());
        }
        ControlFlow::Continue(())
    }

    fn xpush32(&mut self, src: XReg) -> ControlFlow<Done> {
        let me = self.current_pc::<crate::XPush32>();
        self.push(self.state[src].get_u32(), me)?;
        ControlFlow::Continue(())
    }

    fn xpush32_many(&mut self, srcs: RegSet<XReg>) -> ControlFlow<Done> {
        let me = self.current_pc::<crate::XPush32Many>();
        for src in srcs {
            self.push(self.state[src].get_u32(), me)?;
        }
        ControlFlow::Continue(())
    }

    fn xpush64(&mut self, src: XReg) -> ControlFlow<Done> {
        let me = self.current_pc::<crate::XPush64>();
        self.push(self.state[src].get_u64(), me)?;
        ControlFlow::Continue(())
    }

    fn xpush64_many(&mut self, srcs: RegSet<XReg>) -> ControlFlow<Done> {
        let me = self.current_pc::<crate::XPush64Many>();
        for src in srcs {
            self.push(self.state[src].get_u64(), me)?;
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
        let me = self.current_pc::<crate::PushFrame>();
        self.push(self.state[XReg::lr].get_ptr::<u8>(), me)?;
        self.push(self.state[XReg::fp].get_ptr::<u8>(), me)?;
        self.state[XReg::fp] = self.state[XReg::sp];
        ControlFlow::Continue(())
    }

    fn pop_frame(&mut self) -> ControlFlow<Done> {
        self.set_sp_unchecked(self.state[XReg::fp].get_ptr::<u8>());
        let fp = self.pop();
        let lr = self.pop();
        self.state[XReg::fp].set_ptr::<u8>(fp);
        self.state[XReg::lr].set_ptr::<u8>(lr);
        ControlFlow::Continue(())
    }

    fn bitcast_int_from_float_32(&mut self, dst: XReg, src: FReg) -> ControlFlow<Done> {
        let val = self.state[src].get_f32();
        self.state[dst].set_u64(u32::from_ne_bytes(val.to_ne_bytes()).into());
        ControlFlow::Continue(())
    }

    fn bitcast_int_from_float_64(&mut self, dst: XReg, src: FReg) -> ControlFlow<Done> {
        let val = self.state[src].get_f64();
        self.state[dst].set_u64(u64::from_ne_bytes(val.to_ne_bytes()));
        ControlFlow::Continue(())
    }

    fn bitcast_float_from_int_32(&mut self, dst: FReg, src: XReg) -> ControlFlow<Done> {
        let val = self.state[src].get_u32();
        self.state[dst].set_f32(f32::from_ne_bytes(val.to_ne_bytes()));
        ControlFlow::Continue(())
    }

    fn bitcast_float_from_int_64(&mut self, dst: FReg, src: XReg) -> ControlFlow<Done> {
        let val = self.state[src].get_u64();
        self.state[dst].set_f64(f64::from_ne_bytes(val.to_ne_bytes()));
        ControlFlow::Continue(())
    }

    fn br_table32(&mut self, idx: XReg, amt: u32) -> ControlFlow<Done> {
        let idx = self.state[idx].get_u32().min(amt - 1) as isize;
        // SAFETY: part of the contract of the interpreter is only dealing with
        // valid bytecode, so this offset should be safe.
        self.pc = unsafe { self.pc.offset(idx * 4) };
        let mut tmp = self.pc;
        let rel = unwrap_uninhabited(PcRelOffset::decode(&mut tmp));
        self.pc_rel_jump(rel, 0)
    }

    fn stack_alloc32(&mut self, amt: u32) -> ControlFlow<Done> {
        let me = self.current_pc::<crate::StackAlloc32>();
        let amt = usize::try_from(amt).unwrap();
        let new_sp = self.state[XReg::sp].get_ptr::<u8>().wrapping_sub(amt);
        self.set_sp(new_sp, me)?;
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
}

impl ExtendedOpVisitor for Interpreter<'_> {
    fn nop(&mut self) -> ControlFlow<Done> {
        ControlFlow::Continue(())
    }

    fn trap(&mut self) -> ControlFlow<Done> {
        let trap_pc = self.current_pc::<crate::Trap>();
        ControlFlow::Break(self.done_trap(trap_pc))
    }

    fn call_indirect_host(&mut self, id: u8) -> ControlFlow<Done> {
        ControlFlow::Break(self.done_call_indirect_host(id))
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
}
