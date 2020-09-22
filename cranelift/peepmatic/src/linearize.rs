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
//! * Actions should be pushed as early in the optimization's match chain as
//!   they can be. This means the tail has fewer side effects, and is therefore
//!   more likely to be share-able with other optimizations in the automata that
//!   we build.
//!
//! * RHS actions cannot reference matches from the LHS until they've been
//!   defined. And finally, an RHS operation's operands must be defined before
//!   the RHS operation itself. In general, definitions must come before uses!
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
//! pre-order traversal of the pattern. This ensures that we've already matched
//! an operation before we consider its operands, and therefore we already know
//! the operands exist. See `PatternPreOrder` for details.
//!
//! As we define the match operations for a pattern, we remember the path where
//! each LHS id first occurred. These will later be reused when building the RHS
//! actions. See `LhsIdToPath` for details.
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
use crate::traversals::Dfs;
use peepmatic_runtime::{
    integer_interner::IntegerInterner,
    linear,
    paths::{Path, PathId, PathInterner},
};
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
    let mut paths = PathInterner::new();
    let mut integers = IntegerInterner::new();
    for opt in &opts.optimizations {
        let lin_opt = linearize_optimization(&mut paths, &mut integers, opt);
        optimizations.push(lin_opt);
    }
    linear::Optimizations {
        optimizations,
        paths,
        integers,
    }
}

/// Translate an AST optimization into a linear optimization!
fn linearize_optimization<TOperator>(
    paths: &mut PathInterner,
    integers: &mut IntegerInterner,
    opt: &Optimization<TOperator>,
) -> linear::Optimization<TOperator>
where
    TOperator: Copy + Debug + Eq + Hash + Into<NonZeroU32>,
{
    let mut matches: Vec<linear::Match> = vec![];

    let mut lhs_id_to_path = LhsIdToPath::new();

    // We do a pre-order traversal of the LHS because we don't know whether a
    // child actually exists to match on until we've matched its parent, and we
    // don't want to emit matching operations on things that might not exist!
    let mut patterns = PatternPreOrder::new(&opt.lhs.pattern);
    while let Some((path, pattern)) = patterns.next(paths) {
        // Create the matching parts of an `Match` for this part of the
        // pattern.
        let (operation, expected) = pattern.to_linear_match_op(integers, &lhs_id_to_path, path);
        matches.push(linear::Match {
            operation,
            expected,
        });

        lhs_id_to_path.remember_path_to_pattern_ids(pattern, path);

        // Some operations require type ascriptions for us to infer the correct
        // bit width of their results: `ireduce`, `sextend`, `uextend`, etc.
        // When there is such a type ascription in the pattern, insert another
        // match that checks the instruction-being-matched's bit width.
        if let Pattern::Operation(Operation { r#type, .. }) = pattern {
            if let Some(w) = r#type.get().and_then(|ty| ty.bit_width.fixed_width()) {
                debug_assert!(w != 0, "All fixed-width bit widths are non-zero");
                let expected = Ok(unsafe { NonZeroU32::new_unchecked(w as u32) });

                matches.push(linear::Match {
                    operation: linear::MatchOp::BitWidth { path },
                    expected,
                });
            }
        }
    }

    // Now that we've added all the matches for the LHS pattern, add the
    // matches for its preconditions.
    for pre in &opt.lhs.preconditions {
        matches.push(pre.to_linear_match(&lhs_id_to_path));
    }

    assert!(!matches.is_empty());

    // Finally, generate the RHS-building actions and attach them to the first match.
    let mut rhs_builder = RhsBuilder::new(&opt.rhs);
    let mut actions = vec![];
    rhs_builder.add_rhs_build_actions(integers, &lhs_id_to_path, &mut actions);

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

/// A pre-order, depth-first traversal of left-hand side patterns.
///
/// Keeps track of the path to each pattern, and yields it along side the
/// pattern AST node.
struct PatternPreOrder<'a, TOperator> {
    last_child: Option<u8>,
    path: Vec<u8>,
    dfs: Dfs<'a, TOperator>,
}

