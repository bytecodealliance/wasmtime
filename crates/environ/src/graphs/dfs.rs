use super::*;

/// An iterative depth-first traversal.
pub struct Dfs<Node> {
    stack: Vec<DfsEvent<Node>>,
}

impl<Node> Default for Dfs<Node> {
    fn default() -> Self {
        Self {
            stack: Default::default(),
        }
    }
}

impl<Node> Dfs<Node> {
    /// Create a new DFS traversal, starting at the given roots.
    pub fn new(roots: impl IntoIterator<Item = Node>) -> Self {
        let mut dfs = Self::default();
        dfs.add_roots(roots);
        dfs
    }

    /// Add a single new root to this traversal, to be visited immediately.
    pub fn add_root(&mut self, root: Node) {
        self.stack.push(DfsEvent::Pre(root));
    }

    /// Add multiple new roots to this traversal, to be visited immediately.
    pub fn add_roots(&mut self, roots: impl IntoIterator<Item = Node>) {
        self.stack
            .extend(roots.into_iter().map(|v| DfsEvent::Pre(v)));
    }
}

/// An event during a DFS traversal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DfsEvent<Node> {
    /// The first time seeing this node.
    Pre(Node),

    /// After having just visited the given edge.
    AfterEdge(Node, Node),

    /// Finished visiting this node and all of its successors.
    Post(Node),
}

impl<Node> Dfs<Node>
where
    Node: Copy,
{
    /// Pump the traversal, yielding the next `DfsEvent`.
    ///
    /// Returns `None` when the traversal is complete.
    pub fn next<G>(&mut self, graph: G, seen: impl Fn(Node) -> bool) -> Option<DfsEvent<Node>>
    where
        G: Graph<Node>,
    {
        loop {
            let event = self.stack.pop()?;

            if let DfsEvent::Pre(node) = event {
                if seen(node) {
                    continue;
                }

                let successors = graph.successors(node);

                let (min, max) = successors.size_hint();
                let estimated_successors_len = max.unwrap_or_else(|| 2 * min);
                self.stack.reserve(
                    // We push an after-edge and pre event for each successor.
                    2 * estimated_successors_len
                        // And we push one post event for this node.
                        + 1,
                );

                self.stack.push(DfsEvent::Post(node));
                for succ in successors {
                    self.stack.push(DfsEvent::AfterEdge(node, succ));
                    if !seen(succ) {
                        self.stack.push(DfsEvent::Pre(succ));
                    }
                }
            }

            return Some(event);
        }
    }
}
