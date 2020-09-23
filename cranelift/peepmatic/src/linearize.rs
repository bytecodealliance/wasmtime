//! Convert an AST into its linear equivalent.
//!
//! Convert each optimization's left-hand side into a linear series of match
//! operations. This makes it easy to create an automaton, because automatas
//! typically deal with a linear sequence of inputs. The optimization's
//! right-hand side is built incrementally inside actions that are taken on
//! transitions between match operations.
//!
//! See `crates/runtime/src/linear.rs` for the linear datatype definitions.
//!
//! ## Example
//!
//! As an example, if we linearize this optimization:
//!
//! ```lisp
//! (=> (when (imul $x $C)
//!           (is-power-of-two $C))
//!     (ishl $x $(log2 C)))
//! ```
//!
//! Then the left-hand side becomes the following linear chain of "matches":
//!
//! ```ignore
//! [
//!   // ( Match Operation, Expected Value )
//!   ( Opcode@0,           imul ),
//!   ( IsConst(C),         true ),
//!   ( IsPowerOfTwo(C),    true ),
//! ]
//! ```
//!
//! And the right-hand side becomes this linear chain of "actions":
//!
//! ```ignore
//! [
//!   $rhs0 = get lhs @ 0.0            // $x
//!   $rhs1 = get lhs @ 0.1            // $C
//!   $rhs2 = eval log2 $rhs1
//!   $rhs3 = make ishl $rhs0, $rhs2
//! ]
//! ```
//!
//! Each match will essentially become a state and a transition out of that
//! state in the final automata. The actions record the scope of matches from
//! the left-hand side and also incrementally build the right-hand side's
//! instructions.
//!
//! ## General Principles
//!
//! Here are the general principles that linearization should adhere to:
//!
//! * Don't match on a subtree until we know it exists. That is, match on
//!   parents before matching on children.
//!
//! * Shorter match chains are better! This means fewer tests when matching
//!   left-hand sides, and a more-compact, more-cache-friendly automata, and
//!   ultimately, a faster automata.
//!
//! * An match operation should be a switch rather than a predicate that returns
//!   a boolean. For example, we switch on an instruction's opcode, rather than
//!   ask whether this operation is an `imul`. This allows for more prefix
//!   sharing in the automata, which (again) makes it more compact and more
//!   cache friendly.
//!
//! ## Implementation Overview
//!
//! We emit match operations for a left-hand side's pattern structure, followed
//! by match operations for its preconditions on that structure. This ensures
//! that anything bound in the pattern is defined before it is used in
//! precondition.
//!
//! Within matching the pattern structure, we emit matching operations in a
//! breadth-first traversal of the pattern. This ensures that we've already
//! matched an operation before we consider its operands, and therefore we
//! already know the operands exist. It also lets us fuse "what opcode does this
//! instruction have?" and "define temporary variables for this instruction's
//! operands" into a single operation. See `PatternBfs` for details.
//!
//! As we define the match operations for a pattern, we remember the path where
//! each LHS id first occurred. These will later be reused when building the RHS
//! actions. See `LhsCanonicalizer` for details.
//!
//! After we've generated the match operations and expected result of those
//! match operations, then we generate the right-hand side actions. The
//! right-hand side is built up a post-order traversal, so that operands are
//! defined before they are used. See `RhsPostOrder` and `RhsBuilder` for
//! details.
//!
//! Finally, see `linearize_optimization` for the the main AST optimization into
//! linear optimization translation function.

use crate::ast::*;
use crate::traversals::{Bfs, Dfs};
use peepmatic_runtime::{integer_interner::IntegerInterner, linear};
use std::collections::BTreeMap;
use std::convert::TryInto;
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;
use std::num::NonZeroU32;
use wast::Id;

