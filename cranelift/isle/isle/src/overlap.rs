//! Overlap detection for rules in ISLE.

use rayon::prelude::*;
use std::collections::{HashMap, HashSet};

use crate::error::{Error, Result, Source, Span};
use crate::sema::{
    self, ConstructorKind, Rule, RuleId, Sym, Term, TermEnv, TermId, TermKind, Type, TypeEnv,
    TypeId, VarId,
};

/// Check for overlap.
pub fn check(tyenv: &TypeEnv, termenv: &TermEnv) -> Result<()> {
    let env = Env::new(tyenv, termenv);
    let mut errors = termenv
        .terms
        .par_iter()
        .fold(Errors::default, |errs, term| {
            // The only isle declaration that currently produces overlap is constructors whose
            // definition is entirely in isle.
            if env.is_internal_constructor(term.id) {
                errs.union(check_overlap_groups(&env, term))
            } else {
                errs
            }
        })
        .reduce(Errors::default, Errors::union);

    let mut errors = errors.report(&env);
    match errors.len() {
        0 => Ok(()),
        1 => Err(errors.pop().unwrap()),
        _ => Err(Error::Errors(errors)),
    }
}

/// A node in the error graph.
#[derive(Default)]
struct Node {
    /// `true` entries where an edge exists to the node at that index.
    edges: HashSet<RuleId>,
}

impl Node {
    /// Add an edge between this node and the other node.
    fn add_edge(&mut self, other: RuleId) {
        self.edges.insert(other);
    }

    /// Remove an edge between this node and another node.
    fn remove_edge(&mut self, other: RuleId) {
        self.edges.remove(&other);
    }
}

/// A graph of all the rules in the isle source, with bi-directional edges between rules that are
/// discovered to have overlap problems.
#[derive(Default)]
struct Errors {
    /// Edges between rules indicating overlap. As the edges are not directed, the edges are
    /// normalized by ordering the rule ids.
    nodes: HashMap<RuleId, Node>,
}

impl Errors {
    fn union(mut self, other: Self) -> Self {
        for (id, node) in other.nodes {
            self.nodes.entry(id).or_default().edges.extend(node.edges);
        }
        self
    }

    /// Condense the overlap information down into individual errors.
    fn report(&mut self, env: &Env) -> Vec<Error> {
        let mut errors = Vec::new();

        let get_info = |id| {
            let rule = env.get_rule(id);

            let src = env.get_source(rule.pos.file);
            let span = Span::new_single(rule.pos);
            (src, span)
        };

        while let Some(id) = self
            .nodes
            .keys()
            .copied()
            .max_by_key(|id| self.nodes[id].edges.len())
        {
            let node = self.remove_edges(id);
            if node.edges.is_empty() {
                break;
            }

            // build the real error
            let mut rules = vec![get_info(id)];

            rules.extend(node.edges.into_iter().map(get_info));

            errors.push(Error::OverlapError {
                msg: String::from("rules are overlapping"),
                rules,
            });
        }

        errors
    }

    /// Remove all the edges for this rule in the graph, returning the original `Node` contents for
    /// further processing.
    fn remove_edges(&mut self, id: RuleId) -> Node {
        let node = self.nodes.remove(&id).unwrap();

        for other in node.edges.iter() {
            if let Some(other) = self.nodes.get_mut(&other) {
                other.remove_edge(id);
            }
        }

        node
    }

    /// Add a bidirectional edge between two rules in the graph.
    fn add_edge(&mut self, a: RuleId, b: RuleId) {
        // edges are undirected
        self.nodes.entry(a).or_default().add_edge(b);
        self.nodes.entry(b).or_default().add_edge(a);
    }
}

