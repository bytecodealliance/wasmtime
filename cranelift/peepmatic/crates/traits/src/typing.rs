/// A trait to represent a typing context.
///
/// This is used by the macro-generated operator methods that create the type
/// variables for their immediates, parameters, and results. This trait is
/// implemented by the concrete typing context in `peepmatic/src/verify.rs`.
pub trait TypingContext<'a> {
    /// A source span.
    type Span: Copy;

    /// A type variable.
    type TypeVariable;

    /// Create a condition code type.
    fn cc(&mut self, span: Self::Span) -> Self::TypeVariable;

    /// Create a boolean type with a polymorphic bit width.
    ///
    /// Each use of `bNN` by the same operator refers to the same type variable.
    #[allow(non_snake_case)]
    fn bNN(&mut self, span: Self::Span) -> Self::TypeVariable;

    /// Create an integer type with a polymorphic bit width.
    ///
    /// Each use of `iNN` by the same operator refers to the same type variable.
    #[allow(non_snake_case)]
    fn iNN(&mut self, span: Self::Span) -> Self::TypeVariable;

    /// Create an integer type with a polymorphic bit width.
    ///
    /// Each use of `iMM` by the same operator refers to the same type variable.
    #[allow(non_snake_case)]
    fn iMM(&mut self, span: Self::Span) -> Self::TypeVariable;

    /// Create the CPU flags type variable.
    fn cpu_flags(&mut self, span: Self::Span) -> Self::TypeVariable;

    /// Create a boolean type of size one bit.
    fn b1(&mut self, span: Self::Span) -> Self::TypeVariable;

    /// Create the void type, used as the result of operators that branch away,
    /// or do not return anything.
    fn void(&mut self, span: Self::Span) -> Self::TypeVariable;

    /// Create a type variable that may be either a boolean or an integer.
    fn bool_or_int(&mut self, span: Self::Span) -> Self::TypeVariable;

    /// Create a type variable that can be any type T.
    ///
    /// Each use of `any_t` by the same operator refers to the same type
    /// variable.
    fn any_t(&mut self, span: Self::Span) -> Self::TypeVariable;
}

/// The typing rules for a `TOperator` type.
///
/// This trait describes the types of immediates, parameters, and results of an
/// operator type, as well as their arity.
pub trait TypingRules {
    /// Get the result type of this operator.
    fn result_type<'a, C>(&self, span: C::Span, typing_context: &mut C) -> C::TypeVariable
    where
        C: TypingContext<'a>;

    /// Get the number of immediates this operator has.
    fn immediates_arity(&self) -> u8;

    /// Get the types of this operator's immediates.
    fn immediate_types<'a, C>(
        &self,
        span: C::Span,
        typing_context: &mut C,
        types: &mut impl Extend<C::TypeVariable>,
    ) where
        C: TypingContext<'a>;

    /// Get the number of parameters this operator has.
    fn parameters_arity(&self) -> u8;

    /// Get the types of this operator's parameters.
    fn parameter_types<'a, C>(
        &self,
        span: C::Span,
        typing_context: &mut C,
        types: &mut impl Extend<C::TypeVariable>,
    ) where
        C: TypingContext<'a>;

    /// Is this a bit width reducing instruction?
    ///
    /// E.g. Cranelift's `ireduce` instruction.
    fn is_reduce(&self) -> bool;

    /// Is this a bit width extending instruction?
    ///
    /// E.g. Cranelift's `uextend` and `sextend` instructions.
    fn is_extend(&self) -> bool;
}
