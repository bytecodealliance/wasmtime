//! Abstract syntax tree type definitions.
//!
//! This file makes fairly heavy use of macros, which are defined in the
//! `peepmatic_macro` crate that lives at `crates/macro`. Notably, the following
//! traits are all derived via `derive(Ast)`:
//!
//! * `Span` -- access the `wast::Span` where an AST node was parsed from. For
//!   `struct`s, there must be a `span: wast::Span` field, because the macro
//!   always generates an implementation that returns `self.span` for
//!   `struct`s. For `enum`s, every variant must have a single, unnamed field
//!   which implements the `Span` trait. The macro will generate code to return
//!   the span of whatever variant it is.
//!
//! * `ChildNodes` -- get each of the child AST nodes that a given node
//!   references. Some fields in an AST type aren't actually considered an AST
//!   node (like spans) and these are ignored via the `#[peepmatic(skip_child)]`
//!   attribute. Some fields contain multiple AST nodes (like vectors of
//!   operands) and these are flattened with `#[peepmatic(flatten)]`.
//!
//! * `From<&'a Self> for DynAstRef<'a>` -- convert a particular AST type into
//!   `DynAstRef`, which is an `enum` of all the different kinds of AST nodes.

use peepmatic_macro::Ast;
use peepmatic_runtime::{
    r#type::{BitWidth, Type},
    unquote::UnquoteOperator,
};
use std::cell::Cell;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use wast::Id;

/// A reference to any AST node.
#[derive(Debug, Clone, Copy)]
pub enum DynAstRef<'a, TOperator> {
    /// A reference to an `Optimizations`.
    Optimizations(&'a Optimizations<'a, TOperator>),

    /// A reference to an `Optimization`.
    Optimization(&'a Optimization<'a, TOperator>),

    /// A reference to an `Lhs`.
    Lhs(&'a Lhs<'a, TOperator>),

    /// A reference to an `Rhs`.
    Rhs(&'a Rhs<'a, TOperator>),

    /// A reference to a `Pattern`.
    Pattern(&'a Pattern<'a, TOperator>),

    /// A reference to a `Precondition`.
    Precondition(&'a Precondition<'a, TOperator>),

    /// A reference to a `ConstraintOperand`.
    ConstraintOperand(&'a ConstraintOperand<'a, TOperator>),

    /// A reference to a `ValueLiteral`.
    ValueLiteral(&'a ValueLiteral<'a, TOperator>),

    /// A reference to a `Constant`.
    Constant(&'a Constant<'a, TOperator>),

    /// A reference to a `PatternOperation`.
    PatternOperation(&'a Operation<'a, TOperator, Pattern<'a, TOperator>>),

    /// A reference to a `Variable`.
    Variable(&'a Variable<'a, TOperator>),

    /// A reference to an `Integer`.
    Integer(&'a Integer<'a, TOperator>),

    /// A reference to a `Boolean`.
    Boolean(&'a Boolean<'a, TOperator>),

    /// A reference to a `ConditionCode`.
    ConditionCode(&'a ConditionCode<'a, TOperator>),

    /// A reference to an `Unquote`.
    Unquote(&'a Unquote<'a, TOperator>),

    /// A reference to an `RhsOperation`.
    RhsOperation(&'a Operation<'a, TOperator, Rhs<'a, TOperator>>),
}

impl<'a, 'b, TOperator> ChildNodes<'a, 'b, TOperator> for DynAstRef<'a, TOperator> {
    fn child_nodes(&'b self, sink: &mut impl Extend<DynAstRef<'a, TOperator>>) {
        match self {
            Self::Optimizations(x) => x.child_nodes(sink),
            Self::Optimization(x) => x.child_nodes(sink),
            Self::Lhs(x) => x.child_nodes(sink),
            Self::Rhs(x) => x.child_nodes(sink),
            Self::Pattern(x) => x.child_nodes(sink),
            Self::Precondition(x) => x.child_nodes(sink),
            Self::ConstraintOperand(x) => x.child_nodes(sink),
            Self::ValueLiteral(x) => x.child_nodes(sink),
            Self::Constant(x) => x.child_nodes(sink),
            Self::PatternOperation(x) => x.child_nodes(sink),
            Self::Variable(x) => x.child_nodes(sink),
            Self::Integer(x) => x.child_nodes(sink),
            Self::Boolean(x) => x.child_nodes(sink),
            Self::ConditionCode(x) => x.child_nodes(sink),
            Self::Unquote(x) => x.child_nodes(sink),
            Self::RhsOperation(x) => x.child_nodes(sink),
        }
    }
}