/// Check for overlapping rules within individual priority groups.
fn check_overlap_groups(env: &Env, term: &Term) -> Errors {
    let rows: Vec<_> = env
        .rules_for_term(term.id)
        .into_iter()
        .map(|id| Row::from_rule(env, id))
        .collect();

    let mut pairs = Vec::new();
    let mut cursor = &rows[..];
    while let Some((row, rest)) = cursor.split_first() {
        cursor = rest;
        pairs.extend(rest.iter().map(|other| (row, other)));
    }

    // Process rule pairs in parallel
    pairs
        .into_par_iter()
        .fold(Errors::default, |mut errs, (left, right)| {
            let lid = left.rule;
            let rid = right.rule;
            if check_overlap(env, left.clone(), right.clone()) {
                if env.get_rule(lid).prio == env.get_rule(rid).prio {
                    errs.add_edge(lid, rid);
                }
            }
            errs
        })
        .reduce(Errors::default, Errors::union)
}

/// Check for overlapping rules within a single prioirty group.
fn check_overlap(env: &Env, mut left: Row, mut right: Row) -> bool {
    while !left.is_empty() {
        // drop leading wildcards from both
        while !left.is_empty() && left.front().is_wildcard() && right.front().is_wildcard() {
            left.pop();
            right.pop();
        }

        if left.is_empty() {
            break;
        }

        // pick the best pattern from the leading column of the two rows
        let lr = left.leading_pattern();
        let rr = right.leading_pattern();

        let pat = if lr < rr { lr.1.clone() } else { rr.1.clone() };

        if lr.0 || rr.0 {
            left.specialize_and_patterns(&pat);
            right.specialize_and_patterns(&pat);
        }

        // specialize both rows on that pattern, and if specialization fails we know the two don't
        // overlap.
        if !left.specialize(env, &pat) || !right.specialize(env, &pat) {
            return false;
        }
    }

    return true;
}

/// A convenience wrapper around the `TypeEnv` and `TermEnv` environments.
struct Env<'a> {
    tyenv: &'a TypeEnv,
    termenv: &'a TermEnv,
}

impl<'a> Env<'a> {
    /// Construct a new [`Env`].
    fn new(tyenv: &'a TypeEnv, termenv: &'a TermEnv) -> Self {
        Self { tyenv, termenv }
    }

    /// Fetch the string associated with a symbol.
    fn get_sym(&self, id: Sym) -> &str {
        &self.tyenv.syms[id.0]
    }

    /// Fetch the rule associated with this id.
    fn get_rule(&self, id: RuleId) -> &Rule {
        &self.termenv.rules[id.0]
    }

    /// Fetch the term associated with this id.
    fn get_term(&self, id: TermId) -> &Term {
        &self.termenv.terms[id.0]
    }

    /// Fetch the tyep associated with this id.
    fn get_type(&self, id: TypeId) -> &Type {
        &self.tyenv.types[id.0]
    }

    /// Fetch source information for a file id.
    fn get_source(&self, file: usize) -> Source {
        Source::new(
            self.tyenv.filenames[file].clone(),
            self.tyenv.file_texts[file].clone(),
        )
    }

    /// True when this term represents a constructor implemented in isle.
    fn is_internal_constructor(&self, id: TermId) -> bool {
        match self.get_term(id).kind {
            TermKind::Decl {
                constructor_kind: Some(ConstructorKind::InternalConstructor),
                ..
            } => true,
            _ => false,
        }
    }

    /// The ids of all [`Rule`]s defined for this term.
    fn rules_for_term(&self, id: TermId) -> Vec<RuleId> {
        self.termenv
            .rules
            .iter()
            .filter_map(|rule| {
                if let sema::Pattern::Term(_, tid, _) = rule.lhs {
                    if tid == id {
                        return Some(rule.id);
                    }
                }
                None
            })
            .collect()
    }

    /// Returns true when this type is an enum with only a single constructor.
    fn is_single_constructor_enum(&self, ty: TypeId) -> bool {
        match self.get_type(ty) {
            Type::Primitive(_, _, _) => false,
            Type::Enum { variants, .. } => variants.len() == 1,
        }
    }
}

/// A version of [`sema::Pattern`] with some simplifications to make overlap checking easier.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
enum Pattern {
    /// Integer literal patterns.
    Int {
        value: i128,
    },

    /// Constant literal patterns, such as `$F32`.
    Const {
        name: Sym,
    },

    /// Enum variant constructors.
    Variant {
        id: TermId,
        single_case: bool,
        pats: Vec<Pattern>,
    },

