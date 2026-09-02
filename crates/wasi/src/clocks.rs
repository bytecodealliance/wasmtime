use crate::{NamedId, WasiCtxNamedView};
use std::error::Error;
use std::fmt;
use std::marker;
use std::time::{Duration, Instant, SystemTime};
use wasmtime::component::{HasData, ResourceTable};

/// A helper struct which implements [`HasData`] for the `wasi:clocks` APIs.
///
/// This can be useful when directly calling `add_to_linker` functions directly,
/// such as [`wasmtime_wasi::p2::bindings::clocks::monotonic_clock::add_to_linker`] as
/// the `D` type parameter. See [`HasData`] for more information about the type
/// parameter's purpose.
///
/// When using this type you can skip the [`WasiClocksView`] trait, for
/// example.
///
/// [`wasmtime_wasi::p2::bindings::clocks::monotonic_clock::add_to_linker`]: crate::p2::bindings::clocks::monotonic_clock::add_to_linker
///
/// # Examples
///
/// ```
/// use wasmtime::component::{Linker, ResourceTable};
/// use wasmtime::{Engine, Result};
/// use wasmtime_wasi::clocks::*;
///
/// struct MyStoreState {
///     table: ResourceTable,
///     clocks: WasiClocksCtx,
/// }
///
/// fn main() -> Result<()> {
///     let engine = Engine::default();
///     let mut linker = Linker::new(&engine);
///
///     wasmtime_wasi::p2::bindings::clocks::monotonic_clock::add_to_linker::<MyStoreState, WasiClocks>(
///         &mut linker,
///         |state| WasiClocksCtxView {
///             table: &mut state.table,
///             ctx: &mut state.clocks,
///         },
///     )?;
///     Ok(())
/// }
/// ```
pub struct WasiClocks;

impl HasData for WasiClocks {
    type Data<'a> = WasiClocksCtxView<'a>;
}

pub struct WasiClocksCtx {
    pub(crate) wall_clock: Box<dyn HostWallClock + Send>,
    pub(crate) monotonic_clock: Box<dyn HostMonotonicClock + Send>,
}

impl Default for WasiClocksCtx {
    fn default() -> Self {
        Self {
            wall_clock: wall_clock(),
            monotonic_clock: monotonic_clock(),
        }
    }
}

pub trait WasiClocksView: Send {
    fn clocks(&mut self) -> WasiClocksCtxView<'_>;
}

pub struct WasiClocksCtxView<'a> {
    pub ctx: &'a mut WasiClocksCtx,
    pub table: &'a mut ResourceTable,
}

pub trait HostWallClock: Send {
    fn resolution(&self) -> Duration;
    fn now(&self) -> Duration;
}

pub trait HostMonotonicClock: Send {
    fn resolution(&self) -> u64;
    fn now(&self) -> u64;
}

#[derive(Default)]
pub struct WallClock;

impl WallClock {
    pub fn new() -> Self {
        Self
    }
}

impl HostWallClock for WallClock {
    fn resolution(&self) -> Duration {
        #[cfg(unix)]
        {
            let res = rustix::time::clock_getres(rustix::time::ClockId::Realtime);
            Duration::new(
                res.tv_sec.try_into().unwrap(),
                res.tv_nsec.try_into().unwrap(),
            )
        }
        #[cfg(windows)]
        {
            // According to [this blog post], the system timer resolution
            // is 55ms or 10ms. Use the more conservative of the two.
            //
            // [this blog post]: https://devblogs.microsoft.com/oldnewthing/20170921-00/?p=97057
            Duration::new(0, 55_000_000)
        }
    }

    fn now(&self) -> Duration {
        // WASI defines wall clocks to return "Unix time".
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
    }
}

pub struct MonotonicClock {
    /// The `Instant` this clock was created. All returned times are
    /// durations since that time.
    initial: Instant,
}

impl Default for MonotonicClock {
    fn default() -> Self {
        Self::new()
    }
}

