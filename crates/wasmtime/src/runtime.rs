// Wasmtime's runtime has lots of fiddly bits where we're doing operations like
// casting between wasm i32/i64 and host `usize` values. There's also in general
// just lots of pieces of low-level manipulation of memory and internals of VM
// runtime state. To help keep all the integer casts correct be a bit more
// strict than the default settings to help weed out bugs ahead of time.
//
// This inevitably leads to wordier code than might otherwise be used because,
// for example, `u64 as usize` is warned against and will be an error on CI.
// This happens pretty frequently and needs to be replaced with `val.try_into()`
// or `usize::try_from(val)` where the error is handled. In some cases the
// correct thing to do is to `.unwrap()` the error to indicate a fatal mistake,
// but in some cases the correct thing is to propagate the error.
//
// Some niche cases that explicitly want truncation are recommended to have a
// function along the lines of
//
//     #[allow(clippy::cast_possible_truncation)]
//     fn truncate_i32_to_i8(a: i32) -> i8 { a as i8 }
//
// as this explicitly indicates the intent of truncation is desired. Other
// locations should use fallible conversions.
//
// If performance is absolutely critical then it's recommended to use `#[allow]`
// with a comment indicating why performance is critical as well as a short
// explanation of why truncation shouldn't be happening at runtime. This
// situation should be pretty rare though.
#![warn(clippy::cast_possible_truncation)]

use crate::prelude::*;
use core::marker;
use core::pin::Pin;
use core::task::{Context, Poll};

#[cfg(feature = "component-model-async")]
mod bug;

#[macro_use]
pub(crate) mod func;

pub(crate) mod code;
pub(crate) mod code_memory;
#[cfg(feature = "debug")]
pub(crate) mod debug;
#[cfg(feature = "gc")]
pub(crate) mod exception;
pub(crate) mod externals;
#[cfg(feature = "async")]
pub(crate) mod fiber;
pub(crate) mod gc;
pub(crate) mod instance;
pub(crate) mod instantiate;
pub(crate) mod limits;
pub(crate) mod linker;
pub(crate) mod memory;
pub(crate) mod module;
#[cfg(feature = "debug-builtins")]
pub(crate) mod native_debug;
pub(crate) mod resources;
pub(crate) mod store;
pub(crate) mod trampoline;
pub(crate) mod trap;
pub(crate) mod type_registry;
pub(crate) mod types;
pub(crate) mod v128;
pub(crate) mod values;
pub(crate) mod vm;

#[cfg(feature = "component-model")]
pub mod component;

cfg_if::cfg_if! {
    if #[cfg(miri)] {
        // no extensions on miri
    } else if #[cfg(not(feature = "std"))] {
        // no extensions on no-std
    } else if #[cfg(unix)] {
        pub mod unix;
    } else if #[cfg(windows)] {
        pub mod windows;
    } else {
        // ... unknown os!
    }
}

#[cfg(feature = "component-model-async")]
pub use bug::WasmtimeBug;
#[cfg(feature = "component-model-async")]
pub(crate) use bug::bail_bug;
pub use code_memory::CodeMemory;
#[cfg(feature = "debug")]
pub use debug::*;
#[cfg(feature = "gc")]
pub use exception::*;
pub use externals::*;
pub use func::*;
pub use gc::*;
pub use instance::{Instance, InstancePre};
pub use instantiate::CompiledModule;
pub use limits::*;
pub use linker::*;
pub use memory::*;
pub use module::{Module, ModuleExport};
pub use resources::*;
#[cfg(all(feature = "async", feature = "call-hook"))]
pub use store::CallHookHandler;
pub use store::{
    AsContext, AsContextMut, CallHook, Store, StoreContext, StoreContextMut, UpdateDeadline,
};
pub use trap::*;
pub use types::*;
pub use v128::V128;
pub use values::*;

#[cfg(feature = "pooling-allocator")]
pub use vm::{PoolConcurrencyLimitError, PoolingAllocatorMetrics};

#[cfg(feature = "profiling")]
mod profiling;
#[cfg(feature = "profiling")]
pub use profiling::GuestProfiler;

#[cfg(feature = "async")]
pub(crate) mod stack;
#[cfg(feature = "async")]
pub use stack::*;

#[cfg(feature = "coredump")]
mod coredump;
#[cfg(feature = "coredump")]
pub use coredump::*;

#[cfg(feature = "wave")]
mod wave;

/// Helper method to create a future trait object from the future `F` provided.
///
/// This requires that the output of `F` is a result where the error can be
/// created from `OutOfMemory`. This will handle OOM when allocation of `F` on
/// the heap fails.
fn box_future<'a, F, T, E>(future: F) -> Pin<Box<dyn Future<Output = Result<T, E>> + Send + 'a>>
where
    F: Future<Output = Result<T, E>> + Send + 'a,
    T: 'a,
    E: From<OutOfMemory> + 'a,
{
    if let Ok(future) = try_new::<Box<F>>(future) {
        return Pin::from(future);
    }

    // Use a custom guaranteed-zero-size struct to implement a future that
    // returns an OOM error which satisfies the type signature of this function.
    struct OomFuture<F, T, E>(marker::PhantomData<fn() -> (T, F, E)>);

    impl<F, T, E> Future for OomFuture<F, T, E>
    where
        E: From<OutOfMemory>,
    {
        type Output = Result<T, E>;

        fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
            Poll::Ready(Err(OutOfMemory::new(size_of::<F>()).into()))
        }
    }

    // Zero-size allocations don't actually allocate memory with a `Box`, so
    // it's ok to use the standard `Box::pin` here and not worry about OOM.
    let future = OomFuture::<F, T, E>(marker::PhantomData);
    assert_eq!(size_of_val(&future), 0);
    Box::pin(future)
}

fn _assertions_runtime() {
    use crate::_assert_send_and_sync;

    #[cfg(feature = "async")]
    fn _assert_send<T: Send>(_t: T) {}

    _assert_send_and_sync::<Caller<'_, ()>>();
    _assert_send_and_sync::<ExternRef>();
    _assert_send_and_sync::<(Func, TypedFunc<(), ()>, Global, Table, Memory)>();
    _assert_send_and_sync::<Instance>();
    _assert_send_and_sync::<InstancePre<()>>();
    _assert_send_and_sync::<InstancePre<*mut u8>>();
    _assert_send_and_sync::<Linker<()>>();
    _assert_send_and_sync::<Linker<*mut u8>>();
    _assert_send_and_sync::<Module>();
    _assert_send_and_sync::<Store<()>>();
    _assert_send_and_sync::<StoreContext<'_, ()>>();
    _assert_send_and_sync::<StoreContextMut<'_, ()>>();

    #[cfg(feature = "async")]
    fn _call_async(s: &mut Store<()>, f: Func) {
        _assert_send(f.call_async(&mut *s, &[], &mut []))
    }
    #[cfg(feature = "async")]
    fn _typed_call_async(s: &mut Store<()>, f: TypedFunc<(), ()>) {
        _assert_send(f.call_async(&mut *s, ()))
    }
    #[cfg(feature = "async")]
    fn _instantiate_async(s: &mut Store<()>, m: &Module) {
        _assert_send(Instance::new_async(s, m, &[]))
    }
}