    /// And patterns, with their sub-patterns sorted.
    And {
        pats: Vec<Pattern>,
    },

    /// Extractor uses (both fallible and infallible).
    Extractor {
        id: TermId,
        pats: Vec<Pattern>,
    },

    Wildcard,
}

impl Pattern {
    /// Create a [`Pattern`] from a [`sema::Pattern`]. The major differences between these two
    /// representations are as follows:
    /// 1. Variable bindings are removed and turned into wildcards
    /// 2. Equality constraints are removed and turned into inlined versions of the patterns they
    ///    would have introduced equalities with
    /// 3. [`sema::Pattern::Term`] instances are turned into either [`Pattern::Variant`] or
    ///    [`Pattern::Extractor`] cases depending on their term kind.
    /// 4. [`sema::Pattern::And`] instances are sorted to ensure that we can traverse them quickly
    ///    when specializing the matrix.
    fn from_sema(env: &Env, binds: &mut Vec<(VarId, Pattern)>, pat: &sema::Pattern) -> Self {
        match pat {
            sema::Pattern::BindPattern(_, id, pat) => {
                let pat = Self::from_sema(env, binds, pat);
                binds.push((*id, pat.clone()));
                pat
            }

            sema::Pattern::Var(_, id) => {
                for (vid, pat) in binds.iter().rev() {
                    if vid == id {
                        // We inline equality constraints for two reasons: we specialize on the
                        // spine of related patterns only, so more specific information about
                        // individual values isn't necessarily helpful; we consider overlap
                        // checking to be an over-approximation of overlapping rules, so handling
                        // equalies ends up being best-effort. As an approximation, we use whatever
                        // pattern happened to be at the binding of the variable for all of the
                        // cases where it's used for equality. For example, in the following rule:
                        //
                        // > (rule (example x @ (Enum.Variant y) x) ...)
                        //
                        // we will only specialize up to `(Enum.Variant _)`, so any more specific
                        // runtime values of `y` won't end up helping to identify overlap. As a
                        // result, we rewrite the patterns in the rule to look more like the
                        // following, as it greatly simplifies overlap checking.
                        //
                        // > (rule (example (Enum.Variant _) (Enum.Variant _)) ...)
                        //
                        // Cases that this scheme won't handle look like the following:
                        //
                        // > (rule (example2 2 3) ...)
                        // > (rule (example2 x x) ...)
                        //
                        // As in this case we'll not make use of the information that `2` and `3`
                        // aren't equal to know that the rules don't overlap. One approach that we
                        // could take here is delaying substitution to the point where a variable
                        // binding has been specialized, turning the rules into the following once
                        // specialization had occurred for `2`:
                        //
                        // > (rule (example2 2 3) ...)
                        // > (rule (example2 2 2) ...)
                        return pat.clone();
                    }
                }

                binds.push((*id, Pattern::Wildcard));
                Pattern::Wildcard
            }

            sema::Pattern::ConstInt(_, value) => Pattern::Int { value: *value },
            sema::Pattern::ConstPrim(_, name) => Pattern::Const { name: *name },

            sema::Pattern::Term(ty, id, pats) => {
                let pats = pats
                    .iter()
                    .map(|pat| Pattern::from_sema(env, binds, pat))
                    .collect();

                match &env.get_term(*id).kind {
                    TermKind::EnumVariant { .. } => Pattern::Variant {
                        id: *id,
                        single_case: env.is_single_constructor_enum(*ty),
                        pats,
                    },

                    TermKind::Decl { .. } => Pattern::Extractor { id: *id, pats },
                }
            }

            sema::Pattern::Wildcard(_) => Pattern::Wildcard,

            sema::Pattern::And(_, pats) => {
                let mut pats: Vec<Pattern> = pats
                    .iter()
                    .map(|pat| Pattern::from_sema(env, binds, pat))
                    .collect();

                if pats.len() == 1 {
                    pats.pop().unwrap()
                } else {
                    pats.sort_unstable();
                    Pattern::And { pats }
                }
            }
        }
    }

    /// True when this pattern is a wildcard.
    fn is_wildcard(&self) -> bool {
        match self {
            Pattern::Wildcard => true,
            _ => false,
        }
    }

