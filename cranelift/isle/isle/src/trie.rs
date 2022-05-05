//! Trie construction.

use crate::ir::{lower_rule, ExprSequence, PatternInst, PatternSequence};
use crate::sema::{RuleId, TermEnv, TermId, TypeEnv};
use std::collections::BTreeMap;

/// Construct the tries for each term.
pub fn build_tries(typeenv: &TypeEnv, termenv: &TermEnv) -> BTreeMap<TermId, TrieNode> {
    let mut builder = TermFunctionsBuilder::new(typeenv, termenv);
    builder.build();
    log::trace!("builder: {:?}", builder);
    builder.finalize()
}

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
pub enum TrieSymbol {
    /// Run a match operation to continue matching a LHS.
    Match {
        /// The match operation to run.
        op: PatternInst,
    },
    /// We successfully matched a LHS.
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

/// A priority.
pub type Prio = i64;

/// An inclusive range of priorities.
#[derive(Clone, Copy, Debug)]
pub struct PrioRange {
    /// The minimum of this range.
    pub min: Prio,
    /// The maximum of this range.
    pub max: Prio,
}

impl PrioRange {
    fn contains(&self, prio: Prio) -> bool {
        prio >= self.min && prio <= self.max
    }

    fn is_unit(&self) -> bool {
        self.min == self.max
    }

    fn overlaps(&self, other: PrioRange) -> bool {
        // This can be derived via DeMorgan: !(self.begin > other.end
        // OR other.begin > self.end).
        self.min <= other.max && other.min <= self.max
    }

    fn intersect(&self, other: PrioRange) -> PrioRange {
        PrioRange {
            min: std::cmp::max(self.min, other.min),
            max: std::cmp::min(self.max, other.max),
        }
    }

    fn union(&self, other: PrioRange) -> PrioRange {
        PrioRange {
            min: std::cmp::min(self.min, other.min),
            max: std::cmp::max(self.max, other.max),
        }
    }

    fn split_at(&self, prio: Prio) -> (PrioRange, PrioRange) {
        assert!(self.contains(prio));
        assert!(!self.is_unit());
        if prio == self.min {
            (
                PrioRange {
                    min: self.min,
                    max: self.min,
                },
                PrioRange {
                    min: self.min + 1,
                    max: self.max,
                },
            )
        } else {
            (
                PrioRange {
                    min: self.min,
                    max: prio - 1,
                },
                PrioRange {
                    min: prio,
                    max: self.max,
                },
            )
        }
    }
}

/// An edge in our term trie.
#[derive(Clone, Debug)]
pub struct TrieEdge {
    /// The priority range for this edge's sub-trie.
    pub range: PrioRange,
    /// The match operation to perform for this edge.
    pub symbol: TrieSymbol,
    /// This edge's sub-trie.
    pub node: TrieNode,
}

/// A node in the term trie.
#[derive(Clone, Debug)]
pub enum TrieNode {
    /// One or more patterns could match.
    ///
    /// Maybe one pattern already has matched, but there are more (higher
    /// priority and/or same priority but more specific) patterns that could
    /// still match.
    Decision {
        /// The child sub-tries that we can match from this point on.
        edges: Vec<TrieEdge>,
    },

    /// The successful match of an LHS pattern, and here is its RHS expression.
    Leaf {
        /// The priority of this rule.
        prio: Prio,
        /// The RHS expression to evaluate upon a successful LHS pattern match.
        output: ExprSequence,
    },

    /// No LHS pattern matches.
    Empty,
}

impl TrieNode {
    fn is_empty(&self) -> bool {
        matches!(self, &TrieNode::Empty)
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
            if i == edges.len() || prio > edges[i].range.max {
                // We've passed all edges with overlapping priority
                // ranges. Maybe the last edge we saw with the op
                // we're inserting can have its range expanded,
                // however.
                if last_edge_with_op.is_some() {
                    // Move it to the end of the run of equal-unit-range ops.
                    edges.swap(last_edge_with_op.unwrap(), i - 1);
                    edge = Some(i - 1);
                    edges[i - 1].range.max = prio;
                    break;
                }
                edges.insert(
                    i,
                    TrieEdge {
                        range: PrioRange {
                            min: prio,
                            max: prio,
                        },
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
                last_edge_with_op_prio = Some(edges[i].range.max);
            }
            if last_edge_with_op_prio.is_some()
                && last_edge_with_op_prio.unwrap() < edges[i].range.max
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
                        PrioRange {
                            min: prio,
                            max: prio,
                        },
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

    /// Get a pretty-printed version of this trie, for debugging.
    pub fn pretty(&self) -> String {
        let mut s = String::new();
        pretty_rec(&mut s, self, "");
        return s;

        fn pretty_rec(s: &mut String, node: &TrieNode, indent: &str) {
            match node {
                TrieNode::Decision { edges } => {
                    s.push_str(indent);
                    s.push_str("TrieNode::Decision:\n");

                    let new_indent = indent.to_owned() + "    ";
                    for edge in edges {
                        s.push_str(indent);
                        s.push_str(&format!(
                            "  edge: range = {:?}, symbol: {:?}\n",
                            edge.range, edge.symbol
                        ));
                        pretty_rec(s, &edge.node, &new_indent);
                    }
                }
                TrieNode::Empty | TrieNode::Leaf { .. } => {
                    s.push_str(indent);
                    s.push_str(&format!("{:?}\n", node));
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
    trie: TrieNode,
}

impl TermFunctionBuilder {
    fn new() -> Self {
        TermFunctionBuilder {
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
    builders_by_term: BTreeMap<TermId, TermFunctionBuilder>,
}

impl<'a> TermFunctionsBuilder<'a> {
    fn new(typeenv: &'a TypeEnv, termenv: &'a TermEnv) -> Self {
        log::trace!("typeenv: {:?}", typeenv);
        log::trace!("termenv: {:?}", termenv);
        Self {
            builders_by_term: BTreeMap::new(),
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
                .or_insert_with(|| TermFunctionBuilder::new())
                .add_rule(prio, pattern.clone(), expr.clone());
        }
    }

    fn finalize(self) -> BTreeMap<TermId, TrieNode> {
        let functions_by_term = self
            .builders_by_term
            .into_iter()
            .map(|(term, builder)| (term, builder.trie))
            .collect::<BTreeMap<_, _>>();
        functions_by_term
    }
}
