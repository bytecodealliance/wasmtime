use crate::abi::{self, align_to, scratch, LocalSlot};
use crate::codegen::{CodeGenContext, Emission, FuncEnv};
use crate::isa::{
    reg::{writable, Reg, WritableReg},
    CallingConvention,
};
use anyhow::Result;
use cranelift_codegen::{
    binemit::CodeOffset,
    ir::{Endianness, LibCall, MemFlags, RelSourceLoc, SourceLoc, UserExternalNameRef},
    Final, MachBufferFinalized, MachLabel,
};
use std::{fmt::Debug, ops::Range};
use wasmtime_environ::PtrSize;

pub(crate) use cranelift_codegen::ir::TrapCode;

#[derive(Eq, PartialEq)]
pub(crate) enum DivKind {
    /// Signed division.
    Signed,
    /// Unsigned division.
    Unsigned,
}

/// Remainder kind.
#[derive(Copy, Clone)]
pub(crate) enum RemKind {
    /// Signed remainder.
    Signed,
    /// Unsigned remainder.
    Unsigned,
}

impl RemKind {
    pub fn is_signed(&self) -> bool {
        matches!(self, Self::Signed)
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub(crate) enum MemOpKind {
    /// An atomic memory operation with SeqCst memory ordering.
    Atomic,
    /// A memory operation with no memory ordering constraint.
    Normal,
}

#[derive(Eq, PartialEq)]
pub(crate) enum MulWideKind {
    Signed,
    Unsigned,
}

/// The direction to perform the memory move.
#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum MemMoveDirection {
    /// From high memory addresses to low memory addresses.
    /// Invariant: the source location is closer to the FP than the destination
    /// location, which will be closer to the SP.
    HighToLow,
    /// From low memory addresses to high memory addresses.
    /// Invariant: the source location is closer to the SP than the destination
    /// location, which will be closer to the FP.
    LowToHigh,
}

/// Classifies how to treat float-to-int conversions.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(crate) enum TruncKind {
    /// Saturating conversion. If the source value is greater than the maximum
    /// value of the destination type, the result is clamped to the
    /// destination maximum value.
    Checked,
    /// An exception is raised if the source value is greater than the maximum
    /// value of the destination type.
    Unchecked,
}

impl TruncKind {
    /// Returns true if the truncation kind is checked.
    pub(crate) fn is_checked(&self) -> bool {
        *self == TruncKind::Checked
    }

    /// Returns `true` if the trunc kind is [`Unchecked`].
    ///
    /// [`Unchecked`]: TruncKind::Unchecked
    #[must_use]
    pub(crate) fn is_unchecked(&self) -> bool {
        matches!(self, Self::Unchecked)
    }
}

/// Representation of the stack pointer offset.
#[derive(Copy, Clone, Eq, PartialEq, Debug, PartialOrd, Ord, Default)]
pub struct SPOffset(u32);

impl SPOffset {
    pub fn from_u32(offs: u32) -> Self {
        Self(offs)
    }

    pub fn as_u32(&self) -> u32 {
        self.0
    }
}

/// A stack slot.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct StackSlot {
    /// The location of the slot, relative to the stack pointer.
    pub offset: SPOffset,
    /// The size of the slot, in bytes.
    pub size: u32,
}

impl StackSlot {
    pub fn new(offs: SPOffset, size: u32) -> Self {
        Self { offset: offs, size }
    }
}

/// Kinds of integer binary comparison in WebAssembly. The [`MacroAssembler`]
/// implementation for each ISA is responsible for emitting the correct
/// sequence of instructions when lowering to machine code.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum IntCmpKind {
    /// Equal.
    Eq,
    /// Not equal.
    Ne,
    /// Signed less than.
    LtS,
    /// Unsigned less than.
    LtU,
    /// Signed greater than.
    GtS,
    /// Unsigned greater than.
    GtU,
    /// Signed less than or equal.
    LeS,
    /// Unsigned less than or equal.
    LeU,
    /// Signed greater than or equal.
    GeS,
    /// Unsigned greater than or equal.
    GeU,
}

/// Kinds of float binary comparison in WebAssembly. The [`MacroAssembler`]
/// implementation for each ISA is responsible for emitting the correct
/// sequence of instructions when lowering code.
#[derive(Debug)]
pub(crate) enum FloatCmpKind {
    /// Equal.
    Eq,
    /// Not equal.
    Ne,
    /// Less than.
    Lt,
    /// Greater than.
    Gt,
    /// Less than or equal.
    Le,
    /// Greater than or equal.
    Ge,
}

/// Kinds of shifts in WebAssembly.The [`masm`] implementation for each ISA is
/// responsible for emitting the correct sequence of instructions when
/// lowering to machine code.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum ShiftKind {
    /// Left shift.
    Shl,
    /// Signed right shift.
    ShrS,
    /// Unsigned right shift.
    ShrU,
    /// Left rotate.
    Rotl,
    /// Right rotate.
    Rotr,
}

