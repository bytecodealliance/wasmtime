//! Generate Rust code from a series of Sequences.

use crate::error::Error;
use crate::ir::{lower_rule, ExprInst, ExprSequence, InstId, PatternInst, PatternSequence, Value};
use crate::sema::{RuleId, TermEnv, TermId, TermKind, Type, TypeEnv, TypeId};
use std::collections::{HashMap, HashSet};
use std::fmt::Write;

/// One "input symbol" for the decision tree that handles matching on
/// a term. Each symbol represents one step: we either run a match op,
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
/// Instead, we need a data structure that can associate matching
/// inputs *with priorities* to outputs, and provide us with a
/// decision tree as output.
///
/// Why a tree and not a fully general FSM?  Because we're compiling
/// to a structured language, Rust, and states become *program points*
/// rather than *data*, we cannot easily support a DAG structure. In
/// other words, we are not producing a FSM that we can interpret at
/// runtime; rather we are compiling code in which each state
/// corresponds to a sequence of statements and control-flow that
/// branches to a next state, we naturally need nesting; we cannot
/// codegen arbitrary state transitions in an efficient manner. We
/// could support a limited form of DAG that reifies "diamonds" (two
/// alternate paths that reconverge), but supporting this in a way
/// that lets the output refer to values from either side is very
/// complex (we need to invent phi-nodes), and the cases where we want
/// to do this rather than invoke a sub-term (that is compiled to a
/// separate function) are rare. Finally, note that one reason to
/// deduplicate nodes and turn a tree back into a DAG --
/// "output-suffix sharing" as some other instruction-rewriter
/// engines, such as Peepmatic, do -- is not done. However,
/// "output-prefix sharing" is more important to deduplicate code and
/// we do do this.)
///
/// We prepare for codegen by building a "prioritized trie", where the
/// trie associates input strings with priorities to output values.
/// Each input string is a sequence of match operators followed by an
/// "end of match" token, and each output is a sequence of ops that
/// build the output expression. Each input-output mapping is
/// associated with a priority. The goal of the trie is to generate a
/// decision-tree procedure that lets us execute match ops in a
/// deterministic way, eventually landing at a state that corresponds
/// to the highest-priority matching rule and can produce the output.
///
/// To build this trie, we construct nodes with edges to child nodes;
/// each edge consists of (i) one input token (a `PatternInst` or
/// EOM), and (ii) the minimum and maximum priorities of rules along
/// this edge. In a way this resembles an interval tree, though the
/// intervals of children need not be disjoint.
///
/// To add a rule to this trie, we perform the usual trie-insertion
/// logic, creating edges and subnodes where necessary, and updating
/// the priority-range of each edge that we traverse to include the
/// priority of the inserted rule.
///
/// However, we need to be a little bit careful, because with only
/// priority ranges in place and the potential for overlap, we have
/// something that resembles an NFA. For example, consider the case
/// where we reach a node in the trie and have two edges with two
/// match ops, one corresponding to a rule with priority 10, and the
/// other corresponding to two rules, with priorities 20 and 0. The
/// final match could lie along *either* path, so we have to traverse
/// both.
///
/// So, to avoid this, we perform a sort of NFA-to-DFA conversion "on
/// the fly" as we insert nodes by duplicating subtrees. At any node,
/// when inserting with a priority P and when outgoing edges lie in a
/// range [P_lo, P_hi] such that P >= P_lo and P <= P_hi, we
/// "priority-split the edges" at priority P.
///
/// To priority-split the edges in a node at priority P:
///
/// - For each out-edge with priority [P_lo, P_hi] s.g. P \in [P_lo,
///   P_hi], and token T:
///   - Trim the subnode at P, yielding children C_lo and C_hi.
///   - Both children must be non-empty (have at least one leaf)
///     because the original node must have had a leaf at P_lo
///     and a leaf at P_hi.
///   - Replace the one edge with two edges, one for each child, with
///     the original match op, and with ranges calculated according to
///     the trimmed children.
///
/// To trim a node into range [P_lo, P_hi]:
///
/// - For a decision node:
///   - If any edges have a range outside the bounds of the trimming
///     range, trim the bounds of the edge, and trim the subtree under the
///     edge into the trimmed edge's range. If the subtree is trimmed
///     to `None`, remove the edge.
///   - If all edges are removed, the decision node becomes `None`.
/// - For a leaf node:
///   - If the priority is outside the range, the node becomes `None`.
///
/// As we descend a path to insert a leaf node, we (i) priority-split
/// if any edges' priority ranges overlap the insertion priority
/// range, and (ii) expand priority ranges on edges to include the new
/// leaf node's priority.
///
/// As long as we do this, we ensure the two key priority-trie
/// invariants:
///
/// 1. At a given node, no two edges exist with priority ranges R_1,
///    R_2 such that R_1 ∩ R_2 ≠ ∅, unless R_1 and R_2 are unit ranges
///    ([x, x]) and are on edges with different match-ops.
/// 2. Along the path from the root to any leaf node with priority P,
///    each edge has a priority range R such that P ∈ R.
///
/// Note that this means that multiple edges with a single match-op
/// may exist, with different priorities.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum TrieSymbol {
    Match { op: PatternInst },
    EndOfMatch,
}

