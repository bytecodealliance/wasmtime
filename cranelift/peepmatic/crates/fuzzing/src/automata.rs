//! Helpers for fuzzing the `peepmatic-automata` crate.

use peepmatic_automata::{Automaton, Builder, Output};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::hash::Hash;

fn serde_roundtrip<TAlphabet, TState, TOutput>(
    automata: Automaton<TAlphabet, TState, TOutput>,
) -> Automaton<TAlphabet, TState, TOutput>
where
    TAlphabet: Serialize + for<'de> Deserialize<'de> + Clone + Eq + Hash + Ord,
    TState: Serialize + for<'de> Deserialize<'de> + Clone + Eq + Hash,
    TOutput: Serialize + for<'de> Deserialize<'de> + Output,
{
    let encoded: Vec<u8> = bincode::serialize(&automata).expect("should serialize OK");
    bincode::deserialize(&encoded).expect("should deserialize OK")
}

/// Construct an automaton from the the given input-output pairs, and assert
/// that:
///
/// * Putting in each of the input strings should result in the expected output
///   string.
///
/// * Putting in an input string that is not one of the given inputs from our
///   input-output pairs should never yield an output value.
pub fn simple_automata(input_output_pairs: Vec<Vec<(u8, Vec<u8>)>>) {
    let _ = env_logger::try_init();

    let full_input = |pair: &[(u8, Vec<u8>)]| {
        let mut full_input = vec![];
        for (input, _) in pair {
            full_input.push(*input);
        }
        full_input
    };

    let mut inputs = HashSet::new();

    let mut input_output_pairs: Vec<_> = input_output_pairs
        .into_iter()
        .filter(|pair| {
            !pair.is_empty() && {
                // Make sure we don't have duplicate inputs.
                let is_new = inputs.insert(full_input(pair));
                is_new
            }
        })
        .collect();

    input_output_pairs.sort_by(|a, b| full_input(a).cmp(&full_input(b)));

    if input_output_pairs.is_empty() {
        return;
    }

    // A map from one of our concatenated input strings to its concatenated
    // output.
    let mut expected = HashMap::with_capacity(input_output_pairs.len());

    let mut builder = Builder::<u8, (), Vec<u8>>::new();
    for pair in &input_output_pairs {
        let mut full_input = vec![];
        let mut full_output = vec![];

        let mut ins = builder.insert();
        for (input, output) in pair.iter().cloned() {
            full_input.push(input);
            full_output.extend(output.iter().copied());

            ins.next(input, output);
        }

        let old = expected.insert(full_input, full_output);
        assert!(old.is_none());

        ins.finish();
    }

    let automata = builder.finish();
    let automata = serde_roundtrip(automata);

    // Assert that each of our input strings yields the expected output.
    for (input, expected_output) in &expected {
        log::debug!("Testing input: {:?}", input);
        let actual_output = automata.get(input);
        assert!(actual_output.is_some());
        assert_eq!(actual_output.as_ref().unwrap(), expected_output);
    }

    // Test that mutations of our input strings (that aren't themselves other
    // input strings!) do not yeild any output.
    for input in expected.keys() {
        for i in 0..input.len() {
            let mut mutated = input.clone();
            mutated[i] = mutated[i].wrapping_add(1);
            log::debug!("Testing mutated input: {:?}", mutated);
            if !expected.contains_key(&mutated) {
                assert!(automata.get(&mutated).is_none());
            }
        }
    }
}

/// Do differential testing against the `fst` crate, which is another
/// implementation of the algorithm we use for finite-state transducer
/// construction in `peepmatic-automata`.
pub fn fst_differential(map: HashMap<Vec<u8>, u64>) {
    let _ = env_logger::try_init();

    let mut inputs: Vec<_> = map
        .keys()
        .filter(|k| !k.is_empty() && k.len() < 256)
        .cloned()
        .collect();
    inputs.sort();
    inputs.dedup();
    if inputs.is_empty() {
        return;
    }

    let mut fst = fst::MapBuilder::memory();
    let mut builder = Builder::<u8, (), u64>::new();

    for inp in &inputs {
        fst.insert(inp, map[inp]).unwrap();

        let mut ins = builder.insert();
        for (i, ch) in inp.iter().enumerate() {
            ins.next(*ch, if i == 0 { map[inp] } else { 0 });
        }
        ins.finish();
    }

    let fst = fst.into_map();
    let automata = builder.finish();
    let automata = serde_roundtrip(automata);

    for inp in inputs {
        // Check we have the same result as `fst` for inputs we know are in the
        // automata.
        log::debug!("Testing input {:?}", inp);
        let expected = fst.get(&inp).expect("`fst` should have entry for `inp`");
        let actual = automata
            .get(&inp)
            .expect("automata should have entry for `inp`");
        assert_eq!(expected, actual);

        // Check that we have the same result as `fst` for inputs that may or
        // may not be in the automata.
        for i in 0..inp.len() {
            let mut mutated = inp.clone();
            mutated[i] = mutated[i].wrapping_add(1);
            log::debug!("Testing mutated input {:?}", mutated);
            let expected = fst.get(&mutated);
            let actual = automata.get(&mutated);
            assert_eq!(expected, actual);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_simple_automata() {
        crate::check(simple_automata);
    }

    #[test]
    fn check_fst_differential() {
        crate::check(fst_differential);
    }

    #[test]
    fn regression_test_0() {
        simple_automata(vec![vec![(0, vec![0]), (0, vec![1])], vec![(0, vec![2])]]);
    }

    #[test]
    fn regression_test_1() {
        fst_differential(vec![(vec![1, 3], 5), (vec![1, 2], 4)].into_iter().collect());
    }

    #[test]
    fn regression_test_2() {
        simple_automata(vec![vec![(0, vec![11]), (0, vec![])], vec![(0, vec![11])]]);
    }
}
