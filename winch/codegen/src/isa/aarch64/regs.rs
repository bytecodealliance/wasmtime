//! AArch64 register definition.

use crate::isa::reg::Reg;
use regalloc2::{PReg, RegClass};

/// FPR index bound.
pub(crate) const MAX_FPR: u32 = 32;
/// FPR index bound.
pub(crate) const MAX_GPR: u32 = 32;

/// Construct a X-register from an index.
pub(crate) const fn xreg(num: u8) -> Reg {
    assert!((num as u32) < MAX_GPR);
    Reg::new(PReg::new(num as usize, RegClass::Int))
}

/// Construct a V-register from an index.
pub(crate) const fn vreg(num: u8) -> Reg {
    assert!((num as u32) < MAX_FPR);
    Reg::new(PReg::new(num as usize, RegClass::Float))
}

/// Scratch register.
/// Intra-procedure-call corruptible register.
pub(crate) const fn ip0() -> Reg {
    xreg(16)
}

/// Alias to the IP0 register.
pub(crate) const fn scratch() -> Reg {
    ip0()
}

// Alias to register v31.
pub(crate) const fn float_scratch() -> Reg {
    vreg(31)
}

/// Scratch register.
/// Intra-procedure-call corruptible register.
pub(crate) const fn ip1() -> Reg {
    xreg(17)
}

/// Register used to carry platform state.
const fn platform() -> Reg {
    xreg(18)
}

/// Frame pointer register.
pub(crate) const fn fp() -> Reg {
    xreg(29)
}

/// Link register for function calls.
pub(crate) const fn lr() -> Reg {
    xreg(30)
}

/// Zero register.
pub(crate) const fn zero() -> Reg {
    xreg(31)
}

/// The VM context register.
pub(crate) const fn vmctx() -> Reg {
    xreg(9)
}

/// Stack pointer register.
///
/// In aarch64 the zero and stack pointer registers are contextually
/// different but have the same hardware encoding; to differentiate
/// them, we are following Cranelift's encoding and representing it as
/// 31 + 32.  Ref:
/// https://github.com/bytecodealliance/wasmtime/blob/main/cranelift/codegen/src/isa/aarch64/inst/regs.rs#L70
pub(crate) const fn sp() -> Reg {
    Reg::new(PReg::new(31 + 32, RegClass::Int))
}

/// Shadow stack pointer register.
///
/// The shadow stack pointer is used as the base for memory addressing
/// to workaround Aarch64's constraint on the stack pointer 16-byte
/// alignment for memory addressing. This allows word-size loads and
/// stores.  It's always assumed that the real stack pointer is
/// 16-byte unaligned; the only exceptions to this assumption are the function
/// prologue and epilogue in which we use the real stack pointer for
/// addressing, assuming that the 16-byte alignment is respected.
///
/// The fact that the shadow stack pointer is used for memory
/// addressing, doesn't change the meaning of the real stack pointer,
/// which should always be used to allocate and deallocate stack
/// space. The real stack pointer is always treated as "primary".
/// Throughout the code generation any change to the stack pointer is
/// reflected in the shadow stack pointer via the
/// [MacroAssembler::move_sp_to_shadow_sp] function.
///
/// This approach, requires copying the real stack pointer value into
/// x28 every time the real stack pointer moves, which involves
/// emitting one more instruction. For example, this is generally how
/// the real stack pointer and x28 will look like during a function:
///
/// +-----------+
/// |           |      Save x28 (callee-saved)
/// +-----------+----- SP at function entry (after epilogue, slots for FP and LR)
/// |           |      Copy the value of SP to x28
/// |           |
/// +-----------+----- SP after reserving stack space for locals and arguments
/// |           |      Copy the value of SP to x28
/// |           |
/// +-----------+----- SP after a push
/// |           |      Copy the value of SP to x28 (similar after a pop)
/// |           |      
/// |           |       
/// |           |
/// |           |
/// +-----------+----- At epilogue restore x28 (callee-saved)
/// +-----------+
///
/// In summary, the following invariants must be respected:
///
/// * The real stack pointer is always primary, and must be used to
///   allocate and deallocate stack space(e.g. push, pop). This
///   operation must always be followed by a copy of the real stack
///   pointer to x28.
/// * The real stack pointer must never be used to
///   address memory except when we are certain that the required
///   alignment is respected (e.g.  during the prologue and epilogue)
/// * The value of the real stack pointer is copied to x28 when
///   entering a function.
/// * The value of x28 doesn't change between
///   function calls (as it's callee saved), compliant with
///   Aarch64's ABI.
/// * x28 is not available during register allocation.
/// * Since the real stack pointer is always primary, there's no need
///   to copy the shadow stack pointer into the real stack
///   pointer. The copy is only done SP -> Shadow SP direction.
pub(crate) const fn shadow_sp() -> Reg {
    xreg(28)
}

/// Bitmask for non-allocatable GPR.
pub(crate) const NON_ALLOCATABLE_GPR: u32 = (1 << ip0().hw_enc())
    | (1 << ip1().hw_enc())
    | (1 << platform().hw_enc())
    | (1 << fp().hw_enc())
    | (1 << lr().hw_enc())
    | (1 << zero().hw_enc())
    | (1 << shadow_sp().hw_enc())
    | (1 << vmctx().hw_enc());

/// Bitmask to represent the available general purpose registers.
pub(crate) const ALL_GPR: u32 = u32::MAX & !NON_ALLOCATABLE_GPR;

/// Bitmask for non-allocatable FPR.
/// All FPRs are allocatable, v0..=v7 are generally used for params and results.
pub(crate) const NON_ALLOCATABLE_FPR: u32 = 1 << float_scratch().hw_enc();

/// Bitmask to represent the available floating point registers.
pub(crate) const ALL_FPR: u32 = u32::MAX & !NON_ALLOCATABLE_FPR;
