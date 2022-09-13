//! Overlap detection for rules in ISLE.

use crate::error::{Error, Result, Source, Span};
use crate::sema::{
    self, ConstructorKind, Rule, RuleId, Sym, Term, TermEnv, TermId, TermKind, Type, TypeEnv,
    TypeId, VarId,
};

/// Check for overlap.
pub fn check(tyenv: &TypeEnv, termenv: &TermEnv) -> Result<()> {
    let env = Env::new(tyenv, termenv);
    let mut errors = Errors::new(termenv.rules.len());
    for term in termenv.terms.iter() {
        // The only isle declaration that currently produces overlap is constructors whose
        // definition is entirely in isle.
        if !env.is_internal_constructor(term.id) {
            continue;
        }

        check_overlap_groups(&mut errors, &env, term);
    }

    if !errors.is_empty() {
        let mut errors = errors.report(&env);
        return match errors.len() {
            1 => Err(errors.pop().unwrap()),
            _ => Err(Error::Errors(std::mem::take(&mut errors))),
        };
    }

    Ok(())
}

/// A node in the error graph.
struct Node {
    /// The number of other rules this node overlaps with.
    degree: usize,

    /// `true` entries where an edge exists to the node at that index.
    edges: Vec<bool>,
}

impl Node {
    /// Make a new `Node` in the error graph with the given number of total nodes.
    fn new(len: usize) -> Self {
        let mut edges = Vec::with_capacity(len);
        edges.resize(len, false);
        Self { degree: 0, edges }
    }

    /// Add an edge between this node and the other node.
    fn add_edge(&mut self, other: RuleId) {
        if self.edges[other.0] {
            return;
        }

        self.degree += 1;
        self.edges[other.0] = true;
    }

    /// Remove an edge between this node and another node.
    fn remove_edge(&mut self, other: RuleId) {
        if !self.edges[other.0] {
            return;
        }

        self.degree -= 1;
        self.edges[other.0] = false;
    }
}

/// A graph of all the rules in the isle source, with bi-directional edges between rules that are
/// discovered to have overlap problems.
struct Errors {
    /// Edges between rules indicating overlap. As the edges are not directed, the edges are
    /// normalized by ordering the rule ids.
    nodes: Vec<Node>,
}

impl Errors {
    /// Make a new `Errors` graph for collecting overlap information.
    fn new(len: usize) -> Self {
        let mut nodes = Vec::with_capacity(len);
        nodes.resize_with(len, || Node::new(len));
        Self { nodes }
    }

    /// True when there are no edges in the graph.
    fn is_empty(&self) -> bool {
        self.nodes.iter().all(|node| node.degree == 0)
    }

    /// Condense the overlap information down into individual errors.
    fn report(&mut self, env: &Env) -> Vec<Error> {
        let mut rules: Vec<usize> = self
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(id, node)| if node.degree == 0 { None } else { Some(id) })
            .collect();

        rules.sort_by_cached_key(|id| self.nodes[*id].degree);

        let mut errors = Vec::new();

        let get_info = |id| {
            let rule = env.get_rule(id);

            let src = env.get_source(rule.pos.file);
            let span = Span::new_single(rule.pos);
            (src, span)
        };

        // Work backwards through the ids to find the nodes with the largest conflict first.
        for id in rules.into_iter().rev() {
            let node = self.remove_edges(RuleId(id));
            if node.degree == 0 {
                continue;
            }

            // build the real error
            let mut rules = vec![get_info(RuleId(id))];

            rules.extend(
                node.edges
                    .into_iter()
                    .enumerate()
                    .filter_map(|(ix, present)| {
                        if present {
                            Some(get_info(RuleId(ix)))
                        } else {
                            None
                        }
                    }),
            );

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
        let mut node = Node::new(self.nodes.len());
        std::mem::swap(&mut self.nodes[id.0], &mut node);

        for (ix, other) in node.edges.iter().copied().enumerate() {
            if other {
                self.nodes[ix].remove_edge(id);
            }
        }

        node
    }

    /// Add a bidirectional edge between two rules in the graph.
    fn add_edge(&mut self, a: RuleId, b: RuleId) {
        // edges are undirected
        self.nodes[a.0].add_edge(b);
        self.nodes[b.0].add_edge(a);
    }

