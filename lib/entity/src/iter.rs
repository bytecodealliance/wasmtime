//! A double-ended iterator over entity references and entities.

use EntityRef;
use std::marker::PhantomData;
use std::slice;

/// Iterate over all keys in order.
pub struct Iter<'a, K: EntityRef, V>
where
    V: 'a,
{
    pos: usize,
    iter: slice::Iter<'a, V>,
    unused: PhantomData<K>,
}

impl<'a, K: EntityRef, V> Iter<'a, K, V> {
    /// Create an `Iter` iterator that visits the `PrimaryMap` keys and values
    /// of `iter`.
    pub fn new(key: K, iter: slice::Iter<'a, V>) -> Self {
        Self {
            pos: key.index(),
            iter,
            unused: PhantomData,
        }
    }
}

impl<'a, K: EntityRef, V> Iterator for Iter<'a, K, V> {
    type Item = (K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(next) = self.iter.next() {
            let pos = self.pos;
            self.pos += 1;
            Some((K::new(pos), next))
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'a, K: EntityRef, V> DoubleEndedIterator for Iter<'a, K, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if let Some(next_back) = self.iter.next_back() {
            Some((K::new(self.pos), next_back))
        } else {
            None
        }
    }
}

impl<'a, K: EntityRef, V> ExactSizeIterator for Iter<'a, K, V> {}

/// Iterate over all keys in order.
pub struct IterMut<'a, K: EntityRef, V>
where
    V: 'a,
{
    pos: usize,
    iter: slice::IterMut<'a, V>,
    unused: PhantomData<K>,
}

impl<'a, K: EntityRef, V> IterMut<'a, K, V> {
    /// Create an `IterMut` iterator that visits the `PrimaryMap` keys and values
    /// of `iter`.
    pub fn new(key: K, iter: slice::IterMut<'a, V>) -> Self {
        Self {
            pos: key.index(),
            iter,
            unused: PhantomData,
        }
    }
}

impl<'a, K: EntityRef, V> Iterator for IterMut<'a, K, V> {
    type Item = (K, &'a mut V);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(next) = self.iter.next() {
            let pos = self.pos;
            self.pos += 1;
            Some((K::new(pos), next))
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'a, K: EntityRef, V> DoubleEndedIterator for IterMut<'a, K, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if let Some(next_back) = self.iter.next_back() {
            Some((K::new(self.pos), next_back))
        } else {
            None
        }
    }
}

impl<'a, K: EntityRef, V> ExactSizeIterator for IterMut<'a, K, V> {}