impl TrieSymbol {
    fn is_eom(&self) -> bool {
        match self {
            TrieSymbol::EndOfMatch => true,
            _ => false,
        }
    }
}

type Prio = i64;

#[derive(Clone, Copy, Debug)]
struct PrioRange(Prio, Prio);

impl PrioRange {
    fn contains(&self, prio: Prio) -> bool {
        prio >= self.0 && prio <= self.1
    }

    fn is_unit(&self) -> bool {
        self.0 == self.1
    }

    fn overlaps(&self, other: PrioRange) -> bool {
        // This can be derived via DeMorgan: !(self.begin > other.end
        // OR other.begin > self.end).
        self.0 <= other.1 && other.0 <= self.1
    }

    fn intersect(&self, other: PrioRange) -> PrioRange {
        PrioRange(
            std::cmp::max(self.0, other.0),
            std::cmp::min(self.1, other.1),
        )
    }

    fn union(&self, other: PrioRange) -> PrioRange {
        PrioRange(
            std::cmp::min(self.0, other.0),
            std::cmp::max(self.1, other.1),
        )
    }

    fn split_at(&self, prio: Prio) -> (PrioRange, PrioRange) {
        assert!(self.contains(prio));
        assert!(!self.is_unit());
        if prio == self.0 {
            (PrioRange(self.0, self.0), PrioRange(self.0 + 1, self.1))
        } else {
            (PrioRange(self.0, prio - 1), PrioRange(prio, self.1))
        }
    }
}

#[derive(Clone, Debug)]
struct TrieEdge {
    range: PrioRange,
    symbol: TrieSymbol,
    node: TrieNode,
}

#[derive(Clone, Debug)]
enum TrieNode {
    Decision { edges: Vec<TrieEdge> },
    Leaf { prio: Prio, output: ExprSequence },
    Empty,
}

impl TrieNode {
    fn is_empty(&self) -> bool {
        match self {
            &TrieNode::Empty => true,
            _ => false,
        }
    }

