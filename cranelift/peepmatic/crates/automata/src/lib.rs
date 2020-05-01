//! Finite-state transducer automata.
//!
//! A transducer is a type of automata that has not only an input that it
//! accepts or rejects, but also an output. While regular automata check whether
//! an input string is in the set that the automata accepts, a transducer maps
//! the input strings to values. A regular automata is sort of a compressed,
//! immutable set, and a transducer is sort of a compressed, immutable key-value
//! dictionary. A [trie] compresses a set of strings or map from a string to a
//! value by sharing prefixes of the input string. Automata and transducers can
//! compress even better: they can share both prefixes and suffixes. [*Index
//! 1,600,000,000 Keys with Automata and Rust* by Andrew Gallant (aka
//! burntsushi)][burntsushi-blog-post] is a top-notch introduction.
//!
//! If you're looking for a general-purpose transducers crate in Rust you're
//! probably looking for [the `fst` crate][fst-crate]. While this implementation
//! is fully generic and has no dependencies, its feature set is specific to
//! `peepmatic`'s needs:
//!
//! * We need to associate extra data with each state: the match operation to
//!   evaluate next.
//!
//! * We can't provide the full input string up front, so this crate must
//!   support incremental lookups. This is because the peephole optimizer is
//!   computing the input string incrementally and dynamically: it looks at the
//!   current state's match operation, evaluates it, and then uses the result as
//!   the next character of the input string.
//!
//! * We also support incremental insertion and output when building the
//!   transducer. This is necessary because we don't want to emit output values
//!   that bind a match on an optimization's left-hand side's pattern (for
//!   example) until after we've succeeded in matching it, which might not
//!   happen until we've reached the n^th state.
//!
//! * We need to support generic output values. The `fst` crate only supports
//!   `u64` outputs, while we need to build up an optimization's right-hand side
//!   instructions.
//!
//! This implementation is based on [*Direct Construction of Minimal Acyclic
//! Subsequential Transducers* by Mihov and Maurel][paper]. That means that keys
//! must be inserted in lexicographic order during construction.
//!
//! [trie]: https://en.wikipedia.org/wiki/Trie
//! [burntsushi-blog-post]: https://blog.burntsushi.net/transducers/#ordered-maps
//! [fst-crate]: https://crates.io/crates/fst
//! [paper]: http://citeseerx.ist.psu.edu/viewdoc/download?doi=10.1.1.24.3698&rep=rep1&type=pdf

#![deny(missing_debug_implementations)]
#![deny(missing_docs)]

mod output_impls;

#[cfg(feature = "serde")]
mod serde_impls;

#[cfg(feature = "dot")]
pub mod dot;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::convert::TryInto;
use std::hash::Hash;
use std::iter;
use std::mem;

/// An output type for a transducer automata.
///
/// Not every type can be the output of a transducer. For correctness (not
/// memory safety) each type that implements this trait must satisfy the
/// following laws:
///
/// 1. `concat(empty(), x) == x` -- concatenating something with the empty
///    instance produces that same something.
///
/// 2. `prefix(a, b) == prefix(b, a)` -- taking the prefix of two instances is
///    commutative.
///
/// 3. `prefix(empty(), x) == empty()` -- the prefix of any value and the empty
///    instance is the empty instance.
///
/// 4. `difference(concat(a, b), a) == b` -- concatenating a prefix value and
///    then removing it is the identity function.
///
/// ## Example
///
/// Here is an example implementation for unsigned integers:
///
/// ```
/// use peepmatic_automata::Output;
///
/// #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// struct MyInt(u64);
///
/// impl Output for MyInt {
///     // The empty value is zero.
///     fn empty() -> Self {
///         MyInt(0)
///     }
///
///     // The prefix of two values is their min.
///     fn prefix(a: &MyInt, b: &MyInt) -> Self {
///         std::cmp::min(*a, *b)
///     }
///
///     // The difference is subtraction.
///     fn difference(a: &MyInt, b: &MyInt) -> Self {
///         MyInt(a.0 - b.0)
///     }
///
///     // Concatenation is addition.
///     fn concat(a: &MyInt, b: &MyInt) -> Self {
///         MyInt(a.0 + b.0)
///     }
/// }
///
/// // Law 1
/// assert_eq!(
///     MyInt::concat(&MyInt::empty(), &MyInt(5)),
///     MyInt(5),
/// );
///
/// // Law 2
/// assert_eq!(
///     MyInt::prefix(&MyInt(3), &MyInt(5)),
///     MyInt::prefix(&MyInt(5), &MyInt(3))
/// );
///
/// // Law 3
/// assert_eq!(
///     MyInt::prefix(&MyInt::empty(), &MyInt(5)),
///     MyInt::empty()
/// );
///
/// // Law 4
/// assert_eq!(
///     MyInt::difference(&MyInt::concat(&MyInt(2), &MyInt(3)), &MyInt(2)),
///     MyInt(3),
/// );
/// ```
pub trait Output: Sized + Eq + Hash + Clone {
    /// Construct the empty instance.
    fn empty() -> Self;

