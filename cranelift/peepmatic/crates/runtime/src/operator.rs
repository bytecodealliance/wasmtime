//! Operator definitions.

use peepmatic_macro::PeepmaticOperator;
use serde::{Deserialize, Serialize};

/// An operator.
///
/// These are a subset of Cranelift IR's operators.
///
/// ## Caveats for Branching and Trapping Operators
///
/// Branching operators are not fully modeled: we do not represent their label
/// and jump arguments. It is up to the interpreter doing the instruction
/// replacement to recognize when we are replacing one branch with another, and
/// copy over the extra information.
///
/// Affected operations: `brz`, `brnz`, `trapz`, `trapnz`.
#[derive(PeepmaticOperator, Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
#[repr(u32)]
pub enum Operator {
    /// `adjust_sp_down`
    #[peepmatic(params(iNN), result(void))]
    // NB: We convert `Operator`s into `NonZeroU32`s with unchecked casts;
    // memory safety relies on `Operator` starting at `1` and no variant ever
    // being zero.
    AdjustSpDown = 1,

    /// `adjust_sp_down_imm`
    #[peepmatic(immediates(iNN), result(void))]
    AdjustSpDownImm,

    /// `band`
    #[peepmatic(params(iNN, iNN), result(iNN))]
    Band,

    /// `band_imm`
    #[peepmatic(immediates(iNN), params(iNN), result(iNN))]
    BandImm,

    /// `bconst`
    #[peepmatic(immediates(b1), result(bNN))]
    Bconst,

    /// `bint`
    #[peepmatic(params(bNN), result(iNN))]
    Bint,

    /// `bor`
    #[peepmatic(params(iNN, iNN), result(iNN))]
    Bor,

    /// `bor_imm`
    #[peepmatic(immediates(iNN), params(iNN), result(iNN))]
    BorImm,

    /// `brnz`
    #[peepmatic(params(bool_or_int), result(void))]
    Brnz,

    /// `brz`
    #[peepmatic(params(bool_or_int), result(void))]
    Brz,

    /// `bxor`
    #[peepmatic(params(iNN, iNN), result(iNN))]
    Bxor,

    /// `bxor_imm`
    #[peepmatic(immediates(iNN), params(iNN), result(iNN))]
    BxorImm,

    /// `iadd`
    #[peepmatic(params(iNN, iNN), result(iNN))]
    Iadd,

    /// `iadd_imm`
    #[peepmatic(immediates(iNN), params(iNN), result(iNN))]
    IaddImm,

    /// `icmp`
    #[peepmatic(immediates(cc), params(iNN, iNN), result(b1))]
    Icmp,

    /// `icmp_imm`
    #[peepmatic(immediates(cc, iNN), params(iNN), result(b1))]
    IcmpImm,

    /// `iconst`
    #[peepmatic(immediates(iNN), result(iNN))]
    Iconst,

    /// `ifcmp`
    #[peepmatic(params(iNN, iNN), result(cpu_flags))]
    Ifcmp,

    /// `ifcmp_imm`
    #[peepmatic(immediates(iNN), params(iNN), result(cpu_flags))]
    IfcmpImm,

    /// `imul`
    #[peepmatic(params(iNN, iNN), result(iNN))]
    Imul,

    /// `imul_imm`
    #[peepmatic(immediates(iNN), params(iNN), result(iNN))]
    ImulImm,

    /// `ireduce`
    #[peepmatic(params(iNN), result(iMM))]
    Ireduce,

    /// `irsub_imm`
    #[peepmatic(immediates(iNN), params(iNN), result(iNN))]
    IrsubImm,

    /// `ishl`
    #[peepmatic(params(iNN, iNN), result(iNN))]
    Ishl,

    /// `ishl_imm`
    #[peepmatic(immediates(iNN), params(iNN), result(iNN))]
    IshlImm,

    /// `isub`
    #[peepmatic(params(iNN, iNN), result(iNN))]
    Isub,

    /// `rotl`
    #[peepmatic(params(iNN, iNN), result(iNN))]
    Rotl,

    /// `rotl_imm`
    #[peepmatic(immediates(iNN), params(iNN), result(iNN))]
    RotlImm,

    /// `rotr`
    #[peepmatic(params(iNN, iNN), result(iNN))]
    Rotr,

    /// `rotr_imm`
    #[peepmatic(immediates(iNN), params(iNN), result(iNN))]
    RotrImm,

    /// `sdiv`
    #[peepmatic(params(iNN, iNN), result(iNN))]
    Sdiv,

    /// `sdiv_imm`
    #[peepmatic(immediates(iNN), params(iNN), result(iNN))]
    SdivImm,

    /// `select`
    #[peepmatic(params(bool_or_int, any_t, any_t), result(any_t))]
    Select,

