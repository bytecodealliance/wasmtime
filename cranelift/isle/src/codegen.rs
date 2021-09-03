//! Generate Rust code from a series of Sequences.

use crate::ir::{lower_rule, ExprSequence, PatternInst, PatternSequence, Value};
use crate::sema::{RuleId, TermEnv, TermId, TypeEnv};
use peepmatic_automata::{Automaton, Builder as AutomatonBuilder};
use std::collections::HashMap;

// TODO: automata built by output term as well

/// Builder context for one function in generated code corresponding
/// to one root input term.
struct TermFunctionBuilder {
    root_term: TermId,
    automaton: AutomatonBuilder<PatternInst, (), ExprSequence>,
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

pub struct Automata {
    pub automata_by_input: HashMap<TermId, Automaton<PatternInst, (), ExprSequence>>,
    pub automata_by_output: HashMap<TermId, Automaton<PatternInst, (), ExprSequence>>,
}

impl Automata {}
