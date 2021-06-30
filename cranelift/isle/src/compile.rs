//! Compilation process, from AST to Sema to Sequences of Insts.

use crate::error::*;
use crate::{ast, ir, sema};
use std::collections::HashMap;

/// A Compiler manages the compilation pipeline from AST to Sequences.
pub struct Compiler<'a> {
    ast: &'a ast::Defs,
    type_env: sema::TypeEnv,
    term_env: sema::TermEnv,
    seqs: Vec<ir::Sequence>,
    // TODO: if this becomes a perf issue, then build a better data
    // structure. For now we index on root term/variant.
    //
    // TODO: index at callsites (extractors/constructors) too. We'll
    // need tree-summaries of arg and expected return value at each
    // callsite.
    term_db: HashMap<ir::TermOrVariant, TermData>,
}

#[derive(Clone, Debug, Default)]
struct TermData {
    producers: Vec<(ir::TreeSummary, sema::RuleId)>,
    consumers: Vec<(ir::TreeSummary, sema::RuleId)>,
    has_constructor: bool,
    has_extractor: bool,
}

pub type CompileResult<T> = Result<T, Error>;

impl<'a> Compiler<'a> {
    pub fn new(ast: &'a ast::Defs) -> CompileResult<Compiler<'a>> {
        let mut type_env = sema::TypeEnv::from_ast(ast)?;
        let term_env = sema::TermEnv::from_ast(&mut type_env, ast)?;
        Ok(Compiler {
            ast,
            type_env,
            term_env,
            seqs: vec![],
            term_db: HashMap::new(),
        })
    }

    pub fn build_sequences(&mut self) -> CompileResult<()> {
        for rid in 0..self.term_env.rules.len() {
            let rid = sema::RuleId(rid);
            let seq = ir::Sequence::from_rule(&self.type_env, &self.term_env, rid);
            self.seqs.push(seq);
        }
        Ok(())
    }

    pub fn collect_tree_summaries(&mut self) -> CompileResult<()> {
        // For each rule, compute summaries of its LHS and RHS, then
        // index it in the appropriate TermData.
        for (i, seq) in self.seqs.iter().enumerate() {
            let rule_id = sema::RuleId(i);
            let consumer_summary = seq.input_tree_summary();
            let producer_summary = seq.output_tree_summary();
            if let Some(consumer_root_term) = consumer_summary.root() {
                let consumer_termdb = self
                    .term_db
                    .entry(consumer_root_term.clone())
                    .or_insert_with(|| Default::default());
                consumer_termdb.consumers.push((consumer_summary, rule_id));
            }
            if let Some(producer_root_term) = producer_summary.root() {
                let producer_termdb = self
                    .term_db
                    .entry(producer_root_term.clone())
                    .or_insert_with(|| Default::default());
                producer_termdb.consumers.push((producer_summary, rule_id));
            }
        }

        // For each term, if a constructor and/or extractor is
        // present, note that.
        for term in &self.term_env.terms {
            if let sema::TermKind::Regular {
                extractor,
                constructor,
            } = term.kind
            {
                if !extractor.is_some() && !constructor.is_some() {
                    continue;
                }
                let entry = self
                    .term_db
                    .entry(ir::TermOrVariant::Term(term.id))
                    .or_insert_with(|| Default::default());
                if extractor.is_some() {
                    entry.has_extractor = true;
                }
                if constructor.is_some() {
                    entry.has_constructor = true;
                }
            }
        }

        Ok(())
    }

    pub fn inline_internal_terms(&mut self) -> CompileResult<()> {
        unimplemented!()
    }

    pub fn to_sequences(self) -> Vec<ir::Sequence> {
        self.seqs
    }
}
