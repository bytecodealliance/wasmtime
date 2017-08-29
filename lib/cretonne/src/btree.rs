//! Generic B-Tree implementation.
//!
//! This module defines a `Btree<K, V>` type which provides similar functionality to
//! `BtreeMap<K, V>`, but with some important differences in the implementation:
//!
//! 1. Memory is allocated from a `NodePool<K, V>` instead of the global heap.
//! 2. The footprint of a BTree is only 4 bytes.
//! 3. A BTree doesn't implement `Drop`, leaving it to the pool to manage memory.
//!
//! The node pool is intended to be used as a LIFO allocator. After building up a larger data
//! structure with many list references, the whole thing can be discarded quickly by clearing the
//! pool.

use std::marker::PhantomData;

// A Node reference is a direct index to an element of the pool.
type NodeRef = u32;

/// A B-tree data structure which nodes are allocated from a pool.
pub struct BTree<K, V> {
    index: NodeRef,
    unused1: PhantomData<K>,
    unused2: PhantomData<V>,
}

/// An enum representing a B-tree node.
/// Keys and values are required to implement Default.
enum Node<K, V> {
    Inner {
        size: u8,
        keys: [K; 7],
        nodes: [NodeRef; 8],
    },
    Leaf {
        size: u8,
        keys: [K; 7],
        values: [V; 7],
    },
}

/// Memory pool for nodes.
struct NodePool<K, V> {
    // The array containing the nodes.
    data: Vec<Node<K, V>>,

    // A free list
    freelist: Vec<NodeRef>,
}

impl<K: Default, V: Default> NodePool<K, V> {
    /// Create a new NodePool.
    pub fn new() -> NodePool<K, V> {
        NodePool {
            data: Vec::new(),
            freelist: Vec::new(),
        }
    }

    /// Get a B-tree node.
    pub fn get(&self, index: u32) -> Option<&Node<K, V>> {
        unimplemented!()
    }
}

impl<K: Default, V: Default> BTree<K, V> {
    /// Search for `key` and return a `Cursor` that either points at `key` or the position where it would be inserted.
    pub fn search(&mut self, key: K) -> Cursor<K, V> {
        unimplemented!()
    }
}

pub struct Cursor<'a, K: 'a, V: 'a> {
    pool: &'a mut NodePool<K, V>,
    height: usize,
    path: [(NodeRef, u8); 16],
}

impl<'a, K: Default, V: Default> Cursor<'a, K, V> {
    /// The key at the cursor position. Returns `None` when the cursor points off the end.
    pub fn key(&self) -> Option<K> {
        unimplemented!()
    }

    /// The value at the cursor position. Returns `None` when the cursor points off the end.
    pub fn value(&self) -> Option<&V> {
        unimplemented!()
    }

    /// Move to the next element.
    /// Returns `false` if that moves the cursor off the end.
    pub fn next(&mut self) -> bool {
        unimplemented!()
    }

    /// Move to the previous element.
    /// Returns `false` if this moves the cursor before the beginning.
    pub fn prev(&mut self) -> bool {
        unimplemented!()
    }

    /// Insert a `(key, value)` pair at the cursor position.
    /// It is an error to insert a key that would be out of order at this position.
    pub fn insert(&mut self, key: K, value: V) {
        unimplemented!()
    }

    /// Remove the current element.
    pub fn remove(&mut self) {
        unimplemented!()
    }
}