    /// `sextend`
    #[peepmatic(params(iNN), result(iMM))]
    Sextend,

    /// `srem`
    #[peepmatic(params(iNN, iNN), result(iNN))]
    Srem,

    /// `srem_imm`
    #[peepmatic(immediates(iNN), params(iNN), result(iNN))]
    SremImm,

    /// `sshr`
    #[peepmatic(params(iNN, iNN), result(iNN))]
    Sshr,

    /// `sshr_imm`
    #[peepmatic(immediates(iNN), params(iNN), result(iNN))]
    SshrImm,

    /// `trapnz`
    #[peepmatic(params(bool_or_int), result(void))]
    Trapnz,

    /// `trapz`
    #[peepmatic(params(bool_or_int), result(void))]
    Trapz,

    /// `udiv`
    #[peepmatic(params(iNN, iNN), result(iNN))]
    Udiv,

    /// `udiv_imm`
    #[peepmatic(immediates(iNN), params(iNN), result(iNN))]
    UdivImm,

    /// `uextend`
    #[peepmatic(params(iNN), result(iMM))]
    Uextend,

    /// `urem`
    #[peepmatic(params(iNN, iNN), result(iNN))]
    Urem,

    /// `urem_imm`
    #[peepmatic(immediates(iNN), params(iNN), result(iNN))]
    UremImm,

    /// `ushr`
    #[peepmatic(params(iNN, iNN), result(iNN))]
    Ushr,

    /// `ushr_imm`
    #[peepmatic(immediates(iNN), params(iNN), result(iNN))]
    UshrImm,
}

/// Compile-time unquote operators.
///
/// These are used in the right-hand side to perform compile-time evaluation of
/// constants matched on the left-hand side.
#[derive(PeepmaticOperator, Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
#[repr(u32)]
pub enum UnquoteOperator {
    /// Compile-time `band` of two constant values.
    #[peepmatic(params(iNN, iNN), result(iNN))]
    Band,

    /// Compile-time `bor` of two constant values.
    #[peepmatic(params(iNN, iNN), result(iNN))]
    Bor,

    /// Compile-time `bxor` of two constant values.
    #[peepmatic(params(iNN, iNN), result(iNN))]
    Bxor,

    /// Compile-time `iadd` of two constant values.
    #[peepmatic(params(iNN, iNN), result(iNN))]
    Iadd,

    /// Compile-time `imul` of two constant values.
    #[peepmatic(params(iNN, iNN), result(iNN))]
    Imul,

    /// Compile-time `isub` of two constant values.
    #[peepmatic(params(iNN, iNN), result(iNN))]
    Isub,

    /// Take the base-2 log of a power of two integer.
    #[peepmatic(params(iNN), result(iNN))]
    Log2,

    /// Wrapping negation of an integer.
    #[peepmatic(params(iNN), result(iNN))]
    Neg,
}

/// A trait to represent a typing context.
///
/// This is used by the macro-generated operator methods that create the type
/// variables for their immediates, parameters, and results. This trait is
/// implemented by the concrete typing context in `peepmatic/src/verify.rs`.
#[cfg(feature = "construct")]
pub trait TypingContext<'a> {
    /// A type variable.
    type TypeVariable;

    /// Create a condition code type.
    fn cc(&mut self, span: wast::Span) -> Self::TypeVariable;

    /// Create a boolean type with a polymorphic bit width.
    ///
    /// Each use of `bNN` by the same operator refers to the same type variable.
    #[allow(non_snake_case)]
    fn bNN(&mut self, span: wast::Span) -> Self::TypeVariable;

    /// Create an integer type with a polymorphic bit width.
    ///
    /// Each use of `iNN` by the same operator refers to the same type variable.
    #[allow(non_snake_case)]
    fn iNN(&mut self, span: wast::Span) -> Self::TypeVariable;

    /// Create an integer type with a polymorphic bit width.
    ///
    /// Each use of `iMM` by the same operator refers to the same type variable.
    #[allow(non_snake_case)]
    fn iMM(&mut self, span: wast::Span) -> Self::TypeVariable;

    /// Create the CPU flags type variable.
    fn cpu_flags(&mut self, span: wast::Span) -> Self::TypeVariable;

    /// Create a boolean type of size one bit.
    fn b1(&mut self, span: wast::Span) -> Self::TypeVariable;

    /// Create the void type, used as the result of operators that branch away,
    /// or do not return anything.
    fn void(&mut self, span: wast::Span) -> Self::TypeVariable;

    /// Create a type variable that may be either a boolean or an integer.
    fn bool_or_int(&mut self, span: wast::Span) -> Self::TypeVariable;

    /// Create a type variable that can be any type T.
    ///
    /// Each use of `any_t` by the same operator refers to the same type
    /// variable.
    fn any_t(&mut self, span: wast::Span) -> Self::TypeVariable;
}