/// Translate the given AST optimizations into linear optimizations.
pub fn linearize<TOperator>(opts: &Optimizations<TOperator>) -> linear::Optimizations<TOperator>
where
    TOperator: Copy + Debug + Eq + Hash + Into<NonZeroU32>,
{
    let mut optimizations = vec![];
    let mut integers = IntegerInterner::new();
    for opt in &opts.optimizations {
        let lin_opt = linearize_optimization(&mut integers, opt);
        optimizations.push(lin_opt);
    }
    linear::Optimizations {
        optimizations,
        integers,
    }
}

/// Translate an AST optimization into a linear optimization!
fn linearize_optimization<TOperator>(
    integers: &mut IntegerInterner,
    opt: &Optimization<TOperator>,
) -> linear::Optimization<TOperator>
where
    TOperator: Copy + Debug + Eq + Hash + Into<NonZeroU32>,
{
    let mut matches: Vec<linear::Match> = vec![];

    let mut lhs_canonicalizer = LhsCanonicalizer::new();

    // We do a breadth-first traversal of the LHS because we don't know whether
    // a child actually exists to match on until we've matched its parent, and
    // we don't want to emit matching operations on things that might not exist!
    for (id, pattern) in PatternBfs::new(&opt.lhs.pattern) {
        // Create the matching parts of an `Match` for this part of the
        // pattern.
        let (operation, expected) = pattern.to_linear_match_op(integers, &lhs_canonicalizer, id);
        matches.push(linear::Match {
            operation,
            expected,
        });

        lhs_canonicalizer.remember_linear_id(pattern, id);

        // Some operations require type ascriptions for us to infer the correct
        // bit width of their results: `ireduce`, `sextend`, `uextend`, etc.
        // When there is such a type ascription in the pattern, insert another
        // match that checks the instruction-being-matched's bit width.
        if let Pattern::Operation(Operation { r#type, .. }) = pattern {
            if let Some(w) = r#type.get().and_then(|ty| ty.bit_width.fixed_width()) {
                debug_assert!(w != 0, "All fixed-width bit widths are non-zero");
                let expected = Ok(unsafe { NonZeroU32::new_unchecked(w as u32) });

                matches.push(linear::Match {
                    operation: linear::MatchOp::BitWidth(id),
                    expected,
                });
            }
        }
    }

    // Now that we've added all the matches for the LHS pattern, add the
    // matches for its preconditions.
    for pre in &opt.lhs.preconditions {
        matches.push(pre.to_linear_match(&lhs_canonicalizer));
    }

    assert!(!matches.is_empty());

    // Finally, generate the RHS-building actions and attach them to the first match.
    let mut rhs_builder = RhsBuilder::new(&opt.rhs);
    let mut actions = vec![];
    rhs_builder.add_rhs_build_actions(integers, &lhs_canonicalizer, &mut actions);

    linear::Optimization { matches, actions }
}

/// A post-order, depth-first traversal of right-hand sides.
///
/// Does not maintain any extra state about the traversal, such as where in the
/// tree each yielded node comes from.
struct RhsPostOrder<'a, TOperator> {
    dfs: Dfs<'a, TOperator>,
}

impl<'a, TOperator> RhsPostOrder<'a, TOperator>
where
    TOperator: Copy + Debug + Eq + Hash,
{
    fn new(rhs: &'a Rhs<'a, TOperator>) -> Self {
        Self { dfs: Dfs::new(rhs) }
    }
}

impl<'a, TOperator> Iterator for RhsPostOrder<'a, TOperator>
where
    TOperator: Copy,
{
    type Item = &'a Rhs<'a, TOperator>;

    fn next(&mut self) -> Option<&'a Rhs<'a, TOperator>> {
        use crate::traversals::TraversalEvent as TE;
        loop {
            match self.dfs.next()? {
                (TE::Exit, DynAstRef::Rhs(rhs)) => return Some(rhs),
                _ => continue,
            }
        }
    }
}

