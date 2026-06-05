use crate::{program::Program, reachability::Reachability};
use anyhow::{Result, bail, format_err};
use cranelift_isle::{
    disjointsets::DisjointSets,
    sema::{RuleId, TermId},
    trie_again::{Binding, BindingId, Constraint, Rule, RuleSet},
};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub enum Constrain {
    Match(BindingId, Constraint),
    NotAll(Vec<Constrain>),
}

impl Constrain {
    fn bindings(&self) -> Vec<BindingId> {
        match self {
            Constrain::Match(binding_id, _) => vec![*binding_id],
            Constrain::NotAll(constrains) => constrains.iter().flat_map(|c| c.bindings()).collect(),
        }
    }

    fn substitute(&self, reindex: &Reindex) -> Self {
        match self {
            Constrain::Match(binding_id, constraint) => {
                Constrain::Match(reindex.id(*binding_id), *constraint)
            }
            Constrain::NotAll(constrains) => {
                Constrain::NotAll(constrains.iter().map(|c| c.substitute(reindex)).collect())
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Expansion {
    pub term: TermId,
    pub rules: Vec<RuleId>,
    pub negated: Vec<RuleId>,
    pub bindings: Vec<Option<Binding>>,
    pub constraints: Vec<Constrain>,
    pub equals: DisjointSets<BindingId>,
    pub parameters: Vec<BindingId>,
    pub result: BindingId,
}

impl Expansion {
    /// Check basic feasibility of the expansion.
    ///
    /// A false return value means the expansion is known to never be viable.
    /// However, a successful feasibility check leaves open the possibility that
    /// the expansion is inapplicable for other reasons.
    pub fn is_feasible(&self) -> bool {
        // Assert data structure invariants.
        self.validate();

        // Check if any constraints are incompatible.
        for constrain in &self.constraints {
            match constrain {
                Constrain::Match(binding_id, constraint) => {
                    let binding = self.bindings[binding_id.index()]
                        .as_ref()
                        .expect("constrained binding must be defined");
                    if !constraint.compatible(binding) {
                        return false;
                    }
                }
                Constrain::NotAll(_) => {
                    // Conservatively assume negated constraints can be met.
                    continue;
                }
            }
        }

        true
    }

    fn constrain(&mut self, constrain: Constrain) {
        self.constraints.push(constrain);
    }

    fn add_match(&mut self, binding_id: BindingId, constraint: Constraint) {
        self.constrain(Constrain::Match(binding_id, constraint));
    }

    fn push_binding(&mut self, binding: Binding) -> BindingId {
        let binding_id = self.bindings.len().try_into().unwrap();
        self.bindings.push(Some(binding));
        binding_id
    }

    fn validate(&self) {
        // TODO(mbm): return errors in expansion validation rather than assert?

        // Bindings: all references should be valid.
        for binding in self.bindings.iter().flatten() {
            for source in binding.sources() {
                assert!(self.is_defined(*source));
            }
        }

        // Constraints: should refer to defined bindings.
        for constrain in &self.constraints {
            for binding_id in constrain.bindings() {
                assert!(self.is_defined(binding_id));
            }
        }

        // Parameters: should be defined argument bindings.
        for binding in &self.parameters {
            assert!(matches!(
                self.binding(*binding),
                Some(Binding::Argument { .. })
            ));
        }

        // Result: should be defined.
        assert!(self.is_defined(self.result));
    }

    fn is_defined(&self, binding_id: BindingId) -> bool {
        self.binding(binding_id).is_some()
    }

    pub fn binding(&self, binding_id: BindingId) -> Option<&Binding> {
        self.bindings.get(binding_id.index())?.as_ref()
    }

    pub fn equalities(&self) -> Vec<(BindingId, BindingId)> {
        let mut eqs = Vec::new();
        for (i, binding) in self.bindings.iter().enumerate() {
            if binding.is_none() {
                continue;
            }
            let binding_id = i.try_into().unwrap();
            if let Some(eq) = self.equals.find(binding_id)
                && eq != binding_id
            {
                eqs.push((binding_id, eq));
            }
        }
        eqs
    }

    pub fn terms(&self, prog: &Program) -> Vec<TermId> {
        let mut terms: Vec<_> = self
            .bindings
            .iter()
            .flatten()
            .filter_map(|b| b.term(&prog.tyenv, &prog.termenv))
            .collect();
        // TODO(mbm): dedupe and preserve order
        terms.sort();
        terms.dedup();
        terms
    }

    /// Tags that appear on rules and term in the expansion.
    pub fn tags(&self, prog: &Program) -> HashSet<String> {
        let mut tags = HashSet::new();

        // Root term
        if let Some(term_tags) = prog.specenv.term_tags.get(&self.term) {
            tags = &tags | term_tags;
        }

        // Rules
        for rule_id in &self.rules {
            if let Some(rule_tags) = prog.specenv.rule_tags.get(rule_id) {
                tags = &tags | rule_tags;
            }
        }

        // Terms
        for term_id in self.terms(prog) {
            if let Some(term_tags) = prog.specenv.term_tags.get(&term_id) {
                tags = &tags | term_tags;
            }
        }

        tags
    }

    fn constructor_bindings(&self) -> Vec<(BindingId, TermId)> {
        self.bindings
            .iter()
            .enumerate()
            .flat_map(|(i, binding)| match binding {
                Some(Binding::Constructor { term, .. }) => Some((i.try_into().unwrap(), *term)),
                _ => None,
            })
            .collect()
    }

    fn substitute(&mut self, target: BindingId, replace: BindingId) {
        // Reindex bindings.
        let mut reindex = Reindex::new();
        reindex.map(target, replace);
        for binding in self.bindings.iter_mut().flatten() {
            *binding = reindex.binding(binding);
        }

        // Delete the target binding.
        self.bindings[target.index()] = None;

        // Constraints.
        self.constraints = self
            .constraints
            .iter()
            .map(|c| c.substitute(&reindex))
            .collect();

        // Result.
        self.result = reindex.id(self.result);
    }
}

/// Chaining configuration.
pub struct Chaining<'a> {
    prog: &'a Program,
    term_rule_sets: &'a HashMap<TermId, RuleSet>,
    reach: Reachability,
    exclude: HashSet<TermId>,
    include: HashSet<TermId>,
    include_macros: bool,
    max_rules: usize,
    default: bool,
}

impl<'a> Chaining<'a> {
    pub fn new(prog: &'a Program, term_rule_sets: &'a HashMap<TermId, RuleSet>) -> Result<Self> {
        Ok(Self {
            prog,
            term_rule_sets,
            reach: Reachability::build(term_rule_sets),
            include: HashSet::new(),
            exclude: HashSet::new(),
            include_macros: true,
            max_rules: 0,
            default: false,
        })
    }

    pub fn chain_term(&mut self, term_name: &str) -> Result<()> {
        let term_id = self
            .prog
            .get_term_by_name(term_name)
            .ok_or(format_err!("unknown term {term_name}"))?;
        self.include.insert(term_id);
        Ok(())
    }

    pub fn chain_terms(&mut self, term_names: &Vec<String>) -> Result<()> {
        for term_name in term_names {
            self.chain_term(term_name)?;
        }
        Ok(())
    }

    /// Set whether to chain "macro" terms. Macro terms are wrapper terms with
    /// only one rule that has no constraints.
    pub fn chain_macros(&mut self, enabled: bool) {
        self.include_macros = enabled;
    }

    pub fn set_max_rules(&mut self, max_rules: usize) {
        self.max_rules = max_rules;
    }

    pub fn exclude_chain_term(&mut self, term_name: &str) -> Result<()> {
        let term_id = self
            .prog
            .get_term_by_name(term_name)
            .ok_or(format_err!("unknown term {term_name}"))?;
        self.exclude.insert(term_id);
        Ok(())
    }

    pub fn exclude_chain_terms(&mut self, term_names: &Vec<String>) -> Result<()> {
        for term_name in term_names {
            self.exclude_chain_term(term_name)?;
        }
        Ok(())
    }

    /// Configure whether terms should be considered for chaining by default, if
    /// no other rules apply.
    pub fn set_default(&mut self, default: bool) {
        self.default = default;
    }

    /// Report whether the term has expansions.
    ///
    /// From the point of view of the expansion graph, this means the term is
    /// not a leaf node.  Therefore, we can either apply chaining or produce
    /// expansions rooted at this term.
    pub fn is_expandable(&self, term_id: TermId) -> bool {
        // Is it an internal constructor?
        let term = self.prog.term(term_id);
        if !term.has_internal_constructor() {
            return false;
        }

        // Term should have rules.
        if self.num_rules(term_id) == 0 {
            return false;
        }

        true
    }

    /// Reports whether the given term can be chained.
    ///
    /// Terms can be chained if they are expandable and acyclic.
    pub fn is_chainable(&self, term_id: TermId) -> bool {
        // At minimum, it should be a term that has expansions.
        if !self.is_expandable(term_id) {
            return false;
        }

        // Cyclic terms cannot be chained.
        if self.reach.is_cyclic(term_id) {
            return false;
        }

        true
    }

    /// Validate chaining configuration.
    pub fn validate(&self) -> Result<()> {
        for term_id in (0..self.prog.termenv.terms.len()).map(TermId) {
            if self.has_chain_attribute(term_id) && !self.is_chainable(term_id) {
                bail!(
                    "term '{name}' has chain attribute but is not chainable",
                    name = self.prog.term_name(term_id)
                );
            }
        }
        Ok(())
    }

    pub fn should_chain(&self, term_id: TermId) -> bool {
        // Check baseline requirements.
        if !self.is_chainable(term_id) {
            return false;
        }

        // Terms with specs should not be chained.
        if self.prog.specenv.has_spec(term_id) {
            return false;
        }

        // Explicit exclusions.
        if self.exclude.contains(&term_id) {
            return false;
        }

        // Explicit inclusions.
        if self.include.contains(&term_id) {
            return true;
        }

        // Marked with chaining attribute.
        if self.prog.specenv.chain.contains(&term_id) {
            return true;
        }

        // Chain macros, if configured.
        if self.include_macros && self.is_macro(term_id) {
            return true;
        }

        // Max rules threshold, if set.
        if self.max_rules > 0 && self.num_rules(term_id) > self.max_rules {
            return false;
        }

        // Default fallback.
        self.default
    }

    fn has_chain_attribute(&self, term_id: TermId) -> bool {
        self.prog.specenv.chain.contains(&term_id)
    }

    fn num_rules(&self, term_id: TermId) -> usize {
        self.term_rule_sets
            .get(&term_id)
            .map(|rule_set| rule_set.rules.len())
            .unwrap_or_default()
    }

    fn is_macro(&self, term_id: TermId) -> bool {
        // "Macro" terms have only one rule.
        if self.num_rules(term_id) != 1 {
            return false;
        }

        // Rule should be "trivial".
        let rule = &self.term_rule_sets[&term_id].rules[0];
        rule.total_constraints() == 0
    }
}

/// Partially completed expansion.
struct Partial {
    /// Current state of the expansion.
    expansion: Expansion,

    /// Stack of bindings to apply chaining to. May be non-exhaustive.
    chain_candidates: Vec<BindingId>,
}

pub struct Expander<'a> {
    prog: &'a Program,
    term_rule_sets: &'a HashMap<TermId, RuleSet>,

    /// Chaining configuration: which terms should be chained.
    chaining: Chaining<'a>,

    /// Whether to drop expansions as soon as they are deemed to be infeasible.
    prune_infeasible: bool,

    /// Expansions under construction.
    stack: Vec<Partial>,

    /// Root terms expansion has been initiated for.
    roots: HashSet<TermId>,

    /// Completed expansions.
    complete: Vec<Expansion>,
}

impl<'a> Expander<'a> {
    pub fn new(
        prog: &'a Program,
        term_rule_sets: &'a HashMap<TermId, RuleSet>,
        chaining: Chaining<'a>,
    ) -> Self {
        Self {
            prog,
            term_rule_sets,
            chaining,
            prune_infeasible: true,
            stack: Vec::new(),
            roots: HashSet::new(),
            complete: Vec::new(),
        }
    }

    /// Add the given named term as an expansion root.
    pub fn add_root_term_name(&mut self, term_name: &str) -> Result<()> {
        let term_id = self
            .prog
            .get_term_by_name(term_name)
            .ok_or(format_err!("unknown term {term_name}"))?;
        self.add_root(term_id);
        Ok(())
    }

    /// Add the given term as an expansion root. That is, start expanding rules
    /// from this point.
    pub fn add_root(&mut self, term_id: TermId) {
        // Skip if the root has already been added.
        if self.roots.contains(&term_id) {
            return;
        }

        // Initialize an expansion at this root.
        self.constructor(term_id);

        // Record root.
        self.roots.insert(term_id);
    }

    // Push an initial expansion for a constructor call of the given term.
    fn constructor(&mut self, term_id: TermId) {
        // Lookup term.
        let term = self.prog.term(term_id);
        assert!(term.has_constructor());

        // Push argument bindings.
        let sig = term
            .constructor_sig(&self.prog.tyenv)
            .expect("should have constructor signature");
        let mut bindings = Vec::new();
        let mut parameters = Vec::new();
        for i in 0..sig.param_tys.len() {
            let parameter = bindings.len().try_into().unwrap();
            bindings.push(Some(Binding::Argument {
                index: i.try_into().unwrap(),
            }));
            parameters.push(parameter);
        }

        // Binding for the constructor call.
        let result = bindings.len().try_into().unwrap();
        bindings.push(Some(Binding::Constructor {
            term: term_id,
            parameters: parameters.clone().into(),
            instance: 0,
        }));

        // Root constructor call should be the first term to be expanded.
        let chain_candidates = vec![result];

        // Store.
        let expansion = Expansion {
            term: term_id,
            rules: Vec::new(),
            negated: Vec::new(),
            bindings,
            constraints: Vec::new(),
            equals: DisjointSets::default(),
            parameters,
            result,
        };
        assert!(expansion.is_feasible());
        self.stack.push(Partial {
            expansion,
            chain_candidates,
        });
    }

    /// Set whether to prune infeasible expansions. If enabled, expansions will
    /// be dropped as soon as they are deemed to be not feasible.
    pub fn set_prune_infeasible(&mut self, enabled: bool) {
        self.prune_infeasible = enabled;
    }

    fn finish(&mut self, expansion: Expansion) {
        // Cascade into any remaining constructors.
        //
        // Any remaining constructor calls could not be chained. Therefore, in
        // order to consider rules that apply to these terms, we need to
        // initiate expansion from them as root terms. We only do so for terms
        // that have an explicit specification: spec-less terms are assumed to
        // be part of some real chain and are not verified standalone.
        for (_, term_id) in expansion.constructor_bindings() {
            if self.chaining.is_expandable(term_id) && self.prog.specenv.has_spec(term_id) {
                self.add_root(term_id);
            }
        }

        // Add to completed list.
        //
        // As an internal consistency check, ensure we have produced a valid
        // result.
        expansion.validate();
        self.complete.push(expansion);
    }

    pub fn chaining(&self) -> &Chaining<'_> {
        &self.chaining
    }

    pub fn expansions(&self) -> &Vec<Expansion> {
        &self.complete
    }

    pub fn expand(&mut self) {
        while let Some(partial) = self.stack.pop() {
            self.extend(partial);
        }
    }

    fn extend(&mut self, partial: Partial) {
        let Partial {
            expansion,
            chain_candidates,
        } = partial;

        // Determine candidates for chaining.
        //
        // If we have pre-existing candidates at this stage, we'll process
        // those. Otherwise, revisit the expansion and see if previous chain
        // applications have produced more candidates.
        let mut chain_candidates = if !chain_candidates.is_empty() {
            chain_candidates
        } else {
            self.chain_candidates(&expansion)
        };

        // Select a candidate to chain. If none, we're done.
        let Some(chain_binding_id) = chain_candidates.pop() else {
            self.finish(expansion);
            return;
        };

        // Chain constructor.
        let binding = &expansion.bindings[chain_binding_id.index()];
        let Some(Binding::Constructor {
            term, parameters, ..
        }) = binding
        else {
            unreachable!("expect constructor binding")
        };

        let rule_set = &self.term_rule_sets[term];
        assert!(!rule_set.rules.is_empty());
        for rule in rule_set.rules.iter().rev() {
            // Apply rule.
            let mut apply = Application::new(expansion.clone());
            apply.rule(rule_set, rule, parameters, chain_binding_id);

            // Apply negations of higher-priority overlapping rules that are
            // tagged to indicate priority is significant.
            if let Some(overlaps) = self.prog.overlaps.get(&rule.id) {
                for other in rule_set.rules.iter().rev() {
                    let priority_significant = self.prog.specenv.priority.contains(&other.id);
                    if priority_significant && overlaps.contains(&other.id) {
                        apply.negation(rule_set, other);
                    }
                }
            }

            // Finalize the chaining application.
            let chained = apply.build();

            // Push onto stack for further expansion, optionally checking if the
            // current partial state is feasible.
            if !self.prune_infeasible || chained.is_feasible() {
                self.stack.push(Partial {
                    expansion: chained,
                    chain_candidates: chain_candidates.clone(),
                });
            }
        }
    }

    // Identify bindings that could be chained.
    fn chain_candidates(&mut self, expansion: &Expansion) -> Vec<BindingId> {
        expansion
            .constructor_bindings()
            .iter()
            .filter_map(|(binding_id, term_id)| {
                if self.chaining.should_chain(*term_id) {
                    Some(*binding_id)
                } else {
                    None
                }
            })
            .collect()
    }
}

struct Substitution {
    target: BindingId,
    replace: BindingId,
}

struct Application {
    expansion: Expansion,
    substitutions: Vec<Substitution>,
    import_reindex: Reindex,
}

impl Application {
    fn new(expansion: Expansion) -> Self {
        Self {
            expansion,
            substitutions: Vec::new(),
            import_reindex: Reindex::new(),
        }
    }

