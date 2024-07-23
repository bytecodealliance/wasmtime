//! Interpretation of pulley bytecode.

use crate::decode::*;
use crate::imms::*;
use crate::regs::*;
use alloc::string::ToString;
use alloc::{vec, vec::Vec};
use core::mem;
use core::ptr::{self, NonNull};

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
    ) -> Result<impl Iterator<Item = Val> + 'a, *mut u8> {
        // NB: make sure this method stays in sync with
        // `Pbc64MachineDeps::compute_arg_locs`!

        let mut x_args = (0..16).map(|x| XReg::unchecked_new(x));
        let mut f_args = (0..16).map(|f| FReg::unchecked_new(f));
        let mut v_args = (0..16).map(|v| VReg::unchecked_new(v));

        for arg in args {
            match arg {
                Val::XReg(val) => match x_args.next() {
                    Some(reg) => self.state.set_x(reg, *val),
                    None => todo!("stack slots"),
                },
                Val::FReg(val) => match f_args.next() {
                    Some(reg) => self.state.set_f(reg, *val),
                    None => todo!("stack slots"),
                },
                Val::VReg(val) => match v_args.next() {
                    Some(reg) => self.state.set_v(reg, *val),
                    None => todo!("stack slots"),
                },
            }
        }

        self.run(func.as_ptr())?;

        let mut x_rets = (0..16).map(|x| XReg::unchecked_new(x));
        let mut f_rets = (0..16).map(|f| FReg::unchecked_new(f));
        let mut v_rets = (0..16).map(|v| VReg::unchecked_new(v));

        Ok(rets.into_iter().map(move |ty| match ty {
            RegType::XReg => match x_rets.next() {
                Some(reg) => Val::XReg(self.state.get_x(reg)),
                None => todo!("stack slots"),
            },
            RegType::FReg => match f_rets.next() {
                Some(reg) => Val::FReg(self.state.get_f(reg)),
                None => todo!("stack slots"),
            },
            RegType::VReg => match v_rets.next() {
                Some(reg) => Val::VReg(self.state.get_v(reg)),
                None => todo!("stack slots"),
            },
        }))
    }

    unsafe fn run(&mut self, pc: *mut u8) -> Result<(), *mut u8> {
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
    fn return_to_host(&self) -> Result<(), *mut u8> {
        Ok(())
    }

    #[cold]
    #[inline(never)]
    fn trap(&self, pc: *mut u8) -> Result<(), *mut u8> {
        Err(pc)
    }

    #[cold]
    #[inline(never)]
    fn host_call(&self) -> Result<(), *mut u8> {
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

/// An `x` register value: integers.
#[derive(Copy, Clone)]
pub struct XRegVal(XRegUnion);

impl core::fmt::Debug for XRegVal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("XRegVal")
            .field("as_u64", &self.get_u64())
            .finish()
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
    isize: isize,
    usize: usize,
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

    pub fn new_isize(x: isize) -> Self {
        let mut val = XRegVal::default();
        val.set_isize(x);
        val
    }

    pub fn new_usize(x: usize) -> Self {
        let mut val = XRegVal::default();
        val.set_usize(x);
        val
    }

    pub fn get_i32(&self) -> i32 {
        let x = unsafe { self.0.i32 };
        i32::from_le_bytes(x.to_ne_bytes())
    }

    pub fn get_u32(&self) -> u32 {
        let x = unsafe { self.0.u32 };
        u32::from_le_bytes(x.to_ne_bytes())
    }

    pub fn get_i64(&self) -> i64 {
        let x = unsafe { self.0.i64 };
        i64::from_le_bytes(x.to_ne_bytes())
    }

    pub fn get_u64(&self) -> u64 {
        let x = unsafe { self.0.u64 };
        u64::from_le_bytes(x.to_ne_bytes())
    }

    pub fn get_isize(&self) -> isize {
        let x = unsafe { self.0.isize };
        isize::from_le_bytes(x.to_ne_bytes())
    }

    pub fn get_usize(&self) -> usize {
        let x = unsafe { self.0.usize };
        usize::from_le_bytes(x.to_ne_bytes())
    }

    pub fn set_i32(&mut self, x: i32) {
        let x = i32::from_ne_bytes(x.to_le_bytes());
        self.0.i32 = x;
    }

    pub fn set_u32(&mut self, x: u32) {
        let x = u32::from_ne_bytes(x.to_le_bytes());
        self.0.u32 = x;
    }

    pub fn set_i64(&mut self, x: i64) {
        let x = i64::from_ne_bytes(x.to_le_bytes());
        self.0.i64 = x;
    }

    pub fn set_u64(&mut self, x: u64) {
        let x = u64::from_ne_bytes(x.to_le_bytes());
        self.0.u64 = x;
    }

    pub fn set_isize(&mut self, x: isize) {
        let x = isize::from_ne_bytes(x.to_le_bytes());
        self.0.isize = x;
    }

    pub fn set_usize(&mut self, x: usize) {
        let x = usize::from_ne_bytes(x.to_le_bytes());
        self.0.usize = x;
    }
}

/// An `f` register value: floats.
#[derive(Copy, Clone)]
pub struct FRegVal(FRegUnion);

impl core::fmt::Debug for FRegVal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("FRegVal")
            .field("as_f32", &self.get_f32())
            .field("as_f64", &self.get_f64())
            .finish()
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

impl core::fmt::Debug for VRegVal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("VRegVal")
            .field("as_u128", &unsafe { self.0.u128 })
            .finish()
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

impl core::fmt::Debug for MachineState {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let MachineState {
            x_regs,
            f_regs,
            v_regs,
            stack: _,
        } = self;

        struct RegMap<'a, R>(&'a [R], fn(u8) -> alloc::string::String);

        impl<R: core::fmt::Debug> core::fmt::Debug for RegMap<'_, R> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
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

impl MachineState {
    fn with_stack(stack: Vec<u8>) -> Self {
        assert!(stack.len() > 0);
        let mut state = Self {
            x_regs: [Default::default(); XReg::RANGE.end as usize],
            f_regs: Default::default(),
            v_regs: Default::default(),
            stack,
        };

        let sp = state.stack.last().unwrap() as *const u8 as usize;
        state.set_x(XReg::SP, XRegVal::new_usize(sp));

        state.set_x(XReg::FP, XRegVal::new_i64(-1));
        state.set_x(XReg::LR, XRegVal::new_i64(-1));

        state
    }

    /// Get a shared reference to the value of the given `x` register.
    #[inline(always)]
    pub fn x(&self, x: XReg) -> &XRegVal {
        debug_assert!(x.index() < self.x_regs.len());
        unsafe { self.x_regs.get_unchecked(x.index()) }
    }

    /// Get an exclusive reference to the value of the given `x` register.
    #[inline(always)]
    pub fn x_mut(&mut self, x: XReg) -> &mut XRegVal {
        debug_assert!(x.index() < self.x_regs.len());
        unsafe { self.x_regs.get_unchecked_mut(x.index()) }
    }

    /// Copy the value of the given `x` register.
    #[inline(always)]
    pub fn get_x(&self, x: XReg) -> XRegVal {
        *self.x(x)
    }

    /// Set the value of the given `x` register.
    #[inline(always)]
    pub fn set_x(&mut self, x: XReg, val: XRegVal) {
        debug_assert!(x.index() < self.x_regs.len());
        unsafe {
            *self.x_regs.get_unchecked_mut(x.index()) = val;
        }
    }

    /// Get a shared reference to the value of the given `f` register.
    #[inline(always)]
    pub fn f(&self, f: FReg) -> &FRegVal {
        debug_assert!(f.index() < self.f_regs.len());
        unsafe { self.f_regs.get_unchecked(f.index()) }
    }

    /// Get an exclusive reference to the value of the given `f` register.
    #[inline(always)]
    pub fn f_mut(&mut self, f: FReg) -> &mut FRegVal {
        debug_assert!(f.index() < self.f_regs.len());
        unsafe { self.f_regs.get_unchecked_mut(f.index()) }
    }

    /// Copy the value of the given `f` register.
    #[inline(always)]
    pub fn get_f(&self, f: FReg) -> FRegVal {
        debug_assert!(f.index() < self.f_regs.len());
        unsafe { *self.f_regs.get_unchecked(f.index()) }
    }

    /// Set the value of the given `f` register.
    #[inline(always)]
    pub fn set_f(&mut self, f: FReg, val: FRegVal) {
        debug_assert!(f.index() < self.f_regs.len());
        unsafe {
            *self.f_regs.get_unchecked_mut(f.index()) = val;
        }
    }

    /// Get a shared reference to the value of the given `v` register.
    #[inline(always)]
    pub fn v(&self, v: VReg) -> &VRegVal {
        debug_assert!(v.index() < self.v_regs.len());
        unsafe { self.v_regs.get_unchecked(v.index()) }
    }

    /// Get an exclusive reference to the value of the given `v` register.
    #[inline(always)]
    pub fn v_mut(&mut self, v: VReg) -> &mut VRegVal {
        debug_assert!(v.index() < self.v_regs.len());
        unsafe { self.v_regs.get_unchecked_mut(v.index()) }
    }

    /// Copy the value of the given `v` register.
    #[inline(always)]
    pub fn get_v(&self, v: VReg) -> VRegVal {
        debug_assert!(v.index() < self.v_regs.len());
        unsafe { *self.v_regs.get_unchecked(v.index()) }
    }

    /// Set the value of the given `v` register.
    #[inline(always)]
    pub fn set_v(&mut self, v: VReg, val: VRegVal) {
        debug_assert!(v.index() < self.v_regs.len());
        unsafe {
            *self.v_regs.get_unchecked_mut(v.index()) = val;
        }
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
        if self.state.x(XReg::LR).get_u64() == u64::MAX {
            Continuation::ReturnToHost
        } else {
            let return_addr = self.state.x(XReg::LR).get_usize() as *mut u8;
            self.pc = unsafe { UnsafeBytecodeStream::new(return_addr) };
            // log::trace!("returning to {return_addr:#p}");
            Continuation::Continue
        }
    }

    fn call(&mut self, offset: PcRelOffset) -> Self::Return {
        let return_addr = u64::try_from(self.pc.as_ptr() as usize).unwrap();
        self.state.x_mut(XReg::LR).set_u64(return_addr);
        self.pc_rel_jump(offset, 5)
    }

    fn jump(&mut self, offset: PcRelOffset) -> Self::Return {
        self.pc_rel_jump(offset, 5)
    }

    fn br_if(&mut self, cond: XReg, offset: PcRelOffset) -> Self::Return {
        let cond = self.state.x(cond).get_u64();
        if cond != 0 {
            self.pc_rel_jump(offset, 6)
        } else {
            Continuation::Continue
        }
    }

    fn br_if_not(&mut self, cond: XReg, offset: PcRelOffset) -> Self::Return {
        let cond = self.state.x(cond).get_u64();
        if cond == 0 {
            self.pc_rel_jump(offset, 6)
        } else {
            Continuation::Continue
        }
    }

    fn br_if_xeq32(&mut self, a: XReg, b: XReg, offset: PcRelOffset) -> Self::Return {
        let a = self.state.x(a).get_u32();
        let b = self.state.x(b).get_u32();
        if a == b {
            self.pc_rel_jump(offset, 7)
        } else {
            Continuation::Continue
        }
    }

    fn br_if_xneq32(&mut self, a: XReg, b: XReg, offset: PcRelOffset) -> Self::Return {
        let a = self.state.x(a).get_u32();
        let b = self.state.x(b).get_u32();
        if a != b {
            self.pc_rel_jump(offset, 7)
        } else {
            Continuation::Continue
        }
    }

    fn br_if_xslt32(&mut self, a: XReg, b: XReg, offset: PcRelOffset) -> Self::Return {
        let a = self.state.x(a).get_i32();
        let b = self.state.x(b).get_i32();
        if a < b {
            self.pc_rel_jump(offset, 7)
        } else {
            Continuation::Continue
        }
    }

    fn br_if_xslteq32(&mut self, a: XReg, b: XReg, offset: PcRelOffset) -> Self::Return {
        let a = self.state.x(a).get_i32();
        let b = self.state.x(b).get_i32();
        if a <= b {
            self.pc_rel_jump(offset, 7)
        } else {
            Continuation::Continue
        }
    }

    fn br_if_xult32(&mut self, a: XReg, b: XReg, offset: PcRelOffset) -> Self::Return {
        let a = self.state.x(a).get_u32();
        let b = self.state.x(b).get_u32();
        if a < b {
            self.pc_rel_jump(offset, 7)
        } else {
            Continuation::Continue
        }
    }

    fn br_if_xulteq32(&mut self, a: XReg, b: XReg, offset: PcRelOffset) -> Self::Return {
        let a = self.state.x(a).get_u32();
        let b = self.state.x(b).get_u32();
        if a <= b {
            self.pc_rel_jump(offset, 7)
        } else {
            Continuation::Continue
        }
    }

    fn xmov(&mut self, dst: XReg, src: XReg) -> Self::Return {
        let val = self.state.get_x(src);
        self.state.set_x(dst, val);
        Continuation::Continue
    }

    fn fmov(&mut self, dst: FReg, src: FReg) -> Self::Return {
        let val = self.state.get_f(src);
        self.state.set_f(dst, val);
        Continuation::Continue
    }

    fn vmov(&mut self, dst: VReg, src: VReg) -> Self::Return {
        let val = self.state.get_v(src);
        self.state.set_v(dst, val);
        Continuation::Continue
    }

    fn xconst8(&mut self, dst: XReg, imm: u8) -> Self::Return {
        self.state.x_mut(dst).set_u64(u64::from(imm));
        Continuation::Continue
    }

    fn xconst16(&mut self, dst: XReg, imm: u16) -> Self::Return {
        self.state.x_mut(dst).set_u64(u64::from(imm));
        Continuation::Continue
    }

    fn xconst32(&mut self, dst: XReg, imm: u32) -> Self::Return {
        self.state.x_mut(dst).set_u64(u64::from(imm));
        Continuation::Continue
    }

    fn xconst64(&mut self, dst: XReg, imm: u64) -> Self::Return {
        self.state.x_mut(dst).set_u64(imm);
        Continuation::Continue
    }

    fn xadd32(&mut self, dst: XReg, src1: XReg, src2: XReg) -> Self::Return {
        let a = self.state.x(src1).get_u32();
        let b = self.state.x(src2).get_u32();
        self.state.x_mut(dst).set_u32(a.wrapping_add(b));
        Continuation::Continue
    }

    fn xadd64(&mut self, dst: XReg, src1: XReg, src2: XReg) -> Self::Return {
        let a = self.state.x(src1).get_u64();
        let b = self.state.x(src2).get_u64();
        self.state.x_mut(dst).set_u64(a.wrapping_add(b));
        Continuation::Continue
    }

    fn xeq64(&mut self, dst: XReg, src1: XReg, src2: XReg) -> Self::Return {
        let a = self.state.x(src1).get_u64();
        let b = self.state.x(src2).get_u64();
        self.state.x_mut(dst).set_u64(u64::from(a == b));
        Continuation::Continue
    }

    fn xneq64(&mut self, dst: XReg, src1: XReg, src2: XReg) -> Self::Return {
        let a = self.state.x(src1).get_u64();
        let b = self.state.x(src2).get_u64();
        self.state.x_mut(dst).set_u64(u64::from(a != b));
        Continuation::Continue
    }

    fn xslt64(&mut self, dst: XReg, src1: XReg, src2: XReg) -> Self::Return {
        let a = self.state.x(src1).get_i64();
        let b = self.state.x(src2).get_i64();
        self.state.x_mut(dst).set_u64(u64::from(a < b));
        Continuation::Continue
    }

    fn xslteq64(&mut self, dst: XReg, src1: XReg, src2: XReg) -> Self::Return {
        let a = self.state.x(src1).get_i64();
        let b = self.state.x(src2).get_i64();
        self.state.x_mut(dst).set_u64(u64::from(a <= b));
        Continuation::Continue
    }

    fn xult64(&mut self, dst: XReg, src1: XReg, src2: XReg) -> Self::Return {
        let a = self.state.x(src1).get_u64();
        let b = self.state.x(src2).get_u64();
        self.state.x_mut(dst).set_u64(u64::from(a < b));
        Continuation::Continue
    }

    fn xulteq64(&mut self, dst: XReg, src1: XReg, src2: XReg) -> Self::Return {
        let a = self.state.x(src1).get_u64();
        let b = self.state.x(src2).get_u64();
        self.state.x_mut(dst).set_u64(u64::from(a <= b));
        Continuation::Continue
    }

    fn xeq32(&mut self, dst: XReg, src1: XReg, src2: XReg) -> Self::Return {
        let a = self.state.x(src1).get_u32();
        let b = self.state.x(src2).get_u32();
        self.state.x_mut(dst).set_u64(u64::from(a == b));
        Continuation::Continue
    }

    fn xneq32(&mut self, dst: XReg, src1: XReg, src2: XReg) -> Self::Return {
        let a = self.state.x(src1).get_u32();
        let b = self.state.x(src2).get_u32();
        self.state.x_mut(dst).set_u64(u64::from(a != b));
        Continuation::Continue
    }

    fn xslt32(&mut self, dst: XReg, src1: XReg, src2: XReg) -> Self::Return {
        let a = self.state.x(src1).get_i32();
        let b = self.state.x(src2).get_i32();
        self.state.x_mut(dst).set_u64(u64::from(a < b));
        Continuation::Continue
    }

    fn xslteq32(&mut self, dst: XReg, src1: XReg, src2: XReg) -> Self::Return {
        let a = self.state.x(src1).get_i32();
        let b = self.state.x(src2).get_i32();
        self.state.x_mut(dst).set_u64(u64::from(a <= b));
        Continuation::Continue
    }

    fn xult32(&mut self, dst: XReg, src1: XReg, src2: XReg) -> Self::Return {
        let a = self.state.x(src1).get_u32();
        let b = self.state.x(src2).get_u32();
        self.state.x_mut(dst).set_u64(u64::from(a < b));
        Continuation::Continue
    }

    fn xulteq32(&mut self, dst: XReg, src1: XReg, src2: XReg) -> Self::Return {
        let a = self.state.x(src1).get_u32();
        let b = self.state.x(src2).get_u32();
        self.state.x_mut(dst).set_u64(u64::from(a <= b));
        Continuation::Continue
    }

    fn load32_u(&mut self, dst: XReg, ptr: XReg) -> Self::Return {
        let ptr = self.state.x(ptr).get_usize();
        let ptr = ptr as *mut u32;
        let val = unsafe { ptr::read(ptr) };
        self.state.x_mut(dst).set_u64(u64::from(val));
        Continuation::Continue
    }

    fn load32_s(&mut self, dst: XReg, ptr: XReg) -> Self::Return {
        let ptr = self.state.x(ptr).get_usize();
        let ptr = ptr as *mut i32;
        let val = unsafe { ptr::read(ptr) };
        self.state.x_mut(dst).set_i64(i64::from(val));
        Continuation::Continue
    }

    fn load64(&mut self, dst: XReg, ptr: XReg) -> Self::Return {
        let ptr = self.state.x(ptr).get_usize();
        let ptr = ptr as *mut u64;
        let val = unsafe { ptr::read(ptr) };
        self.state.x_mut(dst).set_u64(val);
        Continuation::Continue
    }

    fn load32_u_offset8(&mut self, dst: XReg, ptr: XReg, offset: i8) -> Self::Return {
        let ptr = self.state.x(ptr).get_usize();
        let offset = isize::from(offset);
        let ptr = ptr.wrapping_add(offset as usize);
        let ptr = ptr as *mut u32;
        let val = unsafe { ptr::read(ptr) };
        self.state.x_mut(dst).set_u64(u64::from(val));
        Continuation::Continue
    }

    fn load32_s_offset8(&mut self, dst: XReg, ptr: XReg, offset: i8) -> Self::Return {
        let ptr = self.state.x(ptr).get_usize();
        let offset = isize::from(offset);
        let ptr = ptr.wrapping_add(offset as usize);
        let ptr = ptr as *mut i32;
        let val = unsafe { ptr::read(ptr) };
        self.state.x_mut(dst).set_i64(i64::from(val));
        Continuation::Continue
    }

    fn load64_offset8(&mut self, dst: XReg, ptr: XReg, offset: i8) -> Self::Return {
        let ptr = self.state.x(ptr).get_usize();
        let offset = isize::from(offset);
        let ptr = ptr.wrapping_add(offset as usize);
        let ptr = ptr as *mut u64;
        let val = unsafe { ptr::read(ptr) };
        self.state.x_mut(dst).set_u64(val);
        Continuation::Continue
    }

    fn store32(&mut self, ptr: XReg, src: XReg) -> Self::Return {
        let ptr = self.state.x(ptr).get_usize();
        let ptr = ptr as *mut u32;
        let val = self.state.x(src).get_u32();
        unsafe {
            ptr::write(ptr, val);
        }
        Continuation::Continue
    }

    fn store64(&mut self, ptr: XReg, src: XReg) -> Self::Return {
        let ptr = self.state.x(ptr).get_usize();
        let ptr = ptr as *mut u64;
        let val = self.state.x(src).get_u64();
        unsafe {
            ptr::write(ptr, val);
        }
        Continuation::Continue
    }

    fn store32_offset8(&mut self, ptr: XReg, offset: i8, src: XReg) -> Self::Return {
        let ptr = self.state.x(ptr).get_usize();
        let offset = isize::from(offset);
        let ptr = ptr.wrapping_add(offset as usize);
        let ptr = ptr as *mut u32;
        let val = self.state.x(src).get_u32();
        unsafe {
            ptr::write(ptr, val);
        }
        Continuation::Continue
    }

    fn store64_offset8(&mut self, ptr: XReg, offset: i8, src: XReg) -> Self::Return {
        let ptr = self.state.x(ptr).get_usize();
        let offset = isize::from(offset);
        let ptr = ptr.wrapping_add(offset as usize);
        let ptr = ptr as *mut u64;
        let val = self.state.x(src).get_u64();
        unsafe {
            ptr::write(ptr, val);
        }
        Continuation::Continue
    }

    fn bitcast_int_from_float_32(&mut self, dst: XReg, src: FReg) -> Self::Return {
        let val = self.state.f(src).get_f32();
        self.state
            .x_mut(dst)
            .set_u64(u32::from_ne_bytes(val.to_ne_bytes()).into());
        Continuation::Continue
    }

    fn bitcast_int_from_float_64(&mut self, dst: XReg, src: FReg) -> Self::Return {
        let val = self.state.f(src).get_f64();
        self.state
            .x_mut(dst)
            .set_u64(u64::from_ne_bytes(val.to_ne_bytes()));
        Continuation::Continue
    }

    fn bitcast_float_from_int_32(&mut self, dst: FReg, src: XReg) -> Self::Return {
        let val = self.state.x(src).get_u32();
        self.state
            .f_mut(dst)
            .set_f32(f32::from_ne_bytes(val.to_ne_bytes()));
        Continuation::Continue
    }

    fn bitcast_float_from_int_64(&mut self, dst: FReg, src: XReg) -> Self::Return {
        let val = self.state.x(src).get_u64();
        self.state
            .f_mut(dst)
            .set_f64(f64::from_ne_bytes(val.to_ne_bytes()));
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
        let sp = self.state.x(XReg::SP).get_u64();
        self.state.x_mut(dst).set_u64(sp);
        Continuation::Continue
    }
}
