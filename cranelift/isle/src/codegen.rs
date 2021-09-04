//! Generate Rust code from a series of Sequences.

use crate::error::Error;
use crate::ir::{lower_rule, ExprSequence, PatternInst, PatternSequence, Value};
use crate::sema::{RuleId, TermEnv, TermId, TypeEnv};
use peepmatic_automata::{Automaton, Builder as AutomatonBuilder};
use std::collections::HashMap;

/// One "input symbol" for the automaton that handles matching on a
/// term. Each symbol represents one step: we either run a match op,
/// or we get a result from it.
///
/// Note that in the original Peepmatic scheme, the problem that this
/// solves was handled slightly differently. The automaton responded
/// to alphabet symbols that corresponded only to match results, and
/// the "extra state" was used at each automaton node to represent the
/// op to run next. This extra state differentiated nodes that would
/// otherwise be merged together by deduplication. That scheme works
/// well enough, but the "extra state" is slightly confusing and
/// diverges slightly from a pure automaton.
///
/// Instead, here, we imagine that the user of the automaton can query
/// the possible transition edges out of the current state. Each of
/// these edges corresponds to one possible match op to run. After
/// running a match op, we reach a new state corresponding to
/// successful matches up to that point.
///
/// However, it's a bit more subtle than this; we add one additional
/// dimension to each match op, and an additional alphabet symbol.
///
/// First, consider the prioritization problem. We want to give the
/// DSL user the ability to change the order in which rules apply, for
/// example to have a tier of "fallback rules" that apply only if more
/// custom rules do not match.
///
/// A somewhat simplistic answer to this problem is "more specific
/// rule wins". However, this implies the existence of a total
/// ordering of linearized match sequences that may not fully capture
/// the intuitive meaning of "more specific". Consider four left-hand
/// sides:
///
/// - (A _ _)
/// - (A (B _) _)
/// - (A _ (B _))
///
/// Intuitively, the first is the least specific. Given the input `(A
/// (B 1) (B 2)`, we can say for sure that the first should not be
/// chosen, because either the second or third would match "more" of
/// the input tree. But which of the second and third should be
/// chosen? A "lexicographic ordering" rule would say that we sort
/// left-hand sides such that the `(B _)` sub-pattern comes before the
/// wildcard `_`, so the second rule wins. But that is arbitrarily
/// privileging one over the other based on the order of the
/// arguments.
///
/// Instead, we add a priority to every rule (optionally specified in
/// the source and defaulting to `0` otherwise) that conceptually
/// augments match-ops. Then, when we examine out-edges from a state
/// to decide on the next match, we sort these by highest priority
/// first.
///
/// This, too, sacrifices some deduplication, so we refine the idea a
/// bit. First, we add an "End of Match" alphabet symbol that
/// represents a successful match. Then we stipulate that priorities
/// are attached *only* to "End of Match"...
///
/// -- ah, this doesn't work because we need the (min, max) priority
/// range on outbound edges. When we see a possible transition to EOM
/// at prio 10 or a match op that could lead to an EOM at prio 0 or
/// 20, we need to do both, NFA-style.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum AutomataInput {
    Match { op: PatternInst },
    EndOfMatch { prio: i32 },
}

/// Builder context for one function in generated code corresponding
/// to one root input term.
///
/// A `TermFunctionBuilder` can correspond to the matching
/// control-flow and operations that we execute either when evaluating
/// *forward* on a term, trying to match left-hand sides against it
/// and transforming it into another term; or *backward* on a term,
/// trying to match another rule's left-hand side against an input to
/// produce the term in question (when the term is used in the LHS of
/// the calling term).
struct TermFunctionBuilder {
    root_term: TermId,
    automaton: AutomatonBuilder<AutomataInput, (), ExprSequence>,
}

impl TermFunctionBuilder {
    fn new(root_term: TermId) -> Self {
        TermFunctionBuilder {
            root_term,
            automaton: AutomatonBuilder::new(),
        }
    }

