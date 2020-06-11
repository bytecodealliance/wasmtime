//! Traversals over the AST.

use crate::ast::*;

/// A low-level DFS traversal event: either entering or exiting the traversal of
/// an AST node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TraversalEvent {
    /// Entering traversal of an AST node.
    ///
    /// Processing an AST node upon this event corresponds to a pre-order
    /// DFS traversal.
    Enter,

    /// Exiting traversal of an AST node.
    ///
    /// Processing an AST node upon this event corresponds to a post-order DFS
    /// traversal.
    Exit,
}

/// A depth-first traversal of an AST.
///
/// This is a fairly low-level traversal type, and is intended to be used as a
/// building block for making specific pre-order or post-order traversals for
/// whatever problem is at hand.
///
/// This implementation is not recursive, and exposes an `Iterator` interface
/// that yields pairs of `(TraversalEvent, DynAstRef)` items.
///
/// The traversal can walk a whole set of `Optimization`s or just a subtree of
/// the AST, because the `new` constructor takes anything that can convert into
/// a `DynAstRef`.
#[derive(Debug, Clone)]
pub struct Dfs<'a> {
    stack: Vec<(TraversalEvent, DynAstRef<'a>)>,
}

impl<'a> Dfs<'a> {
    /// Construct a new `Dfs` traversal starting at the given `start` AST node.
    pub fn new(start: impl Into<DynAstRef<'a>>) -> Self {
        let start = start.into();
        Dfs {
            stack: vec![
                (TraversalEvent::Exit, start),
                (TraversalEvent::Enter, start),
            ],
        }
    }

    /// Peek at the next traversal event and AST node pair, if any.
    pub fn peek(&self) -> Option<(TraversalEvent, DynAstRef<'a>)> {
        self.stack.last().cloned()
    }
}