impl MonotonicClock {
    pub fn new() -> Self {
        Self {
            initial: Instant::now(),
        }
    }
}

impl HostMonotonicClock for MonotonicClock {
    fn resolution(&self) -> u64 {
        #[cfg(unix)]
        {
            let res = rustix::time::clock_getres(rustix::time::ClockId::Monotonic);
            u64::try_from(res.tv_sec).unwrap() * 1_000_000_000 + u64::try_from(res.tv_nsec).unwrap()
        }
        #[cfg(windows)]
        {
            use windows_sys::Win32::System::Performance::QueryPerformanceFrequency;

            unsafe {
                let mut frequency = 0;
                if QueryPerformanceFrequency(&mut frequency) == 0 {
                    panic!(
                        "QueryPerformanceFrequency failed: {}",
                        std::io::Error::last_os_error()
                    );
                }
                1_000_000_000 / u64::try_from(frequency).unwrap()
            }
        }
    }

    fn now(&self) -> u64 {
        // Unwrap here and in `resolution` above; a `u64` is wide enough to
        // hold over 584 years of nanoseconds.
        Instant::now()
            .duration_since(self.initial)
            .as_nanos()
            .try_into()
            .unwrap()
    }
}

pub fn monotonic_clock() -> Box<dyn HostMonotonicClock + Send> {
    Box::new(MonotonicClock::default())
}

pub fn wall_clock() -> Box<dyn HostWallClock + Send> {
    Box::new(WallClock::default())
}

pub(crate) struct Datetime {
    pub seconds: i64,
    pub nanoseconds: u32,
}

impl TryFrom<SystemTime> for Datetime {
    type Error = DatetimeError;

    fn try_from(time: SystemTime) -> Result<Self, Self::Error> {
        let epoch = SystemTime::UNIX_EPOCH;

        if time >= epoch {
            let duration = time.duration_since(epoch)?;
            Ok(Self {
                seconds: duration.as_secs().try_into()?,
                nanoseconds: duration.subsec_nanos(),
            })
        } else {
            let duration = epoch.duration_since(time)?;
            Ok(Self {
                seconds: -duration.as_secs().try_into()?,
                nanoseconds: duration.subsec_nanos(),
            })
        }
    }
}

#[derive(Debug)]
pub struct DatetimeError;

impl fmt::Display for DatetimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("couldn't represent time as a WASI `Datetime`")
    }
}

impl Error for DatetimeError {}

impl From<std::time::SystemTimeError> for DatetimeError {
    fn from(_: std::time::SystemTimeError) -> Self {
        DatetimeError
    }
}

impl From<std::num::TryFromIntError> for DatetimeError {
    fn from(_: std::num::TryFromIntError) -> Self {
        DatetimeError
    }
}

/// A helper struct which implements [`HasData`] for the `wasi:clocks` APIs
/// when used in combination with named imports.
///
/// This structure is similar in purpose to [`WasiClocks`] and is used
/// when using the [`named_imports`] module for `wasi:clocks`. This structure
/// serves as the `D` type parameter for `add_to_linker` functions.
///
/// [`named_imports`]: crate::p3::bindings::named_imports::wasi::clocks
///
/// # Meaning of the `T` parameter
///
/// Here the `T` must be something that implements [`WasiClocksNamedView`]. The
/// corresponding `Data` for this type is [`WasiCtxNamedView`] which internally
/// will contain `&mut T`.
///
/// Effectively you're going to implement [`WasiClocksNamedView`] for something in
/// your embedding, and that's the `T` you'll fill in here.
///
/// # Examples
///
/// ```
/// use wasmtime::component::{Linker, Component, ResourceTable};
/// use wasmtime::{Engine, Result};
/// use wasmtime_wasi::{NamedId, WasiCtxNamedView};
/// use wasmtime_wasi::clocks::*;
/// use wasmtime_wasi::p2::bindings::named_imports;
/// use std::collections::HashMap;
///
/// struct MyStoreState {
///     table: ResourceTable,
///     states: HashMap<NamedId, WasiClocksCtx>,
/// }
///
/// fn main() -> Result<()> {
///     let engine = Engine::default();
///     let mut linker = Linker::new(&engine);
///     let component = Component::new(&engine, "(component)")?;
///     let mut name_map = HashMap::new();
///
///     named_imports::wasi::clocks::wall_clock::add_to_linker::<MyStoreState, WasiClocksNamed<MyStoreState>>(
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
/// impl WasiClocksNamedView for MyStoreState {
///     fn clocks(&mut self, id: NamedId) -> WasiClocksCtxView<'_> {
///         let ctx = self.states.get_mut(&id).expect("state for id");
///         WasiClocksCtxView {
///             table: &mut self.table,
///             ctx,
///         }
///     }
/// }
/// ```
pub struct WasiClocksNamed<T>(marker::PhantomData<fn() -> T>);

