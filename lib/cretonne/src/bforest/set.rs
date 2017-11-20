//! Forest of sets.

use packed_option::PackedOption;
use std::marker::PhantomData;
use super::{INNER_SIZE, Comparator, Forest, NodePool, Node, NodeData, Path, SetValue};

/// Tag type defining forest types for a set.
struct SetTypes<K, C>(PhantomData<(K, C)>);

impl<K, C> Forest for SetTypes<K, C>
where
    K: Copy,
    C: Comparator<K>,
{
    type Key = K;
    type Value = SetValue;
    type LeafKeys = [K; 2 * INNER_SIZE - 1];
    type LeafValues = [SetValue; 2 * INNER_SIZE - 1];
    type Comparator = C;

    fn splat_key(key: Self::Key) -> Self::LeafKeys {
        [key; 2 * INNER_SIZE - 1]
    }

    fn splat_value(value: Self::Value) -> Self::LeafValues {
        [value; 2 * INNER_SIZE - 1]
    }
}

/// Memory pool for a forest of `Set` instances.
pub struct SetForest<K, C>
where
    K: Copy,
    C: Comparator<K>,
{
    nodes: NodePool<SetTypes<K, C>>,
}

impl<K, C> SetForest<K, C>
where
    K: Copy,
    C: Comparator<K>,
{
    /// Create a new empty forest.
    pub fn new() -> SetForest<K, C> {
        SetForest { nodes: NodePool::new() }
    }

    /// Clear all sets in the forest.
    ///
    /// All `Set` instances belong to this forest are invalidated and should no longer be used.
    pub fn clear(&mut self) {
        self.nodes.clear();
    }
}

/// B-tree representing an ordered set of `K`s using `C` for comparing elements.
///
/// This is not a general-purpose replacement for `BTreeSet`. See the [module
/// documentation](index.html) for more information about design tradeoffs.
///
/// Sets can be cloned, but that operation should only be used as part of cloning the whole forest
/// they belong to. *Cloning a set does not allocate new memory for the clone*. It creates an alias
/// of the same memory.
#[derive(Clone)]
pub struct Set<K, C>
where
    K: Copy,
    C: Comparator<K>,
{
    root: PackedOption<Node>,
    unused: PhantomData<(K, C)>,
}

impl<K, C> Set<K, C>
where
    K: Copy,
    C: Comparator<K>,
{
    /// Make an empty set.
    pub fn new() -> Set<K, C> {
        Set {
            root: None.into(),
            unused: PhantomData,
        }
    }

    /// Is this an empty set?
    pub fn is_empty(&self) -> bool {
        self.root.is_none()
    }

    /// Does the set contain `key`?.
    pub fn contains(&self, key: K, forest: &SetForest<K, C>, comp: &C) -> bool {
        self.root
            .expand()
            .and_then(|root| Path::default().find(key, root, &forest.nodes, comp))
            .is_some()
    }

    /// Try to insert `key` into the set.
    ///
    /// If the set did not contain `key`, insert it and return true.
    ///
    /// If `key` is already present, don't change the set and return false.
    pub fn insert(&mut self, key: K, forest: &mut SetForest<K, C>, comp: &C) -> bool {
        self.cursor(forest, comp).insert(key)
    }

    /// Remove `key` from the set and return true.
    ///
    /// If `key` was not present in the set, return false.
    pub fn remove(&mut self, key: K, forest: &mut SetForest<K, C>, comp: &C) -> bool {
        let mut c = self.cursor(forest, comp);
        if c.goto(key) {
            c.remove();
            true
        } else {
            false
        }
    }

    /// Remove all entries.
    pub fn clear(&mut self, forest: &mut SetForest<K, C>) {
        if let Some(root) = self.root.take() {
            forest.nodes.free_tree(root);
        }
    }

    /// Create a cursor for navigating this set. The cursor is initially positioned off the end of
    /// the set.
    pub fn cursor<'a>(
        &'a mut self,
        forest: &'a mut SetForest<K, C>,
        comp: &'a C,
    ) -> SetCursor<'a, K, C> {
        SetCursor::new(self, forest, comp)
    }

    /// Create an iterator traversing this set. The iterator type is `K`.
    pub fn iter<'a>(&'a self, forest: &'a SetForest<K, C>) -> SetIter<'a, K, C> {
        SetIter {
            root: self.root,
            pool: &forest.nodes,
            path: Path::default(),
        }
    }
}