impl<'a, TOperator> PatternPreOrder<'a, TOperator>
where
    TOperator: Copy + Debug + Eq + Hash,
{
    fn new(pattern: &'a Pattern<'a, TOperator>) -> Self {
        Self {
            last_child: None,
            path: vec![],
            dfs: Dfs::new(pattern),
        }
    }

    fn next(&mut self, paths: &mut PathInterner) -> Option<(PathId, &'a Pattern<'a, TOperator>)> {
        use crate::traversals::TraversalEvent as TE;
        loop {
            match self.dfs.next()? {
                (TE::Enter, DynAstRef::Pattern(pattern)) => {
                    let last_child = self.last_child.take();
                    self.path.push(match last_child {
                        None => 0,
                        Some(c) => {
                            assert!(
                                c < std::u8::MAX,
                                "operators must have less than or equal u8::MAX arity"
                            );
                            c + 1
                        }
                    });
                    let path = paths.intern(Path(&self.path));
                    return Some((path, pattern));
                }
                (TE::Exit, DynAstRef::Pattern(_)) => {
                    self.last_child = Some(
                        self.path
                            .pop()
                            .expect("should always have a non-empty path during traversal"),
                    );
                }
                _ => {}
            }
        }
    }
}

/// A map from left-hand side identifiers to the path in the left-hand side
/// where they first occurred.
struct LhsIdToPath<'a, TOperator> {
    id_to_path: BTreeMap<&'a str, PathId>,
    _marker: PhantomData<&'a TOperator>,
}

impl<'a, TOperator> LhsIdToPath<'a, TOperator> {
    /// Construct a new, empty `LhsIdToPath`.
    fn new() -> Self {
        Self {
            id_to_path: Default::default(),
            _marker: PhantomData,
        }
    }

    /// Have we already seen the given identifier?
    fn get_first_occurrence(&self, id: &Id) -> Option<PathId> {
        self.id_to_path.get(id.name()).copied()
    }

    /// Get the path within the left-hand side pattern where we first saw the
    /// given AST id.
    ///
    /// ## Panics
    ///
    /// Panics if the given AST id has not already been canonicalized.
    fn unwrap_first_occurrence(&self, id: &Id) -> PathId {
        self.id_to_path[id.name()]
    }