/// A trait implemented by all AST nodes.
///
/// All AST nodes can:
///
/// * Enumerate their children via `ChildNodes`.
///
/// * Give you the `wast::Span` where they were defined.
///
/// * Be converted into a `DynAstRef`.
///
/// This trait is blanked implemented for everything that does those three
/// things, and in practice those three thrings are all implemented by the
/// `derive(Ast)` macro.
pub trait Ast<'a, TOperator>: 'a + ChildNodes<'a, 'a, TOperator> + Span
where
    DynAstRef<'a, TOperator>: From<&'a Self>,
    TOperator: 'a,
{
}

impl<'a, T, TOperator> Ast<'a, TOperator> for T
where
    T: 'a + ?Sized + ChildNodes<'a, 'a, TOperator> + Span,
    DynAstRef<'a, TOperator>: From<&'a Self>,
    TOperator: 'a,
{
}

/// Enumerate the child AST nodes of a given node.
pub trait ChildNodes<'a, 'b, TOperator>
where
    TOperator: 'a,
{
    /// Get each of this AST node's children, in order.
    fn child_nodes(&'b self, sink: &mut impl Extend<DynAstRef<'a, TOperator>>);
}

/// A trait for getting the span where an AST node was defined.
pub trait Span {
    /// Get the span where this AST node was defined.
    fn span(&self) -> wast::Span;
}

/// A set of optimizations.
///
/// This is the root AST node.
#[derive(Debug, Ast)]
pub struct Optimizations<'a, TOperator> {
    /// Where these `Optimizations` were defined.
    #[peepmatic(skip_child)]
    pub span: wast::Span,

    /// The optimizations.
    #[peepmatic(flatten)]
    pub optimizations: Vec<Optimization<'a, TOperator>>,
}

/// A complete optimization: a left-hand side to match against and a right-hand
/// side replacement.
#[derive(Debug, Ast)]
pub struct Optimization<'a, TOperator> {
    /// Where this `Optimization` was defined.
    #[peepmatic(skip_child)]
    pub span: wast::Span,

    /// The left-hand side that matches when this optimization applies.
    pub lhs: Lhs<'a, TOperator>,

    /// The new sequence of instructions to replace an old sequence that matches
    /// the left-hand side with.
    pub rhs: Rhs<'a, TOperator>,
}

/// A left-hand side describes what is required for a particular optimization to
/// apply.
///
/// A left-hand side has two parts: a structural pattern for describing
/// candidate instruction sequences, and zero or more preconditions that add
/// additional constraints upon instruction sequences matched by the pattern.
#[derive(Debug, Ast)]
pub struct Lhs<'a, TOperator> {
    /// Where this `Lhs` was defined.
    #[peepmatic(skip_child)]
    pub span: wast::Span,

    /// A pattern that describes sequences of instructions to match.
    pub pattern: Pattern<'a, TOperator>,

    /// Additional constraints that a match must satisfy in addition to
    /// structually matching the pattern, e.g. some constant must be a power of
    /// two.
    #[peepmatic(flatten)]
    pub preconditions: Vec<Precondition<'a, TOperator>>,
}

/// A structural pattern, potentially with wildcard variables for matching whole
/// subtrees.
#[derive(Debug, Ast)]
pub enum Pattern<'a, TOperator> {
    /// A specific value. These are written as `1234` or `0x1234` or `true` or
    /// `false`.
    ValueLiteral(ValueLiteral<'a, TOperator>),

    /// A constant that matches any constant value. This subsumes value
    /// patterns. These are upper-case identifiers like `$C`.
    Constant(Constant<'a, TOperator>),

    /// An operation pattern with zero or more operand patterns. These are
    /// s-expressions like `(iadd $x $y)`.
    Operation(Operation<'a, TOperator, Pattern<'a, TOperator>>),

    /// A variable that matches any kind of subexpression. This subsumes all
    /// other patterns. These are lower-case identifiers like `$x`.
    Variable(Variable<'a, TOperator>),
}

/// An integer or boolean value literal.
#[derive(Debug, Ast)]
pub enum ValueLiteral<'a, TOperator> {
    /// An integer value.
    Integer(Integer<'a, TOperator>),

    /// A boolean value: `true` or `false`.
    Boolean(Boolean<'a, TOperator>),

    /// A condition code: `eq`, `ne`, etc...
    ConditionCode(ConditionCode<'a, TOperator>),
}

/// An integer literal.
#[derive(Debug, PartialEq, Eq, Ast)]
pub struct Integer<'a, TOperator> {
    /// Where this `Integer` was defined.
    #[peepmatic(skip_child)]
    pub span: wast::Span,

    /// The integer value.
    ///
    /// Note that although Cranelift allows 128 bits wide values, the widest
    /// supported constants as immediates are 64 bits.
    #[peepmatic(skip_child)]
    pub value: i64,

    /// The bit width of this integer.
    ///
    /// This is either a fixed bit width, or polymorphic over the width of the
    /// optimization.
    ///
    /// This field is initialized from `None` to `Some` by the type checking
    /// pass in `src/verify.rs`.
    #[peepmatic(skip_child)]
    pub bit_width: Cell<Option<BitWidth>>,

    #[allow(missing_docs)]
    #[peepmatic(skip_child)]
    pub marker: PhantomData<&'a TOperator>,
}

impl<TOperator> Hash for Integer<'_, TOperator> {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        let Integer {
            span,
            value,
            bit_width,
            marker: _,
        } = self;
        span.hash(state);
        value.hash(state);
        let bit_width = bit_width.get();
        bit_width.hash(state);
    }
}