    /// Register all of the rules in the matrix as overlapping.
    fn overlap_error(&mut self, matrix: Matrix) {
        for (ix, rule) in matrix.rows.iter().enumerate() {
            for other in &matrix.rows[ix + 1..] {
                self.add_edge(rule.rule, other.rule);
            }
        }
    }
}

/// Check for overlapping rules within individual priority groups.
fn check_overlap_groups(errs: &mut Errors, env: &Env, term: &Term) {
    for matrix in Matrix::from_priority_groups(env, term.id) {
        check_overlap(errs, env, matrix);
    }
}

/// Check for overlapping rules within a single prioirty group.
fn check_overlap(errs: &mut Errors, env: &Env, mut matrix: Matrix) {
    if matrix.is_unique() {
        return;
    }

    matrix.normalize();
    if matrix.cols_empty() {
        errs.overlap_error(matrix);
        return;
    }

    let mut work = Vec::new();
    work.push(matrix);

    while let Some(mut matrix) = work.pop() {
        let pat = matrix.leading_pattern();
        let remainder = matrix.specialize(env, &pat);

        if !remainder.is_empty() && !remainder.is_unique() {
            if remainder.cols_empty() {
                errs.overlap_error(remainder);
            } else if !remainder.is_unique() {
                work.push(remainder);
            }
        }

        if !matrix.is_empty() && !matrix.is_unique() {
            if matrix.cols_empty() {
                errs.overlap_error(matrix);
            } else if !matrix.is_unique() {
                work.push(matrix);
            }
        }
    }
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
                        // TODO: explain why we inline equality constraint
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
}

/// A matrix whose rows consist rules that rewrite the same terms, and whose columns are the
/// positional arguments to those rules.
#[derive(Debug, Clone)]
struct Matrix {
    /// The rows of the rule matrix.
    rows: Vec<Row>,

    /// The term that this matrix represents.
    term: TermId,

    /// The priority of this group of rules matrix.
    prio: i64,
}

impl Matrix {
    /// Construct a new matrix with the given rows.
    fn new(term: TermId, prio: i64, rows: Vec<Row>) -> Self {
        Self { rows, prio, term }
    }

    /// Construct one matrix for each priority group defined for a given term.
    fn from_priority_groups(env: &Env, term: TermId) -> Vec<Self> {
        let mut matrices = Vec::new();

        let mut rules: Vec<(i64, RuleId)> = env
            .rules_for_term(term)
            .into_iter()
            .map(|id| (env.get_rule(id).prio.unwrap_or(0), id))
            .collect();

        if rules.is_empty() {
            return matrices;
        }

        rules.sort_by_key(|(prio, _)| *prio);

        let mut current = {
            let (prio, _) = rules.first().unwrap();
            matrices.push(Matrix::new(term, *prio, Vec::new()));
            matrices.last_mut().unwrap()
        };

        for (p, id) in rules {
            if p != current.prio {
                matrices.push(Matrix::new(term, p, Vec::new()));
                current = matrices.last_mut().unwrap();
            }
            current.rows.push(Row::from_rule(env, id));
        }

        matrices
    }

    /// Normalizing the matrix by removing leading columns that consist of only wildcards, and then
    /// sorting the remaining rows to put those with fallible leading patterns first.
    fn normalize(&mut self) {
        while !self.cols_empty() && self.rows.iter().all(|row| row.front().is_wildcard()) {
            self.drop_leading();
        }

        if !self.cols_empty() {
            self.rows.sort_unstable_by(|a, b| a.front().cmp(b.front()));
        }
    }

    /// Returns true if there are no rows in the matrix.
    fn is_empty(&self) -> bool {
        self.rows.first().is_none()
    }

    /// Returns true if there are no rows, or those that exist have no columns.
    fn cols_empty(&self) -> bool {
        self.rows.first().map_or(true, |row| row.is_empty())
    }

    /// Returns true if there is exactly one row.
    fn is_unique(&self) -> bool {
        self.rows.len() == 1
    }