/// Kinds of extends in WebAssembly. Each MacroAssembler implementation
/// is responsible for emitting the correct sequence of instructions when
/// lowering to machine code.
#[derive(Copy, Clone)]
pub(crate) enum ExtendKind {
    /// Sign extends i32 to i64.
    I64ExtendI32S,
    /// Zero extends i32 to i64.
    I64ExtendI32U,
    // Sign extends the 8 least significant bits to 32 bits.
    I32Extend8S,
    // Sign extends the 16 least significant bits to 32 bits.
    I32Extend16S,
    /// Sign extends the 8 least significant bits to 64 bits.
    I64Extend8S,
    /// Sign extends the 16 least significant bits to 64 bits.
    I64Extend16S,
    /// Sign extends the 32 least significant bits to 64 bits.
    I64Extend32S,
}

impl ExtendKind {
    pub fn signed(&self) -> bool {
        if let Self::I64ExtendI32U = self {
            false
        } else {
            true
        }
    }

    pub fn from_bits(&self) -> u8 {
        match self {
            Self::I64ExtendI32S | Self::I64ExtendI32U | Self::I64Extend32S => 32,
            Self::I32Extend8S | Self::I64Extend8S => 8,
            Self::I32Extend16S | Self::I64Extend16S => 16,
        }
    }

    pub fn to_bits(&self) -> u8 {
        match self {
            Self::I64ExtendI32S
            | Self::I64ExtendI32U
            | Self::I64Extend8S
            | Self::I64Extend16S
            | Self::I64Extend32S => 64,
            Self::I32Extend8S | Self::I32Extend16S => 32,
        }
    }
}

/// Kinds of vector extends in WebAssembly. Each MacroAssembler implementation
/// is responsible for emitting the correct sequence of instructions when
/// lowering to machine code.
pub(crate) enum VectorExtendKind {
    /// Sign extends eight 8 bit integers to eight 16 bit lanes.
    V128Extend8x8S,
    /// Zero extends eight 8 bit integers to eight 16 bit lanes.
    V128Extend8x8U,
    /// Sign extends four 16 bit integers to four 32 bit lanes.
    V128Extend16x4S,
    /// Zero extends four 16 bit integers to four 32 bit lanes.
    V128Extend16x4U,
    /// Sign extends two 32 bit integers to two 64 bit lanes.
    V128Extend32x2S,
    /// Zero extends two 32 bit integers to two 64 bit lanes.
    V128Extend32x2U,
}

/// Kinds of behavior supported by Wasm loads.
pub(crate) enum LoadKind {
    /// Do not extend or splat.
    Simple,
    /// Duplicate value into vector lanes.
    Splat,
    /// Scalar (non-vector) extend.
    ScalarExtend(ExtendKind),
    /// Vector extend.
    VectorExtend(VectorExtendKind),
}

/// Operand size, in bits.
#[derive(Copy, Debug, Clone, Eq, PartialEq)]
pub(crate) enum OperandSize {
    /// 8 bits.
    S8,
    /// 16 bits.
    S16,
    /// 32 bits.
    S32,
    /// 64 bits.
    S64,
    /// 128 bits.
    S128,
}

impl OperandSize {
    /// The number of bits in the operand.
    pub fn num_bits(&self) -> u8 {
        match self {
            OperandSize::S8 => 8,
            OperandSize::S16 => 16,
            OperandSize::S32 => 32,
            OperandSize::S64 => 64,
            OperandSize::S128 => 128,
        }
    }

    /// The number of bytes in the operand.
    pub fn bytes(&self) -> u32 {
        match self {
            Self::S8 => 1,
            Self::S16 => 2,
            Self::S32 => 4,
            Self::S64 => 8,
            Self::S128 => 16,
        }
    }

    /// The binary logarithm of the number of bits in the operand.
    pub fn log2(&self) -> u8 {
        match self {
            OperandSize::S8 => 3,
            OperandSize::S16 => 4,
            OperandSize::S32 => 5,
            OperandSize::S64 => 6,
            OperandSize::S128 => 7,
        }
    }

    /// Create an [`OperandSize`]  from the given number of bytes.
    pub fn from_bytes(bytes: u8) -> Self {
        use OperandSize::*;
        match bytes {
            4 => S32,
            8 => S64,
            16 => S128,
            _ => panic!("Invalid bytes {bytes} for OperandSize"),
        }
    }
}

/// An abstraction over a register or immediate.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum RegImm {
    /// A register.
    Reg(Reg),
    /// A tagged immediate argument.
    Imm(Imm),
}

/// An tagged representation of an immediate.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Imm {
    /// I32 immediate.
    I32(u32),
    /// I64 immediate.
    I64(u64),
    /// F32 immediate.
    F32(u32),
    /// F64 immediate.
    F64(u64),
    /// V128 immediate.
    V128(i128),
}

impl Imm {
    /// Create a new I64 immediate.
    pub fn i64(val: i64) -> Self {
        Self::I64(val as u64)
    }

