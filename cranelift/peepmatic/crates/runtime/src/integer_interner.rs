//! Interner for (potentially large) integer values.
//!
//! We support matching on integers that can be represented by `u64`, but only
//! support automata results that fit in a `u32`. So we intern the (relatively
//! few compared to the full range of `u64`) integers we are matching against
//! here and then reference them by `IntegerId`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::convert::TryInto;

/// An identifier for an interned integer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IntegerId(#[doc(hidden)] pub u32);

/// An interner for integer values.
#[derive(Debug, Default, Serialize, Deserialize)]
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

        let id = IntegerId(self.values.len().try_into().unwrap());

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
        self.values[id.0 as usize]
    }
}

impl From<IntegerId> for u32 {
    #[inline]
    fn from(id: IntegerId) -> u32 {
        id.0
    }
}