impl<T> HasData for WasiClocksNamed<T>
where
    T: WasiClocksNamedView,
{
    type Data<'a> = WasiCtxNamedView<'a, T>;
}

/// A trait used to look up a specific `wasi:clocks` context for a named
/// import.
///
/// This trait is used in conjunction with the [`named_imports`] bindings
/// generated for all WASI interfaces. The purpose of this trait is for
/// embedders to define how a [`NamedId`] maps to a particular `wasi:clocks`
/// context, here returned as [`WasiClocksCtxView`]. Embedders are responsible
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
/// [`add_named_to_linker`]: crate::p3::clocks::add_named_to_linker
/// [`WasiNamedView`]: crate::WasiNamedView
///
/// # Examples
///
/// ```
/// use wasmtime::component::{Linker, Component, ResourceTable};
/// use wasmtime::{Engine, Result};
/// use wasmtime_wasi::{NamedId, WasiCtxNamedView};
/// use wasmtime_wasi::clocks::*;
/// use std::collections::HashMap;
///
/// struct MyStoreState {
///     table: ResourceTable,
///     states: HashMap<NamedId, WasiClocksCtx>,
/// }
///
/// fn main() -> Result<()> {
///     let engine = Engine::default();
///     let mut linker = Linker::new(&engine);
///     let component = Component::new(&engine, "(component)")?;
///     let mut name_map = HashMap::new();
///
///     wasmtime_wasi::p3::clocks::add_named_to_linker::<MyStoreState>(
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
/// impl WasiClocksNamedView for MyStoreState {
///     fn clocks(&mut self, id: NamedId) -> WasiClocksCtxView<'_> {
///         let ctx = self.states.get_mut(&id).expect("state for id");
///         WasiClocksCtxView {
///             table: &mut self.table,
///             ctx,
///         }
///     }
/// }
/// ```
pub trait WasiClocksNamedView: Send + 'static {
    /// Looks up the [`WasiClocksCtxView`] for the given [`NamedId`].
    ///
    /// This method will resolve the `id` specified to a specific clocks
    /// context that is available to be used. Note that this method is
    /// specifically infallible meaning that a clocks context must be returned
    /// and this cannot generate a trap or panic or similar.
    ///
    /// Embedders are responsible for allocating [`NamedId`] and assigning
    /// meaning to ids. When a `Linker` is populated embedders will have the
    /// ability to generate a `NamedId` for all imports found, and then that
    /// embedder-allocated id is then passed back here when the corresponding
    /// imported function is invoked.
    ///
    /// Note that the [`ResourceTable`] referenced in the returned
    /// [`WasiClocksCtxView`] need not be unique. It's ok to use the same
    /// [`ResourceTable`] for all imports. This is not a guest-visible
    /// abstraction and just helps the host allocate and manage state.
    fn clocks(&mut self, id: NamedId) -> WasiClocksCtxView<'_>;
}
