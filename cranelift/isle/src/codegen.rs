//! Generate Rust code from a series of Sequences.

use crate::ir::{lower_rule, ExprInst, ExprSequence, InstId, PatternInst, PatternSequence, Value};
use crate::sema::{RuleId, TermEnv, TermId, Type, TypeEnv, TypeId, Variant};
use crate::{error::Error, sema::ExternalSig};
use std::collections::{HashMap, HashSet};
use std::fmt::Write;

/// One "input symbol" for the decision tree that handles matching on
/// a term. Each symbol represents one step: we either run a match op,
/// or we finish the match.
///
/// Note that in the original Peepmatic scheme, the input-symbol to
/// the FSM was specified slightly differently. The automaton
/// responded to alphabet symbols that corresponded only to match
/// results, and the "extra state" was used at each automaton node to
/// represent the op to run next. This extra state differentiated
/// nodes that would otherwise be merged together by
/// deduplication. That scheme works well enough, but the "extra
/// state" is slightly confusing and diverges slightly from a pure
/// automaton.
///
/// Instead, here, we imagine that the user of the automaton/trie can
/// query the possible transition edges out of the current state. Each
/// of these edges corresponds to one possible match op to run. After
/// running a match op, we reach a new state corresponding to
/// successful matches up to that point.
///
/// However, it's a bit more subtle than this. Consider the
/// prioritization problem. We want to give the DSL user the ability
/// to change the order in which rules apply, for example to have a
/// tier of "fallback rules" that apply only if more custom rules do
/// not match.
///
/// A somewhat simplistic answer to this problem is "more specific
/// rule wins". However, this implies the existence of a total
/// ordering of linearized match sequences that may not fully capture
/// the intuitive meaning of "more specific". Consider three left-hand
/// sides:
///
/// - (A _ _)
/// - (A (B _) _)
/// - (A _ (B _))
///
/// Intuitively, the first is the least specific. Given the input `(A
/// (B 1) (B 2))`, we can say for sure that the first should not be
/// chosen, because either the second or third would match "more" of
/// the input tree. But which of the second and third should be
/// chosen? A "lexicographic ordering" rule would say that we sort
/// left-hand sides such that the `(B _)` sub-pattern comes before the
/// wildcard `_`, so the second rule wins. But that is arbitrarily
/// privileging one over the other based on the order of the
/// arguments.
///
/// Instead, we can accept explicit priorities from the user to allow
/// either choice. So we need a data structure that can associate
/// matching inputs *with priorities* to outputs.
///
/// Next, we build a decision tree rather than an FSM. Why? Because
/// we're compiling to a structured language, Rust, and states become
/// *program points* rather than *data*, we cannot easily support a
/// DAG structure. In other words, we are not producing a FSM that we
/// can interpret at runtime; rather we are compiling code in which
/// each state corresponds to a sequence of statements and
/// control-flow that branches to a next state, we naturally need
/// nesting; we cannot codegen arbitrary state transitions in an
/// efficient manner. We could support a limited form of DAG that
/// reifies "diamonds" (two alternate paths that reconverge), but
/// supporting this in a way that lets the output refer to values from
/// either side is very complex (we need to invent phi-nodes), and the
/// cases where we want to do this rather than invoke a sub-term (that
/// is compiled to a separate function) are rare. Finally, note that
/// one reason to deduplicate nodes and turn a tree back into a DAG --
/// "output-suffix sharing" as some other instruction-rewriter
/// engines, such as Peepmatic, do -- is not done, because all
/// "output" occurs at leaf nodes; this is necessary because we do not
/// want to start invoking external constructors until we are sure of
/// the match. Some of the code-sharing advantages of the "suffix
/// sharing" scheme can be obtained in a more flexible and
/// user-controllable way (with less understanding of internal
/// compiler logic needed) by factoring logic into different internal
/// terms, which become different compiled functions. This is likely
/// to happen anyway as part of good software engineering practice.
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
/// So, to avoid this, we perform a sort of moral equivalent to the
/// NFA-to-DFA conversion "on the fly" as we insert nodes by
/// duplicating subtrees. At any node, when inserting with a priority
/// P and when outgoing edges lie in a range [P_lo, P_hi] such that P
/// >= P_lo and P <= P_hi, we "priority-split the edges" at priority
/// P.
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
        let mut last_edge_with_op: Option<usize> = None;
        let mut last_edge_with_op_prio: Option<Prio> = None;
        for i in 0..(edges.len() + 1) {
            if i == edges.len() || prio > edges[i].range.1 {
                // We've passed all edges with overlapping priority
                // ranges. Maybe the last edge we saw with the op
                // we're inserting can have its range expanded,
                // however.
                if last_edge_with_op.is_some() {
                    // Move it to the end of the run of equal-unit-range ops.
                    edges.swap(last_edge_with_op.unwrap(), i - 1);
                    edge = Some(i - 1);
                    edges[i - 1].range.1 = prio;
                    break;
                }
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
            if i == edges.len() {
                break;
            }
            if edges[i].symbol == op {
                last_edge_with_op = Some(i);
                last_edge_with_op_prio = Some(edges[i].range.1);
            }
            if last_edge_with_op_prio.is_some()
                && last_edge_with_op_prio.unwrap() < edges[i].range.1
            {
                last_edge_with_op = None;
                last_edge_with_op_prio = None;
            }
            if edges[i].range.contains(prio) && edges[i].symbol == op {
                edge = Some(i);
                break;
            }
        }
        let edge = edge.expect("Must have found an edge at least at last iter");
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
    builders_by_term: HashMap<TermId, TermFunctionBuilder>,
}