/// A boolean literal.
#[derive(Debug, PartialEq, Eq, Ast)]
pub struct Boolean<'a, TOperator> {
    /// Where this `Boolean` was defined.
    #[peepmatic(skip_child)]
    pub span: wast::Span,

    /// The boolean value.
    #[peepmatic(skip_child)]
    pub value: bool,

    /// The bit width of this boolean.
    ///
    /// This is either a fixed bit width, or polymorphic over the width of the
    /// optimization.
    ///
    /// This field is initialized from `None` to `Some` by the type checking
    /// pass in `src/verify.rs`.
    #[peepmatic(skip_child)]
    pub bit_width: Cell<Option<BitWidth>>,

    #[allow(missing_docs)]
    #[peepmatic(skip_child)]
    pub marker: PhantomData<&'a TOperator>,
}

impl<TOperator> Hash for Boolean<'_, TOperator> {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        let Boolean {
            span,
            value,
            bit_width,
            marker: _,
        } = self;
        span.hash(state);
        value.hash(state);
        let bit_width = bit_width.get();
        bit_width.hash(state);
    }
}

/// A condition code.
#[derive(Debug, Ast)]
pub struct ConditionCode<'a, TOperator> {
    /// Where this `ConditionCode` was defined.
    #[peepmatic(skip_child)]
    pub span: wast::Span,

    /// The actual condition code.
    #[peepmatic(skip_child)]
    pub cc: peepmatic_runtime::cc::ConditionCode,

    #[allow(missing_docs)]
    #[peepmatic(skip_child)]
    pub marker: PhantomData<&'a TOperator>,
}

/// A symbolic constant.
///
/// These are identifiers containing uppercase letters: `$C`, `$MY-CONST`,
/// `$CONSTANT1`.
#[derive(Debug, Ast)]
pub struct Constant<'a, TOperator> {
    /// Where this `Constant` was defined.
    #[peepmatic(skip_child)]
    pub span: wast::Span,

    /// This constant's identifier.
    #[peepmatic(skip_child)]
    pub id: Id<'a>,

    #[allow(missing_docs)]
    #[peepmatic(skip_child)]
    pub marker: PhantomData<&'a TOperator>,
}

/// A variable that matches any subtree.
///
/// Duplicate uses of the same variable constrain each occurrence's match to
/// being the same as each other occurrence as well, e.g. `(iadd $x $x)` matches
/// `(iadd 5 5)` but not `(iadd 1 2)`.
#[derive(Debug, Ast)]
pub struct Variable<'a, TOperator> {
    /// Where this `Variable` was defined.
    #[peepmatic(skip_child)]
    pub span: wast::Span,

    /// This variable's identifier.
    #[peepmatic(skip_child)]
    pub id: Id<'a>,

    #[allow(missing_docs)]
    #[peepmatic(skip_child)]
    pub marker: PhantomData<&'a TOperator>,
}

/// An operation with an operator, and operands of type `T`.
#[derive(Debug, Ast)]
#[peepmatic(no_into_dyn_node)]
pub struct Operation<'a, TOperator, TOperand>
where
    TOperator: 'a,
    TOperand: 'a + Ast<'a, TOperator>,
    DynAstRef<'a, TOperator>: From<&'a TOperand>,
{
    /// The span where this operation was written.
    #[peepmatic(skip_child)]
    pub span: wast::Span,

    /// The operator for this operation, e.g. `imul` or `iadd`.
    #[peepmatic(skip_child)]
    pub operator: TOperator,

    /// An optional ascribed or inferred type for the operator.
    #[peepmatic(skip_child)]
    pub r#type: Cell<Option<Type>>,

    /// This operation's operands.
    ///
    /// When `Operation` is used in a pattern, these are the sub-patterns for
    /// the operands. When `Operation is used in a right-hand side replacement,
    /// these are the sub-replacements for the operands.
    #[peepmatic(flatten)]
    pub operands: Vec<TOperand>,

    #[allow(missing_docs)]
    #[peepmatic(skip_child)]
    pub marker: PhantomData<&'a ()>,
}