    /// True when this pattern is an extractor.
    fn is_extractor(&self) -> Option<TermId> {
        match self {
            Pattern::Extractor { id, .. } => Some(*id),
            _ => None,
        }
    }

    /// True when this pattern is an and-pattern.
    fn is_and(&self) -> bool {
        match self {
            Pattern::And { .. } => true,
            _ => false,
        }
    }

    /// For `Variant` and `Extractor` this is the number of arguments, otherwise it is `1`.
    fn arity(&self) -> usize {
        match self {
            Pattern::Variant { pats, .. } => pats.len(),
            Pattern::Extractor { pats, .. } => pats.len(),
            _ => 1,
        }
    }

    /// Returns `true` for `Variant` or `Extractor`, as these are the two cases that can be
    /// expanded into sub-patterns.
    fn can_expand(&self) -> bool {
        match self {
            Pattern::Variant { .. } => true,
            Pattern::Extractor { .. } => true,
            _ => false,
        }
    }

    /// Returns `true` if this pattern could match one of the concrete patterns specified by
    /// `other`. NOTE: this is intentionally a shallow match, and any sub-patterns of `other` are
    /// intentionally ignored. We're only interested in the most top-level overlap between these
    /// patterns here.
    fn match_concrete(&self, other: &Pattern) -> bool {
        match (self, other) {
            // these are the cases where we know enough to say definitively yes or no
            (Pattern::Int { value: left }, Pattern::Int { value: right }) => left == right,
            (Pattern::Const { name: left }, Pattern::Const { name: right }) => left == right,
            (Pattern::Variant { id: left, .. }, Pattern::Variant { id: right, .. }) => {
                left == right
            }

            (Pattern::Extractor { id: left, .. }, Pattern::Extractor { id: right, .. }) => {
                left == right
            }

            (Pattern::And { pats }, _) => pats.iter().rev().any(|pat| pat.match_concrete(other)),

            _ => false,
        }
    }

    /// Assuming that this is an and-pattern, extract the sub-pattern that matches the template and
    /// remove it from `self`. This operation is used when specializing columns that contain
    /// and-patterns to another pattern, leaning on the assumption that the and-pattern matches if
    /// any of its sub-patterns also match.
    fn extract_matching(&mut self, template: &Pattern) -> Option<Pattern> {
        if let Pattern::And { pats } = self {
            for i in 0..pats.len() {
                if pats[i].match_concrete(template) {
                    let res = pats.remove(i);

                    // if the and has only a single element left, collapse it
                    if pats.len() == 1 {
                        *self = pats.remove(0);
                    }

                    return Some(res);
                }
            }
        }

        None
    }
}

/// A single row in the pattern matrix.
#[derive(Debug, Clone)]
struct Row {
    pats: Vec<Pattern>,
    rule: RuleId,
}

impl Row {
    /// Construct a rule from this rule id.
    fn from_rule(env: &Env, rule: RuleId) -> Row {
        if let sema::Pattern::Term(_, _, vars) = &env.get_rule(rule).lhs {
            let mut binds = Vec::new();
            Self {
                // NOTE: the patterns are reversed so that it's easier to manipulate the leading
                // column of the row by pushing/popping the pats vector.
                pats: vars
                    .iter()
                    .rev()
                    .map(|pat| Pattern::from_sema(env, &mut binds, pat))
                    .collect(),
                rule,
            }
        } else {
            panic!("Constructing a Row from a malformed rule")
        }
    }

    /// A row is empty when its pattern vector is empty.
    fn is_empty(&self) -> bool {
        self.pats.is_empty()
    }

    /// The pattern from the first column of this row.
    fn front(&self) -> &Pattern {
        assert!(!self.pats.is_empty());
        self.pats.last().unwrap()
    }

    /// A mutable reference to the pattern from the first column of this row.
    fn front_mut(&mut self) -> &mut Pattern {
        assert!(!self.pats.is_empty());
        self.pats.last_mut().unwrap()
    }

    /// Push a new pattern on the front of this row.
    fn push(&mut self, pat: Pattern) {
        self.pats.push(pat);
    }