    /// Specialize the matrix according to the pattern in the first column of the first row. This
    /// assumes that the matrix has already been normalized, and will return normalized results.
    fn specialize(&mut self, env: &Env, pat: &Pattern) -> Self {
        assert!(!self.cols_empty());

        let spec_extractor = pat.is_extractor();

        // we start by specializing and patterns, so that we don't have to consider them when
        // deciding which rows go to which matrix.
        self.specialize_and_patterns(&pat);

        // remove rows from self that we know couldn't possibly match this pattern
        let mut other = Matrix::new(self.term, self.prio, Vec::new());
        let mut i = 0;
        while i < self.rows.len() {
            let row = &mut self.rows[i];
            match row.front() {
                Pattern::Wildcard => {
                    // wildcards always match, and go into both matrices as a result
                    other.rows.push(row.clone());
                    i += 1;
                }

                Pattern::Extractor { id, .. } => {
                    // if we're specializing on this extractor then it shouldn't be duplicated over
                    // to the other matrix. otherwise, we copy the row over to the other matrix and
                    // rewrite this one to a wildcard to model the fact that we can't determine if
                    // the extractor will actually match.
                    if spec_extractor != Some(*id) {
                        other.rows.push(row.clone());
                        *row.front_mut() = Pattern::Wildcard;
                    }

                    i += 1;
                }

                // rows that don't match this pattern get moved to the other matrix
                col if !col.match_concrete(&pat) => {
                    other.rows.push(self.rows.swap_remove(i));
                    // note that we don't increment the index here
                }

                // and all other rows stay in this matrix
                _ => i += 1,
            }
        }

        if pat.can_expand() {
            self.expand_leading(env, &pat);
        } else {
            self.drop_leading();
        }

        self.normalize();
        other.normalize();
        other
    }

    /// Returns a copy of the first pattern for the first row of the matrix, as a candidate for
    /// specialization.
    fn leading_pattern(&self) -> Pattern {
        assert!(
            !self.cols_empty(),
            "leading_pattern called on a matrix with no patterns"
        );

        match self.rows[0].front() {
            Pattern::And { pats } => pats.first().unwrap().clone(),
            pat => pat.clone(),
        }
    }

    /// If there are any `and` patterns in the leading column, extract out the sub-pattern that
    /// matches `pat` and leave the rest in a fresh column. Insert wildcards for other columns
    /// that have no `and` patterns.
    fn specialize_and_patterns(&mut self, template: &Pattern) {
        if !self.rows.iter().any(|row| row.front().is_and()) {
            return;
        }

        for row in self.rows.iter_mut() {
            let mut pat = row.pop();
            if let Some(p) = pat.extract_matching(template) {
                row.push(pat);
                row.push(p);
            } else {
                row.push(Pattern::Wildcard);
                row.push(pat);
            }
        }
    }

    /// Expand the patterns of the leading column according to the given template. This function
    /// will only handle cases where the leading column is a variant, extractor, or wildcard, as
    /// all other cases could not reasonably introduce sub-patterns.
    fn expand_leading(&mut self, env: &Env, template: &Pattern) {
        let arity = template.arity();
        for row in self.rows.iter_mut() {
            let pat = row.pop();
            match pat {
                Pattern::Variant { pats, .. } | Pattern::Extractor { pats, .. }
                    if pat.match_concrete(template) =>
                {
                    row.pats.extend(pats.into_iter().rev())
                }

                Pattern::Wildcard => row
                    .pats
                    .extend(std::iter::repeat(Pattern::Wildcard).take(arity)),

                _ => panic!(
                    "incorrect leading expansion:\nfound: {}\n expected: {}",
                    WithEnv::new(env, &pat),
                    WithEnv::new(env, template)
                ),
            }
        }
    }

    // Drop leading column from the matrix.
    fn drop_leading(&mut self) {
        for row in self.rows.iter_mut() {
            row.pop();
        }
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

impl std::fmt::Display for WithEnv<'_, &Matrix> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.value.rows.is_empty() {
            return writeln!(f, "<empty>");
        }

        let mut lens = vec![0; self.value.rows.first().unwrap().pats.len()];
        let mut rows: Vec<(RuleId, Vec<String>)> = Vec::with_capacity(self.value.rows.len());

        for row in self.value.rows.iter() {
            rows.push((
                row.rule,
                row.pats
                    .iter()
                    .rev()
                    .enumerate()
                    .map(|(col, pat)| {
                        let str = format!("{}", self.with_value(pat));
                        lens[col] = lens[col].max(str.len());
                        str
                    })
                    .collect(),
            ));
        }

        for (rule, row) in rows.into_iter() {
            write!(f, "[")?;
            let mut sep = "";
            for (col, width) in row.into_iter().zip(lens.iter()) {
                write!(f, "{} {:width$} ", sep, col)?;
                sep = "|";
            }
            writeln!(f, "] = {}", rule.0)?
        }

        Ok(())
    }
}