    fn rule(
        &mut self,
        rule_set: &RuleSet,
        rule: &Rule,
        parameters: &[BindingId],
        call_site: BindingId,
    ) {
        // Record the application of this rule.
        self.expansion.rules.push(rule.id);

        // Arguments.
        for (i, parameter) in parameters.iter().enumerate() {
            // Lookup binding ID from the source rule.
            let arg = Binding::Argument {
                index: i.try_into().unwrap(),
            };
            let binding_id = rule_set
                .find_binding(&arg)
                .expect("should have argument binding");

            // Import into expansion.
            let arg_binding_id = self.add_binding(rule_set, binding_id);

            // Substitute argument with the parameter.
            self.substitutions.push(Substitution {
                target: arg_binding_id,
                replace: *parameter,
            });
        }

        // Constraints.
        for i in 0..rule_set.bindings.len() {
            let binding_id = i.try_into().unwrap();
            if let Some(constraint) = rule.get_constraint(binding_id) {
                let expansion_binding_id = self.add_binding(rule_set, binding_id);
                self.expansion.add_match(expansion_binding_id, constraint);
            }
        }

        // Equals.
        for i in 0..rule_set.bindings.len() {
            let binding_id = i.try_into().unwrap();
            if let Some(equal_binding_id) = rule.equals.find(binding_id)
                && equal_binding_id != binding_id
            {
                let expansion_binding_id = self.add_binding(rule_set, binding_id);
                let expansion_equal_binding_id = self.add_binding(rule_set, equal_binding_id);
                self.expansion
                    .equals
                    .merge(expansion_binding_id, expansion_equal_binding_id);
            }
        }

        // TODO: iterators, prio?

        // Impure.
        for impure_binding_id in &rule.impure {
            self.add_binding(rule_set, *impure_binding_id);
        }

        // Result.
        //
        // Once imported, the callsite should be substituted for the result binding.
        let result_binding_id = self.add_binding(rule_set, rule.result);
        self.substitutions.push(Substitution {
            target: call_site,
            replace: result_binding_id,
        });
    }