    /// Remember the path to any LHS ids used in the given pattern.
    fn remember_path_to_pattern_ids(&mut self, pattern: &'a Pattern<'a, TOperator>, path: PathId) {
        match pattern {
            // If this is the first time we've seen an identifier defined on the
            // left-hand side, remember it.
            Pattern::Variable(Variable { id, .. }) | Pattern::Constant(Constant { id, .. }) => {
                self.id_to_path.entry(id.name()).or_insert(path);
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
        lhs_id_to_path: &LhsIdToPath<TOperator>,
        actions: &mut Vec<linear::Action<TOperator>>,
    ) {
        while let Some(rhs) = self.rhs_post_order.next() {
            actions.push(self.rhs_to_linear_action(integers, lhs_id_to_path, rhs));
            let id = linear::RhsId(self.rhs_span_to_id.len().try_into().unwrap());
            self.rhs_span_to_id.insert(rhs.span(), id);
        }
    }

    fn rhs_to_linear_action(
        &self,
        integers: &mut IntegerInterner,
        lhs_id_to_path: &LhsIdToPath<TOperator>,
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
                let path = lhs_id_to_path.unwrap_first_occurrence(id);
                linear::Action::GetLhs { path }
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
    fn to_linear_match(&self, lhs_id_to_path: &LhsIdToPath<TOperator>) -> linear::Match {
        match self.constraint {
            Constraint::IsPowerOfTwo => {
                let id = match &self.operands[0] {
                    ConstraintOperand::Constant(Constant { id, .. }) => id,
                    _ => unreachable!("checked in verification"),
                };
                let path = lhs_id_to_path.unwrap_first_occurrence(&id);
                linear::Match {
                    operation: linear::MatchOp::IsPowerOfTwo { path },
                    expected: linear::bool_to_match_result(true),
                }
            }
            Constraint::BitWidth => {
                let id = match &self.operands[0] {
                    ConstraintOperand::Constant(Constant { id, .. })
                    | ConstraintOperand::Variable(Variable { id, .. }) => id,
                    _ => unreachable!("checked in verification"),
                };
                let path = lhs_id_to_path.unwrap_first_occurrence(&id);

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
                    operation: linear::MatchOp::BitWidth { path },
                    expected,
                }
            }
            Constraint::FitsInNativeWord => {
                let id = match &self.operands[0] {
                    ConstraintOperand::Constant(Constant { id, .. })
                    | ConstraintOperand::Variable(Variable { id, .. }) => id,
                    _ => unreachable!("checked in verification"),
                };
                let path = lhs_id_to_path.unwrap_first_occurrence(&id);
                linear::Match {
                    operation: linear::MatchOp::FitsInNativeWord { path },
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
        lhs_id_to_path: &LhsIdToPath<TOperator>,
        path: PathId,
    ) -> (linear::MatchOp, linear::MatchResult)
    where
        TOperator: Into<NonZeroU32>,
    {
        match self {
            Pattern::ValueLiteral(ValueLiteral::Integer(Integer { value, .. })) => (
                linear::MatchOp::IntegerValue { path },
                Ok(integers.intern(*value as u64).into()),
            ),
            Pattern::ValueLiteral(ValueLiteral::Boolean(Boolean { value, .. })) => (
                linear::MatchOp::BooleanValue { path },
                linear::bool_to_match_result(*value),
            ),
            Pattern::ValueLiteral(ValueLiteral::ConditionCode(ConditionCode { cc, .. })) => {
                let cc = *cc as u32;
                debug_assert!(cc != 0, "no `ConditionCode` variants are zero");
                let expected = Ok(unsafe { NonZeroU32::new_unchecked(cc) });
                (linear::MatchOp::ConditionCode { path }, expected)
            }
            Pattern::Constant(Constant { id, .. }) => {
                if let Some(path_b) = lhs_id_to_path.get_first_occurrence(id) {
                    debug_assert!(path != path_b);
                    (
                        linear::MatchOp::Eq {
                            path_a: path,
                            path_b,
                        },
                        linear::bool_to_match_result(true),
                    )
                } else {
                    (
                        linear::MatchOp::IsConst { path },
                        linear::bool_to_match_result(true),
                    )
                }
            }
            Pattern::Variable(Variable { id, .. }) => {
                if let Some(path_b) = lhs_id_to_path.get_first_occurrence(id) {
                    debug_assert!(path != path_b);
                    (
                        linear::MatchOp::Eq {
                            path_a: path,
                            path_b,
                        },
                        linear::bool_to_match_result(true),
                    )
                } else {
                    (linear::MatchOp::Nop, Err(linear::Else))
                }
            }
            Pattern::Operation(op) => {
                let expected = Ok(op.operator.into());
                (linear::MatchOp::Opcode { path }, expected)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use peepmatic_runtime::{
        integer_interner::IntegerId,
        linear::{bool_to_match_result, Action::*, Else, MatchOp::*},
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

                let mut paths = PathInterner::new();
                let mut p = |p: &[u8]| paths.intern(Path::new(&p));

                let mut integers = IntegerInterner::new();
                let mut i = |i: u64| integers.intern(i);

                #[allow(unused_variables)]
                let make_expected: fn(
                    &mut dyn FnMut(&[u8]) -> PathId,
                    &mut dyn FnMut(u64) -> IntegerId,
                ) -> (Vec<linear::Match>, Vec<linear::Action<_>>) = $make_expected;

                let expected = make_expected(&mut p, &mut i);
                let actual = linearize_optimization(&mut paths, &mut integers, &opts.optimizations[0]);
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
        |p, i| (
            vec![
                linear::Match {
                    operation: Opcode { path: p(&[0]) },
                    expected: Ok(TestOperator::Imul.into()),
                },
                linear::Match {
                    operation: Nop,
                    expected: Err(Else),
                },
                linear::Match {
                    operation: IsConst { path: p(&[0, 1]) },
                    expected: bool_to_match_result(true),
                },
                linear::Match {
                    operation: IsPowerOfTwo { path: p(&[0, 1]) },
                    expected: bool_to_match_result(true),
                },
            ],
            vec![
                GetLhs { path: p(&[0, 0]) },
                GetLhs { path: p(&[0, 1]) },
                UnaryUnquote {
                    operator: UnquoteOperator::Log2,
                    operand: linear::RhsId(1)
                },
                MakeBinaryInst {
                    operator: TestOperator::Ishl,
                    r#type: Type {
                        kind: Kind::Int,
                        bit_width: BitWidth::Polymorphic
                    },
                    operands: [linear::RhsId(0), linear::RhsId(2)]
                }
            ],
        ),
    );

    linearizes_to!(variable_pattern_id_optimization, "(=> $x $x)", |p, i| (
        vec![linear::Match {
            operation: Nop,
            expected: Err(Else),
        }],
        vec![GetLhs { path: p(&[0]) }],
    ));

    linearizes_to!(constant_pattern_id_optimization, "(=> $C $C)", |p, i| (
        vec![linear::Match {
            operation: IsConst { path: p(&[0]) },
            expected: bool_to_match_result(true),
        }],
        vec![GetLhs { path: p(&[0]) }],
    ));

    linearizes_to!(boolean_literal_id_optimization, "(=> true true)", |p, i| (
        vec![linear::Match {
            operation: BooleanValue { path: p(&[0]) },
            expected: bool_to_match_result(true),
        }],
        vec![MakeBooleanConst {
            value: true,
            bit_width: BitWidth::Polymorphic,
        }],
    ));

    linearizes_to!(number_literal_id_optimization, "(=> 5 5)", |p, i| (
        vec![linear::Match {
            operation: IntegerValue { path: p(&[0]) },
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
        |p, i| (
            vec![
                linear::Match {
                    operation: Opcode { path: p(&[0]) },
                    expected: Ok(TestOperator::Iconst.into()),
                },
                linear::Match {
                    operation: IsConst { path: p(&[0, 0]) },
                    expected: bool_to_match_result(true),
                },
            ],
            vec![
                GetLhs { path: p(&[0, 0]) },
                MakeUnaryInst {
                    operator: TestOperator::Iconst,
                    r#type: Type {
                        kind: Kind::Int,
                        bit_width: BitWidth::Polymorphic,
                    },
                    operand: linear::RhsId(0),
                },
            ],
        ),
    );

    linearizes_to!(
        redundant_bor,
        "(=> (bor $x (bor $x $y)) (bor $x $y))",
        |p, i| (
            vec![
                linear::Match {
                    operation: Opcode { path: p(&[0]) },
                    expected: Ok(TestOperator::Bor.into()),
                },
                linear::Match {
                    operation: Nop,
                    expected: Err(Else),
                },
                linear::Match {
                    operation: Opcode { path: p(&[0, 1]) },
                    expected: Ok(TestOperator::Bor.into()),
                },
                linear::Match {
                    operation: Eq {
                        path_a: p(&[0, 1, 0]),
                        path_b: p(&[0, 0]),
                    },
                    expected: bool_to_match_result(true),
                },
                linear::Match {
                    operation: Nop,
                    expected: Err(Else),
                },
            ],
            vec![
                GetLhs { path: p(&[0, 0]) },
                GetLhs {
                    path: p(&[0, 1, 1]),
                },
                MakeBinaryInst {
                    operator: TestOperator::Bor,
                    r#type: Type {
                        kind: Kind::Int,
                        bit_width: BitWidth::Polymorphic,
                    },
                    operands: [linear::RhsId(0), linear::RhsId(1)],
                },
            ],
        ),
    );

    linearizes_to!(
        large_integers,
        // u64::MAX
        "(=> 18446744073709551615 0)",
        |p, i| (
            vec![linear::Match {
                operation: IntegerValue { path: p(&[0]) },
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
        |p, i| (
            vec![
                linear::Match {
                    operation: Opcode { path: p(&[0]) },
                    expected: Ok(TestOperator::Ireduce.into()),
                },
                linear::Match {
                    operation: linear::MatchOp::BitWidth { path: p(&[0]) },
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
