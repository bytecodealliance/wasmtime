use crate::{NamedId, WasiCtxNamedView};
use rand::{Rng, SeedableRng as _, TryRng};
use std::convert::Infallible;
use std::marker;
use wasmtime::component::HasData;

/// A helper struct which implements [`HasData`] for the `wasi:random` APIs.
///
/// This can be useful when directly calling `add_to_linker` functions directly,
/// such as [`wasmtime_wasi::p2::bindings::random::random::add_to_linker`] as
/// the `D` type parameter. See [`HasData`] for more information about the type
/// parameter's purpose.
///
/// When using this type you can skip the [`WasiRandomView`] trait, for
/// example.
///
/// [`wasmtime_wasi::p2::bindings::random::random::add_to_linker`]: crate::p2::bindings::random::random::add_to_linker
///
/// # Examples
///
/// ```
/// use wasmtime::component::Linker;
/// use wasmtime::{Engine, Result};
/// use wasmtime_wasi::random::*;
///
/// struct MyStoreState {
///     random: WasiRandomCtx,
/// }
///
/// fn main() -> Result<()> {
///     let engine = Engine::default();
///     let mut linker = Linker::new(&engine);
///
///     wasmtime_wasi::p2::bindings::random::random::add_to_linker::<MyStoreState, WasiRandom>(
///         &mut linker,
///         |state| &mut state.random,
///     )?;
///     Ok(())
/// }
/// ```
pub struct WasiRandom;

impl HasData for WasiRandom {
    type Data<'a> = &'a mut WasiRandomCtx;
}

/// Default largest length accepted by wasi 0.2 `get-random-bytes` and
/// `get-insecure-random-bytes` methods. This constant must match docs in
/// cli-flags crate.
pub const DEFAULT_MAX_SIZE: u64 = 64 << 20;

pub struct WasiRandomCtx {
    pub(crate) random: Box<dyn Rng + Send>,
    pub(crate) insecure_random: Box<dyn Rng + Send>,
    pub(crate) insecure_random_seed: u128,
    pub(crate) max_size: u64,
}

impl Default for WasiRandomCtx {
    fn default() -> Self {
        // For the insecure random API, use `SmallRng`, which is fast. It's
        // also insecure, but that's the deal here.
        let insecure_random = Box::new(rand::rngs::SmallRng::from_rng(&mut rand::rng()));
        // For the insecure random seed, use a `u128` generated from
        // `rand::random()`, so that it's not guessable from the
        // insecure_random API.
        let insecure_random_seed = rand::random::<u128>();
        let max_size = DEFAULT_MAX_SIZE;
        Self {
            random: thread_rng(),
            insecure_random,
            insecure_random_seed,
            max_size,
        }
    }
}

pub trait WasiRandomView: Send {
    fn random(&mut self) -> &mut WasiRandomCtx;
}

impl WasiRandomView for WasiRandomCtx {
    fn random(&mut self) -> &mut WasiRandomCtx {
        self
    }
}

/// Implement `insecure-random` using a deterministic cycle of bytes.
pub struct Deterministic {
    cycle: std::iter::Cycle<std::vec::IntoIter<u8>>,
}

impl Deterministic {
    pub fn new(bytes: Vec<u8>) -> Self {
        Deterministic {
            cycle: bytes.into_iter().cycle(),
        }
    }
}