    /// Is this the empty instance?
    ///
    /// The default implementation constructs the empty instance and then checks
    /// if `self` is equal to it. Override this default if you can provide a
    /// better implementation.
    fn is_empty(&self) -> bool {
        *self == Self::empty()
    }

    /// Get the shared prefix of two instances.
    ///
    /// This must be commutative.
    fn prefix(a: &Self, b: &Self) -> Self;

    /// When `b` is a prefix of `a`, get the remaining suffix of `a` that is not
    /// shared with `b`.
    fn difference(a: &Self, b: &Self) -> Self;

    /// Concatenate `a` and `b`.
    fn concat(a: &Self, b: &Self) -> Self;
}

/// A builder for a transducer automata.
///
/// ## Type Parameters
///
/// Generic over the following parameters:
///
/// * `TAlphabet` -- the alphabet of the input strings. If your input keys are
///   `String`s, this would be `char`. If your input keys are arbitrary byte
///   strings, this would be `u8`.
///
/// * `TState` -- extra, custom data associated with each state. This isn't used
///   by the automata itself, but you can use it to annotate states with extra
///   information for your own purposes.
///
/// * `TOutput` -- the output type. See [the `Output` trait][crate::Output] for
///   the requirements that any output type must fulfill.
///
/// ## Insertions
///
/// Insertions *must* happen in lexicographic order. Failure to do this, or
/// inserting duplicates, will trigger panics.
///
/// ## Example
///
/// ```
/// use peepmatic_automata::Builder;
///
/// let mut builder = Builder::<u8, (), u64>::new();
///
/// // Insert "mon" -> 1
/// let mut insertion = builder.insert();
/// insertion
///     .next(b'm', 1)
///     .next(b'o', 0)
///     .next(b'n', 0);
/// insertion.finish();
///
/// // Insert "sat" -> 6
/// let mut insertion = builder.insert();
/// insertion
///     .next(b's', 6)
///     .next(b'a', 0)
///     .next(b't', 0);
/// insertion.finish();
///
/// // Insert "sun" -> 0
/// let mut insertion = builder.insert();
/// insertion
///     .next(b's', 0)
///     .next(b'u', 0)
///     .next(b'n', 0);
/// insertion.finish();
///
/// let automata = builder.finish();
///
/// assert_eq!(automata.get(b"sun"), Some(0));
/// assert_eq!(automata.get(b"mon"), Some(1));
/// assert_eq!(automata.get(b"sat"), Some(6));
///
/// assert!(automata.get(b"tues").is_none());
/// ```
#[derive(Debug, Clone)]
pub struct Builder<TAlphabet, TState, TOutput>
where
    TAlphabet: Clone + Eq + Hash + Ord,
    TState: Clone + Eq + Hash,
    TOutput: Output,
{
    inner: Option<BuilderInner<TAlphabet, TState, TOutput>>,
}