impl<'a, TOperator> From<&'a Operation<'a, TOperator, Pattern<'a, TOperator>>>
    for DynAstRef<'a, TOperator>
{
    #[inline]
    fn from(o: &'a Operation<'a, TOperator, Pattern<'a, TOperator>>) -> DynAstRef<'a, TOperator> {
        DynAstRef::PatternOperation(o)
    }
}

impl<'a, TOperator> From<&'a Operation<'a, TOperator, Rhs<'a, TOperator>>>
    for DynAstRef<'a, TOperator>
{
    #[inline]
    fn from(o: &'a Operation<'a, TOperator, Rhs<'a, TOperator>>) -> DynAstRef<'a, TOperator> {
        DynAstRef::RhsOperation(o)
    }
}

/// A precondition adds additional constraints to a pattern, such as "$C must be
/// a power of two".
#[derive(Debug, Ast)]
pub struct Precondition<'a, TOperator> {
    /// Where this `Precondition` was defined.
    #[peepmatic(skip_child)]
    pub span: wast::Span,

    /// The constraint operator.
    #[peepmatic(skip_child)]
    pub constraint: Constraint,

    /// The operands of the constraint.
    #[peepmatic(flatten)]
    pub operands: Vec<ConstraintOperand<'a, TOperator>>,

    #[allow(missing_docs)]
    #[peepmatic(skip_child)]
    pub marker: PhantomData<&'a TOperator>,
}

/// Contraint operators.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Constraint {
    /// Is the operand a power of two?
    IsPowerOfTwo,

    /// Check the bit width of a value.
    BitWidth,

    /// Does the argument fit within our target architecture's native word size?
    FitsInNativeWord,
}

/// An operand of a precondition's constraint.
#[derive(Debug, Ast)]
pub enum ConstraintOperand<'a, TOperator> {
    /// A value literal operand.
    ValueLiteral(ValueLiteral<'a, TOperator>),

    /// A constant operand.
    Constant(Constant<'a, TOperator>),

    /// A variable operand.
    Variable(Variable<'a, TOperator>),
}

/// The right-hand side of an optimization that contains the instructions to
/// replace any matched left-hand side with.
#[derive(Debug, Ast)]
pub enum Rhs<'a, TOperator> {
    /// A value literal right-hand side.
    ValueLiteral(ValueLiteral<'a, TOperator>),

    /// A constant right-hand side (the constant must have been matched and
    /// bound in the left-hand side's pattern).
    Constant(Constant<'a, TOperator>),

    /// A variable right-hand side (the variable must have been matched and
    /// bound in the left-hand side's pattern).
    Variable(Variable<'a, TOperator>),

    /// An unquote expression that is evaluated while replacing the left-hand
    /// side with the right-hand side. The result of the evaluation is used in
    /// the replacement.
    Unquote(Unquote<'a, TOperator>),

    /// A compound right-hand side consisting of an operation and subsequent
    /// right-hand side operands.
    Operation(Operation<'a, TOperator, Rhs<'a, TOperator>>),
}

/// An unquote operation.
///
/// Rather than replaciong a left-hand side, these are evaluated and then the
/// result of the evaluation replaces the left-hand side. This allows for
/// compile-time computation while replacing a matched left-hand side with a
/// right-hand side.
///
/// For example, given the unqouted right-hand side `$(log2 $C)`, we replace any
/// instructions that match its left-hand side with the compile-time result of
/// `log2($C)` (the left-hand side must match and bind the constant `$C`).
#[derive(Debug, Ast)]
pub struct Unquote<'a, TOperator> {
    /// Where this `Unquote` was defined.
    #[peepmatic(skip_child)]
    pub span: wast::Span,

    /// The operator for this unquote operation.
    #[peepmatic(skip_child)]
    pub operator: UnquoteOperator,

    /// The operands for this unquote operation.
    #[peepmatic(flatten)]
    pub operands: Vec<Rhs<'a, TOperator>>,

    #[allow(missing_docs)]
    #[peepmatic(skip_child)]
    pub marker: PhantomData<&'a TOperator>,
}