impl<'a> Iterator for Dfs<'a> {
    type Item = (TraversalEvent, DynAstRef<'a>);

    fn next(&mut self) -> Option<(TraversalEvent, DynAstRef<'a>)> {
        let (event, node) = self.stack.pop()?;
        if let TraversalEvent::Enter = event {
            let mut enqueue_children = EnqueueChildren(self);
            node.child_nodes(&mut enqueue_children)
        }
        return Some((event, node));

        struct EnqueueChildren<'a, 'b>(&'b mut Dfs<'a>)
        where
            'a: 'b;

        impl<'a, 'b> Extend<DynAstRef<'a>> for EnqueueChildren<'a, 'b>
        where
            'a: 'b,
        {
            fn extend<T: IntoIterator<Item = DynAstRef<'a>>>(&mut self, iter: T) {
                let iter = iter.into_iter();

                let (min, max) = iter.size_hint();
                self.0.stack.reserve(max.unwrap_or(min) * 2);

                let start = self.0.stack.len();

                for node in iter {
                    self.0.stack.push((TraversalEvent::Enter, node));
                    self.0.stack.push((TraversalEvent::Exit, node));
                }

                // Reverse to make it so that we visit children in order
                // (e.g. operands are visited in order).
                self.0.stack[start..].reverse();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use DynAstRef::*;

    #[test]
    fn test_dfs_traversal() {
        let input = "
(=> (when (imul $x $C)
          (is-power-of-two $C))
    (ishl $x $(log2 $C)))
";
        let buf = wast::parser::ParseBuffer::new(input).expect("input should lex OK");
        let ast = match wast::parser::parse::<crate::ast::Optimizations>(&buf) {
            Ok(ast) => ast,
            Err(e) => panic!("expected to parse OK, got error:\n\n{}", e),
        };

        let mut dfs = Dfs::new(&ast);
        assert!(matches!(
            dbg!(dfs.next()),
            Some((TraversalEvent::Enter, Optimizations(..)))
        ));
        assert!(matches!(
            dbg!(dfs.next()),
            Some((TraversalEvent::Enter, Optimization(..)))
        ));
        assert!(matches!(
            dbg!(dfs.next()),
            Some((TraversalEvent::Enter, Lhs(..)))
        ));
        assert!(matches!(
            dbg!(dfs.next()),
            Some((TraversalEvent::Enter, Pattern(..)))
        ));
        assert!(matches!(
            dbg!(dfs.next()),
            Some((TraversalEvent::Enter, PatternOperation(..)))
        ));
        assert!(matches!(
            dbg!(dfs.next()),
            Some((TraversalEvent::Enter, Pattern(..)))
        ));
        assert!(matches!(
            dbg!(dfs.next()),
            Some((TraversalEvent::Enter, Variable(..)))
        ));
        assert!(matches!(
            dbg!(dfs.next()),
            Some((TraversalEvent::Exit, Variable(..)))
        ));
        assert!(matches!(
            dbg!(dfs.next()),
            Some((TraversalEvent::Exit, Pattern(..)))
        ));
        assert!(matches!(
            dbg!(dfs.next()),
            Some((TraversalEvent::Enter, Pattern(..)))
        ));
        assert!(matches!(
            dbg!(dfs.next()),
            Some((TraversalEvent::Enter, Constant(..)))
        ));
        assert!(matches!(
            dbg!(dfs.next()),
            Some((TraversalEvent::Exit, Constant(..)))
        ));
        assert!(matches!(
            dbg!(dfs.next()),
            Some((TraversalEvent::Exit, Pattern(..)))
        ));
        assert!(matches!(
            dbg!(dfs.next()),
            Some((TraversalEvent::Exit, PatternOperation(..)))
        ));
        assert!(matches!(
            dbg!(dfs.next()),
            Some((TraversalEvent::Exit, Pattern(..)))
        ));
        assert!(matches!(
            dbg!(dfs.next()),
            Some((TraversalEvent::Enter, Precondition(..)))
        ));
        assert!(matches!(
            dbg!(dfs.next()),
            Some((TraversalEvent::Enter, ConstraintOperand(..)))
        ));
        assert!(matches!(
            dbg!(dfs.next()),
            Some((TraversalEvent::Enter, Constant(..)))
        ));
        assert!(matches!(
            dbg!(dfs.next()),
            Some((TraversalEvent::Exit, Constant(..)))
        ));
        assert!(matches!(
            dbg!(dfs.next()),
            Some((TraversalEvent::Exit, ConstraintOperand(..)))
        ));
        assert!(matches!(
            dbg!(dfs.next()),
            Some((TraversalEvent::Exit, Precondition(..)))
        ));
        assert!(matches!(
            dbg!(dfs.next()),
            Some((TraversalEvent::Exit, Lhs(..)))
        ));
        assert!(matches!(
            dbg!(dfs.next()),
            Some((TraversalEvent::Enter, Rhs(..)))
        ));
        assert!(matches!(
            dbg!(dfs.next()),
            Some((TraversalEvent::Enter, RhsOperation(..)))
        ));
        assert!(matches!(
            dbg!(dfs.next()),
            Some((TraversalEvent::Enter, Rhs(..)))
        ));
        assert!(matches!(
            dbg!(dfs.next()),
            Some((TraversalEvent::Enter, Variable(..)))
        ));
        assert!(matches!(
            dbg!(dfs.next()),
            Some((TraversalEvent::Exit, Variable(..)))
        ));
        assert!(matches!(
            dbg!(dfs.next()),
            Some((TraversalEvent::Exit, Rhs(..)))
        ));
        assert!(matches!(
            dbg!(dfs.next()),
            Some((TraversalEvent::Enter, Rhs(..)))
        ));
        assert!(matches!(
            dbg!(dfs.next()),
            Some((TraversalEvent::Enter, Unquote(..)))
        ));
        assert!(matches!(
            dbg!(dfs.next()),
            Some((TraversalEvent::Enter, Rhs(..)))
        ));
        assert!(matches!(
            dbg!(dfs.next()),
            Some((TraversalEvent::Enter, Constant(..)))
        ));
        assert!(matches!(
            dbg!(dfs.next()),
            Some((TraversalEvent::Exit, Constant(..)))
        ));
        assert!(matches!(
            dbg!(dfs.next()),
            Some((TraversalEvent::Exit, Rhs(..)))
        ));
        assert!(matches!(
            dbg!(dfs.next()),
            Some((TraversalEvent::Exit, Unquote(..)))
        ));
        assert!(matches!(
            dbg!(dfs.next()),
            Some((TraversalEvent::Exit, Rhs(..)))
        ));
        assert!(matches!(
            dbg!(dfs.next()),
            Some((TraversalEvent::Exit, RhsOperation(..)))
        ));
        assert!(matches!(
            dbg!(dfs.next()),
            Some((TraversalEvent::Exit, Rhs(..)))
        ));
        assert!(matches!(
            dbg!(dfs.next()),
            Some((TraversalEvent::Exit, Optimization(..)))
        ));
        assert!(matches!(
            dbg!(dfs.next()),
            Some((TraversalEvent::Exit, Optimizations(..)))
        ));
        assert!(dfs.next().is_none());
    }
}
