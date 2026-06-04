use crate::spec::{self, SpecEnv};
use crate::trie;
use anyhow::{Result, bail};
use cranelift_isle::ast::{Def, Ident};
use cranelift_isle::error::{self, Errors, Span};
use cranelift_isle::files::Files;
use cranelift_isle::lexer::Pos;
use cranelift_isle::sema::{
    self, Rule, RuleId, Term, TermEnv, TermId, Type, TypeEnv, TypeId, VariantId,
};
use cranelift_isle::trie_again::{Overlap, RuleSet};
use cranelift_isle::{lexer, parser};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub struct Program {
    pub files: Arc<Files>,
    pub tyenv: TypeEnv,
    pub termenv: TermEnv,
    pub specenv: SpecEnv,
    pub overlaps: HashMap<RuleId, HashSet<RuleId>>,
}

impl Program {
    pub fn from_files(
        paths: &Vec<std::path::PathBuf>,
        expand_internal_extractors: bool,
    ) -> Result<Self> {
        let files = match Files::from_paths(paths, Default::default()) {
            Ok(files) => files,
            Err((path, err)) => {
                bail!(Errors::from_io(
                    err,
                    format!("cannot read file {}", path.display()),
                ))
            }
        };

        let files = Arc::new(files);

        let mut defs = Vec::new();
        for (file, src) in files.file_texts.iter().enumerate() {
            let lexer = match lexer::Lexer::new(file, src) {
                Ok(lexer) => lexer,
                Err(err) => bail!(Errors::new(vec![err], files)),
            };

            match parser::parse(lexer) {
                Ok(mut ds) => defs.append(&mut ds),
                Err(err) => bail!(Errors::new(vec![err], files)),
            }
        }

        let mut tyenv = match sema::TypeEnv::from_ast(&defs) {
            Ok(type_env) => type_env,
            Err(errs) => bail!(Errors::new(errs, files)),
        };

        let termenv = match sema::TermEnv::from_ast(&mut tyenv, &defs, expand_internal_extractors) {
            Ok(term_env) => term_env,
            Err(errs) => bail!(Errors::new(errs, files)),
        };

        let specenv = spec::SpecEnv::from_ast(&defs, &termenv, &tyenv)?;

        let overlaps = Self::build_overlaps(&defs, files.clone())?;

        Ok(Self {
            files,
            tyenv,
            termenv,
            specenv,
            overlaps,
        })
    }

    pub fn ty(&self, type_id: TypeId) -> &Type {
        self.tyenv
            .types
            .get(type_id.index())
            .expect("invalid type id")
    }

    pub fn type_name(&self, type_id: TypeId) -> &str {
        self.ty(type_id).name(&self.tyenv)
    }

    pub fn term(&self, term_id: TermId) -> &Term {
        self.termenv
            .terms
            .get(term_id.index())
            .expect("invalid term id")
    }

    pub fn term_name(&self, term_id: TermId) -> &str {
        let term = self.term(term_id);
        &self.tyenv.syms[term.name.index()]
    }

    pub fn get_variant_term(&self, ty: TypeId, variant: VariantId) -> TermId {
        self.termenv.get_variant_term(&self.tyenv, ty, variant)
    }

    pub fn rule(&self, rule_id: RuleId) -> &Rule {
        self.termenv
            .rules
            .get(rule_id.index())
            .expect("invalid rule id")
    }

    pub fn rule_identifer(&self, rule_id: RuleId) -> String {
        let rule = self.rule(rule_id);
        rule.identifier(&self.tyenv, &self.files)
    }

    pub fn rules_by_term(&self) -> HashMap<TermId, Vec<RuleId>> {
        let mut rules: HashMap<TermId, Vec<RuleId>> = HashMap::new();
        for rule in &self.termenv.rules {
            rules.entry(rule.root_term).or_default().push(rule.id);
        }
        rules
    }

    pub fn get_rule_by_identifier(&self, id: &str) -> Option<&Rule> {
        self.termenv
            .rules
            .iter()
            .find(|r| r.identifier(&self.tyenv, &self.files) == id)
    }

    pub fn get_term_by_name(&self, name: &str) -> Option<TermId> {
        let sym = Ident(name.to_string(), Pos::default());
        self.termenv.get_term_by_name(&self.tyenv, &sym)
    }

    pub fn build_trie(&self) -> Result<Vec<(TermId, RuleSet)>, Errors> {
        trie::build_trie(&self.termenv, self.files.clone())
    }

    pub(crate) fn error_at_pos(&self, pos: Pos, msg: impl Into<String>) -> Errors {
        // In order to piggy back off the existing diagnostic error reporting in
        // ISLE, we shoehorn our error type into one of the existing error
        // categories.
        //
        // TODO(mbm): cleaner positional error reporting for the verifier
        let err = error::Error::TypeError {
            msg: msg.into(),
            span: Span::new_single(pos),
        };
        Errors::new(vec![err], self.files.clone())
    }

    fn build_overlaps(defs: &[Def], files: Arc<Files>) -> Result<HashMap<RuleId, HashSet<RuleId>>> {
        // Overlap checking relies on term environment constructed with internal
        // extractor expansion enabled, so we need to generate it again.
        let mut tyenv = match sema::TypeEnv::from_ast(defs) {
            Ok(type_env) => type_env,
            Err(errs) => bail!(Errors::new(errs, files)),
        };

        let expand_internal_extractors = true;
        let termenv = match sema::TermEnv::from_ast(&mut tyenv, defs, expand_internal_extractors) {
            Ok(term_env) => term_env,
            Err(errs) => bail!(Errors::new(errs, files)),
        };

        // Check all pairs of rules for overlap.
        let term_rule_sets = trie::build_trie(&termenv, files.clone())?;
        let mut overlaps: HashMap<RuleId, HashSet<RuleId>> = HashMap::new();
        for (_, rule_set) in &term_rule_sets {
            for rule in &rule_set.rules {
                for other in &rule_set.rules {
                    // Ignore same or higher priority rules.
                    if other.prio <= rule.prio {
                        continue;
                    }

                    // Check for overlap.
                    let overlap = rule.may_overlap(other);
                    if overlap == Overlap::No {
                        continue;
                    }

                    // Record overlap.
                    overlaps.entry(rule.id).or_default().insert(other.id);
                }
            }
        }

        Ok(overlaps)
    }
}