/// A breadth-first traversal of left-hand side patterns.
///
/// Keeps track of the `LhsId` of each pattern, and yields it along side the
/// pattern AST node.
///
/// We use a breadth-first traversal because we fuse "which opcode is this?" and
/// "assign operands to temporaries" into a single linear match operation. A
/// breadth-first traversal aligns with "match this opcode, and on success bind
/// all of its operands to temporaries". Fusing these operations into one is
/// important for attaining similar performance as an open-coded Rust `match`
/// expression, which would also fuse these operations via pattern matching.
struct PatternBfs<'a, TOperator> {
    next_id: u16,
    bfs: Bfs<'a, TOperator>,
}

impl<'a, TOperator> PatternBfs<'a, TOperator>
where
    TOperator: Copy + Debug + Eq + Hash,
{
    fn new(pattern: &'a Pattern<'a, TOperator>) -> Self {
        Self {
            next_id: 0,
            bfs: Bfs::new(pattern),
        }
    }
}

impl<'a, TOperator> Iterator for PatternBfs<'a, TOperator>
where
    TOperator: 'a + Copy + Debug + Eq + Hash,
{
    type Item = (linear::LhsId, &'a Pattern<'a, TOperator>);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let DynAstRef::Pattern(pattern) = self.bfs.next()? {
                let id = linear::LhsId(self.next_id);
                self.next_id = self.next_id.checked_add(1).unwrap();
                return Some((id, pattern));
            }
        }
    }
}

/// A map from left-hand side identifiers to the path in the left-hand side
/// where they first occurred.
struct LhsCanonicalizer<'a, TOperator> {
    id_to_linear: BTreeMap<&'a str, linear::LhsId>,
    _marker: PhantomData<&'a TOperator>,
}

impl<'a, TOperator> LhsCanonicalizer<'a, TOperator> {
    /// Construct a new, empty `LhsCanonicalizer`.
    fn new() -> Self {
        Self {
            id_to_linear: Default::default(),
            _marker: PhantomData,
        }
    }

    /// Get the canonical `linear::LhsId` for the given variable, if any.
    fn get(&self, id: &Id) -> Option<linear::LhsId> {
        self.id_to_linear.get(id.name()).copied()
    }

    /// Remember the canonical `linear::LhsId` for any variables or constants
    /// used in the given pattern.
    fn remember_linear_id(
        &mut self,
        pattern: &'a Pattern<'a, TOperator>,
        linear_id: linear::LhsId,
    ) {
        match pattern {
            // If this is the first time we've seen an identifier defined on the
            // left-hand side, remember it.
            Pattern::Variable(Variable { id, .. }) | Pattern::Constant(Constant { id, .. }) => {
                self.id_to_linear.entry(id.name()).or_insert(linear_id);
            }
            _ => {}
        }
    }
}

/// An `RhsBuilder` emits the actions for building the right-hand side
/// instructions.
struct RhsBuilder<'a, TOperator> {
    // We do a post order traversal of the RHS because an RHS instruction cannot
    // be created until after all of its operands are created.
    rhs_post_order: RhsPostOrder<'a, TOperator>,

    // A map from a right-hand side's span to its `linear::RhsId`. This is used
    // by RHS-construction actions to reference operands. In practice the
    // `RhsId` is roughly equivalent to its index in the post-order traversal of
    // the RHS.
    rhs_span_to_id: BTreeMap<wast::Span, linear::RhsId>,
}