    /// Pop the pattern from the front of the row.
    fn pop(&mut self) -> Pattern {
        assert!(!self.pats.is_empty());
        self.pats.pop().unwrap()
    }

    fn leading_pattern(&self) -> (bool, &Pattern) {
        assert!(!self.is_empty(), "leading_pattern called on an emtpy row");

        match self.front() {
            Pattern::And { pats } => (true, pats.first().unwrap()),
            pat => (false, pat),
        }
    }

    /// Specialize any leading and-patterns to this template.
    fn specialize_and_patterns(&mut self, template: &Pattern) {
        let mut pat = self.pop();
        if let Some(p) = pat.extract_matching(template) {
            self.push(pat);
            self.push(p);
        } else {
            self.push(Pattern::Wildcard);
            self.push(pat);
        }
    }

    /// Expand the leading pattern of this row according to the template.
    fn expand_leading(&mut self, env: &Env, template: &Pattern) {
        assert!(!self.front().is_and());

        let arity = template.arity();
        match self.pop() {
            Pattern::Variant { pats, .. } | Pattern::Extractor { pats, .. } => {
                self.pats.extend(pats.into_iter().rev());
            }

            Pattern::Wildcard => self
                .pats
                .extend(std::iter::repeat(Pattern::Wildcard).take(arity)),

            pat => panic!(
                "incorrect leading expansion:\nfound: {}\n expected: {}",
                WithEnv::new(env, &pat),
                WithEnv::new(env, template)
            ),
        }
    }

    /// Expand the leading pattern of this row according to the template.
    fn expand(&mut self, env: &Env, template: &Pattern) {
        if template.can_expand() {
            self.expand_leading(env, template);
        } else {
            self.pop();
        }
    }

    /// Returns true if it was possible to specialize this row to the pattern template provided.
    fn specialize(&mut self, env: &Env, template: &Pattern) -> bool {
        assert!(!self.is_empty());

        if self.front().is_wildcard() || self.front().match_concrete(template) {
            self.expand(env, template);
            return true;
        }

        // If this is an extractor, we already know that it doesn't match the template exactly, so
        // we'll treat it like a wildcard.
        if self.front().is_extractor().is_some() {
            *self.front_mut() = Pattern::Wildcard;
            self.expand(env, template);
            return true;
        }

        return false;
    }
}

/// A convenience struct for pretty-printing values that need an environment.
struct WithEnv<'env, T> {
    env: &'env Env<'env>,
    value: T,
}

impl<'env, T> WithEnv<'env, T> {
    /// Construct a new `WithEnv` for the given environment and value.
    fn new(env: &'env Env, value: T) -> Self {
        Self { env, value }
    }

    /// Construct a new `WithEnv` with the same environment, but a different value.
    fn with_value<U>(&self, value: U) -> WithEnv<'env, U> {
        WithEnv {
            env: self.env,
            value,
        }
    }
}

impl std::fmt::Display for WithEnv<'_, &Pattern> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.value {
            Pattern::Int { value } => write!(f, "{}", value),

            Pattern::Const { name } => write!(f, "${}", self.env.get_sym(*name)),

            Pattern::Variant { id, pats, .. } => {
                write!(f, "({}", self.env.get_sym(self.env.get_term(*id).name))?;
                for pat in pats {
                    write!(f, " {}", self.with_value(pat))?;
                }
                write!(f, ")")
            }

            Pattern::And { pats } => {
                write!(f, "(and")?;
                for pat in pats {
                    write!(f, " {}", self.with_value(pat))?;
                }
                write!(f, ")")
            }

            Pattern::Extractor { id, pats } => {
                write!(f, "({}", self.env.get_sym(self.env.get_term(*id).name))?;
                for pat in pats {
                    write!(f, " {}", self.with_value(pat))?;
                }
                write!(f, ")")
            }

            Pattern::Wildcard => write!(f, "_"),
        }
    }
}

impl std::fmt::Display for WithEnv<'_, &Row> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        for pat in self.value.pats.iter().rev() {
            write!(f, " {}", self.with_value(pat))?;
        }
        write!(f, " ] -> {}", self.value.rule.0)
    }
}