impl<K, C> Default for Set<K, C>
where
    K: Copy,
    C: Comparator<K>,
{
    fn default() -> Self {
        Self::new()
    }
}

/// A position in a `Set` used to navigate and modify the ordered set.
///
/// A cursor always points at an element in the set, or "off the end" which is a position after the
/// last element in the set.
pub struct SetCursor<'a, K, C>
where
    K: 'a + Copy,
    C: 'a + Comparator<K>,
{
    root: &'a mut PackedOption<Node>,
    pool: &'a mut NodePool<SetTypes<K, C>>,
    comp: &'a C,
    path: Path<SetTypes<K, C>>,
}

impl<'a, K, C> SetCursor<'a, K, C>
where
    K: Copy,
    C: Comparator<K>,
{
    /// Create a cursor with a default (invalid) location.
    fn new(
        container: &'a mut Set<K, C>,
        forest: &'a mut SetForest<K, C>,
        comp: &'a C,
    ) -> SetCursor<'a, K, C> {
        SetCursor {
            root: &mut container.root,
            pool: &mut forest.nodes,
            comp,
            path: Path::default(),
        }
    }

    /// Is this cursor pointing to an empty set?
    pub fn is_empty(&self) -> bool {
        self.root.is_none()
    }

    /// Move cursor to the next element and return it.
    ///
    /// If the cursor reaches the end, return `None` and leave the cursor at the off-the-end
    /// position.
    pub fn next(&mut self) -> Option<K> {
        self.path.next(self.pool).map(|(k, _)| k)
    }

    /// Move cursor to the previous element and return it.
    ///
    /// If the cursor is already pointing at the first element, leave it there and return `None`.
    pub fn prev(&mut self) -> Option<K> {
        self.root.expand().and_then(|root| {
            self.path.prev(root, self.pool).map(|(k, _)| k)
        })
    }

    /// Get the current element, or `None` if the cursor is at the end.
    pub fn elem(&self) -> Option<K> {
        self.path.leaf_pos().and_then(|(node, entry)| {
            self.pool[node].unwrap_leaf().0.get(entry).cloned()
        })
    }

    /// Move this cursor to `elem`.
    ///
    /// If `elem` is in the set, place the cursor at `elem` and return true.
    ///
    /// If `elem` is not in the set, place the cursor at the next larger element (or the end) and
    /// return false.
    pub fn goto(&mut self, elem: K) -> bool {
        match self.root.expand() {
            None => false,
            Some(root) => {
                if self.path.find(elem, root, self.pool, self.comp).is_some() {
                    true
                } else {
                    self.path.normalize(self.pool);
                    false
                }
            }
        }
    }

    /// Move this cursor to the first element.
    pub fn goto_first(&mut self) -> Option<K> {
        self.root.map(|root| self.path.first(root, self.pool).0)
    }

    /// Try to insert `elem` into the set and leave the cursor at the inserted element.
    ///
    /// If the set did not contain `elem`, insert it and return true.
    ///
    /// If `elem` is already present, don't change the set, place the cursor at `goto(elem)`, and
    /// return false.
    pub fn insert(&mut self, elem: K) -> bool {
        match self.root.expand() {
            None => {
                let root = self.pool.alloc_node(NodeData::leaf(elem, SetValue()));
                *self.root = root.into();
                self.path.set_root_node(root);
                true
            }
            Some(root) => {
                // TODO: Optimize the case where `self.path` is already at the correct insert pos.
                if self.path.find(elem, root, self.pool, self.comp).is_none() {
                    *self.root = self.path.insert(elem, SetValue(), self.pool).into();
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Remove the current element (if any) and return it.
    /// This advances the cursor to the next element after the removed one.
    pub fn remove(&mut self) -> Option<K> {
        let elem = self.elem();
        if elem.is_some() {
            *self.root = self.path.remove(self.pool).into();
        }
        elem
    }
}

#[cfg(test)]
impl<'a, K, C> SetCursor<'a, K, C>
where
    K: Copy + ::std::fmt::Display,
    C: Comparator<K>,
{
    fn verify(&self) {
        self.path.verify(self.pool);
        self.root.map(|root| self.pool.verify_tree(root, self.comp));
    }

    /// Get a text version of the path to the current position.
    fn tpath(&self) -> String {
        self.path.to_string()
    }
}

/// An iterator visiting the elements of a `Set`.
pub struct SetIter<'a, K, C>
where
    K: 'a + Copy,
    C: 'a + Comparator<K>,
{
    root: PackedOption<Node>,
    pool: &'a NodePool<SetTypes<K, C>>,
    path: Path<SetTypes<K, C>>,
}

impl<'a, K, C> Iterator for SetIter<'a, K, C>
where
    K: 'a + Copy,
    C: 'a + Comparator<K>,
{
    type Item = K;

    fn next(&mut self) -> Option<Self::Item> {
        // We use `self.root` to indicate if we need to go to the first element. Reset to `None`
        // once we've returned the first element. This also works for an empty tree since the
        // `path.next()` call returns `None` when the path is empty. This also fuses the iterator.
        match self.root.take() {
            Some(root) => Some(self.path.first(root, self.pool).0),
            None => self.path.next(self.pool).map(|(k, _)| k),
        }
    }
}

#[cfg(test)]
mod test {
    use std::mem;
    use super::*;
    use super::super::NodeData;

    #[test]
    fn node_size() {
        // check that nodes are cache line sized when keys are 32 bits.
        type F = SetTypes<u32, ()>;
        assert_eq!(mem::size_of::<NodeData<F>>(), 64);
    }

    #[test]
    fn empty() {
        let mut f = SetForest::<u32, ()>::new();
        f.clear();

        let mut s = Set::<u32, ()>::new();
        assert!(s.is_empty());
        s.clear(&mut f);
        assert!(!s.contains(7, &f, &()));

        // Iterator for an empty set.
        assert_eq!(s.iter(&f).next(), None);

        let mut c = SetCursor::new(&mut s, &mut f, &());
        c.verify();
        assert_eq!(c.elem(), None);

        assert_eq!(c.goto_first(), None);
        assert_eq!(c.tpath(), "<empty path>");
    }

    #[test]
    fn simple_cursor() {
        let mut f = SetForest::<u32, ()>::new();
        let mut s = Set::<u32, ()>::new();
        let mut c = SetCursor::new(&mut s, &mut f, &());

        assert!(c.insert(50));
        c.verify();
        assert_eq!(c.elem(), Some(50));

        assert!(c.insert(100));
        c.verify();
        assert_eq!(c.elem(), Some(100));

        assert!(c.insert(10));
        c.verify();
        assert_eq!(c.elem(), Some(10));

        // Basic movement.
        assert_eq!(c.next(), Some(50));
        assert_eq!(c.next(), Some(100));
        assert_eq!(c.next(), None);
        assert_eq!(c.next(), None);
        assert_eq!(c.prev(), Some(100));
        assert_eq!(c.prev(), Some(50));
        assert_eq!(c.prev(), Some(10));
        assert_eq!(c.prev(), None);
        assert_eq!(c.prev(), None);

        assert!(c.goto(50));
        assert_eq!(c.elem(), Some(50));
        assert_eq!(c.remove(), Some(50));
        c.verify();

        assert_eq!(c.elem(), Some(100));
        assert_eq!(c.remove(), Some(100));
        c.verify();
        assert_eq!(c.elem(), None);
        assert_eq!(c.remove(), None);
        c.verify();
    }

    #[test]
    fn two_level_sparse_tree() {
        let mut f = SetForest::<u32, ()>::new();
        let mut s = Set::<u32, ()>::new();
        let mut c = SetCursor::new(&mut s, &mut f, &());

        // Insert enough elements that we get a two-level tree.
        // Each leaf node holds 8 elements
        assert!(c.is_empty());
        for i in 0..50 {
            assert!(c.insert(i));
            assert_eq!(c.elem(), Some(i));
        }
        assert!(!c.is_empty());

        assert_eq!(c.goto_first(), Some(0));
        assert_eq!(c.tpath(), "node2[0]--node0[0]");

        assert_eq!(c.prev(), None);
        for i in 1..50 {
            assert_eq!(c.next(), Some(i));
        }
        assert_eq!(c.next(), None);
        for i in (0..50).rev() {
            assert_eq!(c.prev(), Some(i));
        }
        assert_eq!(c.prev(), None);

        assert!(c.goto(25));
        for i in 25..50 {
            assert_eq!(c.remove(), Some(i));
            assert!(!c.is_empty());
            c.verify();
        }

        for i in (0..25).rev() {
            assert!(!c.is_empty());
            assert_eq!(c.elem(), None);
            assert_eq!(c.prev(), Some(i));
            assert_eq!(c.remove(), Some(i));
            c.verify();
        }
        assert_eq!(c.elem(), None);
        assert!(c.is_empty());
    }

    #[test]
    fn three_level_sparse_tree() {
        let mut f = SetForest::<u32, ()>::new();
        let mut s = Set::<u32, ()>::new();
        let mut c = SetCursor::new(&mut s, &mut f, &());

        // Insert enough elements that we get a 3-level tree.
        // Each leaf node holds 8 elements when filled up sequentially.
        // Inner nodes hold 8 node pointers.
        assert!(c.is_empty());
        for i in 0..150 {
            assert!(c.insert(i));
            assert_eq!(c.elem(), Some(i));
        }
        assert!(!c.is_empty());

        assert!(c.goto(0));
        assert_eq!(c.tpath(), "node11[0]--node2[0]--node0[0]");

        assert_eq!(c.prev(), None);
        for i in 1..150 {
            assert_eq!(c.next(), Some(i));
        }
        assert_eq!(c.next(), None);
        for i in (0..150).rev() {
            assert_eq!(c.prev(), Some(i));
        }
        assert_eq!(c.prev(), None);

        assert!(c.goto(125));
        for i in 125..150 {
            assert_eq!(c.remove(), Some(i));
            assert!(!c.is_empty());
            c.verify();
        }

        for i in (0..125).rev() {
            assert!(!c.is_empty());
            assert_eq!(c.elem(), None);
            assert_eq!(c.prev(), Some(i));
            assert_eq!(c.remove(), Some(i));
            c.verify();
        }
        assert_eq!(c.elem(), None);
        assert!(c.is_empty());
    }

    // Generate a densely populated 4-level tree.
    //
    // Level 1: 1 root
    // Level 2: 8 inner
    // Level 3: 64 inner
    // Level 4: 512 leafs, up to 7680 elements
    //
    // A 3-level tree can hold at most 960 elements.
    fn dense4l(f: &mut SetForest<i32, ()>) -> Set<i32, ()> {
        f.clear();
        let mut s = Set::new();

        // Insert 400 elements in 7 passes over the range to avoid the half-full leaf node pattern
        // that comes from sequential insertion. This will generate a normal leaf layer.
        for n in 0..4000 {
            assert!(s.insert((n * 7) % 4000, f, &()));
        }
        s
    }

    #[test]
    fn four_level() {
        let mut f = SetForest::<i32, ()>::new();
        let mut s = dense4l(&mut f);

        assert_eq!(
            s.iter(&f).collect::<Vec<_>>()[0..10],
            [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]
        );

        let mut c = s.cursor(&mut f, &());

        c.verify();

        // Peel off a whole sub-tree of the root by deleting from the front.
        // The 900 element is near the front of the second sub-tree.
        assert!(c.goto(900));
        assert_eq!(c.tpath(), "node48[1]--node47[0]--node26[0]--node20[4]");
        assert!(c.goto(0));
        for i in 0..900 {
            assert!(!c.is_empty());
            assert_eq!(c.remove(), Some(i));
        }
        c.verify();
        assert_eq!(c.elem(), Some(900));

        // Delete backwards from somewhere in the middle.
        assert!(c.goto(3000));
        for i in (2000..3000).rev() {
            assert_eq!(c.prev(), Some(i));
            assert_eq!(c.remove(), Some(i));
            assert_eq!(c.elem(), Some(3000));
        }
        c.verify();

        // Remove everything in a scattered manner, triggering many collapsing patterns.
        for i in 0..4000 {
            if c.goto((i * 7) % 4000) {
                c.remove();
            }
        }
        assert!(c.is_empty());
    }

    #[test]
    fn four_level_clear() {
        let mut f = SetForest::<i32, ()>::new();
        let mut s = dense4l(&mut f);
        s.clear(&mut f);
    }
}