    fn build(mut self) -> Expansion {
        // Process substitutions.
        for substitution in self.substitutions.iter().rev() {
            self.expansion
                .substitute(substitution.target, substitution.replace);
        }

        // Return expanded rule.
        self.expansion
    }

    fn negation(&mut self, rule_set: &RuleSet, rule: &Rule) {
        // Record negation.
        self.expansion.negated.push(rule.id);

        // Collect the constraints required.
        let mut constraints = Vec::new();
        for i in 0..rule_set.bindings.len() {
            let binding_id = i.try_into().unwrap();
            if let Some(constraint) = rule.get_constraint(binding_id) {
                let expansion_binding_id = self.add_binding(rule_set, binding_id);
                constraints.push(Constrain::Match(expansion_binding_id, constraint));
            }
        }

        // Require their negation.
        self.expansion.constrain(Constrain::NotAll(constraints));

        // TODO(mbm): negation of equality constraints
    }

    fn add_binding(&mut self, rule_set: &RuleSet, binding_id: BindingId) -> BindingId {
        // Check if it has already been added.
        if self.import_reindex.is_mapped(binding_id) {
            return self.import_reindex.id(binding_id);
        }

        // Add dependencies first.
        let binding = &rule_set.bindings[binding_id.index()];
        for source in binding.sources() {
            self.add_binding(rule_set, *source);
        }

        // Reindex this binding.
        let reindexed = self.import_reindex.binding(binding);

        // Insert into expansion bindings list.
        let expansion_binding_id = self.expansion.push_binding(reindexed);

        // Record binding mapping.
        self.import_reindex.map(binding_id, expansion_binding_id);
        expansion_binding_id
    }
}

/// Reindexing binding IDs.
struct Reindex {
    to: HashMap<BindingId, BindingId>,
}

impl Reindex {
    fn new() -> Self {
        Self { to: HashMap::new() }
    }