impl<TAlphabet, TState, TOutput> Builder<TAlphabet, TState, TOutput>
where
    TAlphabet: Clone + Eq + Hash + Ord,
    TState: Clone + Eq + Hash,
    TOutput: Output,
{
    /// Make a new builder to start constructing a new transducer automata.
    pub fn new() -> Self {
        let mut inner = BuilderInner {
            frozen: vec![],
            wip: BTreeMap::new(),
            wip_state_id_counter: 0,
            unfinished: vec![],
            already_frozen: HashMap::new(),
            last_insertion_finished: true,
        };

        // Create the start state.
        let id = inner.new_wip_state();
        inner.unfinished.push(id);

        Builder { inner: Some(inner) }
    }

    fn inner(&mut self) -> &mut BuilderInner<TAlphabet, TState, TOutput> {
        self.inner
            .as_mut()
            .expect("cannot use `Builder` anymore after calling `finish` on it")
    }

    /// Start building a new key/value insertion.
    ///
    /// Insertions are built up incrementally, and a full entry is created from
    /// a series of `TAlphabet` and `TOutput` pairs passed to
    /// [`InsertionBuilder::next`][crate::InsertionBuilder::next].
    ///
    /// ## Panics
    ///
    /// Panics if [`finish`][crate::InsertionBuilder::finish] was not called on
    /// the last `InsertionBuilder` returned from this method.
    pub fn insert(&mut self) -> InsertionBuilder<TAlphabet, TState, TOutput> {
        let inner = self.inner();
        assert!(
            inner.last_insertion_finished,
            "did not call `finish` on the last `InsertionBuilder`"
        );
        inner.last_insertion_finished = false;
        InsertionBuilder {
            inner: inner,
            index: 0,
            output: TOutput::empty(),
        }
    }

    /// Finish building this transducer and return the constructed `Automaton`.
    ///
    /// ## Panics
    ///
    /// Panics if this builder is empty, and has never had anything inserted
    /// into it.
    ///
    /// Panics if the last insertion's
    /// [`InsertionBuilder`][crate::InsertionBuilder] did not call its
    /// [finish][crate::InsertionBuilder::finish] method.
    pub fn finish(&mut self) -> Automaton<TAlphabet, TState, TOutput> {
        let mut inner = self
            .inner
            .take()
            .expect("cannot use `Builder` anymore after calling `finish` on it");
        assert!(inner.last_insertion_finished);

        let wip_start = inner.unfinished[0];

        // Freeze everything! We're done!
        let wip_to_frozen = inner.freeze_from(0);
        assert!(inner.wip.is_empty());
        assert!(inner.unfinished.is_empty());

        // Now transpose our states and transitions into our packed,
        // struct-of-arrays representation that we use inside `Automaton`.
        let FrozenStateId(s) = wip_to_frozen[&wip_start];
        let start_state = State(s);
        let mut state_data = vec![None; inner.frozen.len()];
        let mut transitions = (0..inner.frozen.len())
            .map(|_| BTreeMap::new())
            .collect::<Vec<_>>();
        let mut final_states = BTreeMap::new();

        assert!((inner.frozen.len() as u64) < (std::u32::MAX as u64));
        for (i, state) in inner.frozen.into_iter().enumerate() {
            assert!(state_data[i].is_none());
            assert!(transitions[i].is_empty());

            state_data[i] = state.state_data;

            for (input, (FrozenStateId(to_state), output)) in state.transitions {
                assert!((to_state as usize) < transitions.len());
                transitions[i].insert(input, (State(to_state), output));
            }

            if state.is_final {
                final_states.insert(State(i as u32), state.final_output);
            } else {
                assert!(state.final_output.is_empty());
            }
        }

        let automata = Automaton {
            state_data,
            transitions,
            final_states,
            start_state,
        };

        #[cfg(debug_assertions)]
        {
            if let Err(msg) = automata.check_representation() {
                panic!("Automaton::check_representation failed: {}", msg);
            }
        }

        automata
    }
}

/// A state in an automaton.
///
/// Only use a `State` with the automaton that it came from! Mixing and matching
/// states between automata will result in bogus results and/or panics!
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct State(u32);