    fn insert(
        &mut self,
        prio: Prio,
        mut input: impl Iterator<Item = TrieSymbol>,
        output: ExprSequence,
    ) -> bool {
        // Take one input symbol. There must be *at least* one, EOM if
        // nothing else.
        let op = input
            .next()
            .expect("Cannot insert into trie with empty input sequence");
        let is_last = op.is_eom();

        // If we are empty, turn into a decision node.
        if self.is_empty() {
            *self = TrieNode::Decision { edges: vec![] };
        }

        // We must be a decision node.
        let edges = match self {
            &mut TrieNode::Decision { ref mut edges } => edges,
            _ => panic!("insert on leaf node!"),
        };

        // Do we need to split?
        let needs_split = edges
            .iter()
            .any(|edge| edge.range.contains(prio) && !edge.range.is_unit());

        // If so, pass over all edges/subnodes and split each.
        if needs_split {
            let mut new_edges = vec![];
            for edge in std::mem::take(edges) {
                if !edge.range.contains(prio) || edge.range.is_unit() {
                    new_edges.push(edge);
                    continue;
                }

                let (lo_range, hi_range) = edge.range.split_at(prio);
                let lo = edge.node.trim(lo_range);
                let hi = edge.node.trim(hi_range);
                if let Some((node, range)) = lo {
                    new_edges.push(TrieEdge {
                        range,
                        symbol: edge.symbol.clone(),
                        node,
                    });
                }
                if let Some((node, range)) = hi {
                    new_edges.push(TrieEdge {
                        range,
                        symbol: edge.symbol,
                        node,
                    });
                }
            }
            *edges = new_edges;
        }

        // Now find or insert the appropriate edge.
        let mut edge: Option<usize> = None;
        for i in 0..edges.len() {
            if edges[i].range.contains(prio) && edges[i].symbol == op {
                edge = Some(i);
                break;
            }
            if prio > edges[i].range.1 {
                edges.insert(
                    i,
                    TrieEdge {
                        range: PrioRange(prio, prio),
                        symbol: op.clone(),
                        node: TrieNode::Empty,
                    },
                );
                edge = Some(i);
                break;
            }
        }
        let edge = edge.unwrap_or_else(|| {
            edges.push(TrieEdge {
                range: PrioRange(prio, prio),
                symbol: op.clone(),
                node: TrieNode::Empty,
            });
            edges.len() - 1
        });
        let edge = &mut edges[edge];

        if is_last {
            if !edge.node.is_empty() {
                // If a leaf node already exists at an overlapping
                // prio for this op, there are two competing rules, so
                // we can't insert this one.
                return false;
            }
            edge.node = TrieNode::Leaf { prio, output };
            true
        } else {
            edge.node.insert(prio, input, output)
        }
    }

    fn trim(&self, range: PrioRange) -> Option<(TrieNode, PrioRange)> {
        match self {
            &TrieNode::Empty => None,
            &TrieNode::Leaf { prio, ref output } => {
                if range.contains(prio) {
                    Some((
                        TrieNode::Leaf {
                            prio,
                            output: output.clone(),
                        },
                        PrioRange(prio, prio),
                    ))
                } else {
                    None
                }
            }
            &TrieNode::Decision { ref edges } => {
                let edges = edges
                    .iter()
                    .filter_map(|edge| {
                        if !edge.range.overlaps(range) {
                            None
                        } else {
                            let range = range.intersect(edge.range);
                            if let Some((node, range)) = edge.node.trim(range) {
                                Some(TrieEdge {
                                    range,
                                    symbol: edge.symbol.clone(),
                                    node,
                                })
                            } else {
                                None
                            }
                        }
                    })
                    .collect::<Vec<_>>();

                if edges.is_empty() {
                    None
                } else {
                    let range = edges
                        .iter()
                        .map(|edge| edge.range)
                        .reduce(|a, b| a.union(b))
                        .expect("reduce on non-empty vec must not return None");
                    Some((TrieNode::Decision { edges }, range))
                }
            }
        }
    }
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
#[derive(Debug)]
struct TermFunctionBuilder {
    root_term: TermId,
    trie: TrieNode,
}

impl TermFunctionBuilder {
    fn new(root_term: TermId) -> Self {
        TermFunctionBuilder {
            root_term,
            trie: TrieNode::Empty,
        }
    }

    fn add_rule(&mut self, prio: Prio, pattern_seq: PatternSequence, expr_seq: ExprSequence) {
        let symbols = pattern_seq
            .insts
            .into_iter()
            .map(|op| TrieSymbol::Match { op })
            .chain(std::iter::once(TrieSymbol::EndOfMatch));
        self.trie.insert(prio, symbols, expr_seq);
    }
}

#[derive(Debug)]
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
            let prio = self.termenv.rules[rule.index()].prio.unwrap_or(0);