    /// Create a new I32 immediate.
    pub fn i32(val: i32) -> Self {
        Self::I32(val as u32)
    }

    /// Create a new F32 immediate.
    pub fn f32(bits: u32) -> Self {
        Self::F32(bits)
    }

    /// Create a new F64 immediate.
    pub fn f64(bits: u64) -> Self {
        Self::F64(bits)
    }

    /// Create a new V128 immediate.
    pub fn v128(bits: i128) -> Self {
        Self::V128(bits)
    }

    /// Convert the immediate to i32, if possible.
    pub fn to_i32(&self) -> Option<i32> {
        match self {
            Self::I32(v) => Some(*v as i32),
            Self::I64(v) => i32::try_from(*v as i64).ok(),
            _ => None,
        }
    }

    /// Returns true if the [`Imm`] is float.
    pub fn is_float(&self) -> bool {
        match self {
            Self::F32(_) | Self::F64(_) => true,
            _ => false,
        }
    }

    /// Get the operand size of the immediate.
    pub fn size(&self) -> OperandSize {
        match self {
            Self::I32(_) | Self::F32(_) => OperandSize::S32,
            Self::I64(_) | Self::F64(_) => OperandSize::S64,
            Self::V128(_) => OperandSize::S128,
        }
    }
}

/// The location of the [VMcontext] used for function calls.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum VMContextLoc {
    /// Dynamic, stored in the given register.
    Reg(Reg),
    /// The pinned [VMContext] register.
    Pinned,
}

/// The maximum number of context arguments currently used across the compiler.
pub(crate) const MAX_CONTEXT_ARGS: usize = 2;

/// Out-of-band special purpose arguments used for function call emission.
///
/// We cannot rely on the value stack for these values given that inserting
/// register or memory values at arbitrary locations of the value stack has the
/// potential to break the stack ordering principle, which states that older
/// values must always precede newer values, effectively simulating the order of
/// values in the machine stack.
/// The [ContextArgs] are meant to be resolved at every callsite; in some cases
/// it might be possible to construct it early on, but given that it might
/// contain allocatable registers, it's preferred to construct it in
/// [FnCall::emit].
#[derive(Clone, Debug)]
pub(crate) enum ContextArgs {
    /// No context arguments required. This is used for libcalls that don't
    /// require any special context arguments. For example builtin functions
    /// that perform float calculations.
    None,
    /// A single context argument is required; the current pinned [VMcontext]
    /// register must be passed as the first argument of the function call.
    VMContext([VMContextLoc; 1]),
    /// The callee and caller context arguments are required. In this case, the
    /// callee context argument is usually stored into an allocatable register
    /// and the caller is always the current pinned [VMContext] pointer.
    CalleeAndCallerVMContext([VMContextLoc; MAX_CONTEXT_ARGS]),
}

impl ContextArgs {
    /// Construct an empty [ContextArgs].
    pub fn none() -> Self {
        Self::None
    }

    /// Construct a [ContextArgs] declaring the usage of the pinned [VMContext]
    /// register as both the caller and callee context arguments.
    pub fn pinned_callee_and_caller_vmctx() -> Self {
        Self::CalleeAndCallerVMContext([VMContextLoc::Pinned, VMContextLoc::Pinned])
    }

    /// Construct a [ContextArgs] that declares the usage of the pinned
    /// [VMContext] register as the only context argument.
    pub fn pinned_vmctx() -> Self {
        Self::VMContext([VMContextLoc::Pinned])
    }

    /// Construct a [ContextArgs] that declares a dynamic callee context and the
    /// pinned [VMContext] register as the context arguments.
    pub fn with_callee_and_pinned_caller(callee_vmctx: Reg) -> Self {
        Self::CalleeAndCallerVMContext([VMContextLoc::Reg(callee_vmctx), VMContextLoc::Pinned])
    }

    /// Get the length of the [ContextArgs].
    pub fn len(&self) -> usize {
        self.as_slice().len()
    }

