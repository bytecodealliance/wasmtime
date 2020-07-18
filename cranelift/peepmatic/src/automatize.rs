//! Compile a set of linear optimizations into an automaton.

use peepmatic_automata::{Automaton, Builder};
use peepmatic_runtime::linear;
use std::fmt::Debug;
use std::hash::Hash;

/// Construct an automaton from a set of linear optimizations.
pub fn automatize<TOperator>(
    opts: &linear::Optimizations<TOperator>,
) -> Automaton<linear::MatchResult, linear::MatchOp, Box<[linear::Action<TOperator>]>>
where
    TOperator: Copy + Debug + Eq + Hash,
{
    debug_assert!(crate::linear_passes::is_sorted_lexicographically(opts));

    let mut builder =
        Builder::<linear::MatchResult, linear::MatchOp, Box<[linear::Action<TOperator>]>>::new();

    for opt in &opts.optimizations {
        let mut insertion = builder.insert();
        for inc in &opt.increments {
            // Ensure that this state's associated data is this increment's
            // match operation.
            if let Some(op) = insertion.get_state_data() {
                assert_eq!(*op, inc.operation);
            } else {
                insertion.set_state_data(inc.operation);
            }

            insertion.next(inc.expected, inc.actions.clone().into_boxed_slice());
        }
        insertion.finish();
    }

    builder.finish()
}