            if let Some((pattern, expr, lhs_root)) = lower_rule(
                self.typeenv,
                self.termenv,
                rule,
                /* forward_dir = */ true,
            ) {
                log::trace!(
                    "build:\n- rule {:?}\n- fwd pattern {:?}\n- fwd expr {:?}",
                    self.termenv.rules[rule.index()],
                    pattern,
                    expr
                );
                self.builders_by_input
                    .entry(lhs_root)
                    .or_insert_with(|| TermFunctionBuilder::new(lhs_root))
                    .add_rule(prio, pattern.clone(), expr.clone());
            }

            if let Some((pattern, expr, rhs_root)) = lower_rule(
                self.typeenv,
                self.termenv,
                rule,
                /* forward_dir = */ false,
            ) {
                log::trace!(
                    "build:\n- rule {:?}\n- rev pattern {:?}\n- rev expr {:?}",
                    self.termenv.rules[rule.index()],
                    pattern,
                    expr
                );
                self.builders_by_output
                    .entry(rhs_root)
                    .or_insert_with(|| TermFunctionBuilder::new(rhs_root))
                    .add_rule(prio, pattern, expr);
            }
        }
    }

    fn finalize(self) -> (HashMap<TermId, TrieNode>, HashMap<TermId, TrieNode>) {
        let functions_by_input = self
            .builders_by_input
            .into_iter()
            .map(|(term, builder)| (term, builder.trie))
            .collect::<HashMap<_, _>>();
        let functions_by_output = self
            .builders_by_output
            .into_iter()
            .map(|(term, builder)| (term, builder.trie))
            .collect::<HashMap<_, _>>();
        (functions_by_input, functions_by_output)
    }
}

#[derive(Clone, Debug)]
pub struct Codegen<'a> {
    typeenv: &'a TypeEnv,
    termenv: &'a TermEnv,
    functions_by_input: HashMap<TermId, TrieNode>,
    functions_by_output: HashMap<TermId, TrieNode>,
}

#[derive(Clone, Debug, Default)]
struct BodyContext {
    borrowed_values: HashSet<Value>,
    expected_return_vals: usize,
    tuple_return: bool,
}