    /// Get a slice of the context arguments.
    pub fn as_slice(&self) -> &[VMContextLoc] {
        match self {
            Self::None => &[],
            Self::VMContext(a) => a.as_slice(),
            Self::CalleeAndCallerVMContext(a) => a.as_slice(),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub(crate) enum CalleeKind {
    /// A function call to a raw address.
    Indirect(Reg),
    /// A function call to a local function.
    Direct(UserExternalNameRef),
    /// Call to a well known LibCall.
    LibCall(LibCall),
}

impl CalleeKind {
    /// Creates a callee kind from a register.
    pub fn indirect(reg: Reg) -> Self {
        Self::Indirect(reg)
    }

    /// Creates a direct callee kind from a function name.
    pub fn direct(name: UserExternalNameRef) -> Self {
        Self::Direct(name)
    }

    /// Creates a known callee kind from a libcall.
    pub fn libcall(call: LibCall) -> Self {
        Self::LibCall(call)
    }
}

impl RegImm {
    /// Register constructor.
    pub fn reg(r: Reg) -> Self {
        RegImm::Reg(r)
    }

    /// I64 immediate constructor.
    pub fn i64(val: i64) -> Self {
        RegImm::Imm(Imm::i64(val))
    }

    /// I32 immediate constructor.
    pub fn i32(val: i32) -> Self {
        RegImm::Imm(Imm::i32(val))
    }

    /// F32 immediate, stored using its bits representation.
    pub fn f32(bits: u32) -> Self {
        RegImm::Imm(Imm::f32(bits))
    }

    /// F64 immediate, stored using its bits representation.
    pub fn f64(bits: u64) -> Self {
        RegImm::Imm(Imm::f64(bits))
    }

    /// V128 immediate.
    pub fn v128(bits: i128) -> Self {
        RegImm::Imm(Imm::v128(bits))
    }
}

impl From<Reg> for RegImm {
    fn from(r: Reg) -> Self {
        Self::Reg(r)
    }
}

#[derive(Debug)]
pub enum RoundingMode {
    Nearest,
    Up,
    Down,
    Zero,
}

/// Memory flags for trusted loads/stores.
pub const TRUSTED_FLAGS: MemFlags = MemFlags::trusted();

/// Flags used for WebAssembly loads / stores.
/// Untrusted by default so we don't set `no_trap`.
/// We also ensure that the endianness is the right one for WebAssembly.
pub const UNTRUSTED_FLAGS: MemFlags = MemFlags::new().with_endianness(Endianness::Little);

/// Generic MacroAssembler interface used by the code generation.
///
/// The MacroAssembler trait aims to expose an interface, high-level enough,
/// so that each ISA can provide its own lowering to machine code. For example,
/// for WebAssembly operators that don't have a direct mapping to a machine
/// a instruction, the interface defines a signature matching the WebAssembly
/// operator, allowing each implementation to lower such operator entirely.
/// This approach attributes more responsibility to the MacroAssembler, but frees
/// the caller from concerning about assembling the right sequence of
/// instructions at the operator callsite.
///
/// The interface defaults to a three-argument form for binary operations;
/// this allows a natural mapping to instructions for RISC architectures,
/// that use three-argument form.
/// This approach allows for a more general interface that can be restricted
/// where needed, in the case of architectures that use a two-argument form.

pub(crate) trait MacroAssembler {
    /// The addressing mode.
    type Address: Copy + Debug;

    /// The pointer representation of the target ISA,
    /// used to access information from [`VMOffsets`].
    type Ptr: PtrSize;

    /// The ABI details of the target.
    type ABI: abi::ABI;

    /// Emit the function prologue.
    fn prologue(&mut self, vmctx: Reg) -> Result<()> {
        self.frame_setup()?;
        self.check_stack(vmctx)
    }

    /// Generate the frame setup sequence.
    fn frame_setup(&mut self) -> Result<()>;

    /// Generate the frame restore sequence.
    fn frame_restore(&mut self) -> Result<()>;

    /// Emit a stack check.
    fn check_stack(&mut self, vmctx: Reg) -> Result<()>;

    /// Emit the function epilogue.
    fn epilogue(&mut self) -> Result<()> {
        self.frame_restore()
    }

    /// Reserve stack space.
    fn reserve_stack(&mut self, bytes: u32) -> Result<()>;

    /// Free stack space.
    fn free_stack(&mut self, bytes: u32) -> Result<()>;

    /// Reset the stack pointer to the given offset;
    ///
    /// Used to reset the stack pointer to a given offset
    /// when dealing with unreachable code.
    fn reset_stack_pointer(&mut self, offset: SPOffset) -> Result<()>;

    /// Get the address of a local slot.
    fn local_address(&mut self, local: &LocalSlot) -> Result<Self::Address>;

    /// Constructs an address with an offset that is relative to the
    /// current position of the stack pointer (e.g. [sp + (sp_offset -
    /// offset)].
    fn address_from_sp(&self, offset: SPOffset) -> Result<Self::Address>;

    /// Constructs an address with an offset that is absolute to the
    /// current position of the stack pointer (e.g. [sp + offset].
    fn address_at_sp(&self, offset: SPOffset) -> Result<Self::Address>;

    /// Alias for [`Self::address_at_reg`] using the VMContext register as
    /// a base. The VMContext register is derived from the ABI type that is
    /// associated to the MacroAssembler.
    fn address_at_vmctx(&self, offset: u32) -> Result<Self::Address>;

    /// Construct an address that is absolute to the current position
    /// of the given register.
    fn address_at_reg(&self, reg: Reg, offset: u32) -> Result<Self::Address>;

    /// Emit a function call to either a local or external function.
    fn call(
        &mut self,
        stack_args_size: u32,
        f: impl FnMut(&mut Self) -> Result<(CalleeKind, CallingConvention)>,
    ) -> Result<u32>;

    /// Get stack pointer offset.
    fn sp_offset(&self) -> Result<SPOffset>;

    /// Perform a stack store.
    fn store(&mut self, src: RegImm, dst: Self::Address, size: OperandSize) -> Result<()>;

    /// Alias for `MacroAssembler::store` with the operand size corresponding
    /// to the pointer size of the target.
    fn store_ptr(&mut self, src: Reg, dst: Self::Address) -> Result<()>;

    /// Perform a WebAssembly store.
    /// A WebAssembly store introduces several additional invariants compared to
    /// [Self::store], more precisely, it can implicitly trap, in certain
    /// circumstances, even if explicit bounds checks are elided, in that sense,
    /// we consider this type of load as untrusted. It can also differ with
    /// regards to the endianness depending on the target ISA. For this reason,
    /// [Self::wasm_store], should be explicitly used when emitting WebAssembly
    /// stores.
    fn wasm_store(&mut self, src: Reg, dst: Self::Address, size: OperandSize) -> Result<()>;

    /// Perform a zero-extended stack load.
    fn load(&mut self, src: Self::Address, dst: WritableReg, size: OperandSize) -> Result<()>;

    /// Perform a WebAssembly load.
    /// A WebAssembly load introduces several additional invariants compared to
    /// [Self::load], more precisely, it can implicitly trap, in certain
    /// circumstances, even if explicit bounds checks are elided, in that sense,
    /// we consider this type of load as untrusted. It can also differ with
    /// regards to the endianness depending on the target ISA. For this reason,
    /// [Self::wasm_load], should be explicitly used when emitting WebAssembly
    /// loads.
    fn wasm_load(
        &mut self,
        src: Self::Address,
        dst: WritableReg,
        size: OperandSize,
        kind: LoadKind,
        op_kind: MemOpKind,
    ) -> Result<()>;

    /// Alias for `MacroAssembler::load` with the operand size corresponding
    /// to the pointer size of the target.
    fn load_ptr(&mut self, src: Self::Address, dst: WritableReg) -> Result<()>;

    /// Loads the effective address into destination.
    fn load_addr(
        &mut self,
        _src: Self::Address,
        _dst: WritableReg,
        _size: OperandSize,
    ) -> Result<()>;

    /// Pop a value from the machine stack into the given register.
    fn pop(&mut self, dst: WritableReg, size: OperandSize) -> Result<()>;

    /// Perform a move.
    fn mov(&mut self, dst: WritableReg, src: RegImm, size: OperandSize) -> Result<()>;

    /// Perform a conditional move.
    fn cmov(&mut self, dst: WritableReg, src: Reg, cc: IntCmpKind, size: OperandSize)
        -> Result<()>;

    /// Performs a memory move of bytes from src to dest.
    /// Bytes are moved in blocks of 8 bytes, where possible.
    fn memmove(
        &mut self,
        src: SPOffset,
        dst: SPOffset,
        bytes: u32,
        direction: MemMoveDirection,
    ) -> Result<()> {
        match direction {
            MemMoveDirection::LowToHigh => debug_assert!(dst.as_u32() < src.as_u32()),
            MemMoveDirection::HighToLow => debug_assert!(dst.as_u32() > src.as_u32()),
        }
        // At least 4 byte aligned.
        debug_assert!(bytes % 4 == 0);
        let mut remaining = bytes;
        let word_bytes = <Self::ABI as abi::ABI>::word_bytes();
        let scratch = scratch!(Self);

        let mut dst_offs = dst.as_u32() - bytes;
        let mut src_offs = src.as_u32() - bytes;

        let word_bytes = word_bytes as u32;
        while remaining >= word_bytes {
            remaining -= word_bytes;
            dst_offs += word_bytes;
            src_offs += word_bytes;

            self.load_ptr(
                self.address_from_sp(SPOffset::from_u32(src_offs))?,
                writable!(scratch),
            )?;
            self.store_ptr(
                scratch.into(),
                self.address_from_sp(SPOffset::from_u32(dst_offs))?,
            )?;
        }

        if remaining > 0 {
            let half_word = word_bytes / 2;
            let ptr_size = OperandSize::from_bytes(half_word as u8);
            debug_assert!(remaining == half_word);
            dst_offs += half_word;
            src_offs += half_word;

            self.load(
                self.address_from_sp(SPOffset::from_u32(src_offs))?,
                writable!(scratch),
                ptr_size,
            )?;
            self.store(
                scratch.into(),
                self.address_from_sp(SPOffset::from_u32(dst_offs))?,
                ptr_size,
            )?;
        }
        Ok(())
    }

    /// Perform add operation.
    fn add(&mut self, dst: WritableReg, lhs: Reg, rhs: RegImm, size: OperandSize) -> Result<()>;

    /// Perform a checked unsigned integer addition, emitting the provided trap
    /// if the addition overflows.
    fn checked_uadd(
        &mut self,
        dst: WritableReg,
        lhs: Reg,
        rhs: RegImm,
        size: OperandSize,
        trap: TrapCode,
    ) -> Result<()>;

    /// Perform subtraction operation.
    fn sub(&mut self, dst: WritableReg, lhs: Reg, rhs: RegImm, size: OperandSize) -> Result<()>;

    /// Perform multiplication operation.
    fn mul(&mut self, dst: WritableReg, lhs: Reg, rhs: RegImm, size: OperandSize) -> Result<()>;

    /// Perform a floating point add operation.
    fn float_add(&mut self, dst: WritableReg, lhs: Reg, rhs: Reg, size: OperandSize) -> Result<()>;

    /// Perform a floating point subtraction operation.
    fn float_sub(&mut self, dst: WritableReg, lhs: Reg, rhs: Reg, size: OperandSize) -> Result<()>;

    /// Perform a floating point multiply operation.
    fn float_mul(&mut self, dst: WritableReg, lhs: Reg, rhs: Reg, size: OperandSize) -> Result<()>;

    /// Perform a floating point divide operation.
    fn float_div(&mut self, dst: WritableReg, lhs: Reg, rhs: Reg, size: OperandSize) -> Result<()>;

    /// Perform a floating point minimum operation. In x86, this will emit
    /// multiple instructions.
    fn float_min(&mut self, dst: WritableReg, lhs: Reg, rhs: Reg, size: OperandSize) -> Result<()>;

    /// Perform a floating point maximum operation. In x86, this will emit
    /// multiple instructions.
    fn float_max(&mut self, dst: WritableReg, lhs: Reg, rhs: Reg, size: OperandSize) -> Result<()>;

    /// Perform a floating point copysign operation. In x86, this will emit
    /// multiple instructions.
    fn float_copysign(
        &mut self,
        dst: WritableReg,
        lhs: Reg,
        rhs: Reg,
        size: OperandSize,
    ) -> Result<()>;

    /// Perform a floating point abs operation.
    fn float_abs(&mut self, dst: WritableReg, size: OperandSize) -> Result<()>;

    /// Perform a floating point negation operation.
    fn float_neg(&mut self, dst: WritableReg, size: OperandSize) -> Result<()>;

    /// Perform a floating point floor operation.
    fn float_round<
        F: FnMut(&mut FuncEnv<Self::Ptr>, &mut CodeGenContext<Emission>, &mut Self) -> Result<()>,
    >(
        &mut self,
        mode: RoundingMode,
        env: &mut FuncEnv<Self::Ptr>,
        context: &mut CodeGenContext<Emission>,
        size: OperandSize,
        fallback: F,
    ) -> Result<()>;

    /// Perform a floating point square root operation.
    fn float_sqrt(&mut self, dst: WritableReg, src: Reg, size: OperandSize) -> Result<()>;

    /// Perform logical and operation.
    fn and(&mut self, dst: WritableReg, lhs: Reg, rhs: RegImm, size: OperandSize) -> Result<()>;

    /// Perform logical or operation.
    fn or(&mut self, dst: WritableReg, lhs: Reg, rhs: RegImm, size: OperandSize) -> Result<()>;

    /// Perform logical exclusive or operation.
    fn xor(&mut self, dst: WritableReg, lhs: Reg, rhs: RegImm, size: OperandSize) -> Result<()>;

    /// Perform a shift operation between a register and an immediate.
    fn shift_ir(
        &mut self,
        dst: WritableReg,
        imm: u64,
        lhs: Reg,
        kind: ShiftKind,
        size: OperandSize,
    ) -> Result<()>;

    /// Perform a shift operation between two registers.
    /// This case is special in that some architectures have specific expectations
    /// regarding the location of the instruction arguments. To free the
    /// caller from having to deal with the architecture specific constraints
    /// we give this function access to the code generation context, allowing
    /// each implementation to decide the lowering path.
    fn shift(
        &mut self,
        context: &mut CodeGenContext<Emission>,
        kind: ShiftKind,
        size: OperandSize,
    ) -> Result<()>;

    /// Perform division operation.
    /// Division is special in that some architectures have specific
    /// expectations regarding the location of the instruction
    /// arguments and regarding the location of the quotient /
    /// remainder. To free the caller from having to deal with the
    /// architecture specific constraints we give this function access
    /// to the code generation context, allowing each implementation
    /// to decide the lowering path.  For cases in which division is a
    /// unconstrained binary operation, the caller can decide to use
    /// the `CodeGenContext::i32_binop` or `CodeGenContext::i64_binop`
    /// functions.
    fn div(
        &mut self,
        context: &mut CodeGenContext<Emission>,
        kind: DivKind,
        size: OperandSize,
    ) -> Result<()>;

    /// Calculate remainder.
    fn rem(
        &mut self,
        context: &mut CodeGenContext<Emission>,
        kind: RemKind,
        size: OperandSize,
    ) -> Result<()>;

    /// Compares `src1` against `src2` for the side effect of setting processor
    /// flags.
    ///
    /// Note that `src1` is the left-hand-side of the comparison and `src2` is
    /// the right-hand-side, so if testing `a < b` then `src1 == a` and
    /// `src2 == b`
    fn cmp(&mut self, src1: Reg, src2: RegImm, size: OperandSize) -> Result<()>;

    /// Compare src and dst and put the result in dst.
    /// This function will potentially emit a series of instructions.
    ///
    /// The initial value in `dst` is the left-hand-side of the comparison and
    /// the initial value in `src` is the right-hand-side of the comparison.
    /// That means for `a < b` then `dst == a` and `src == b`.
    fn cmp_with_set(
        &mut self,
        dst: WritableReg,
        src: RegImm,
        kind: IntCmpKind,
        size: OperandSize,
    ) -> Result<()>;

    /// Compare floats in src1 and src2 and put the result in dst.
    /// In x86, this will emit multiple instructions.
    fn float_cmp_with_set(
        &mut self,
        dst: WritableReg,
        src1: Reg,
        src2: Reg,
        kind: FloatCmpKind,
        size: OperandSize,
    ) -> Result<()>;

    /// Count the number of leading zeroes in src and put the result in dst.
    /// In x64, this will emit multiple instructions if the `has_lzcnt` flag is
    /// false.
    fn clz(&mut self, dst: WritableReg, src: Reg, size: OperandSize) -> Result<()>;

    /// Count the number of trailing zeroes in src and put the result in dst.masm
    /// In x64, this will emit multiple instructions if the `has_tzcnt` flag is
    /// false.
    fn ctz(&mut self, dst: WritableReg, src: Reg, size: OperandSize) -> Result<()>;

    /// Push the register to the stack, returning the stack slot metadata.
    // NB
    // The stack alignment should not be assumed after any call to `push`,
    // unless explicitly aligned otherwise.  Typically, stack alignment is
    // maintained at call sites and during the execution of
    // epilogues.
    fn push(&mut self, src: Reg, size: OperandSize) -> Result<StackSlot>;

    /// Finalize the assembly and return the result.
    fn finalize(self, base: Option<SourceLoc>) -> Result<MachBufferFinalized<Final>>;

    /// Zero a particular register.
    fn zero(&mut self, reg: WritableReg) -> Result<()>;

    /// Count the number of 1 bits in src and put the result in dst. In x64,
    /// this will emit multiple instructions if the `has_popcnt` flag is false.
    fn popcnt(&mut self, context: &mut CodeGenContext<Emission>, size: OperandSize) -> Result<()>;

    /// Converts an i64 to an i32 by discarding the high 32 bits.
    fn wrap(&mut self, dst: WritableReg, src: Reg) -> Result<()>;

    /// Extends an integer of a given size to a larger size.
    fn extend(&mut self, dst: WritableReg, src: Reg, kind: ExtendKind) -> Result<()>;

    /// Emits one or more instructions to perform a signed truncation of a
    /// float into an integer.
    fn signed_truncate(
        &mut self,
        dst: WritableReg,
        src: Reg,
        src_size: OperandSize,
        dst_size: OperandSize,
        kind: TruncKind,
    ) -> Result<()>;

    /// Emits one or more instructions to perform an unsigned truncation of a
    /// float into an integer.
    fn unsigned_truncate(
        &mut self,
        context: &mut CodeGenContext<Emission>,
        src_size: OperandSize,
        dst_size: OperandSize,
        kind: TruncKind,
    ) -> Result<()>;

    /// Emits one or more instructions to perform a signed convert of an
    /// integer into a float.
    fn signed_convert(
        &mut self,
        dst: WritableReg,
        src: Reg,
        src_size: OperandSize,
        dst_size: OperandSize,
    ) -> Result<()>;

    /// Emits one or more instructions to perform an unsigned convert of an
    /// integer into a float.
    fn unsigned_convert(
        &mut self,
        dst: WritableReg,
        src: Reg,
        tmp_gpr: Reg,
        src_size: OperandSize,
        dst_size: OperandSize,
    ) -> Result<()>;

    /// Reinterpret a float as an integer.
    fn reinterpret_float_as_int(
        &mut self,
        dst: WritableReg,
        src: Reg,
        size: OperandSize,
    ) -> Result<()>;

    /// Reinterpret an integer as a float.
    fn reinterpret_int_as_float(
        &mut self,
        dst: WritableReg,
        src: Reg,
        size: OperandSize,
    ) -> Result<()>;

    /// Demote an f64 to an f32.
    fn demote(&mut self, dst: WritableReg, src: Reg) -> Result<()>;

    /// Promote an f32 to an f64.
    fn promote(&mut self, dst: WritableReg, src: Reg) -> Result<()>;

    /// Zero a given memory range.
    ///
    /// The default implementation divides the given memory range
    /// into word-sized slots. Then it unrolls a series of store
    /// instructions, effectively assigning zero to each slot.
    fn zero_mem_range(&mut self, mem: &Range<u32>) -> Result<()> {
        let word_size = <Self::ABI as abi::ABI>::word_bytes() as u32;
        if mem.is_empty() {
            return Ok(());
        }

        let start = if mem.start % word_size == 0 {
            mem.start
        } else {
            // Ensure that the start of the range is at least 4-byte aligned.
            assert!(mem.start % 4 == 0);
            let start = align_to(mem.start, word_size);
            let addr: Self::Address = self.local_address(&LocalSlot::i32(start))?;
            self.store(RegImm::i32(0), addr, OperandSize::S32)?;
            // Ensure that the new start of the range, is word-size aligned.
            assert!(start % word_size == 0);
            start
        };

        let end = align_to(mem.end, word_size);
        let slots = (end - start) / word_size;

        if slots == 1 {
            let slot = LocalSlot::i64(start + word_size);
            let addr: Self::Address = self.local_address(&slot)?;
            self.store(RegImm::i64(0), addr, OperandSize::S64)?;
        } else {
            // TODO
            // Add an upper bound to this generation;
            // given a considerably large amount of slots
            // this will be inefficient.
            let zero = scratch!(Self);
            self.zero(writable!(zero))?;
            let zero = RegImm::reg(zero);

            for step in (start..end).into_iter().step_by(word_size as usize) {
                let slot = LocalSlot::i64(step + word_size);
                let addr: Self::Address = self.local_address(&slot)?;
                self.store(zero, addr, OperandSize::S64)?;
            }
        }

        Ok(())
    }

    /// Generate a label.
    fn get_label(&mut self) -> Result<MachLabel>;

    /// Bind the given label at the current code offset.
    fn bind(&mut self, label: MachLabel) -> Result<()>;

    /// Conditional branch.
    ///
    /// Performs a comparison between the two operands,
    /// and immediately after emits a jump to the given
    /// label destination if the condition is met.
    fn branch(
        &mut self,
        kind: IntCmpKind,
        lhs: Reg,
        rhs: RegImm,
        taken: MachLabel,
        size: OperandSize,
    ) -> Result<()>;

    /// Emits and unconditional jump to the given label.
    fn jmp(&mut self, target: MachLabel) -> Result<()>;

    /// Emits a jump table sequence. The default label is specified as
    /// the last element of the targets slice.
    fn jmp_table(&mut self, targets: &[MachLabel], index: Reg, tmp: Reg) -> Result<()>;

    /// Emit an unreachable code trap.
    fn unreachable(&mut self) -> Result<()>;

    /// Emit an unconditional trap.
    fn trap(&mut self, code: TrapCode) -> Result<()>;

    /// Traps if the condition code is met.
    fn trapif(&mut self, cc: IntCmpKind, code: TrapCode) -> Result<()>;

    /// Trap if the source register is zero.
    fn trapz(&mut self, src: Reg, code: TrapCode) -> Result<()>;

    /// Ensures that the stack pointer is correctly positioned before an unconditional
    /// jump according to the requirements of the destination target.
    fn ensure_sp_for_jump(&mut self, target: SPOffset) -> Result<()> {
        let bytes = self
            .sp_offset()?
            .as_u32()
            .checked_sub(target.as_u32())
            .unwrap_or(0);

        if bytes > 0 {
            self.free_stack(bytes)?;
        }

        Ok(())
    }

    /// Mark the start of a source location returning the machine code offset
    /// and the relative source code location.
    fn start_source_loc(&mut self, loc: RelSourceLoc) -> Result<(CodeOffset, RelSourceLoc)>;

    /// Mark the end of a source location.
    fn end_source_loc(&mut self) -> Result<()>;

    /// The current offset, in bytes from the beginning of the function.
    fn current_code_offset(&self) -> Result<CodeOffset>;

    /// Performs a 128-bit addition
    fn add128(
        &mut self,
        dst_lo: WritableReg,
        dst_hi: WritableReg,
        lhs_lo: Reg,
        lhs_hi: Reg,
        rhs_lo: Reg,
        rhs_hi: Reg,
    ) -> Result<()>;

    /// Performs a 128-bit subtraction
    fn sub128(
        &mut self,
        dst_lo: WritableReg,
        dst_hi: WritableReg,
        lhs_lo: Reg,
        lhs_hi: Reg,
        rhs_lo: Reg,
        rhs_hi: Reg,
    ) -> Result<()>;

    /// Performs a widening multiplication from two 64-bit operands into a
    /// 128-bit result.
    ///
    /// Note that some platforms require special handling of registers in this
    /// instruction (e.g. x64) so full access to `CodeGenContext` is provided.
    fn mul_wide(&mut self, context: &mut CodeGenContext<Emission>, kind: MulWideKind)
        -> Result<()>;

    /// Performs a shuffle between two 128-bit vectors into a 128-bit result
    /// using lanes as a mask to select which indexes to copy.
    fn shuffle(&mut self, dst: WritableReg, lhs: Reg, rhs: Reg, lanes: [u8; 16]) -> Result<()>;
}
