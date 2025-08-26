//! A Cranelift-specific x64 assembler.
//!
//! All instructions known to this assembler are listed in the [`inst`] module.
//! The [`Inst`] enumeration contains a variant for each, allowing matching over
//! all these instructions. All of this is parameterized by a [`Registers`]
//! trait, allowing users of this assembler to plug in their own register types.
//!
//! ```
//! # use cranelift_assembler_x64::{Fixed, Imm8, inst, Inst, Registers};
//! // Tell the assembler the type of registers we're using; we can always
//! // encode a HW register as a `u8` (e.g., `eax = 0`).
//! pub struct Regs;
//! impl Registers for Regs {
//!     type ReadGpr = u8;
//!     type ReadWriteGpr = u8;
//!     type WriteGpr = u8;
//!     type ReadXmm = u8;
//!     type ReadWriteXmm = u8;
//!     type WriteXmm = u8;
//! }
//!
//! // Then, build one of the `AND` instructions; this one operates on an
//! // implicit `AL` register with an immediate. We can collect a sequence of
//! // instructions by converting to the `Inst` type.
//! let rax: u8 = 0;
//! let and = inst::andb_i::new(Fixed(rax), Imm8::new(0b10101010));
//! let seq: Vec<Inst<Regs>> = vec![and.into()];
//!
//! // Now we can encode this sequence into a code buffer.
//! let mut buffer = vec![];
//! for inst in seq {
//!     inst.encode(&mut buffer);
//! }
//! assert_eq!(buffer, vec![0x24, 0b10101010]);
//! ```
//!
//! With an [`Inst`], we can encode the instruction into a code buffer; see the
//! [example](Inst).

#![allow(
    non_camel_case_types,
    reason = "all of the generated struct names use snake case"
)]

mod api;
mod custom;
mod evex;
mod features;
mod fixed;
pub mod gpr;
mod imm;
pub mod inst;
mod mem;
mod rex;
mod vex;
pub mod xmm;

#[cfg(any(test, feature = "fuzz"))]
pub mod fuzz;

/// An assembly instruction; contains all instructions known to the assembler.
///
/// This wraps all [`inst`] structures into a single enumeration for collecting
/// instructions.
#[doc(inline)]
// This re-exports, and documents, a module that is more convenient to use at
// the library top-level.
pub use inst::Inst;

pub use api::{
    AsReg, CodeSink, Constant, KnownOffset, Label, RegisterVisitor, Registers, TrapCode,
};
pub use features::{AvailableFeatures, Feature, Features};
pub use fixed::Fixed;
pub use gpr::{Gpr, NonRspGpr, Size};
pub use imm::{Extension, Imm8, Imm16, Imm32, Imm64, Simm8, Simm16, Simm32};
pub use mem::{
    Amode, AmodeOffset, AmodeOffsetPlusKnownOffset, DeferredTarget, GprMem, Scale, XmmMem,
};
pub use rex::RexPrefix;
pub use xmm::Xmm;