#[derive(Clone, Debug)]
struct BuilderInner<TAlphabet, TState, TOutput>
where
    TAlphabet: Clone + Eq + Hash + Ord,
    TState: Clone + Eq + Hash,
    TOutput: Output,
{
    // The `i`th entry maps `FrozenStateId(i)` to its state.
    frozen: Vec<FrozenState<TAlphabet, TState, TOutput>>,

    // Our mutable, work-in-progress states.
    wip: BTreeMap<WipStateId, WipState<TAlphabet, TState, TOutput>>,

    // A counter for WIP state ids.
    wip_state_id_counter: u32,

    // A stack of our work-in-progress states.
    unfinished: Vec<WipStateId>,

    // A map from `WipState`s that we've already frozen to their canonical,
    // de-duplicated frozen state. This is used for hash-consing frozen states
    // so that we share suffixes in the automata.
    already_frozen: HashMap<WipState<TAlphabet, TState, TOutput>, FrozenStateId>,

    // The the last `InsertionBuilder` have its `finish` method invoked?
    last_insertion_finished: bool,
}

impl<TAlphabet, TState, TOutput> BuilderInner<TAlphabet, TState, TOutput>
where
    TAlphabet: Clone + Eq + Hash + Ord,
    TState: Clone + Eq + Hash,
    TOutput: Output,
{
    fn new_wip_state(&mut self) -> WipStateId {
        let id = WipStateId(self.wip_state_id_counter);
        self.wip_state_id_counter += 1;
        let old = self.wip.insert(
            id,
            WipState {
                state_data: None,
                transitions: BTreeMap::new(),
                is_final: false,
                final_output: TOutput::empty(),
            },
        );
        debug_assert!(old.is_none());
        id
    }

    fn freeze_from(&mut self, index: usize) -> BTreeMap<WipStateId, FrozenStateId> {
        assert!(index <= self.unfinished.len());

        let mut wip_to_frozen = BTreeMap::new();

        if index == self.unfinished.len() {
            // Nothing to freeze.
            return wip_to_frozen;
        }

        // Freeze `self.inner.unfinished[self.index + 1..]` from the end
        // back. We're essentially hash-consing each state.
        for _ in (index..self.unfinished.len()).rev() {
            let wip_id = self.unfinished.pop().unwrap();
            let mut wip = self.wip.remove(&wip_id).unwrap();

            // Update transitions to any state we just froze in an earlier
            // iteration of this loop.
            wip.update_transitions(&wip_to_frozen);

            // Get or create the canonical frozen state for this WIP state.
            //
            // Note: we're not using the entry API here because this way we can
            // avoid cloning `wip`, which would be more costly than the double
            // lookup we're doing instead.
            let frozen_id = if let Some(id) = self.already_frozen.get(&wip) {
                *id
            } else {
                let id = FrozenStateId(self.frozen.len().try_into().unwrap());
                self.frozen.push(FrozenState {
                    state_data: wip.state_data.clone(),
                    transitions: wip
                        .transitions
                        .clone()
                        .into_iter()
                        .map(|(input, (id, output))| {
                            let id = match id {
                                WipOrFrozenStateId::Frozen(id) => id,
                                WipOrFrozenStateId::Wip(_) => panic!(
                                    "when we are freezing a WIP state, it should never have \
                                     any transitions to another WIP state"
                                ),
                            };
                            (input, (id, output))
                        })
                        .collect(),
                    is_final: wip.is_final,
                    final_output: wip.final_output.clone(),
                });
                self.already_frozen.insert(wip, id);
                id
            };

            // Record the id for this newly frozen state, so that other states
            // which referenced it when it wasn't frozen can reference it as a
            // frozen state.
            wip_to_frozen.insert(wip_id, frozen_id);
        }

        // Update references to newly frozen states from the rest of the
        // unfinished stack that we didn't freeze.
        for wip_id in &self.unfinished {
            self.wip
                .get_mut(wip_id)
                .unwrap()
                .update_transitions(&wip_to_frozen);
        }

        wip_to_frozen
    }
}

/// A builder for a new entry in a transducer automata.
#[derive(Debug)]
pub struct InsertionBuilder<'a, TAlphabet, TState, TOutput>
where
    TAlphabet: Clone + Eq + Hash + Ord,
    TState: Clone + Eq + Hash,
    TOutput: Output,
{
    inner: &'a mut BuilderInner<TAlphabet, TState, TOutput>,

    // The index within `inner.unfinished` where we will transition out of next.
    index: usize,

    // Any leftover output from the last transition that we need to roll over
    // into the next transition.
    output: TOutput,
}