impl<'a> TermFunctionsBuilder<'a> {
    fn new(typeenv: &'a TypeEnv, termenv: &'a TermEnv) -> Self {
        log::trace!("typeenv: {:?}", typeenv);
        log::trace!("termenv: {:?}", termenv);
        Self {
            builders_by_term: HashMap::new(),
            typeenv,
            termenv,
        }
    }

    fn build(&mut self) {
        for rule in 0..self.termenv.rules.len() {
            let rule = RuleId(rule);
            let prio = self.termenv.rules[rule.index()].prio.unwrap_or(0);

            let (pattern, expr) = lower_rule(self.typeenv, self.termenv, rule);
            let root_term = self.termenv.rules[rule.index()].lhs.root_term().unwrap();

            log::trace!(
                "build:\n- rule {:?}\n- pattern {:?}\n- expr {:?}",
                self.termenv.rules[rule.index()],
                pattern,
                expr
            );
            self.builders_by_term
                .entry(root_term)
                .or_insert_with(|| TermFunctionBuilder::new(root_term))
                .add_rule(prio, pattern.clone(), expr.clone());
        }
    }

    fn finalize(self) -> HashMap<TermId, TrieNode> {
        let functions_by_term = self
            .builders_by_term
            .into_iter()
            .map(|(term, builder)| (term, builder.trie))
            .collect::<HashMap<_, _>>();
        functions_by_term
    }
}

#[derive(Clone, Debug)]
pub struct Codegen<'a> {
    typeenv: &'a TypeEnv,
    termenv: &'a TermEnv,
    functions_by_term: HashMap<TermId, TrieNode>,
}

#[derive(Clone, Debug, Default)]
struct BodyContext {
    /// For each value: (is_ref, ty).
    values: HashMap<Value, (bool, TypeId)>,
}

