//! `serde::Serialize` and `serde::Deserialize` implementations for `Automaton`.
//!
//! Rather than prefix each serialized field with which field it is, we always
//! serialize fields in alphabetical order. Make sure to maintain this if you
//! add or remove fields!
//!
//! Each time you add/remove a field, or change serialization in any other way,
//! make sure to bump `SERIALIZATION_VERSION`.

use crate::{Automaton, Output, State};
use serde::{
    de::{self, Deserializer, SeqAccess, Visitor},
    ser::SerializeTupleStruct,
    Deserialize, Serialize, Serializer,
};
use std::collections::BTreeMap;
use std::fmt;
use std::hash::Hash;
use std::marker::PhantomData;

const SERIALIZATION_VERSION: u32 = 1;

impl Serialize for State {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u32(self.0)
    }
}

impl<'de> Deserialize<'de> for State {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(State(deserializer.deserialize_u32(U32Visitor)?))
    }
}

struct U32Visitor;

impl<'de> Visitor<'de> for U32Visitor {
    type Value = u32;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("an integer between `0` and `2^32 - 1`")
    }

    fn visit_u8<E>(self, value: u8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(u32::from(value))
    }

    fn visit_u32<E>(self, value: u32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(value)
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        use std::u32;
        if value <= u64::from(u32::MAX) {
            Ok(value as u32)
        } else {
            Err(E::custom(format!("u32 out of range: {}", value)))
        }
    }
}

impl<TAlphabet, TState, TOutput> Serialize for Automaton<TAlphabet, TState, TOutput>
where
    TAlphabet: Serialize + Clone + Eq + Hash + Ord,
    TState: Serialize + Clone + Eq + Hash,
    TOutput: Serialize + Output,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let Automaton {
            final_states,
            start_state,
            state_data,
            transitions,
        } = self;

        let mut s = serializer.serialize_tuple_struct("Automaton", 5)?;
        s.serialize_field(&SERIALIZATION_VERSION)?;
        s.serialize_field(final_states)?;
        s.serialize_field(start_state)?;
        s.serialize_field(state_data)?;
        s.serialize_field(transitions)?;
        s.end()
    }
}

impl<'de, TAlphabet, TState, TOutput> Deserialize<'de> for Automaton<TAlphabet, TState, TOutput>
where
    TAlphabet: 'de + Deserialize<'de> + Clone + Eq + Hash + Ord,
    TState: 'de + Deserialize<'de> + Clone + Eq + Hash,
    TOutput: 'de + Deserialize<'de> + Output,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_tuple_struct(
            "Automaton",
            5,
            AutomatonVisitor {
                phantom: PhantomData,
            },
        )
    }
}

struct AutomatonVisitor<'de, TAlphabet, TState, TOutput>
where
    TAlphabet: 'de + Deserialize<'de> + Clone + Eq + Hash + Ord,
    TState: 'de + Deserialize<'de> + Clone + Eq + Hash,
    TOutput: 'de + Deserialize<'de> + Output,
{
    phantom: PhantomData<&'de (TAlphabet, TState, TOutput)>,
}

impl<'de, TAlphabet, TState, TOutput> Visitor<'de>
    for AutomatonVisitor<'de, TAlphabet, TState, TOutput>
where
    TAlphabet: 'de + Deserialize<'de> + Clone + Eq + Hash + Ord,
    TState: 'de + Deserialize<'de> + Clone + Eq + Hash,
    TOutput: 'de + Deserialize<'de> + Output,
{
    type Value = Automaton<TAlphabet, TState, TOutput>;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("Automaton")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        match seq.next_element::<u32>()? {
            Some(v) if v == SERIALIZATION_VERSION => {}
            Some(v) => {
                return Err(de::Error::invalid_value(
                    de::Unexpected::Unsigned(v as u64),
                    &self,
                ));
            }
            None => return Err(de::Error::invalid_length(0, &"Automaton expects 5 elements")),
        }

        let final_states = match seq.next_element::<BTreeMap<State, TOutput>>()? {
            Some(x) => x,
            None => return Err(de::Error::invalid_length(1, &"Automaton expects 5 elements")),
        };

        let start_state = match seq.next_element::<State>()? {
            Some(x) => x,
            None => return Err(de::Error::invalid_length(2, &"Automaton expects 5 elements")),
        };

        let state_data = match seq.next_element::<Vec<Option<TState>>>()? {
            Some(x) => x,
            None => return Err(de::Error::invalid_length(3, &"Automaton expects 5 elements")),
        };

        let transitions = match seq.next_element::<Vec<BTreeMap<TAlphabet, (State, TOutput)>>>()? {
            Some(x) => x,
            None => return Err(de::Error::invalid_length(4, &"Automaton expects 5 elements")),
        };

        let automata = Automaton {
            final_states,
            start_state,
            state_data,
            transitions,
        };

        // Ensure that the deserialized automata is well-formed.
        automata
            .check_representation()
            .map_err(|msg| de::Error::custom(msg))?;

        Ok(automata)
    }
}