impl<'a, TAlphabet, TState, TOutput> InsertionBuilder<'a, TAlphabet, TState, TOutput>
where
    TAlphabet: Clone + Eq + Hash + Ord,
    TState: Clone + Eq + Hash,
    TOutput: Output,
{
    /// Insert the next character of input for this entry, and the associated
    /// output that should be emitted along with it.
    ///
    /// In general, you want to add all of your output on the very first `next`
    /// call, and use [`Output::empty()`][crate::Output::empty] for all the
    /// rest. This enables the most tail-sharing of suffixes, which leads to the
    /// most compact automatas.
    ///
    /// However, there are times when you *cannot* emit output yet, as it
    /// depends on having moved throught he automata further. For example, with
    /// `peepmatic` we cannot bind something from an optimization's left-hand
    /// side's pattern until after we know it exists, which only happens after
    /// we've moved some distance through the automata.
    pub fn next(&mut self, input: TAlphabet, output: TOutput) -> &mut Self {
        assert!(self.index < self.inner.unfinished.len());

        if output.is_empty() {
            // Leave `self.output` as it is.
        } else if self.output.is_empty() {
            self.output = output;
        } else {
            self.output = TOutput::concat(&self.output, &output);
        }

        let wip_id = self.inner.unfinished[self.index];
        let wip = self.inner.wip.get_mut(&wip_id).unwrap();

        match wip.transitions.get_mut(&input) {
            Some((WipOrFrozenStateId::Frozen(_), _)) => {
                panic!("out of order insertion: wip->frozen edge in shared prefix")
            }

            // We're still in a shared prefix with the last insertion. That
            // means that the state we are transitioning to must be the next
            // state in `unfinished`. All we have to do is make sure the
            // transition's output is the common prefix of the this insertion
            // and the last, and push any excess suffix output out to other
            // transition edges.
            Some((WipOrFrozenStateId::Wip(next_id), out)) => {
                let next_id = *next_id;
                assert_eq!(next_id, self.inner.unfinished[self.index + 1]);

                // Find the common prefix of `out` and `self.output`.
                let prefix = TOutput::prefix(&self.output, out);

                // Carry over this key's suffix for the next input's transition.
                self.output = TOutput::difference(&self.output, &prefix);

                let rest = TOutput::difference(out, &prefix);
                *out = prefix;

                let next_wip = self.inner.wip.get_mut(&next_id).unwrap();

                // Push the leftover suffix of `out` along its other
                // transitions. As a small optimization, only iterate over the
                // edges if there is a non-empty value to push out along them.
                if !rest.is_empty() {
                    if next_wip.is_final {
                        next_wip.final_output = TOutput::concat(&rest, &next_wip.final_output);
                    }
                    for (_input, (_state, output)) in &mut next_wip.transitions {
                        *output = TOutput::concat(&rest, output);
                    }
                }
            }

            // We've diverged from the shared prefix with the last
            // insertion. Freeze the last insertion's unshared suffix and create
            // a new WIP state for us to transition into.
            None => {
                self.inner.freeze_from(self.index + 1);

                let output = mem::replace(&mut self.output, TOutput::empty());

                let new_id = self.inner.new_wip_state();
                self.inner.unfinished.push(new_id);
                self.inner
                    .wip
                    .get_mut(&wip_id)
                    .unwrap()
                    .transitions
                    .insert(input, (WipOrFrozenStateId::Wip(new_id), output));
            }
        }

        self.index += 1;
        assert!(self.index < self.inner.unfinished.len());

        self
    }

    /// Finish this insertion.
    ///
    /// Failure to call this method before this `InsertionBuilder` is dropped
    /// means that the insertion is *not* committed in the builder, and future
    /// calls to [`InsertionBuilder::next`][crate::InsertionBuilder::next] will
    /// panic!
    pub fn finish(self) {
        assert!(!self.inner.unfinished.is_empty());
        assert_eq!(
            self.index,
            self.inner.unfinished.len() - 1,
            "out of order insertion"
        );

        let wip_id = *self.inner.unfinished.last().unwrap();
        let wip = self.inner.wip.get_mut(&wip_id).unwrap();
        wip.is_final = true;
        wip.final_output = self.output;

        self.inner.last_insertion_finished = true;
    }

    /// Set the optional, custom data for the current state.
    ///
    /// If you assign different state data to two otherwise-identical states
    /// within the same shared *prefix* during insertion, it is implementation
    /// defined which state and custom state data is kept.
    ///
    /// For *suffixes*, assigning different state data to two
    /// otehrwise-identical states will result in the duplication of those
    /// states: they won't get de-duplicated.
    pub fn set_state_data(&mut self, data: TState) -> &mut Self {
        assert!(self.index < self.inner.unfinished.len());
        let id = self.inner.unfinished[self.index];
        self.inner.wip.get_mut(&id).unwrap().state_data = Some(data);
        self
    }

    /// Get the current state's optional, custom data, if any.
    ///
    /// For shared prefixes, this may return state data that was assigned to an
    /// equivalent state that was added earlier in the build process.
    pub fn get_state_data(&self) -> Option<&TState> {
        let id = self.inner.unfinished[self.index];
        self.inner.wip.get(&id).unwrap().state_data.as_ref()
    }
}

