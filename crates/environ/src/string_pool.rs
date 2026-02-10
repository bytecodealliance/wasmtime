//! Simple string interning.

use crate::{
    collections::{HashMap, String, Vec},
    error::OutOfMemory,
    prelude::*,
};
use core::{fmt, mem, num::NonZeroU32};
use wasmtime_core::alloc::TryClone;

/// A pool of interned strings.
///
/// Insert new strings with [`StringPool::insert`] to get an `Atom` that is
/// unique per string within the context of the associated pool.
///
/// Once you have interned a string into the pool and have its `Atom`, you can
/// get the interned string slice via `&pool[atom]` or `pool.get(atom)`.
///
/// In general, there are no correctness protections against indexing into a
/// different `StringPool` from the one that the `Atom` was not allocated
/// inside. Doing so is memory safe but may panic or otherwise return incorrect
/// results.
#[derive(Default)]
pub struct StringPool {
    /// A map from each string in this pool (as an unsafe borrow from
    /// `self.strings`) to its `Atom`.
    map: mem::ManuallyDrop<HashMap<&'static str, Atom>>,

    /// Strings in this pool. These must never be mutated or reallocated once
    /// inserted.
    strings: mem::ManuallyDrop<Vec<Box<str>>>,
}

impl Drop for StringPool {
    fn drop(&mut self) {
        // Ensure that `self.map` is dropped before `self.strings`, since
        // `self.map` borrows from `self.strings`.
        //
        // Safety: Neither field will be used again.
        unsafe {
            mem::ManuallyDrop::drop(&mut self.map);
            mem::ManuallyDrop::drop(&mut self.strings);
        }
    }
}

/// An interned string associated with a particular string in a `StringPool`.
///
/// Allows for $O(1)$ equality tests, $O(1)$ hashing, and $O(1)$
/// arbitrary-but-stable ordering.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Atom {
    index: NonZeroU32,
}

impl fmt::Debug for StringPool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct Strings<'a>(&'a StringPool);
        impl fmt::Debug for Strings<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_map()
                    .entries(
                        self.0
                            .strings
                            .iter()
                            .enumerate()
                            .map(|(i, s)| (Atom::new(i), s)),
                    )
                    .finish()
            }
        }

        f.debug_struct("StringPool")
            .field("strings", &Strings(self))
            .finish()
    }
}

impl TryClone for StringPool {
    fn try_clone(&self) -> Result<Self, OutOfMemory> {
        Ok(StringPool {
            map: self.map.try_clone()?,
            strings: self.strings.try_clone()?,
        })
    }
}

impl TryClone for Atom {
    fn try_clone(&self) -> Result<Self, OutOfMemory> {
        Ok(*self)
    }
}

impl core::ops::Index<Atom> for StringPool {
    type Output = str;

    #[inline]
    #[track_caller]
    fn index(&self, atom: Atom) -> &Self::Output {
        self.get(atom).unwrap()
    }
}

impl StringPool {
    /// Create a new, empty pool.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a new string into this pool.
    pub fn insert(&mut self, s: &str) -> Result<Atom, OutOfMemory> {
        if let Some(atom) = self.map.get(s) {
            return Ok(*atom);
        }

        self.map.reserve(1)?;
        self.strings.reserve(1)?;

        let mut owned = String::new();
        owned.reserve_exact(s.len())?;
        owned.push_str(s).expect("reserved capacity");
        let owned = owned
            .into_boxed_str()
            .expect("reserved exact capacity, so shouldn't need to realloc");

        let index = self.strings.len();
        let atom = Atom::new(index);
        self.strings.push(owned).expect("reserved capacity");

        // SAFETY: We never expose this borrow and never mutate or reallocate
        // strings once inserted into the pool.
        let s = unsafe { mem::transmute::<&str, &'static str>(&self.strings[index]) };

        let old = self.map.insert(s, atom).expect("reserved capacity");
        debug_assert!(old.is_none());

        Ok(atom)
    }

    /// Get the `Atom` for the given string, if it has already been inserted
    /// into this pool.
    pub fn get_atom(&self, s: &str) -> Option<Atom> {
        self.map.get(s).copied()
    }

    /// Does this pool contain the given `atom`?
    #[inline]
    pub fn contains(&self, atom: Atom) -> bool {
        atom.index() < self.strings.len()
    }

    /// Get the string associated with the given `atom`, if the pool contains
    /// the atom.
    #[inline]
    pub fn get(&self, atom: Atom) -> Option<&str> {
        if self.contains(atom) {
            Some(&self.strings[atom.index()])
        } else {
            None
        }
    }
}

impl fmt::Debug for Atom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Atom")
            .field("index", &self.index())
            .finish()
    }
}

impl Atom {
    fn new(index: usize) -> Self {
        assert!(index < usize::try_from(u32::MAX).unwrap());
        let index = u32::try_from(index).unwrap();
        let index = NonZeroU32::new(index + 1).unwrap();
        Self { index }
    }

    /// Get this atom's index in its pool.
    pub fn index(&self) -> usize {
        let index = self.index.get() - 1;
        usize::try_from(index).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() -> Result<()> {
        let mut pool = StringPool::new();

        let a = pool.insert("a")?;
        assert_eq!(&pool[a], "a");
        assert_eq!(pool.get_atom("a"), Some(a));

        let a2 = pool.insert("a")?;
        assert_eq!(a, a2);
        assert_eq!(&pool[a2], "a");

        let b = pool.insert("b")?;
        assert_eq!(&pool[b], "b");
        assert_ne!(a, b);
        assert_eq!(pool.get_atom("b"), Some(b));

        assert!(pool.get_atom("zzz").is_none());

        let mut pool2 = StringPool::new();
        let c = pool2.insert("c")?;
        assert_eq!(&pool2[c], "c");
        assert_eq!(a, c);
        assert_eq!(&pool2[a], "c");
        assert!(!pool2.contains(b));
        assert!(pool2.get(b).is_none());

        Ok(())
    }

    #[test]
    fn stress() -> Result<()> {
        let mut pool = StringPool::new();

        let n = if cfg!(miri) { 100 } else { 10_000 };

        for _ in 0..2 {
            let atoms: Vec<_> = (0..n).map(|i| pool.insert(&i.to_string())).try_collect()?;

            for atom in atoms {
                assert!(pool.contains(atom));
                assert_eq!(&pool[atom], atom.index().to_string());
            }
        }

        Ok(())
    }
}