impl<'a, TOperator> RhsBuilder<'a, TOperator>
where
    TOperator: Copy + Debug + Eq + Hash,
{
    /// Create a new builder for the given right-hand side.
    fn new(rhs: &'a Rhs<'a, TOperator>) -> Self {
        let rhs_post_order = RhsPostOrder::new(rhs);
        let rhs_span_to_id = Default::default();
        Self {
            rhs_post_order,
            rhs_span_to_id,
        }
    }

    /// Get the `linear::RhsId` for the given right-hand side.
    ///
    /// ## Panics
    ///
    /// Panics if we haven't already emitted the action for building this RHS's
    /// instruction.
    fn get_rhs_id(&self, rhs: &Rhs<TOperator>) -> linear::RhsId {
        self.rhs_span_to_id[&rhs.span()]
    }

    /// Create actions for building up this right-hand side of an optimization.
    ///
    /// Because we are walking the right-hand side with a post-order traversal,
    /// we know that we already created an instruction's operands that are
    /// defined in the right-hand side, before we get to the parent instruction.
    fn add_rhs_build_actions(
        &mut self,
        integers: &mut IntegerInterner,
        lhs_canonicalizer: &LhsCanonicalizer<TOperator>,
        actions: &mut Vec<linear::Action<TOperator>>,
    ) {
        while let Some(rhs) = self.rhs_post_order.next() {
            actions.push(self.rhs_to_linear_action(integers, lhs_canonicalizer, rhs));
            let id = linear::RhsId(self.rhs_span_to_id.len().try_into().unwrap());
            self.rhs_span_to_id.insert(rhs.span(), id);
        }
    }

    fn rhs_to_linear_action(
        &self,
        integers: &mut IntegerInterner,
        lhs_canonicalizer: &LhsCanonicalizer<TOperator>,
        rhs: &Rhs<TOperator>,
    ) -> linear::Action<TOperator> {
        match rhs {
            Rhs::ValueLiteral(ValueLiteral::Integer(i)) => linear::Action::MakeIntegerConst {
                value: integers.intern(i.value as u64),
                bit_width: i
                    .bit_width
                    .get()
                    .expect("should be initialized after type checking"),
            },
            Rhs::ValueLiteral(ValueLiteral::Boolean(b)) => linear::Action::MakeBooleanConst {
                value: b.value,
                bit_width: b
                    .bit_width
                    .get()
                    .expect("should be initialized after type checking"),
            },
            Rhs::ValueLiteral(ValueLiteral::ConditionCode(ConditionCode { cc, .. })) => {
                linear::Action::MakeConditionCode { cc: *cc }
            }
            Rhs::Variable(Variable { id, .. }) | Rhs::Constant(Constant { id, .. }) => {
                let lhs = lhs_canonicalizer.get(id).unwrap();
                linear::Action::GetLhs { lhs }
            }
            Rhs::Unquote(unq) => match unq.operands.len() {
                1 => linear::Action::UnaryUnquote {
                    operator: unq.operator,
                    operand: self.get_rhs_id(&unq.operands[0]),
                },
                2 => linear::Action::BinaryUnquote {
                    operator: unq.operator,
                    operands: [
                        self.get_rhs_id(&unq.operands[0]),
                        self.get_rhs_id(&unq.operands[1]),
                    ],
                },
                n => unreachable!("no unquote operators of arity {}", n),
            },
            Rhs::Operation(op) => match op.operands.len() {
                1 => linear::Action::MakeUnaryInst {
                    operator: op.operator,
                    r#type: op
                        .r#type
                        .get()
                        .expect("should be initialized after type checking"),
                    operand: self.get_rhs_id(&op.operands[0]),
                },
                2 => linear::Action::MakeBinaryInst {
                    operator: op.operator,
                    r#type: op
                        .r#type
                        .get()
                        .expect("should be initialized after type checking"),
                    operands: [
                        self.get_rhs_id(&op.operands[0]),
                        self.get_rhs_id(&op.operands[1]),
                    ],
                },
                3 => linear::Action::MakeTernaryInst {
                    operator: op.operator,
                    r#type: op
                        .r#type
                        .get()
                        .expect("should be initialized after type checking"),
                    operands: [
                        self.get_rhs_id(&op.operands[0]),
                        self.get_rhs_id(&op.operands[1]),
                        self.get_rhs_id(&op.operands[2]),
                    ],
                },
                n => unreachable!("no instructions of arity {}", n),
            },
        }
    }
}