/// The id of an immutable, frozen state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct FrozenStateId(u32);

/// The id of a mutable, work-in-progress state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct WipStateId(u32);

/// The id of either a frozen or a WIP state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum WipOrFrozenStateId {
    Wip(WipStateId),
    Frozen(FrozenStateId),
}

/// A frozen, immutable state inside a `Builder`.
///
/// These states are from earlier in the lexicographic sorting on input keys,
/// and have already been processed.
#[derive(Clone, Debug, Hash)]
struct FrozenState<TAlphabet, TState, TOutput>
where
    TAlphabet: Clone + Eq + Hash + Ord,
    TState: Clone + Eq + Hash,
    TOutput: Output,
{
    state_data: Option<TState>,
    transitions: BTreeMap<TAlphabet, (FrozenStateId, TOutput)>,
    is_final: bool,
    final_output: TOutput,
}

/// A mutable, work-in-progress state inside a `Builder`.
///
/// These states only exist for the last-inserted and currently-being-inserted
/// input keys. As soon as we find the end of their shared prefix, the last
/// key's unshared suffix is frozen, and then only the currently-being-inserted
/// input key has associated WIP states.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct WipState<TAlphabet, TState, TOutput>
where
    TAlphabet: Clone + Eq + Hash + Ord,
    TState: Clone + Eq + Hash,
    TOutput: Output,
{
    state_data: Option<TState>,
    transitions: BTreeMap<TAlphabet, (WipOrFrozenStateId, TOutput)>,
    is_final: bool,
    final_output: TOutput,
}

impl<TAlphabet, TState, TOutput> WipState<TAlphabet, TState, TOutput>
where
    TAlphabet: Clone + Eq + Hash + Ord,
    TState: Clone + Eq + Hash,
    TOutput: Output,
{
    /// Given that we froze some old, WIP state, update any transitions out of
    /// this WIP state so they point to the new, frozen state.
    fn update_transitions(&mut self, wip_to_frozen: &BTreeMap<WipStateId, FrozenStateId>) {
        for (to, _) in self.transitions.values_mut() {
            if let WipOrFrozenStateId::Wip(w) = *to {
                if let Some(f) = wip_to_frozen.get(&w) {
                    *to = WipOrFrozenStateId::Frozen(*f);
                }
            }
        }
    }
}

/// A finite-state transducer automata.
///
/// These are constructed via [`Builder`][crate::Builder].
///
/// An `Automaton` is immutable: new entries cannot be inserted and existing
/// entries cannot be removed.
///
/// To query an `Automaton`, there are two APIs:
///
/// 1. [`get`][crate::Automaton::get] -- a high-level method to get the associated
///    output value of a full input sequence.
///
/// 2. [`query`][crate::Automaton::query] -- a low-level method to
///    incrementally query the automata. It does not require that you have the
///    full input sequence on hand all at once, only the next character. It also
///    allows you to process the output as it it built up, rather than only at
///    giving you the final, complete output value.
#[derive(Debug, Clone)]
pub struct Automaton<TAlphabet, TState, TOutput>
where
    TAlphabet: Clone + Eq + Hash + Ord,
    TState: Clone + Eq + Hash,
    TOutput: Output,
{
    // The `i`th entry is `State(i)`'s associated custom data.
    state_data: Vec<Option<TState>>,

    // The `i`th entry contains `State(i)`'s transitions.
    transitions: Vec<BTreeMap<TAlphabet, (State, TOutput)>>,

    // Keeps track of which states are final, and if so, what their final output
    // is.
    final_states: BTreeMap<State, TOutput>,

    // The starting state.
    start_state: State,
}

