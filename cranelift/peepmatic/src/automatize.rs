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
        let mut is_first = true;
        for m in &opt.matches {
            // Ensure that this state's associated data is this match's
            // operation.
            if let Some(op) = insertion.get_state_data() {
                assert_eq!(*op, m.operation);
            } else {
                insertion.set_state_data(m.operation);
            }

            let actions = if is_first {
                is_first = false;
                opt.actions.clone().into_boxed_slice()
            } else {
                vec![].into_boxed_slice()
            };
            insertion.next(m.expected, actions);
        }
        insertion.finish();
    }

    builder.finish()
}