impl<'a> Codegen<'a> {
    pub fn compile(typeenv: &'a TypeEnv, termenv: &'a TermEnv) -> Result<Codegen<'a>, Error> {
        let mut builder = TermFunctionsBuilder::new(typeenv, termenv);
        builder.build();
        log::trace!("builder: {:?}", builder);
        let functions_by_term = builder.finalize();
        Ok(Codegen {
            typeenv,
            termenv,
            functions_by_term,
        })
    }

    pub fn generate_rust(&self) -> Result<String, Error> {
        let mut code = String::new();

        self.generate_header(&mut code)?;
        self.generate_ctx_trait(&mut code)?;
        self.generate_internal_types(&mut code)?;
        self.generate_internal_term_constructors(&mut code)?;

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
            "\n#![allow(dead_code, unreachable_code, unreachable_patterns)]"
        )?;
        writeln!(
            code,
            "#![allow(unused_imports, unused_variables, non_snake_case)]"
        )?;

        writeln!(code, "\nuse super::*;  // Pulls in all external types.")?;

        Ok(())
    }

    fn generate_trait_sig(
        &self,
        code: &mut dyn Write,
        indent: &str,
        sig: &ExternalSig,
    ) -> Result<(), Error> {
        writeln!(
            code,
            "{}fn {}(&mut self, {}) -> {}({},){};",
            indent,
            sig.func_name,
            sig.arg_tys
                .iter()
                .enumerate()
                .map(|(i, &ty)| format!("arg{}: {}", i, self.type_name(ty, /* by_ref = */ true)))
                .collect::<Vec<_>>()
                .join(", "),
            if sig.infallible { "" } else { "Option<" },
            sig.ret_tys
                .iter()
                .map(|&ty| self.type_name(ty, /* by_ref = */ false))
                .collect::<Vec<_>>()
                .join(", "),
            if sig.infallible { "" } else { ">" },
        )?;
        Ok(())
    }

    fn generate_ctx_trait(&self, code: &mut dyn Write) -> Result<(), Error> {
        writeln!(code, "")?;
        writeln!(
            code,
            "/// Context during lowering: an implementation of this trait"
        )?;
        writeln!(
            code,
            "/// must be provided with all external constructors and extractors."
        )?;
        writeln!(
            code,
            "/// A mutable borrow is passed along through all lowering logic."
        )?;
        writeln!(code, "pub trait Context {{")?;
        for term in &self.termenv.terms {
            if term.is_external() {
                let ext_sig = term.to_sig(self.typeenv).unwrap();
                self.generate_trait_sig(code, "    ", &ext_sig)?;
            }
        }
        writeln!(code, "}}")?;

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
                        if variant.fields.is_empty() {
                            writeln!(code, "    {},", name)?;
                        } else {
                            writeln!(code, "    {} {{", name)?;
                            for field in &variant.fields {
                                let name = &self.typeenv.syms[field.name.index()];
                                let ty_name =
                                    self.typeenv.types[field.ty.index()].name(&self.typeenv);
                                writeln!(code, "        {}: {},", name, ty_name)?;
                            }
                            writeln!(code, "    }},")?;
                        }
                    }
                    writeln!(code, "}}")?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn type_name(&self, typeid: TypeId, by_ref: bool) -> String {
        match &self.typeenv.types[typeid.index()] {
            &Type::Primitive(_, sym) => self.typeenv.syms[sym.index()].clone(),
            &Type::Enum { name, .. } => {
                let r = if by_ref { "&" } else { "" };
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

    fn ty_prim(&self, ty: TypeId) -> bool {
        self.typeenv.types[ty.index()].is_prim()
    }

    fn value_binder(&self, value: &Value, is_ref: bool, ty: TypeId) -> String {
        let prim = self.ty_prim(ty);
        if prim || !is_ref {
            format!("{}", self.value_name(value))
        } else {
            format!("ref {}", self.value_name(value))
        }
    }

    fn value_by_ref(&self, value: &Value, ctx: &BodyContext) -> String {
        let raw_name = self.value_name(value);
        let &(is_ref, ty) = ctx.values.get(value).unwrap();
        let prim = self.ty_prim(ty);
        if is_ref || prim {
            raw_name
        } else {
            format!("&{}", raw_name)
        }
    }

    fn value_by_val(&self, value: &Value, ctx: &BodyContext) -> String {
        let raw_name = self.value_name(value);
        let &(is_ref, _) = ctx.values.get(value).unwrap();
        if is_ref {
            format!("{}.clone()", raw_name)
        } else {
            raw_name
        }
    }

    fn define_val(&self, value: &Value, ctx: &mut BodyContext, is_ref: bool, ty: TypeId) {
        let is_ref = !self.ty_prim(ty) && is_ref;
        ctx.values.insert(value.clone(), (is_ref, ty));
    }

    fn generate_internal_term_constructors(&self, code: &mut dyn Write) -> Result<(), Error> {
        for (&termid, trie) in &self.functions_by_term {
            let termdata = &self.termenv.terms[termid.index()];

            // Skip terms that are enum variants or that have external
            // constructors/extractors.
            if !termdata.is_constructor() || termdata.is_external() {
                continue;
            }

            let sig = termdata.to_sig(self.typeenv).unwrap();

            let args = sig
                .arg_tys
                .iter()
                .enumerate()
                .map(|(i, &ty)| format!("arg{}: {}", i, self.type_name(ty, true)))
                .collect::<Vec<_>>()
                .join(", ");
            assert_eq!(sig.ret_tys.len(), 1);
            let ret = self.type_name(sig.ret_tys[0], false);

            writeln!(
                code,
                "\n// Generated as internal constructor for term {}.",
                self.typeenv.syms[termdata.name.index()],
            )?;
            writeln!(
                code,
                "pub fn {}<C: Context>(ctx: &mut C, {}) -> Option<{}> {{",
                sig.func_name, args, ret,
            )?;

            let mut body_ctx: BodyContext = Default::default();
            let returned =
                self.generate_body(code, /* depth = */ 0, trie, "    ", &mut body_ctx)?;
            if !returned {
                writeln!(code, "    return None;")?;
            }

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
        log::trace!("generate_expr_inst: {:?}", inst);
        match inst {
            &ExprInst::ConstInt { ty, val } => {
                let value = Value::Expr {
                    inst: id,
                    output: 0,
                };
                self.define_val(&value, ctx, /* is_ref = */ false, ty);
                let name = self.value_name(&value);
                let ty = self.type_name(ty, /* by_ref = */ false);
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
                    self.type_name(ty, false),
                    self.typeenv.syms[variantinfo.name.index()]
                );
                if input_fields.is_empty() {
                    writeln!(
                        code,
                        "{}let {} = {};",
                        indent, outputname, full_variant_name
                    )?;
                } else {
                    writeln!(
                        code,
                        "{}let {} = {} {{",
                        indent, outputname, full_variant_name
                    )?;
                    for input_field in input_fields {
                        writeln!(code, "{}    {},", indent, input_field)?;
                    }
                    writeln!(code, "{}}};", indent)?;
                }
                self.define_val(&output, ctx, /* is_ref = */ false, ty);
            }
            &ExprInst::Construct {
                ref inputs,
                term,
                infallible,
                ..
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
                let termdata = &self.termenv.terms[term.index()];
                let sig = termdata.to_sig(self.typeenv).unwrap();
                assert_eq!(input_exprs.len(), sig.arg_tys.len());
                let fallible_try = if infallible { "" } else { "?" };
                writeln!(
                    code,
                    "{}let {} = {}(ctx, {}){};",
                    indent,
                    outputname,
                    sig.full_name,
                    input_exprs.join(", "),
                    fallible_try,
                )?;
                self.define_val(&output, ctx, /* is_ref = */ false, termdata.ret_ty);
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

    fn match_variant_binders(
        &self,
        variant: &Variant,
        arg_tys: &[TypeId],
        id: InstId,
        ctx: &mut BodyContext,
    ) -> Vec<String> {
        arg_tys
            .iter()
            .zip(variant.fields.iter())
            .enumerate()
            .map(|(i, (&ty, field))| {
                let value = Value::Pattern {
                    inst: id,
                    output: i,
                };
                let valuename = self.value_binder(&value, /* is_ref = */ true, ty);
                let fieldname = &self.typeenv.syms[field.name.index()];
                self.define_val(&value, ctx, /* is_ref = */ false, field.ty);
                format!("{}: {}", fieldname, valuename)
            })
            .collect::<Vec<_>>()
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
                self.define_val(
                    &Value::Pattern {
                        inst: id,
                        output: 0,
                    },
                    ctx,
                    is_ref,
                    ty,
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
                let ty_name = self.type_name(input_ty, /* is_ref = */ true);
                let variant = &variants[variant.index()];
                let variantname = &self.typeenv.syms[variant.name.index()];
                let args = self.match_variant_binders(variant, &arg_tys[..], id, ctx);
                let args = if args.is_empty() {
                    "".to_string()
                } else {
                    format!("{{ {} }}", args.join(", "))
                };
                writeln!(
                    code,
                    "{}if let {}::{} {} = {} {{",
                    indent, ty_name, variantname, args, input
                )?;
                Ok(false)
            }
            &PatternInst::Extract {
                ref inputs,
                ref output_tys,
                term,
                infallible,
                ..
            } => {
                let termdata = &self.termenv.terms[term.index()];
                let sig = termdata.to_sig(self.typeenv).unwrap();

                let input_values = inputs
                    .iter()
                    .map(|input| self.value_by_ref(input, ctx))
                    .collect::<Vec<_>>();
                let output_binders = output_tys
                    .iter()
                    .enumerate()
                    .map(|(i, &ty)| {
                        let output_val = Value::Pattern {
                            inst: id,
                            output: i,
                        };
                        self.define_val(&output_val, ctx, /* is_ref = */ false, ty);
                        self.value_binder(&output_val, /* is_ref = */ false, ty)
                    })
                    .collect::<Vec<_>>();

                if infallible {
                    writeln!(
                        code,
                        "{}let ({},) = {}(ctx, {});",
                        indent,
                        output_binders.join(", "),
                        sig.full_name,
                        input_values.join(", "),
                    )?;
                    Ok(true)
                } else {
                    writeln!(
                        code,
                        "{}if let Some(({},)) = {}(ctx, {}) {{",
                        indent,
                        output_binders.join(", "),
                        sig.full_name,
                        input_values.join(", "),
                    )?;
                    Ok(false)
                }
            }
            &PatternInst::Expr {
                ref seq, output_ty, ..
            } if seq.is_const_int().is_some() => {
                let (ty, val) = seq.is_const_int().unwrap();
                assert_eq!(ty, output_ty);

                let output = Value::Pattern {
                    inst: id,
                    output: 0,
                };
                writeln!(
                    code,
                    "{}let {} = {};",
                    indent,
                    self.value_name(&output),
                    val
                )?;
                self.define_val(&output, ctx, /* is_ref = */ false, ty);
                Ok(true)
            }
            &PatternInst::Expr {
                ref seq, output_ty, ..
            } => {
                let closure_name = format!("closure{}", id.index());
                writeln!(code, "{}let {} = || {{", indent, closure_name)?;
                let subindent = format!("{}    ", indent);
                let mut subctx = ctx.clone();
                let mut returns = vec![];
                for (id, inst) in seq.insts.iter().enumerate() {
                    let id = InstId(id);
                    self.generate_expr_inst(code, id, inst, &subindent, &mut subctx, &mut returns)?;
                }
                assert_eq!(returns.len(), 1);
                writeln!(code, "{}return Some({});", subindent, returns[0].1)?;
                writeln!(code, "{}}};", indent)?;

                let output = Value::Pattern {
                    inst: id,
                    output: 0,
                };
                writeln!(
                    code,
                    "{}if let Some({}) = {}() {{",
                    indent,
                    self.value_binder(&output, /* is_ref = */ false, output_ty),
                    closure_name
                )?;
                self.define_val(&output, ctx, /* is_ref = */ false, output_ty);

                Ok(false)
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

                assert_eq!(returns.len(), 1);
                writeln!(code, "{}return Some({});", indent, returns[0].1)?;

                returned = true;
            }

            &TrieNode::Decision { ref edges } => {
                let subindent = format!("{}    ", indent);
                // if this is a decision node, generate each match op
                // in turn (in priority order). Sort the ops within
                // each priority, and gather together adjacent
                // MatchVariant ops with the same input and disjoint
                // variants in order to create a `match` rather than a
                // chain of if-lets.
                let mut edges = edges.clone();
                edges.sort_by(|e1, e2| (-e1.range.0, &e1.symbol).cmp(&(-e2.range.0, &e2.symbol)));

                let mut i = 0;
                while i < edges.len() {
                    let mut last = i;
                    let mut adjacent_variants = HashSet::new();
                    let mut adjacent_variant_input = None;
                    log::trace!("edge: {:?}", edges[i]);
                    while last < edges.len() {
                        match &edges[last].symbol {
                            &TrieSymbol::Match {
                                op: PatternInst::MatchVariant { input, variant, .. },
                            } => {
                                if adjacent_variant_input.is_none() {
                                    adjacent_variant_input = Some(input);
                                }
                                if adjacent_variant_input == Some(input)
                                    && !adjacent_variants.contains(&variant)
                                {
                                    adjacent_variants.insert(variant);
                                    last += 1;
                                } else {
                                    break;
                                }
                            }
                            _ => {
                                break;
                            }
                        }
                    }

                    // edges[i..last] is a run of adjacent
                    // MatchVariants (possibly an empty one). Only use
                    // a `match` form if there are at least two
                    // adjacent options.
                    if last - i > 1 {
                        self.generate_body_matches(code, depth, &edges[i..last], indent, ctx)?;
                        i = last;
                        continue;
                    } else {
                        let &TrieEdge {
                            ref symbol,
                            ref node,
                            ..
                        } = &edges[i];
                        i += 1;

                        match symbol {
                            &TrieSymbol::EndOfMatch => {
                                returned =
                                    self.generate_body(code, depth + 1, node, indent, ctx)?;
                            }
                            &TrieSymbol::Match { ref op } => {
                                let id = InstId(depth);
                                let infallible =
                                    self.generate_pattern_inst(code, id, op, indent, ctx)?;
                                let i = if infallible { indent } else { &subindent[..] };
                                let sub_returned =
                                    self.generate_body(code, depth + 1, node, i, ctx)?;
                                if !infallible {
                                    writeln!(code, "{}}}", indent)?;
                                }
                                if infallible && sub_returned {
                                    returned = true;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(returned)
    }

    fn generate_body_matches(
        &self,
        code: &mut dyn Write,
        depth: usize,
        edges: &[TrieEdge],
        indent: &str,
        ctx: &mut BodyContext,
    ) -> Result<(), Error> {
        let (input, input_ty) = match &edges[0].symbol {
            &TrieSymbol::Match {
                op:
                    PatternInst::MatchVariant {
                        input, input_ty, ..
                    },
            } => (input, input_ty),
            _ => unreachable!(),
        };
        let (input_ty_sym, variants) = match &self.typeenv.types[input_ty.index()] {
            &Type::Enum {
                ref name,
                ref variants,
                ..
            } => (name, variants),
            _ => unreachable!(),
        };
        let input_ty_name = &self.typeenv.syms[input_ty_sym.index()];

        // Emit the `match`.
        writeln!(
            code,
            "{}match {} {{",
            indent,
            self.value_by_ref(&input, ctx)
        )?;

        // Emit each case.
        for &TrieEdge {
            ref symbol,
            ref node,
            ..
        } in edges
        {
            let id = InstId(depth);
            let (variant, arg_tys) = match symbol {
                &TrieSymbol::Match {
                    op:
                        PatternInst::MatchVariant {
                            variant,
                            ref arg_tys,
                            ..
                        },
                } => (variant, arg_tys),
                _ => unreachable!(),
            };

            let variantinfo = &variants[variant.index()];
            let variantname = &self.typeenv.syms[variantinfo.name.index()];
            let fields = self.match_variant_binders(variantinfo, arg_tys, id, ctx);
            let fields = if fields.is_empty() {
                "".to_string()
            } else {
                format!("{{ {} }}", fields.join(", "))
            };
            writeln!(
                code,
                "{}    &{}::{} {} => {{",
                indent, input_ty_name, variantname, fields,
            )?;
            let subindent = format!("{}        ", indent);
            self.generate_body(code, depth + 1, node, &subindent, ctx)?;
            writeln!(code, "{}    }}", indent)?;
        }

        // Always add a catchall, because we don't do exhaustiveness
        // checking on the MatcHVariants.
        writeln!(code, "{}    _ => {{}}", indent)?;

        writeln!(code, "{}}}", indent)?;

        Ok(())
    }
}