impl TryRng for Deterministic {
    type Error = Infallible;
    fn try_next_u32(&mut self) -> Result<u32, Infallible> {
        let b0 = self.cycle.next().expect("infinite sequence");
        let b1 = self.cycle.next().expect("infinite sequence");
        let b2 = self.cycle.next().expect("infinite sequence");
        let b3 = self.cycle.next().expect("infinite sequence");
        Ok(((b0 as u32) << 24) + ((b1 as u32) << 16) + ((b2 as u32) << 8) + (b3 as u32))
    }
    fn try_next_u64(&mut self) -> Result<u64, Infallible> {
        let w0 = self.next_u32();
        let w1 = self.next_u32();
        Ok(((w0 as u64) << 32) + (w1 as u64))
    }
    fn try_fill_bytes(&mut self, buf: &mut [u8]) -> Result<(), Infallible> {
        for b in buf.iter_mut() {
            *b = self.cycle.next().expect("infinite sequence");
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn deterministic() {
        let mut det = Deterministic::new(vec![1, 2, 3, 4]);
        let mut buf = vec![0; 1024];
        det.try_fill_bytes(&mut buf).expect("get randomness");
        for (ix, b) in buf.iter().enumerate() {
            assert_eq!(*b, (ix % 4) as u8 + 1)
        }
    }
}

pub fn thread_rng() -> Box<dyn Rng + Send> {
    let mut rng = rand::rng();
    Box::new(rand::rngs::StdRng::from_rng(&mut rng))
}

/// A helper struct which implements [`HasData`] for the `wasi:random` APIs
/// when used in combination with named imports.
///
/// This structure is similar in purpose to [`WasiRandom`] and is used
/// when using the [`named_imports`] module for `wasi:random`. This structure
/// serves as the `D` type parameter for `add_to_linker` functions.
///
/// [`named_imports`]: crate::p3::bindings::named_imports::wasi::random
///
/// # Meaning of the `T` parameter
///
/// Here the `T` must be something that implements [`WasiRandomNamedView`]. The
/// corresponding `Data` for this type is [`WasiCtxNamedView`] which internally
/// will contain `&mut T`.
///
/// Effectively you're going to implement [`WasiRandomNamedView`] for something in
/// your embedding, and that's the `T` you'll fill in here.
///
/// # Examples
///
/// ```
/// use wasmtime::component::{Linker, Component};
/// use wasmtime::{Engine, Result};
/// use wasmtime_wasi::{NamedId, WasiCtxNamedView};
/// use wasmtime_wasi::random::*;
/// use wasmtime_wasi::p2::bindings::named_imports;
/// use std::collections::HashMap;
///
/// struct MyStoreState {
///     states: HashMap<NamedId, WasiRandomCtx>,
/// }
///
/// fn main() -> Result<()> {
///     let engine = Engine::default();
///     let mut linker = Linker::new(&engine);
///     let component = Component::new(&engine, "(component)")?;
///     let mut name_map = HashMap::new();
///
///     named_imports::wasi::random::random::add_to_linker::<MyStoreState, WasiRandomNamed<MyStoreState>>(
///         &mut linker,
///         &component,
///         |name| {
///             let len = name_map.len();
///             Ok(NamedId(*name_map.entry(name.to_string()).or_insert(len)))
///         },
///         |state| WasiCtxNamedView(state),
///     )?;
///     Ok(())
/// }
///
/// impl WasiRandomNamedView for MyStoreState {
///     fn random(&mut self, id: NamedId) -> &mut WasiRandomCtx {
///         self.states.get_mut(&id).expect("state for id")
///     }
/// }
/// ```
pub struct WasiRandomNamed<T>(marker::PhantomData<fn() -> T>);

impl<T> HasData for WasiRandomNamed<T>
where
    T: WasiRandomNamedView,
{
    type Data<'a> = WasiCtxNamedView<'a, T>;
}

/// A trait used to look up a specific `wasi:random` context for a named
/// import.
///
/// This trait is used in conjunction with the [`named_imports`] bindings
/// generated for all WASI interfaces. The purpose of this trait is for
/// embedders to define how a [`NamedId`] maps to a particular `wasi:random`
/// context, here returned as [`WasiRandomCtx`]. Embedders are responsible
/// for assigning meaning to [`NamedId`] values themselves. These IDs are
/// assigned when [`add_named_to_linker`] is called, for example, as the
/// `lookup` argument to that function.
///
/// When using [`add_named_to_linker`] it's sufficient to implement this trait
/// for the `T` in `Store<T>`. You can also instead implement the
/// [`WasiNamedView`] trait for `T` which implies an implementation of this
/// trait.
///
/// When using `add_to_linker` in the generated `bindings::named_imports`
/// module then values implementing this live within the `T` of `Store<T>`, and
/// be temporarily referenced in [`WasiCtxNamedView`] where internally that'll
/// hold `WasiCtxNamedView(&mut your_type)`.
///
/// [`named_imports`]: crate::p3::bindings::named_imports
/// [`add_named_to_linker`]: crate::p3::random::add_named_to_linker
/// [`WasiNamedView`]: crate::WasiNamedView
///
/// # Examples
///
/// ```
/// use wasmtime::component::{Linker, Component};
/// use wasmtime::{Engine, Result};
/// use wasmtime_wasi::{NamedId, WasiCtxNamedView};
/// use wasmtime_wasi::random::*;
/// use std::collections::HashMap;
///
/// struct MyStoreState {
///     states: HashMap<NamedId, WasiRandomCtx>,
/// }
///
/// fn main() -> Result<()> {
///     let engine = Engine::default();
///     let mut linker = Linker::new(&engine);
///     let component = Component::new(&engine, "(component)")?;
///     let mut name_map = HashMap::new();
///
///     wasmtime_wasi::p3::random::add_named_to_linker::<MyStoreState>(
///         &mut linker,
///         &component,
///         |_, name| {
///             let len = name_map.len();
///             Ok(NamedId(*name_map.entry(name.to_string()).or_insert(len)))
///         },
///     )?;
///     Ok(())
/// }
///
/// impl WasiRandomNamedView for MyStoreState {
///     fn random(&mut self, id: NamedId) -> &mut WasiRandomCtx {
///         self.states.get_mut(&id).expect("state for id")
///     }
/// }
/// ```
pub trait WasiRandomNamedView: Send + 'static {
    /// Looks up the [`WasiRandomCtx`] for the given [`NamedId`].
    ///
    /// This method will resolve the `id` specified to a specific random
    /// context that is available to be used. Note that this method is
    /// specifically infallible meaning that a random context must be returned
    /// and this cannot generate a trap or panic or similar.
    ///
    /// Embedders are responsible for allocating [`NamedId`] and assigning
    /// meaning to ids. When a `Linker` is populated embedders will have the
    /// ability to generate a `NamedId` for all imports found, and then that
    /// embedder-allocated id is then passed back here when the corresponding
    /// imported function is invoked.
    fn random(&mut self, id: NamedId) -> &mut WasiRandomCtx;
}