impl<TOperator> Precondition<'_, TOperator>
where
    TOperator: Copy + Debug + Eq + Hash + Into<NonZeroU32>,
{
    /// Convert this precondition into a `linear::Match`.
    fn to_linear_match(&self, lhs_canonicalizer: &LhsCanonicalizer<TOperator>) -> linear::Match {
        match self.constraint {
            Constraint::IsPowerOfTwo => {
                let id = match &self.operands[0] {
                    ConstraintOperand::Constant(Constant { id, .. }) => id,
                    _ => unreachable!("checked in verification"),
                };
                let id = lhs_canonicalizer.get(&id).unwrap();
                linear::Match {
                    operation: linear::MatchOp::IsPowerOfTwo(id),
                    expected: linear::bool_to_match_result(true),
                }
            }
            Constraint::BitWidth => {
                let id = match &self.operands[0] {
                    ConstraintOperand::Constant(Constant { id, .. })
                    | ConstraintOperand::Variable(Variable { id, .. }) => id,
                    _ => unreachable!("checked in verification"),
                };
                let id = lhs_canonicalizer.get(&id).unwrap();

                let width = match &self.operands[1] {
                    ConstraintOperand::ValueLiteral(ValueLiteral::Integer(Integer {
                        value,
                        ..
                    })) => *value,
                    _ => unreachable!("checked in verification"),
                };

                assert!(0 < width && width <= 128);
                assert!((width as u8).is_power_of_two());
                let expected = Ok(unsafe { NonZeroU32::new_unchecked(width as u32) });

                linear::Match {
                    operation: linear::MatchOp::BitWidth(id),
                    expected,
                }
            }
            Constraint::FitsInNativeWord => {
                let id = match &self.operands[0] {
                    ConstraintOperand::Constant(Constant { id, .. })
                    | ConstraintOperand::Variable(Variable { id, .. }) => id,
                    _ => unreachable!("checked in verification"),
                };
                let id = lhs_canonicalizer.get(&id).unwrap();
                linear::Match {
                    operation: linear::MatchOp::FitsInNativeWord(id),
                    expected: linear::bool_to_match_result(true),
                }
            }
        }
    }
}

