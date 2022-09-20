//! Trie construction.

use crate::ir::{lower_rule, ExprSequence, PatternInst, PatternSequence};
use crate::log;
use crate::sema::{RuleId, TermEnv, TermId, TypeEnv};
use std::collections::BTreeMap;

/// Construct the tries for each term.
pub fn build_tries(typeenv: &TypeEnv, termenv: &TermEnv) -> BTreeMap<TermId, TrieNode> {
    let mut builder = TermFunctionsBuilder::new(typeenv, termenv);
    builder.build();
    log!("builder: {:?}", builder);
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
/// We prepare for codegen by building a trie, where the trie
/// associates input strings to output values.  Each input string is a
/// sequence of match operators followed by an "end of match" token,
/// and each output is a sequence of ops that build the output
/// expression. The goal of the trie is to generate a decision-tree
/// procedure that lets us execute match ops in a deterministic way,
/// eventually landing at a state that corresponds to the
/// highest-priority matching rule and can produce the output.
///
/// To build this trie, we construct nodes with edges to child nodes.
/// Each edge consists of one input token (a `PatternInst` or EOM).
/// The trie is built in a way that respects rule priorities, but the
/// trie itself does not encode priorities. Instead, we build the trie
/// from the rules in highest-to-lowest priority order and ensure that
/// priorities are respected during this build process.  To do so,
/// each node has a "frontier" corresponding to the *latest* edge that
/// had any rule in the sub-tree added in any higher priority. (We use
/// the last priority's frontier and update the current one as we
/// insert, moving "current" to "last" when we move to the next lower
/// priority.) When we add a rule to the trie, we can insert into an
/// existing subtree only if it is after this frontier. Within the
/// acceptable range of insertion points, we reuse edges as much as
/// possible, and we insert in sorted order so that match edges for
/// the same value and enum are grouped as much as possible (to enable
/// use of `match` rather than `if let` statements in the generated
/// Rust code).
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

/// An edge in our term trie.
#[derive(Clone, Debug)]
pub struct TrieEdge {
    /// The match operation to perform for this edge.
    pub symbol: TrieSymbol,
    /// This edge's sub-trie.
    pub node: TrieNode,
}

/// In a decision node, a "frontier": at a given priority, rules have
/// been inserted up to the given edge index.
#[derive(Clone, Copy, Debug)]
pub struct PrioFrontier {
    /// The priority level of this frontier.
    prio: Prio,
    /// The latest edge at which a rule of this priority appears. If
    /// `None`, then no rules with this priority have been inserted.
    edge_idx: Option<usize>,
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
        /// The "frontier" used to maintain priority ordering: the
        /// last priority at which a subtrie had a new leaf insertion,
        /// and the latest edge which has had such an insertion.
        ///
        /// To maintain proper ordering of rule application, we need
        /// to insert rules in descending priority order, and we need
        /// to not insert prior to this point if the current rule's
        /// priority is less than the priority stored here.
        last_prio: Option<PrioFrontier>,
        /// The current priority's last insertion point. Becomes
        /// `last_prio` if a rule is inserted with a priority less
        /// than this one.
        cur_prio: Option<PrioFrontier>,
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
            *self = TrieNode::Decision {
                edges: vec![],
                last_prio: None,
                cur_prio: None,
            };
        }

        // We must be a decision node.
        let (edges, last_prio, cur_prio) = match self {
            &mut TrieNode::Decision {
                ref mut edges,
                ref mut last_prio,
                ref mut cur_prio,
            } => (edges, last_prio, cur_prio),
            _ => panic!("insert on leaf node!"),
        };

        // If we are inserting at a lower prio than in `cur_prio`,
        // `cur_prio` moves to `last_prio` (and controls our minimum
        // insertion point) and we initialize `cur_prio` with our
        // current priority and new max insertion index (of 0).
        if cur_prio.is_none() || prio < cur_prio.unwrap().prio {
            *last_prio = *cur_prio;
            *cur_prio = Some(PrioFrontier {
                prio,
                // This is `None` initially but will be updated below
                // to at least the frontier, as we always reuse or
                // insert an edge and that edge is always >=
                // `last_prio.edge_idx`, if not `None`.
                edge_idx: None,
            });
        }
        let cur_prio = cur_prio.as_mut().unwrap();

        // Determine the minimum edge index under which we can insert
        // while respecting priorities.
        let start = last_prio.and_then(|frontier| frontier.edge_idx);

        // Now find or insert the appropriate edge.
        let edge = edges
            .iter()
            .skip(start.unwrap_or(0))
            .position(|edge| edge.symbol == op)
            .map(|pos| pos + start.unwrap_or(0))
            .unwrap_or_else(|| {
                // Insert in a position among our allowed range
                // (strictly after `start` now) that would be sorted
                // according to the `op` symbol.
                let first_after_prev_prio = start.map(|x| x + 1).unwrap_or(0);
                assert!(first_after_prev_prio <= edges.len());
                let insert_pos = edges[first_after_prev_prio..]
                    .binary_search_by(|edge| edge.symbol.cmp(&op))
                    .unwrap_err()
                    + first_after_prev_prio;

                if cur_prio.edge_idx.is_some() && insert_pos <= cur_prio.edge_idx.unwrap() {
                    *cur_prio.edge_idx.as_mut().unwrap() += 1;
                }
                edges.insert(
                    insert_pos,
                    TrieEdge {
                        symbol: op,
                        node: TrieNode::Empty,
                    },
                );
                insert_pos
            });

        cur_prio.edge_idx = Some(std::cmp::max(cur_prio.edge_idx.unwrap_or(0), edge));

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

    /// Get a pretty-printed version of this trie, for debugging.
    pub fn pretty(&self) -> String {
        let mut s = String::new();
        pretty_rec(&mut s, self, "");
        return s;

        fn pretty_rec(s: &mut String, node: &TrieNode, indent: &str) {
            match node {
                TrieNode::Decision { edges, .. } => {
                    s.push_str(indent);
                    s.push_str("TrieNode::Decision:\n");

                    let new_indent = indent.to_owned() + "    ";
                    for edge in edges {
                        s.push_str(indent);
                        s.push_str(&format!("  edge: symbol: {:?}\n", edge.symbol));
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
        log!("typeenv: {:?}", typeenv);
        log!("termenv: {:?}", termenv);
        Self {
            builders_by_term: BTreeMap::new(),
            typeenv,
            termenv,
        }
    }

    fn build(&mut self) {
        // Sort rules by priority, descending, and insert in that order.
        let mut rule_indices: Vec<usize> = (0..self.termenv.rules.len()).collect();
        // Sort in descending order (highest priority first).
        rule_indices.sort_unstable_by_key(|&index| {
            std::cmp::Reverse(self.termenv.rules[index].prio.unwrap_or(0))
        });

        for rule in rule_indices {
            let rule = RuleId(rule);
            let prio = self.termenv.rules[rule.index()].prio.unwrap_or(0);

            let (pattern, expr) = lower_rule(self.typeenv, self.termenv, rule);
            let root_term = self.termenv.rules[rule.index()].lhs.root_term().unwrap();

            log!(
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
