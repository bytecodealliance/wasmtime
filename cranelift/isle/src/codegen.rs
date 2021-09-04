//! Generate Rust code from a series of Sequences.

use crate::error::Error;
use crate::ir::{lower_rule, ExprSequence, PatternInst, PatternSequence, Value};
use crate::sema::{RuleId, TermEnv, TermId, TypeEnv};
use std::collections::{BTreeMap, HashMap};

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

#[derive(Clone, Debug)]
struct TrieEdge {
    key: (PrioRange, TrieSymbol),
    node: Box<TrieNode>,
}

type Prio = i64;

#[derive(Clone, Copy, Debug)]
struct PrioRange(Prio, Prio);

impl std::cmp::PartialOrd for PrioRange {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl std::cmp::Ord for PrioRange {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.1 < other.0 {
            std::cmp::Ordering::Less
        } else if self.0 > other.1 {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Equal
        }
    }
}
impl std::cmp::PartialEq for PrioRange {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == std::cmp::Ordering::Equal
    }
}
impl std::cmp::Eq for PrioRange {}

#[derive(Clone, Debug)]
enum TrieNode {
    Decision {
        edges: BTreeMap<(PrioRange, TrieSymbol), TrieNode>,
    },
    Leaf {
        prio: Prio,
        output: Vec<ExprSequence>,
    },
    Empty,
}

impl TrieNode {
    fn insert(
        &mut self,
        prio: Prio,
        input: impl Iterator<Item = PatternInst>,
        output: ExprSequence,
    ) {
        unimplemented!()
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
        self.trie
            .insert(prio, pattern_seq.insts.into_iter(), expr_seq);
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
            let prio = self.termenv.rules[rule.index()].prio.unwrap_or(0);

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
                    .add_rule(prio, pattern.clone(), expr.clone());
            }
            if let Some(output_root_term) = rhs_root {
                self.builders_by_output
                    .entry(output_root_term)
                    .or_insert_with(|| TermFunctionBuilder::new(output_root_term))
                    .add_rule(prio, pattern, expr);
            }
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Automata {
    pub automata_by_input: HashMap<TermId, ()>,
    pub automata_by_output: HashMap<TermId, ()>,
}

impl Automata {
    pub fn compile(typeenv: &TypeEnv, termenv: &TermEnv) -> Result<Automata, Error> {
        let mut builder = TermFunctionsBuilder::new(typeenv, termenv);
        builder.build();
        // TODO
        Ok(Automata::default())
    }
}