    fn add_rule(&mut self, pattern_seq: PatternSequence, expr_seq: ExprSequence) {
        let mut insertion = self.automaton.insert();

        let mut out_idx = 0;
        for (i, inst) in pattern_seq.insts.into_iter().enumerate() {
            // Determine how much of the output we can emit at this
            // stage (with the `Value`s that will be defined so far,
            // given input insts 0..=i).
            let out_start = out_idx;
            let mut out_end = out_start;
            while out_end < expr_seq.insts.len() {
                let mut max_input_inst = 0;
                expr_seq.insts[out_end].visit_values(|val| {
                    if let Value::Pattern { inst, .. } = val {
                        max_input_inst = std::cmp::max(max_input_inst, inst.index());
                    }
                });
                if max_input_inst > i {
                    break;
                }
                out_end += 1;
            }

            // Create an ExprSequence for the instructions that we can
            // output at this point.
            let out_insts = expr_seq.insts[out_start..out_end]
                .iter()
                .cloned()
                .collect::<Vec<_>>();
            let out_seq = ExprSequence { insts: out_insts };
            out_idx = out_end;

            insertion.next(inst, out_seq);
        }

        insertion.finish();
    }
}

struct TermFunctionsBuilder<'a> {
    typeenv: &'a TypeEnv,
    termenv: &'a TermEnv,
    builders_by_input: HashMap<TermId, TermFunctionBuilder>,
    builders_by_output: HashMap<TermId, TermFunctionBuilder>,
}

impl<'a> TermFunctionsBuilder<'a> {
    fn new(typeenv: &'a TypeEnv, termenv: &'a TermEnv) -> Self {
        log::trace!("typeenv: {:?}", typeenv);
        log::trace!("termenv: {:?}", termenv);
        Self {
            builders_by_input: HashMap::new(),
            builders_by_output: HashMap::new(),
            typeenv,
            termenv,
        }
    }

    fn build(&mut self) {
        for rule in 0..self.termenv.rules.len() {
            let rule = RuleId(rule);
            let (lhs_root, pattern, rhs_root, expr) = lower_rule(self.typeenv, self.termenv, rule);
            log::trace!(
                "build:\n- rule {:?}\n- lhs_root {:?} rhs_root {:?}\n- pattern {:?}\n- expr {:?}",
                self.termenv.rules[rule.index()],
                lhs_root,
                rhs_root,
                pattern,
                expr
            );
            if let Some(input_root_term) = lhs_root {
                self.builders_by_input
                    .entry(input_root_term)
                    .or_insert_with(|| TermFunctionBuilder::new(input_root_term))
                    .add_rule(pattern.clone(), expr.clone());
            }
            if let Some(output_root_term) = rhs_root {
                self.builders_by_output
                    .entry(output_root_term)
                    .or_insert_with(|| TermFunctionBuilder::new(output_root_term))
                    .add_rule(pattern, expr);
            }
        }
    }

    fn create_automata(self) -> Automata {
        let automata_by_input = self
            .builders_by_input
            .into_iter()
            .map(|(k, mut v)| (k, v.automaton.finish()))
            .collect::<HashMap<_, _>>();
        let automata_by_output = self
            .builders_by_output
            .into_iter()
            .map(|(k, mut v)| (k, v.automaton.finish()))
            .collect::<HashMap<_, _>>();
        Automata {
            automata_by_input,
            automata_by_output,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Automata {
    pub automata_by_input: HashMap<TermId, Automaton<PatternInst, (), ExprSequence>>,
    pub automata_by_output: HashMap<TermId, Automaton<PatternInst, (), ExprSequence>>,
}

impl Automata {
    pub fn compile(typeenv: &TypeEnv, termenv: &TermEnv) -> Result<Automata, Error> {
        let mut builder = TermFunctionsBuilder::new(typeenv, termenv);
        builder.build();
        Ok(builder.create_automata())
    }
}
