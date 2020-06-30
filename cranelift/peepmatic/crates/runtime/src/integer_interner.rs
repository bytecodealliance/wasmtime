//! Interner for (potentially large) integer values.
//!
//! We support matching on integers that can be represented by `u64`, but only
//! support automata results that fit in a `u32`. So we intern the (relatively
//! few compared to the full range of `u64`) integers we are matching against
//! here and then reference them by `IntegerId`.

use serde::de::{Deserializer, SeqAccess, Visitor};
use serde::ser::{SerializeSeq, Serializer};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::marker::PhantomData;
use std::num::{NonZeroU16, NonZeroU32};

/// An identifier for an interned integer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IntegerId(#[doc(hidden)] pub NonZeroU16);

/// An interner for integer values.
#[derive(Debug, Default)]
pub struct IntegerInterner {
    // Note: we use `BTreeMap`s for deterministic serialization.
    map: BTreeMap<u64, IntegerId>,
    values: Vec<u64>,
}

impl IntegerInterner {
    /// Construct a new `IntegerInterner`.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Intern a value into this `IntegerInterner`, returning its canonical
    /// `IntegerId`.
    #[inline]
    pub fn intern(&mut self, value: impl Into<u64>) -> IntegerId {
        debug_assert_eq!(self.map.len(), self.values.len());

        let value = value.into();

        if let Some(id) = self.map.get(&value) {
            return *id;
        }

        assert!((self.values.len() as u64) < (std::u16::MAX as u64));
        let id = IntegerId(unsafe { NonZeroU16::new_unchecked(self.values.len() as u16 + 1) });

        self.values.push(value);
        self.map.insert(value, id);
        debug_assert_eq!(self.map.len(), self.values.len());

        id
    }

    /// Get the id of an already-interned integer, or `None` if it has not been
    /// interned.
    pub fn already_interned(&self, value: impl Into<u64>) -> Option<IntegerId> {
        let value = value.into();
        self.map.get(&value).copied()
    }

    /// Lookup a previously interned integer by id.
    #[inline]
    pub fn lookup(&self, id: IntegerId) -> u64 {
        let index = id.0.get() as usize - 1;
        self.values[index]
    }
}

impl From<IntegerId> for NonZeroU32 {
    #[inline]
    fn from(id: IntegerId) -> NonZeroU32 {
        id.0.into()
    }
}

impl Serialize for IntegerInterner {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.values.len()))?;
        for p in &self.values {
            seq.serialize_element(&p)?;
        }
        seq.end()
    }
}

impl<'de> Deserialize<'de> for IntegerInterner {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(IntegerInternerVisitor {
            marker: PhantomData,
        })
    }
}

struct IntegerInternerVisitor {
    marker: PhantomData<fn() -> IntegerInterner>,
}

impl<'de> Visitor<'de> for IntegerInternerVisitor {
    type Value = IntegerInterner;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "a `peepmatic_runtime::integer_interner::IntegerInterner`"
        )
    }

    fn visit_seq<M>(self, mut access: M) -> Result<Self::Value, M::Error>
    where
        M: SeqAccess<'de>,
    {
        const DEFAULT_CAPACITY: usize = 16;
        let capacity = access.size_hint().unwrap_or(DEFAULT_CAPACITY);

        let mut interner = IntegerInterner {
            map: BTreeMap::new(),
            values: Vec::with_capacity(capacity),
        };

        while let Some(path) = access.next_element::<u64>()? {
            interner.intern(path);
        }

        Ok(interner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_test::{assert_tokens, Token};
    use std::iter::successors;

    #[derive(Debug, Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct OrderedIntegerInterner(IntegerInterner);

    impl PartialEq for OrderedIntegerInterner {
        fn eq(&self, other: &OrderedIntegerInterner) -> bool {
            self.0.values.iter().eq(other.0.values.iter())
        }
    }

    fn intern_fib(interner: &mut IntegerInterner, skip: usize, take: usize) {
        successors(Some((0, 1)), |(a, b): &(u64, u64)| {
            a.checked_add(*b).map(|c| (*b, c))
        })
        .skip(skip)
        .take(take)
        .for_each(|(i, _)| {
            interner.intern(i);
        })
    }

    #[test]
    fn test_ser_de_empty_interner() {
        let interner = IntegerInterner::new();

        assert_tokens(
            &OrderedIntegerInterner(interner),
            &[Token::Seq { len: Some(0) }, Token::SeqEnd],
        );
    }

    #[test]
    fn test_ser_de_fibonacci_interner() {
        let mut interner = IntegerInterner::new();
        intern_fib(&mut interner, 10, 5);

        assert_tokens(
            &OrderedIntegerInterner(interner),
            &[
                Token::Seq { len: Some(5) },
                Token::U64(55),
                Token::U64(89),
                Token::U64(144),
                Token::U64(233),
                Token::U64(377),
                Token::SeqEnd,
            ],
        );
    }
}