    fn is_mapped(&self, binding_id: BindingId) -> bool {
        self.to.contains_key(&binding_id)
    }

    fn map(&mut self, from: BindingId, to: BindingId) {
        self.to.insert(from, to);
    }

    fn id(&self, binding_id: BindingId) -> BindingId {
        self.to.get(&binding_id).copied().unwrap_or(binding_id)
    }

    fn ids(&self, binding_ids: &[BindingId]) -> Box<[BindingId]> {
        binding_ids
            .iter()
            .map(|binding_id| self.id(*binding_id))
            .collect()
    }

    fn binding(&self, binding: &Binding) -> Binding {
        match binding {
            Binding::Argument { .. }
            | Binding::ConstInt { .. }
            | Binding::ConstBool { .. }
            | Binding::ConstPrim { .. } => binding.clone(),

            Binding::Extractor { term, parameter } => Binding::Extractor {
                term: *term,
                parameter: self.id(*parameter),
            },

            Binding::Constructor {
                term,
                parameters,
                instance,
            } => Binding::Constructor {
                term: *term,
                parameters: self.ids(parameters),
                instance: *instance,
            },

            Binding::MakeVariant {
                ty,
                variant,
                fields,
            } => Binding::MakeVariant {
                ty: *ty,
                variant: *variant,
                fields: self.ids(fields),
            },

            Binding::MatchVariant {
                source,
                variant,
                field,
            } => Binding::MatchVariant {
                source: self.id(*source),
                variant: *variant,
                field: *field,
            },

            Binding::MakeStruct { ty, fields } => Binding::MakeStruct {
                ty: *ty,
                fields: self.ids(fields),
            },

            Binding::ExtractStruct { source, field } => Binding::ExtractStruct {
                source: self.id(*source),
                field: *field,
            },

            Binding::MakeSome { inner } => Binding::MakeSome {
                inner: self.id(*inner),
            },

            Binding::MatchSome { source } => Binding::MatchSome {
                source: self.id(*source),
            },

            Binding::MatchTuple { source, field } => Binding::MatchTuple {
                source: self.id(*source),
                field: *field,
            },

            Binding::Iterator { .. } => unimplemented!("iterator bindings not supported"),
        }
    }
}