impl<TAlphabet, TState, TOutput> Automaton<TAlphabet, TState, TOutput>
where
    TAlphabet: Clone + Eq + Hash + Ord,
    TState: Clone + Eq + Hash,
    TOutput: Output,
{
    /// Get the output value associated with the given input sequence.
    ///
    /// Returns `None` if the input sequence is not a member of this
    /// `Automaton`'s keys. Otherwise, returns `Some(output)`.
    pub fn get<'a>(&self, input: impl IntoIterator<Item = &'a TAlphabet>) -> Option<TOutput>
    where
        TAlphabet: 'a,
    {
        let mut query = self.query();
        let mut output = TOutput::empty();

        for inp in input {
            let this_out = query.next(inp)?;
            output = TOutput::concat(&output, &this_out);
        }

        let final_output = query.finish()?;
        Some(TOutput::concat(&output, final_output))
    }

    /// Create a low-level query.
    ///
    /// This allows you to incrementally query this `Automaton`, without
    /// providing the full input sequence ahead of time, and also incrementally
    /// build up the output.
    ///
    /// See [`Query`][crate::Query] for details.
    pub fn query(&self) -> Query<TAlphabet, TState, TOutput> {
        Query {
            automata: self,
            current_state: self.start_state,
        }
    }

    /// Check that the internal representaton is OK.
    ///
    /// Checks that we don't have any transitions to unknown states, that there
    /// aren't any cycles, that ever path through the automata eventually ends
    /// in a final state, etc.
    ///
    /// This property is `debug_assert!`ed in `Builder::finish`, and checked
    /// when deserializing an `Automaton`.
    ///
    /// Returns `true` if the representation is okay, `false` otherwise.
    fn check_representation(&self) -> Result<(), &'static str> {
        macro_rules! bail_if {
            ($condition:expr, $msg:expr) => {
                if $condition {
                    return Err($msg);
                }
            };
        }

        bail_if!(
            self.state_data.len() != self.transitions.len(),
            "different number of states and transition sets"
        );
        bail_if!(
            self.final_states.is_empty(),
            "the set of final states is empty"
        );

        bail_if!(
            (self.start_state.0 as usize) >= self.transitions.len(),
            "the start state is not a valid state"
        );

        for (f, _out) in &self.final_states {
            bail_if!(
                (f.0 as usize) >= self.transitions.len(),
                "one of the final states is not a valid state"
            );
        }

        // Walk the state transition graph and ensure that
        //
        // 1. there are no cycles, and
        //
        // 2. every path ends in a final state.
        let mut on_stack = HashSet::new();
        let mut stack = vec![
            (Traversal::Stop, self.start_state),
            (Traversal::Start, self.start_state),
        ];
        loop {
            match stack.pop() {
                None => break,
                Some((Traversal::Start, state)) => {
                    let is_new = on_stack.insert(state);
                    debug_assert!(is_new);

                    let mut has_any_transitions = false;
                    for (_input, (to_state, _output)) in &self.transitions[state.0 as usize] {
                        has_any_transitions = true;

                        // A transition to a state that we walked through to get
                        // here means that there is a cycle.
                        bail_if!(
                            on_stack.contains(to_state),
                            "there is a cycle in the state transition graph"
                        );

                        stack.extend(
                            iter::once((Traversal::Stop, *to_state))
                                .chain(iter::once((Traversal::Start, *to_state))),
                        );
                    }

                    if !has_any_transitions {
                        // All paths must end in a final state.
                        bail_if!(
                            !self.final_states.contains_key(&state),
                            "a path through the state transition graph does not end in a final state"
                        );
                    }
                }
                Some((Traversal::Stop, state)) => {
                    debug_assert!(on_stack.contains(&state));
                    on_stack.remove(&state);
                }
            }
        }

        return Ok(());

        enum Traversal {
            Start,
            Stop,
        }
    }
}