impl<'a> Codegen<'a> {
    pub fn compile(typeenv: &'a TypeEnv, termenv: &'a TermEnv) -> Result<Codegen<'a>, Error> {
        let mut builder = TermFunctionsBuilder::new(typeenv, termenv);
        builder.build();
        log::trace!("builder: {:?}", builder);
        let (functions_by_input, functions_by_output) = builder.finalize();
        Ok(Codegen {
            typeenv,
            termenv,
            functions_by_input,
            functions_by_output,
        })
    }

    pub fn generate_rust(&self) -> Result<String, Error> {
        let mut code = String::new();

        self.generate_header(&mut code)?;
        self.generate_internal_types(&mut code)?;
        self.generate_internal_term_constructors(&mut code)?;
        self.generate_internal_term_extractors(&mut code)?;

        Ok(code)
    }

    fn generate_header(&self, code: &mut dyn Write) -> Result<(), Error> {
        writeln!(code, "// GENERATED BY ISLE. DO NOT EDIT!")?;
        writeln!(code, "//")?;
        writeln!(
            code,
            "// Generated automatically from the instruction-selection DSL code in:",
        )?;
        for file in &self.typeenv.filenames {
            writeln!(code, "// - {}", file)?;
        }
        writeln!(
            code,
            "\nuse super::*;  // Pulls in all external types and ctors/etors"
        )?;

        Ok(())
    }

    fn generate_internal_types(&self, code: &mut dyn Write) -> Result<(), Error> {
        for ty in &self.typeenv.types {
            match ty {
                &Type::Enum {
                    name,
                    is_extern,
                    ref variants,
                    pos,
                    ..
                } if !is_extern => {
                    let name = &self.typeenv.syms[name.index()];
                    writeln!(
                        code,
                        "\n/// Internal type {}: defined at {}.",
                        name,
                        pos.pretty_print_line(&self.typeenv.filenames[..])
                    )?;
                    writeln!(code, "#[derive(Clone, Debug)]")?;
                    writeln!(code, "pub enum {} {{", name)?;
                    for variant in variants {
                        let name = &self.typeenv.syms[variant.name.index()];
                        writeln!(code, "    {} {{", name)?;
                        for field in &variant.fields {
                            let name = &self.typeenv.syms[field.name.index()];
                            let ty_name = self.typeenv.types[field.ty.index()].name(&self.typeenv);
                            writeln!(code, "        {}: {},", name, ty_name)?;
                        }
                        writeln!(code, "    }},")?;
                    }
                    writeln!(code, "}}")?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn constructor_name(&self, term: TermId) -> String {
        let termdata = &self.termenv.terms[term.index()];
        match &termdata.kind {
            &TermKind::EnumVariant { .. } => panic!("using enum variant as constructor"),
            &TermKind::Regular {
                constructor: Some(sym),
                ..
            } => self.typeenv.syms[sym.index()].clone(),
            &TermKind::Regular {
                constructor: None, ..
            } => {
                format!("constructor_{}", self.typeenv.syms[termdata.name.index()])
            }
        }
    }

    fn extractor_name_and_infallible(&self, term: TermId) -> (String, bool) {
        let termdata = &self.termenv.terms[term.index()];
        match &termdata.kind {
            &TermKind::EnumVariant { .. } => panic!("using enum variant as extractor"),
            &TermKind::Regular {
                extractor: Some((sym, infallible)),
                ..
            } => (self.typeenv.syms[sym.index()].clone(), infallible),
            &TermKind::Regular {
                extractor: None, ..
            } => (
                format!("extractor_{}", self.typeenv.syms[termdata.name.index()]),
                false,
            ),
        }
    }

    fn type_name(&self, typeid: TypeId, by_ref: Option<&str>) -> String {
        match &self.typeenv.types[typeid.index()] {
            &Type::Primitive(_, sym) => self.typeenv.syms[sym.index()].clone(),
            &Type::Enum { name, .. } => {
                let r = by_ref.unwrap_or("");
                format!("{}{}", r, self.typeenv.syms[name.index()])
            }
        }
    }

    fn value_name(&self, value: &Value) -> String {
        match value {
            &Value::Pattern { inst, output } => format!("pattern{}_{}", inst.index(), output),
            &Value::Expr { inst, output } => format!("expr{}_{}", inst.index(), output),
        }
    }

    fn value_by_ref(&self, value: &Value, ctx: &BodyContext) -> String {
        let raw_name = self.value_name(value);
        let name_is_ref = ctx.borrowed_values.contains(value);
        if name_is_ref {
            raw_name
        } else {
            format!("&{}", raw_name)
        }
    }

    fn value_by_val(&self, value: &Value, ctx: &BodyContext) -> String {
        let raw_name = self.value_name(value);
        let name_is_ref = ctx.borrowed_values.contains(value);
        if name_is_ref {
            format!("{}.clone()", raw_name)
        } else {
            raw_name
        }
    }

    fn define_val(&self, value: &Value, ctx: &mut BodyContext, is_ref: bool) {
        if is_ref {
            ctx.borrowed_values.insert(value.clone());
        }
    }

    fn generate_internal_term_constructors(&self, code: &mut dyn Write) -> Result<(), Error> {
        for (&termid, trie) in &self.functions_by_input {
            let termdata = &self.termenv.terms[termid.index()];

            // Skip terms that are enum variants or that have external constructors.
            match &termdata.kind {
                &TermKind::EnumVariant { .. } => continue,
                &TermKind::Regular { constructor, .. } if constructor.is_some() => continue,
                _ => {}
            }

            // Get the name of the term and build up the signature.
            let func_name = self.constructor_name(termid);
            let args = termdata
                .arg_tys
                .iter()
                .enumerate()
                .map(|(i, &arg_ty)| {
                    format!(
                        "arg{}: {}",
                        i,
                        self.type_name(arg_ty, /* by_ref = */ Some("&"))
                    )
                })
                .collect::<Vec<_>>();
            writeln!(
                code,
                "\n// Generated as internal constructor for term {}.",
                self.typeenv.syms[termdata.name.index()],
            )?;
            writeln!(
                code,
                "fn {}<C>(ctx: &mut C, {}) -> Option<{}> {{",
                func_name,
                args.join(", "),
                self.type_name(termdata.ret_ty, /* by_ref = */ None)
            )?;

            let mut body_ctx: BodyContext = Default::default();
            body_ctx.expected_return_vals = 1;
            self.generate_body(code, /* depth = */ 0, trie, "    ", &mut body_ctx)?;

            writeln!(code, "}}")?;
        }

        Ok(())
    }

    fn generate_internal_term_extractors(&self, code: &mut dyn Write) -> Result<(), Error> {
        for (&termid, trie) in &self.functions_by_output {
            let termdata = &self.termenv.terms[termid.index()];

            // Skip terms that are enum variants or that have external extractors.
            match &termdata.kind {
                &TermKind::EnumVariant { .. } => continue,
                &TermKind::Regular { extractor, .. } if extractor.is_some() => continue,
                _ => {}
            }

            // Get the name of the term and build up the signature.
            let (func_name, _) = self.extractor_name_and_infallible(termid);
            let arg_is_prim = match &self.typeenv.types[termdata.ret_ty.index()] {
                &Type::Primitive(..) => true,
                _ => false,
            };
            let arg = format!(
                "arg0: {}",
                self.type_name(
                    termdata.ret_ty,
                    /* by_ref = */ if arg_is_prim { None } else { Some("&") }
                ),
            );
            let ret_tuple_tys = termdata
                .arg_tys
                .iter()
                .map(|ty| {
                    self.type_name(*ty, /* by_ref = */ None)
                })
                .collect::<Vec<_>>();

            writeln!(
                code,
                "\n// Generated as internal extractor for term {}.",
                self.typeenv.syms[termdata.name.index()],
            )?;
            writeln!(
                code,
                "fn {}<C>(ctx: &mut C, {}) -> Option<({},)> {{",
                func_name,
                arg,
                ret_tuple_tys.join(", "),
            )?;

            let mut body_ctx: BodyContext = Default::default();
            body_ctx.expected_return_vals = ret_tuple_tys.len();
            body_ctx.tuple_return = true;
            self.generate_body(code, /* depth = */ 0, trie, "    ", &mut body_ctx)?;
            writeln!(code, "}}")?;
        }

        Ok(())
    }

    fn generate_expr_inst(
        &self,
        code: &mut dyn Write,
        id: InstId,
        inst: &ExprInst,
        indent: &str,
        ctx: &mut BodyContext,
        returns: &mut Vec<(usize, String)>,
    ) -> Result<(), Error> {
        match inst {
            &ExprInst::ConstInt { ty, val } => {
                let value = Value::Expr {
                    inst: id,
                    output: 0,
                };
                let name = self.value_name(&value);
                let ty = self.type_name(ty, /* by_ref = */ None);
                self.define_val(&value, ctx, /* is_ref = */ false);
                writeln!(code, "{}let {}: {} = {};", indent, name, ty, val)?;
            }
            &ExprInst::CreateVariant {
                ref inputs,
                ty,
                variant,
            } => {
                let variantinfo = match &self.typeenv.types[ty.index()] {
                    &Type::Primitive(..) => panic!("CreateVariant with primitive type"),
                    &Type::Enum { ref variants, .. } => &variants[variant.index()],
                };
                let mut input_fields = vec![];
                for ((input_value, _), field) in inputs.iter().zip(variantinfo.fields.iter()) {
                    let field_name = &self.typeenv.syms[field.name.index()];
                    let value_expr = self.value_by_val(input_value, ctx);
                    input_fields.push(format!("{}: {}", field_name, value_expr));
                }

                let output = Value::Expr {
                    inst: id,
                    output: 0,
                };
                let outputname = self.value_name(&output);
                let full_variant_name = format!(
                    "{}::{}",
                    self.type_name(ty, None),
                    self.typeenv.syms[variantinfo.name.index()]
                );
                writeln!(
                    code,
                    "{}let {} = {} {{",
                    indent, outputname, full_variant_name
                )?;
                for input_field in input_fields {
                    writeln!(code, "{}    {},", indent, input_field)?;
                }
                writeln!(code, "{}}};", indent)?;
                self.define_val(&output, ctx, /* is_ref = */ false);
            }
            &ExprInst::Construct {
                ref inputs, term, ..
            } => {
                let mut input_exprs = vec![];
                for (input_value, _) in inputs {
                    let value_expr = self.value_by_val(input_value, ctx);
                    input_exprs.push(value_expr);
                }

                let output = Value::Expr {
                    inst: id,
                    output: 0,
                };
                let outputname = self.value_name(&output);
                let ctor_name = self.constructor_name(term);
                writeln!(
                    code,
                    "{}let {} = {}(ctx, {});",
                    indent,
                    outputname,
                    ctor_name,
                    input_exprs.join(", "),
                )?;
                self.define_val(&output, ctx, /* is_ref = */ false);
            }
            &ExprInst::Return {
                index, ref value, ..
            } => {
                let value_expr = self.value_by_val(value, ctx);
                returns.push((index, value_expr));
            }
        }

        Ok(())
    }

    /// Returns a `bool` indicating whether this pattern inst is
    /// infallible.
    fn generate_pattern_inst(
        &self,
        code: &mut dyn Write,
        id: InstId,
        inst: &PatternInst,
        indent: &str,
        ctx: &mut BodyContext,
    ) -> Result<bool, Error> {
        match inst {
            &PatternInst::Arg { index, ty } => {
                let output = Value::Pattern {
                    inst: id,
                    output: 0,
                };
                let outputname = self.value_name(&output);
                let is_ref = match &self.typeenv.types[ty.index()] {
                    &Type::Primitive(..) => false,
                    _ => true,
                };
                writeln!(code, "{}let {} = arg{};", indent, outputname, index)?;
                writeln!(code, "{}{{", indent)?;
                self.define_val(
                    &Value::Pattern {
                        inst: id,
                        output: 0,
                    },
                    ctx,
                    is_ref,
                );
                Ok(true)
            }
            &PatternInst::MatchEqual { ref a, ref b, .. } => {
                let a = self.value_by_ref(a, ctx);
                let b = self.value_by_ref(b, ctx);
                writeln!(code, "{}if {} == {} {{", indent, a, b)?;
                Ok(false)
            }
            &PatternInst::MatchInt {
                ref input, int_val, ..
            } => {
                let input = self.value_by_val(input, ctx);
                writeln!(code, "{}if {} == {} {{", indent, input, int_val)?;
                Ok(false)
            }
            &PatternInst::MatchVariant {
                ref input,
                input_ty,
                variant,
                ref arg_tys,
            } => {
                let input = self.value_by_ref(input, ctx);
                let variants = match &self.typeenv.types[input_ty.index()] {
                    &Type::Primitive(..) => panic!("primitive type input to MatchVariant"),
                    &Type::Enum { ref variants, .. } => variants,
                };
                let ty_name = self.type_name(input_ty, /* is_ref = */ Some("&"));
                let variant = &variants[variant.index()];
                let variantname = &self.typeenv.syms[variant.name.index()];
                let args = arg_tys
                    .iter()
                    .zip(variant.fields.iter())
                    .enumerate()
                    .map(|(i, (ty, field))| {
                        let value = Value::Pattern {
                            inst: id,
                            output: i,
                        };
                        let valuename = self.value_name(&value);
                        let fieldname = &self.typeenv.syms[field.name.index()];
                        match &self.typeenv.types[ty.index()] {
                            &Type::Primitive(..) => {
                                self.define_val(&value, ctx, /* is_ref = */ false);
                                format!("{}: {}", fieldname, valuename)
                            }
                            &Type::Enum { .. } => {
                                self.define_val(&value, ctx, /* is_ref = */ true);
                                format!("{}: ref {}", fieldname, valuename)
                            }
                        }
                    })
                    .collect::<Vec<_>>();
                writeln!(
                    code,
                    "{}if let {}::{} {{ {} }} = {} {{",
                    indent,
                    ty_name,
                    variantname,
                    args.join(", "),
                    input
                )?;
                Ok(false)
            }
            &PatternInst::Extract {
                ref input,
                input_ty,
                ref arg_tys,
                term,
                ..
            } => {
                let input_ty_prim = match &self.typeenv.types[input_ty.index()] {
                    &Type::Primitive(..) => true,
                    _ => false,
                };
                let input = if input_ty_prim {
                    self.value_by_val(input, ctx)
                } else {
                    self.value_by_ref(input, ctx)
                };
                let (etor_name, infallible) = self.extractor_name_and_infallible(term);

                let args = arg_tys
                    .iter()
                    .enumerate()
                    .map(|(i, _ty)| {
                        let value = Value::Pattern {
                            inst: id,
                            output: i,
                        };
                        self.define_val(&value, ctx, /* is_ref = */ false);
                        self.value_name(&value)
                    })
                    .collect::<Vec<_>>();

                if infallible {
                    writeln!(
                        code,
                        "{}let Some(({},)) = {}(ctx, {});",
                        indent,
                        args.join(", "),
                        etor_name,
                        input
                    )?;
                    writeln!(code, "{}{{", indent)?;
                } else {
                    writeln!(
                        code,
                        "{}if let Some(({},)) = {}(ctx, {}) {{",
                        indent,
                        args.join(", "),
                        etor_name,
                        input
                    )?;
                }
                Ok(infallible)
            }
        }
    }

    fn generate_body(
        &self,
        code: &mut dyn Write,
        depth: usize,
        trie: &TrieNode,
        indent: &str,
        ctx: &mut BodyContext,
    ) -> Result<bool, Error> {
        log::trace!("generate_body: trie {:?}", trie);
        let mut returned = false;
        match trie {
            &TrieNode::Empty => {}

            &TrieNode::Leaf { ref output, .. } => {
                writeln!(
                    code,
                    "{}// Rule at {}.",
                    indent,
                    output.pos.pretty_print_line(&self.typeenv.filenames[..])
                )?;
                // If this is a leaf node, generate the ExprSequence and return.
                let mut returns = vec![];
                for (id, inst) in output.insts.iter().enumerate() {
                    let id = InstId(id);
                    self.generate_expr_inst(code, id, inst, indent, ctx, &mut returns)?;
                }

                assert_eq!(returns.len(), ctx.expected_return_vals);
                returns.sort_by_key(|(index, _)| *index);
                if ctx.tuple_return {
                    let return_values = returns
                        .into_iter()
                        .map(|(_, expr)| expr)
                        .collect::<Vec<_>>()
                        .join(", ");
                    writeln!(code, "{}return Some(({},));", indent, return_values)?;
                } else {
                    writeln!(code, "{}return Some({});", indent, returns[0].1)?;
                }

                returned = true;
            }

            &TrieNode::Decision { ref edges } => {
                let subindent = format!("{}    ", indent);
                // if this is a decision node, generate each match op
                // in turn (in priority order).
                for &TrieEdge {
                    ref symbol,
                    ref node,
                    ..
                } in edges
                {
                    match symbol {
                        &TrieSymbol::EndOfMatch => {
                            returned = self.generate_body(code, depth + 1, node, indent, ctx)?;
                        }
                        &TrieSymbol::Match { ref op } => {
                            let id = InstId(depth);
                            let infallible =
                                self.generate_pattern_inst(code, id, op, indent, ctx)?;
                            let sub_returned =
                                self.generate_body(code, depth + 1, node, &subindent, ctx)?;
                            writeln!(code, "{}}}", indent)?;
                            if infallible && sub_returned {
                                returned = true;
                                break;
                            }
                        }
                    }
                }
            }
        }

        if !returned {
            writeln!(code, "{}return None;", indent)?;
        }
        Ok(returned)
    }
}
