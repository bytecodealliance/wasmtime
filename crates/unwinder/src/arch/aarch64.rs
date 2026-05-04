//! Arm64-specific definitions of architecture-specific functions in Wasmtime.
//!
//! ## ILP32 vs LP64 on aarch64
//!
//! Aarch64 GPRs are 64 bits regardless of pointer width. On the usual
//! `aarch64-*` (LP64) targets `usize` is `u64` and the two are
//! interchangeable, but on `arm64_32-apple-watchos` (ILP32 ABI: 64-bit
//! registers, 32-bit pointers) `usize` is `u32`. An inline-asm operand
//! typed as `usize` is therefore ambiguous on `arm64_32` between the
//! `w<N>` (32-bit lane) and `x<N>` (64-bit GPR) views — exactly what
//! rustc's `asm_sub_register` lint flags. The Rust Reference is also
//! explicit that the upper bits of a register holding a sub-register-
//! width input are *undefined*[0]; relying on the ISA-side zero-extend
//! that aarch64 happens to perform on `mov w<N>, ...` would be relying
//! on a property the language doesn't promise.
//!
//! The public functions in this module keep their `usize` signatures —
//! that's the convention shared with the other unwinder backends
//! (`x86.rs`, `riscv64.rs`, `s390x.rs`), and the `u64`-vs-pointer-width
//! split is unique to aarch64-on-ILP32. Inside this module, any
//! register-bearing local that participates in inline asm is typed
//! `u64` so the operand class is unambiguously the 64-bit GPR view.
//! The cross-boundary casts are explicit:
//!
//!   - `u64::try_from(v).unwrap()` widens `usize` → `u64`. Infallible
//!     on every supported Rust target (`usize` is at most 64 bits
//!     everywhere today), and the `.unwrap()` documents that any
//!     failure would be a target-property issue rather than a runtime
//!     one.
//!   - `as usize` narrows `u64` → `usize` at the return. Truncates on
//!     `arm64_32` by design — the saved PC/SP there is a 32-bit host
//!     pointer that fits exactly in the low 32 bits of the register —
//!     and is the identity on aarch64 LP64 targets.
//!
//! ## AAPCS64 frame-record stride
//!
//! Reads of the AAPCS64 frame record (saved FP / saved LR) use
//! `*mut u64` rather than `*mut usize`. AAPCS64 reserves two 64-bit
//! slots for the frame record on every aarch64 ABI variant — including
//! `arm64_32` — so `.offset(1)` advancing by 8 bytes is correct
//! regardless of pointer width. With `*mut usize` on `arm64_32`
//! `.offset(1)` would advance by only 4 bytes and read the upper half
//! of the saved-FP slot. This matters for a future `arm64_32` Cranelift
//! port; today the `arm64_32-apple-watchos` toolchain only runs Pulley.
//!
//! [0]: https://doc.rust-lang.org/reference/inline-assembly.html#r-asm.register-operands.smaller-value

#[inline]
pub fn get_stack_pointer() -> usize {
    let stack_pointer: u64;
    unsafe {
        core::arch::asm!(
            "mov {}, sp",
            out(reg) stack_pointer,
            options(nostack,nomem),
        );
    }
    // Truncates u64 → u32 on arm64_32 (the host SP is a 32-bit pointer
    // that fits in the low 32 bits); identity on aarch64 LP64.
    stack_pointer as usize
}

// The aarch64 calling conventions save the return PC one i64 above the FP and
// the previous FP is pointed to by the current FP:
//
// > Each frame shall link to the frame of its caller by means of a frame record
// > of two 64-bit values on the stack [...] The frame record for the innermost
// > frame [...] shall be pointed to by the frame pointer register (FP). The
// > lowest addressed double-word shall point to the previous frame record and the
// > highest addressed double-word shall contain the value passed in LR on entry
// > to the current function.
//
// - AAPCS64 section 6.2.3 The Frame Pointer[0]
pub unsafe fn get_next_older_pc_from_fp(fp: usize) -> usize {
    unsafe {
        // `*mut u64` (not `*mut usize`) so `.offset(1)` advances by 8 bytes
        // on every aarch64 ABI variant — see module docs.
        let mut pc: u64 = *(fp as *mut u64).offset(1);

        // The return address might be signed, so we need to strip the highest bits
        // (where the authentication code might be located) in order to obtain a
        // valid address. We use the `XPACLRI` instruction, which is executed as a
        // no-op by processors that do not support pointer authentication, so that
        // the implementation is backward-compatible and there is no duplication.
        // However, this instruction requires the LR register for both its input and
        // output.
        //
        // `pc` is `u64` so the operand class is unambiguously `x<N>` (the 64-bit
        // GPR view); see module docs for why.
        core::arch::asm!(
            "mov lr, {pc}",
            "xpaclri",
            "mov {pc}, lr",
            pc = inout(reg) pc,
            out("lr") _,
            options(nomem, nostack, preserves_flags, pure),
        );

        // Truncates u64 → u32 on arm64_32 (the saved PC there is a 32-bit
        // host pointer; XPACLRI strips any PAC bits on v8.3+, no-op on
        // earlier cores like the A12 in Apple Watch SE2's S8 SoC);
        // identity on aarch64 LP64.
        pc as usize
    }
}

pub unsafe fn resume_to_exception_handler(
    pc: usize,
    sp: usize,
    fp: usize,
    payload1: usize,
    payload2: usize,
) -> ! {
    // The asm operands name registers explicitly (`in("x0")` etc.), so the
    // `asm_sub_register` lint doesn't fire here even with `usize` operands.
    // Widen anyway for consistency with the rest of this module's "register-
    // bearing locals are `u64`" rule — see module docs.
    let pc = u64::try_from(pc).unwrap();
    let sp = u64::try_from(sp).unwrap();
    let fp = u64::try_from(fp).unwrap();
    let payload1 = u64::try_from(payload1).unwrap();
    let payload2 = u64::try_from(payload2).unwrap();
    unsafe {
        core::arch::asm!(
            "mov sp, x2",
            "mov fp, x3",
            "br x4",
            in("x0") payload1,
            in("x1") payload2,
            in("x2") sp,
            in("x3") fp,
            in("x4") pc,
            options(nostack, nomem, noreturn),
        );
    }
}

// And the current frame pointer points to the next older frame pointer.
pub const NEXT_OLDER_FP_FROM_FP_OFFSET: usize = 0;

// SP of caller is FP in callee plus size of FP/return address pair.
pub const NEXT_OLDER_SP_FROM_FP_OFFSET: usize = 16;

pub fn assert_fp_is_aligned(_fp: usize) {
    // From AAPCS64, section 6.2.3 The Frame Pointer[0]:
    //
    // > The location of the frame record within a stack frame is not specified.
    //
    // So this presumably means that the FP can have any alignment, as its
    // location is not specified and nothing further is said about constraining
    // alignment.
    //
    // [0]: https://github.com/ARM-software/abi-aa/blob/2022Q1/aapcs64/aapcs64.rst#the-frame-pointer
}