/// A low-level query of an `Automaton`.
///
/// This allows you to incrementally query an `Automaton`, without providing the
/// full input sequence ahead of time, and also to incrementally build up the
/// output.
///
/// The typical usage pattern is:
///
/// * First, a series of [`next`][crate::Query::next] calls that each provide
///   one character of the input sequence.
///
///   If this query is still on a path towards a known entry of the
///   automata, then `Some` is returned with the partial output of the
///   transition that was just taken. Otherwise, `None` is returned, signifying
///   that the input string has been rejected by the automata.
///
///   You may also inspect the current state's associated custom data, if any,
///   in between `next` calls via the
///   [`current_state_data`][crate::Query::current_state_data] method.
///
/// * When the input sequence is exhausted, call
///   [`is_in_final_state`][crate::Query::is_in_final_state] to determine if this
///   query is in a final state of the automata. If it is not, then the
///   input string has been rejected by the automata.
///
/// * Given that the input sequence is exhausted, you may call
///   [`finish`][crate::Query::finish] to get the final bit of partial output.
#[derive(Debug, Clone)]
pub struct Query<'a, TAlphabet, TState, TOutput>
where
    TAlphabet: Clone + Eq + Hash + Ord,
    TState: Clone + Eq + Hash,
    TOutput: Output,
{
    automata: &'a Automaton<TAlphabet, TState, TOutput>,
    current_state: State,
}

impl<'a, TAlphabet, TState, TOutput> Query<'a, TAlphabet, TState, TOutput>
where
    TAlphabet: Clone + Eq + Hash + Ord,
    TState: Clone + Eq + Hash,
    TOutput: Output,
{
    /// Get the current state in the automaton that this query is at.
    pub fn current_state(&self) -> State {
        self.current_state
    }

    /// Move this query to the given state in the automaton.
    ///
    /// This can be used to implement backtracking, if you can also reset your
    /// output to the way it was when you previously visited the given `State`.
    ///
    /// Only use a `State` that came from this query's automaton! Mixing and
    /// matching states between automata will result in bogus results and/or
    /// panics!
    pub fn go_to_state(&mut self, state: State) {
        assert!((state.0 as usize) < self.automata.transitions.len());
        debug_assert_eq!(
            self.automata.state_data.len(),
            self.automata.transitions.len()
        );
        self.current_state = state;
    }

    /// Does the query's current state have a transition on the given input?
    ///
    /// Regardless whether a transition on the given input exists for the
    /// current state or not, the query remains in the current state.
    pub fn has_transition_on(&self, input: &TAlphabet) -> bool {
        let State(i) = self.current_state;
        self.automata.transitions[i as usize].contains_key(input)
    }

    /// Transition to the next state given the next input character, and return
    /// the partial output for that transition.
    ///
    /// If `None` is returned, then the input sequence has been rejected by the
    /// automata, and this query remains in its current state.
    #[inline]
    pub fn next(&mut self, input: &TAlphabet) -> Option<&'a TOutput> {
        let State(i) = self.current_state;
        match self.automata.transitions[i as usize].get(input) {
            None => None,
            Some((next_state, output)) => {
                self.current_state = *next_state;
                Some(output)
            }
        }
    }

    /// Get the current state's associated custom data, if any.
    ///
    /// See also
    /// [`InsertionBuilder::set_state_data`][crate::InsertionBuilder::set_state_data].
    #[inline]
    pub fn current_state_data(&self) -> Option<&'a TState> {
        let State(i) = self.current_state;
        self.automata.state_data[i as usize].as_ref()
    }

    /// Is this query currently in a final state?
    #[inline]
    pub fn is_in_final_state(&self) -> bool {
        self.automata.final_states.contains_key(&self.current_state)
    }

    /// Given that the input sequence is exhausted, get the final bit of partial
    /// output.
    ///
    /// Returns `None` if this query is not currently in a final state, meaning
    /// that the automata has rejected this input sequence. You can check
    /// whether that is the case or not with the
    /// [`is_in_final_state`][crate::Query::is_in_final_state] method.
    pub fn finish(self) -> Option<&'a TOutput> {
        self.automata.final_states.get(&self.current_state)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