impl<TOperator> Pattern<'_, TOperator>
where
    TOperator: Copy,
{
    /// Convert this pattern into its linear match operation and the expected
    /// result of that operation.
    ///
    /// NB: these mappings to expected values need to stay sync'd with the
    /// runtime!
    fn to_linear_match_op(
        &self,
        integers: &mut IntegerInterner,
        lhs_canonicalizer: &LhsCanonicalizer<TOperator>,
        linear_lhs_id: linear::LhsId,
    ) -> (linear::MatchOp, linear::MatchResult)
    where
        TOperator: Into<NonZeroU32>,
    {
        match self {
            Pattern::ValueLiteral(ValueLiteral::Integer(Integer { value, .. })) => (
                linear::MatchOp::IntegerValue(linear_lhs_id),
                Ok(integers.intern(*value as u64).into()),
            ),
            Pattern::ValueLiteral(ValueLiteral::Boolean(Boolean { value, .. })) => (
                linear::MatchOp::BooleanValue(linear_lhs_id),
                linear::bool_to_match_result(*value),
            ),
            Pattern::ValueLiteral(ValueLiteral::ConditionCode(ConditionCode { cc, .. })) => {
                let cc = *cc as u32;
                debug_assert!(cc != 0, "no `ConditionCode` variants are zero");
                let expected = Ok(unsafe { NonZeroU32::new_unchecked(cc) });
                (linear::MatchOp::ConditionCode(linear_lhs_id), expected)
            }
            Pattern::Constant(Constant { id, .. }) => {
                if let Some(linear_lhs_id2) = lhs_canonicalizer.get(id) {
                    debug_assert!(linear_lhs_id != linear_lhs_id2);
                    (
                        linear::MatchOp::Eq(linear_lhs_id, linear_lhs_id2),
                        linear::bool_to_match_result(true),
                    )
                } else {
                    (
                        linear::MatchOp::IsConst(linear_lhs_id),
                        linear::bool_to_match_result(true),
                    )
                }
            }
            Pattern::Variable(Variable { id, .. }) => {
                if let Some(linear_lhs_id2) = lhs_canonicalizer.get(id) {
                    debug_assert!(linear_lhs_id != linear_lhs_id2);
                    (
                        linear::MatchOp::Eq(linear_lhs_id, linear_lhs_id2),
                        linear::bool_to_match_result(true),
                    )
                } else {
                    (linear::MatchOp::Nop, Err(linear::Else))
                }
            }
            Pattern::Operation(op) => {
                let expected = Ok(op.operator.into());
                (linear::MatchOp::Opcode(linear_lhs_id), expected)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use peepmatic_runtime::{
        integer_interner::IntegerId,
        linear::{bool_to_match_result, Action::*, Else, LhsId, MatchOp::*, RhsId},
        r#type::{BitWidth, Kind, Type},
        unquote::UnquoteOperator,
    };
    use peepmatic_test_operator::TestOperator;

    macro_rules! linearizes_to {
        ($name:ident, $source:expr, $make_expected:expr $(,)* ) => {
            #[test]
            fn $name() {
                let buf = wast::parser::ParseBuffer::new($source).expect("should lex OK");

                let opts = match wast::parser::parse::<Optimizations<TestOperator>>(&buf) {
                    Ok(opts) => opts,
                    Err(mut e) => {
                        e.set_path(std::path::Path::new(stringify!($name)));
                        e.set_text($source);
                        eprintln!("{}", e);
                        panic!("should parse OK")
                    }
                };

                assert_eq!(
                    opts.optimizations.len(),
                    1,
                    "`linearizes_to!` only supports a single optimization; split the big test into \
                     multiple small tests"
                );

                if let Err(mut e) = crate::verify(&opts) {
                    e.set_path(std::path::Path::new(stringify!($name)));
                    e.set_text($source);
                    eprintln!("{}", e);
                    panic!("should verify OK")
                }

                let mut integers = IntegerInterner::new();
                let mut i = |i: u64| integers.intern(i);

                #[allow(unused_variables)]
                let make_expected: fn(
                    &mut dyn FnMut(u64) -> IntegerId,
                ) -> (Vec<linear::Match>, Vec<linear::Action<_>>) = $make_expected;

                let expected = make_expected(&mut i);
                let actual = linearize_optimization(&mut integers, &opts.optimizations[0]);
                assert_eq!(expected.0, actual.matches);
                assert_eq!(expected.1, actual.actions);
            }
        };
    }

    linearizes_to!(
        mul_by_pow2_into_shift,
        "
(=> (when (imul $x $C)
          (is-power-of-two $C))
    (ishl $x $(log2 $C)))
        ",
        |i| (
            vec![
                linear::Match {
                    operation: Opcode(LhsId(0)),
                    expected: Ok(TestOperator::Imul.into()),
                },
                linear::Match {
                    operation: Nop,
                    expected: Err(Else),
                },
                linear::Match {
                    operation: IsConst(LhsId(2)),
                    expected: bool_to_match_result(true),
                },
                linear::Match {
                    operation: IsPowerOfTwo(LhsId(2)),
                    expected: bool_to_match_result(true),
                },
            ],
            vec![
                GetLhs { lhs: LhsId(1) },
                GetLhs { lhs: LhsId(2) },
                UnaryUnquote {
                    operator: UnquoteOperator::Log2,
                    operand: RhsId(1)
                },
                MakeBinaryInst {
                    operator: TestOperator::Ishl,
                    r#type: Type {
                        kind: Kind::Int,
                        bit_width: BitWidth::Polymorphic
                    },
                    operands: [RhsId(0), RhsId(2)]
                }
            ],
        ),
    );

    linearizes_to!(variable_pattern_id_optimization, "(=> $x $x)", |i| (
        vec![linear::Match {
            operation: Nop,
            expected: Err(Else),
        }],
        vec![GetLhs { lhs: LhsId(0) }],
    ));

    linearizes_to!(constant_pattern_id_optimization, "(=> $C $C)", |i| (
        vec![linear::Match {
            operation: IsConst(LhsId(0)),
            expected: bool_to_match_result(true),
        }],
        vec![GetLhs { lhs: LhsId(0) }],
    ));

    linearizes_to!(boolean_literal_id_optimization, "(=> true true)", |i| (
        vec![linear::Match {
            operation: BooleanValue(LhsId(0)),
            expected: bool_to_match_result(true),
        }],
        vec![MakeBooleanConst {
            value: true,
            bit_width: BitWidth::Polymorphic,
        }],
    ));

    linearizes_to!(number_literal_id_optimization, "(=> 5 5)", |i| (
        vec![linear::Match {
            operation: IntegerValue(LhsId(0)),
            expected: Ok(i(5).into()),
        }],
        vec![MakeIntegerConst {
            value: i(5),
            bit_width: BitWidth::Polymorphic,
        }],
    ));

    linearizes_to!(
        operation_id_optimization,
        "(=> (iconst $C) (iconst $C))",
        |i| (
            vec![
                linear::Match {
                    operation: Opcode(LhsId(0)),
                    expected: Ok(TestOperator::Iconst.into()),
                },
                linear::Match {
                    operation: IsConst(LhsId(1)),
                    expected: bool_to_match_result(true),
                },
            ],
            vec![
                GetLhs { lhs: LhsId(1) },
                MakeUnaryInst {
                    operator: TestOperator::Iconst,
                    r#type: Type {
                        kind: Kind::Int,
                        bit_width: BitWidth::Polymorphic,
                    },
                    operand: RhsId(0),
                },
            ],
        ),
    );

    linearizes_to!(
        redundant_bor,
        "(=> (bor $x (bor $x $y)) (bor $x $y))",
        |i| (
            vec![
                linear::Match {
                    operation: Opcode(LhsId(0)),
                    expected: Ok(TestOperator::Bor.into()),
                },
                linear::Match {
                    operation: Nop,
                    expected: Err(Else),
                },
                linear::Match {
                    operation: Opcode(LhsId(2)),
                    expected: Ok(TestOperator::Bor.into()),
                },
                linear::Match {
                    operation: Eq(LhsId(3), LhsId(1)),
                    expected: bool_to_match_result(true),
                },
                linear::Match {
                    operation: Nop,
                    expected: Err(Else),
                },
            ],
            vec![
                GetLhs { lhs: LhsId(1) },
                GetLhs { lhs: LhsId(4) },
                MakeBinaryInst {
                    operator: TestOperator::Bor,
                    r#type: Type {
                        kind: Kind::Int,
                        bit_width: BitWidth::Polymorphic,
                    },
                    operands: [RhsId(0), RhsId(1)],
                },
            ],
        ),
    );

    linearizes_to!(
        large_integers,
        // u64::MAX
        "(=> 18446744073709551615 0)",
        |i| (
            vec![linear::Match {
                operation: IntegerValue(LhsId(0)),
                expected: Ok(i(std::u64::MAX).into()),
            }],
            vec![MakeIntegerConst {
                value: i(0),
                bit_width: BitWidth::Polymorphic,
            }],
        ),
    );

    linearizes_to!(
        ireduce_with_type_ascription,
        "(=> (ireduce{i32} $x) 0)",
        |i| (
            vec![
                linear::Match {
                    operation: Opcode(LhsId(0)),
                    expected: Ok(TestOperator::Ireduce.into()),
                },
                linear::Match {
                    operation: linear::MatchOp::BitWidth(LhsId(0)),
                    expected: Ok(NonZeroU32::new(32).unwrap()),
                },
                linear::Match {
                    operation: Nop,
                    expected: Err(Else),
                },
            ],
            vec![MakeIntegerConst {
                value: i(0),
                bit_width: BitWidth::ThirtyTwo,
            }],
        ),
    );
}
