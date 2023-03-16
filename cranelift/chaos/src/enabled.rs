use std::{
    fmt::Debug,
    sync::{Arc, Mutex},
};

use arbitrary::{Arbitrary, Unstructured};

struct ChaosEngineData {
    /// # Safety
    ///
    /// This field must never be moved from, as it is referenced by
    /// the field `unstructured` for its entire lifetime.
    ///
    /// This pattern is the ["self-referential" type](
    /// https://morestina.net/blog/1868/self-referential-types-for-fun-and-profit)
    #[allow(dead_code)]
    data: Vec<u8>,
    /// We use internal mutability such that a `ChaosEngine` can be passed
    /// through the call stack without having to be declared as mutable.
    /// Besides the convenience, the mutation of the internal unstructured
    /// data should be opaque to users anyway.
    ///
    /// # Safety
    ///
    /// The lifetime of this is actually not static, but tied to `data`.
    unstructured: Mutex<Unstructured<'static>>,
}

impl Debug for ChaosEngineData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let data_len = self.data.len();
        let remaining_len = self
            .unstructured
            .lock()
            .expect("poisoned ChaosEngineData mutex")
            .len();
        let consumed_len = data_len - remaining_len;
        f.debug_struct("ChaosEngineData")
            .field("data", &self.data)
            .field(
                "unstructured",
                &format!(
                    "(consumed {consumed_len}/{data_len} bytes, \
                    {remaining_len} remaining)"
                ),
            )
            .finish()
    }
}

/// The control plane of chaos mode.
/// Please see the [crate-level documentation](crate).
///
/// **Clone liberally!** The chaos engine is reference counted.
#[derive(Debug, Clone)]
pub struct ChaosEngine {
    data: Arc<ChaosEngineData>,
    is_todo: bool,
}

impl ChaosEngine {
    fn new(data: Vec<u8>, is_todo: bool) -> Self {
        let unstructured = Unstructured::new(&data);
        // safety: this is ok because we never move out of the vector
        let unstructured = Mutex::new(unsafe { std::mem::transmute(unstructured) });
        Self {
            data: Arc::new(ChaosEngineData { data, unstructured }),
            is_todo,
        }
    }

    /// This is a zero-sized dummy for use during any builds without the
    /// feature `chaos` enabled, especially release builds. It has no
    /// functionality, so the programmer is prevented from using it in any
    /// way in release builds, which could degrade performance.
    ///
    /// This should not be used on code paths that may execute while the
    /// feature `chaos` is enabled. That would break the assumption that
    /// [ChaosEngine] is a singleton, responsible for centrally managing
    /// the pseudo-randomness injected at runtimme.
    ///
    /// Use [todo](ChaosEngine::todo) instead, for stubbing out code paths
    /// you don't expect to be reached (yet) during chaos mode fuzzing.
    ///
    /// # Pancis
    ///
    /// Panics if it is called while the feature `chaos` is enabled.
    #[track_caller]
    pub fn noop() -> Self {
        panic!(
            "attempted to create a NOOP chaos engine \
            (while chaos mode was enabled)"
        );
    }

    /// This is the same as [noop](ChaosEngine::noop) when the the feature
    /// `chaos` is *disabled*. When `chaos` is enabled, it returns a
    /// chaos engine that returns [Error::Todo] when
    /// [get_arbitrary](ChaosEngine::get_arbitrary) is called.
    ///
    /// This may be used during development, in places which are (supposed
    /// to be) unreachable during fuzzing. Use of this function should be
    /// reduced as the chaos mode is introduced in more parts of the
    /// wasmtime codebase. Eventually, it should be deleted.
    pub fn todo() -> Self {
        Self::new(Vec::new(), true)
    }
}

impl<'a> Arbitrary<'a> for ChaosEngine {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(Self::new(u.arbitrary()?, false))
    }
    fn arbitrary_take_rest(u: Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(Self::new(Arbitrary::arbitrary_take_rest(u)?, false))
    }
}

/// An enumeration of chaos engine API errors, mostly propagating
/// [arbitrary::Error].
#[derive(Debug, Clone, Copy)]
pub enum Error {
    /// No choices were provided to the Unstructured::choose call.
    EmptyChoose,
    /// There was not enough underlying data to fulfill some request for raw
    /// bytes.
    NotEnoughData,
    /// The input bytes were not of the right format.
    IncorrectFormat,
    /// The chaos engine API was accessed on a [ChaosEngine::todo].
    Todo,
}

impl From<arbitrary::Error> for Error {
    fn from(value: arbitrary::Error) -> Self {
        // Force this match statement to be updated when arbitrary
        // introduces new error variants.
        #[deny(clippy::wildcard_enum_match_arm)]
        match value {
            arbitrary::Error::EmptyChoose => Error::EmptyChoose,
            arbitrary::Error::NotEnoughData => Error::NotEnoughData,
            arbitrary::Error::IncorrectFormat => Error::IncorrectFormat,
            _ => unreachable!(),
        }
    }
}

impl ChaosEngine {
    /// Request an arbitrary value from the chaos engine.
    ///
    /// # Errors
    ///
    /// - Errors from an underlying call to [arbitrary] will be
    ///   propagated as-is.
    /// - Calling this function on a chaos engine received from a call to
    ///   [todo] will return an [Error::Todo].
    ///
    /// [arbitrary]: arbitrary::Arbitrary::arbitrary
    /// [todo]: ChaosEngine::todo
    pub fn get_arbitrary<T: Arbitrary<'static>>(&self) -> Result<T, Error> {
        if self.is_todo {
            return Err(Error::Todo);
        }
        self.data
            .unstructured
            .lock()
            .expect("poisoned ChaosEngineData mutex")
            .arbitrary()
            .map_err(Error::from)
    }
}
