#![deny(unsafe_op_in_unsafe_fn)]

use crate::Engine;
use crate::store::{Executor, StoreOpaque};
use crate::vm::mpk::{self, ProtectionMask};
use crate::vm::{AsyncWasmCallState, Interpreter, SendSyncPtr, VMStore};
use anyhow::{Result, anyhow};
use core::future;
use core::mem;
use core::ops::Range;
use core::pin::Pin;
use core::ptr::{self, NonNull};
use core::task::{Context, Poll};
use futures::channel::oneshot;
use wasmtime_environ::TripleExt;
use wasmtime_fiber::{Fiber, Suspend};

/// Helper struct for reseting a raw pointer to its original value on drop.
struct Reset<T: Copy>(*mut T, T);

impl<T: Copy> Drop for Reset<T> {
    fn drop(&mut self) {
        unsafe {
            *self.0 = self.1;
        }
    }
}

/// Represents the context of a `Future::poll` operation which involves resuming
/// a fiber.
///
/// See `self::poll_fn` for details.
#[derive(Clone, Copy)]
struct PollContext {
    future_context: *mut Context<'static>,
    guard_range_start: *mut u8,
    guard_range_end: *mut u8,
}

impl Default for PollContext {
    fn default() -> PollContext {
        PollContext {
            future_context: ptr::null_mut(),
            guard_range_start: ptr::null_mut(),
            guard_range_end: ptr::null_mut(),
        }
    }
}

/// Represents the state of a currently executing fiber which has been resumed
/// via `self::poll_fn`.
pub(crate) struct AsyncState {
    /// The `Suspend` for the current fiber (or null if no such fiber is running).
    ///
    /// See `StoreFiber` for an explanation of the signature types we use here.
    current_suspend: *mut Suspend<Result<()>, StoreFiberYield, Result<()>>,

    /// The current Wasm executor.
    ///
    /// Each fiber has its own executor, and we update this pointer to point to
    /// the appropriate one whenever we switch fibers.
    pub(crate) current_executor: *mut Executor,

    /// See `PollContext`
    current_poll_cx: PollContext,

    /// The last fiber stack that was in use by the store.
    ///
    /// We use this to cache and reuse stacks as a performance optimization.
    // TODO: With stack switching and the Component Model Async ABI, there may
    // be multiple concurrent fibers in play; consider caching more than one
    // stack at a time and making the number tunable via `Config`.
    pub(crate) last_fiber_stack: Option<wasmtime_fiber::FiberStack>,
}

impl Default for AsyncState {
    fn default() -> Self {
        Self {
            current_suspend: ptr::null_mut(),
            current_executor: ptr::null_mut(),
            current_poll_cx: PollContext::default(),
            last_fiber_stack: None,
        }
    }
}

impl AsyncState {
    pub(crate) fn async_guard_range(&self) -> Range<*mut u8> {
        let context = self.current_poll_cx;
        context.guard_range_start..context.guard_range_end
    }
}

// Lots of pesky unsafe cells and pointers in this structure. This means we need
// to declare explicitly that we use this in a threadsafe fashion.
unsafe impl Send for AsyncState {}
unsafe impl Sync for AsyncState {}

/// Used to "stackfully" poll a future by suspending the current fiber
/// repeatedly in a loop until the future completes.
pub(crate) struct AsyncCx {
    current_suspend: *mut *mut wasmtime_fiber::Suspend<Result<()>, StoreFiberYield, Result<()>>,
    current_stack_limit: *mut usize,
    current_poll_cx: *mut PollContext,
}

impl AsyncCx {
    /// Create a new `AsyncCx`.
    ///
    /// This will panic if called outside the scope of a `self::poll_fn` call.
    /// Consider using `Self::try_new` instead to avoid panicking.
    pub(crate) fn new(store: &mut StoreOpaque) -> Self {
        Self::try_new(store).unwrap()
    }

    /// Create a new `AsyncCx`.
    ///
    /// This will return `None` if called outside the scope of a `self::poll_fn`
    /// call.
    pub(crate) fn try_new(store: &mut StoreOpaque) -> Option<Self> {
        let current_poll_cx = unsafe { &raw mut (*store.async_state()).current_poll_cx };
        if unsafe { (*current_poll_cx).future_context.is_null() } {
            None
        } else {
            Some(Self {
                current_suspend: unsafe { &raw mut (*store.async_state()).current_suspend },
                current_stack_limit: store.vm_store_context().stack_limit.get(),
                current_poll_cx,
            })
        }
    }

    /// Poll the specified future using `Self::current_poll_cx`.
    ///
    /// This will panic if called recursively using the same `AsyncState`.
    fn poll<U>(&self, mut future: Pin<&mut (dyn Future<Output = U> + Send)>) -> Poll<U> {
        unsafe {
            let poll_cx = *self.current_poll_cx;
            let _reset = Reset(self.current_poll_cx, poll_cx);
            *self.current_poll_cx = PollContext::default();
            assert!(!poll_cx.future_context.is_null());
            future.as_mut().poll(&mut *poll_cx.future_context)
        }
    }

    /// Blocks on the asynchronous computation represented by `future` and
    /// produces the result here, in-line.
    ///
    /// This function is designed to only work when it's currently executing on
    /// a native fiber. This fiber provides the ability for us to handle the
    /// future's `Pending` state as "jump back to whomever called the fiber in
    /// an asynchronous fashion and propagate `Pending`". This tight coupling
    /// with `on_fiber` below is what powers the asynchronicity of calling wasm.
    ///
    /// This function takes a `future` and will (appear to) synchronously wait
    /// on the result. While this function is executing it will fiber switch
    /// to-and-from the original frame calling `on_fiber` which should be a
    /// guarantee due to how async stores are configured.
    ///
    /// The return value here is either the output of the future `T`, or a trap
    /// which represents that the asynchronous computation was cancelled. It is
    /// not recommended to catch the trap and try to keep executing wasm, so
    /// we've tried to liberally document this.
    ///
    /// Note that this function suspends (if needed) with
    /// `StoreFiberYield::KeepStore`, indicating that the store must not be used
    /// (and that no other fibers may be resumed) until this fiber resumes.
    /// Therefore, it is not appropriate for use in e.g. guest calls to
    /// async-lowered imports implemented as host functions, since it will
    /// prevent any other tasks from being run.  Use `Instance::suspend` to
    /// suspend and release the store to allow other tasks to run before this
    /// fiber is resumed.
    pub(crate) fn block_on<U>(
        &self,
        mut future: Pin<&mut (dyn Future<Output = U> + Send)>,
    ) -> Result<U> {
        loop {
            match self.poll(future.as_mut()) {
                Poll::Ready(v) => break Ok(v),
                Poll::Pending => {
                    self.suspend(StoreFiberYield::KeepStore)?;
                }
            }
        }
    }

    /// Suspend the current fiber, optionally transfering exclusive access to
    /// the store back to the code which resumed it.
    pub(crate) fn suspend(&self, yield_: StoreFiberYield) -> Result<()> {
        unsafe { suspend_fiber(self.current_suspend, self.current_stack_limit, yield_) }
    }
}

/// Indicates whether or not a fiber needs to retain exclusive access to its
/// store across a suspend/resume interval.
pub(crate) enum StoreFiberYield {
    /// Indicates the fiber needs to retain exclusive access, meaning the store
    /// should not be used outside of the fiber until after the fiber either
    /// suspends with `ReleaseStore` or resolves.
    KeepStore,
    /// Indicates the fiber does _not_ need exclusive access across the
    /// suspend/resume interval, meaning the store may be used as needed until
    /// the fiber is resumed.
    // TODO: This will be used once full `component-model-async` support is
    // merged:
    #[allow(dead_code)]
    ReleaseStore,
}

pub(crate) struct StoreFiber<'a> {
    /// The raw `wasmtime_fiber::Fiber`.
    ///
    /// Note that using `StoreFiberYield` as the `Yield` type parameter allows
    /// the fiber to indicate whether it needs exclusive access to the store
    /// across suspend points (in which case it will pass `KeepStore` when
    /// suspending , meaning the store must not be used at all until the fiber
    /// is resumed again) or whether it is giving up exclusive access (in which
    /// case it will pass `ReleaseStore` when yielding, meaning exclusive access
    /// may be given to another fiber that runs concurrently.
    ///
    /// Note also that every `StoreFiber` is implicitly granted exclusive access
    /// to the store when it is resumed.
    pub(crate) fiber: Option<Fiber<'a, Result<()>, StoreFiberYield, Result<()>>>,
    /// See `FiberResumeState`
    state: Option<FiberResumeState>,
    /// The Wasmtime `Engine` to which this fiber belongs.
    engine: Engine,
    /// The current `Suspend` for this fiber (or null if it's not currently
    /// running).
    suspend: *mut *mut Suspend<Result<()>, StoreFiberYield, Result<()>>,
    executor_ptr: *mut *mut Executor,
    executor: Executor,
}

impl StoreFiber<'_> {
    pub(crate) fn guard_range(&self) -> (Option<SendSyncPtr<u8>>, Option<SendSyncPtr<u8>>) {
        self.fiber
            .as_ref()
            .unwrap()
            .stack()
            .guard_range()
            .map(|r| {
                (
                    NonNull::new(r.start).map(SendSyncPtr::new),
                    NonNull::new(r.end).map(SendSyncPtr::new),
                )
            })
            .unwrap_or((None, None))
    }
}

// Here we run the risk of dropping an in-progress fiber, and if we were to do
// nothing then the fiber would leak all its owned stack resources.
//
// To handle this we implement `Drop` here and, if the fiber isn't done, resume
// execution of the fiber saying "hey please stop you're interrupted". Our
// `Trap` created here (which has the stack trace of whomever dropped us) should
// then get propagate all the way back up to the original fiber start, finishing
// execution.
//
// We don't actually care about the fiber's return value here (no one's around
// to look at it), we just assert the fiber finished to completion.
impl Drop for StoreFiber<'_> {
    fn drop(&mut self) {
        if self.fiber.is_none() {
            return;
        }

        if !self.fiber.as_ref().unwrap().done() {
            // SAFETY: We must temporarily grant the fiber exclusive access to
            // its store until resolves, meaning this function must only be
            // called from a context where that's sound.  As of this writing,
            // the only place unresolved fibers are dropped is in
            // `ComponentStoreData::drop_fibers` which does in fact have `&mut
            // StoreOpaque`.
            let result = unsafe { resume_fiber_raw(self, Err(anyhow!("future dropped"))) };
            debug_assert!(result.is_ok());
        }

        self.state.take().unwrap().dispose();

        unsafe {
            self.engine
                .allocator()
                .deallocate_fiber_stack(self.fiber.take().unwrap().into_stack());
        }
    }
}

// This is surely the most dangerous `unsafe impl Send` in the entire
// crate. There are two members in `FiberFuture` which cause it to not be
// `Send`. One is `current_poll_cx` and is entirely uninteresting.  This is just
// used to manage `Context` pointers across `await` points in the future, and
// requires raw pointers to get it to happen easily.  Nothing too weird about
// the `Send`-ness, values aren't actually crossing threads.
//
// The really interesting piece is `fiber`. Now the "fiber" here is actual
// honest-to-god Rust code which we're moving around. What we're doing is the
// equivalent of moving our thread's stack to another OS thread. Turns out we,
// in general, have no idea what's on the stack and would generally have no way
// to verify that this is actually safe to do!
//
// Thankfully, though, Wasmtime has the power. Without being glib it's actually
// worth examining what's on the stack. It's unfortunately not super-local to
// this function itself. Our closure to `Fiber::new` runs `func`, which is given
// to us from the outside. Thankfully, though, we have tight control over
// this. Usage of `on_fiber` or `Instance::resume_fiber` is typically done
// *just* before entering WebAssembly itself, so we'll have a few stack frames
// of Rust code (all in Wasmtime itself) before we enter wasm.
//
// Once we've entered wasm, well then we have a whole bunch of wasm frames on
// the stack. We've got this nifty thing called Cranelift, though, which allows
// us to also have complete control over everything on the stack!
//
// Finally, when wasm switches back to the fiber's starting pointer (this future
// we're returning) then it means wasm has reentered Rust.  Suspension can only
// happen via either `block_on` or `Instance::suspend`. This, conveniently, also
// happens entirely in Wasmtime controlled code!
//
// There's an extremely important point that should be called out here.
// User-provided futures **are not on the stack** during suspension points. This
// is extremely crucial because we in general cannot reason about Send/Sync for
// stack-local variables since rustc doesn't analyze them at all. With our
// construction, though, we are guaranteed that Wasmtime owns all stack frames
// between the stack of a fiber and when the fiber suspends (and it could move
// across threads). At this time the only user-provided piece of data on the
// stack is the future itself given to us. Lo-and-behold as you might notice the
// future is required to be `Send`!
//
// What this all boils down to is that we, as the authors of Wasmtime, need to
// be extremely careful that on the async fiber stack we only store Send
// things. For example we can't start using `Rc` willy nilly by accident and
// leave a copy in TLS somewhere. (similarly we have to be ready for TLS to
// change while we're executing wasm code between suspension points).
//
// While somewhat onerous it shouldn't be too too hard (the TLS bit is the
// hardest bit so far). This does mean, though, that no user should ever have to
// worry about the `Send`-ness of Wasmtime. If rustc says it's ok, then it's ok.
//
// With all that in mind we unsafely assert here that Wasmtime is correct. We
// declare the fiber as only containing Send data on its stack, despite not
// knowing for sure at compile time that this is correct. That's what `unsafe`
// in Rust is all about, though, right?
unsafe impl Send for StoreFiber<'_> {}
// See the docs about the `Send` impl above, which also apply to this `Sync`
// impl.  `Sync` is needed since we store `StoreFiber`s and switch between them
// when executing components that export async-lifted functions.
unsafe impl Sync for StoreFiber<'_> {}

/// State of the world when a fiber last suspended.
///
/// This structure represents global state that a fiber clobbers during its
/// execution. For example TLS variables are updated, system resources like MPK
/// masks are updated, etc. The purpose of this structure is to track all of
/// this state and appropriately save/restore it around fiber suspension points.
struct FiberResumeState {
    /// Saved list of `CallThreadState` activations that are stored on a fiber
    /// stack.
    ///
    /// This is a linked list that references stack-stored nodes on the fiber
    /// stack that is currently suspended. The `AsyncWasmCallState` type
    /// documents this more thoroughly but the general gist is that when we this
    /// fiber is resumed this linked list needs to be pushed on to the current
    /// thread's linked list of activations.
    tls: crate::runtime::vm::AsyncWasmCallState,

    /// Saved MPK protection mask, if enabled.
    ///
    /// When MPK is enabled then executing WebAssembly will modify the
    /// processor's current mask of addressable protection keys. This means that
    /// our current state may get clobbered when a fiber suspends. To ensure
    /// that this function preserves context it will, when MPK is enabled, save
    /// the current mask when this function is called and then restore the mask
    /// when the function returns (aka the fiber suspends).
    mpk: Option<ProtectionMask>,
}

impl FiberResumeState {
    unsafe fn replace(self) -> PriorFiberResumeState {
        let tls = unsafe { self.tls.push() };
        let mpk = swap_mpk_states(self.mpk);
        PriorFiberResumeState { tls, mpk }
    }

    fn dispose(self) {
        self.tls.assert_null();
    }
}

struct PriorFiberResumeState {
    tls: crate::runtime::vm::PreviousAsyncWasmCallState,
    mpk: Option<ProtectionMask>,
}

impl PriorFiberResumeState {
    unsafe fn replace(self) -> FiberResumeState {
        let tls = unsafe { self.tls.restore() };
        let mpk = swap_mpk_states(self.mpk);
        FiberResumeState { tls, mpk }
    }
}

fn swap_mpk_states(mask: Option<ProtectionMask>) -> Option<ProtectionMask> {
    mask.map(|mask| {
        let current = mpk::current_mask();
        mpk::allow(mask);
        current
    })
}

/// Resume the specified fiber, granting it exclusive access to the store with
/// which it was created.
///
/// This will return `Ok(result)` if the fiber resolved, where `result` is the
/// returned value; it will return `Err(yield_)` if the fiber suspended, where
/// `yield_` indicates whether it released access to the store or not.  See
/// `StoreFiber::fiber` for details.
///
/// SAFETY: The caller must confer exclusive access to the store to the fiber
/// until the fiber is either dropped, resolved, or forgotten, or until it
/// releases the store when suspending.
unsafe fn resume_fiber_raw<'a>(
    fiber: &mut StoreFiber<'a>,
    result: Result<()>,
) -> Result<Result<()>, StoreFiberYield> {
    struct Restore<'a, 'b> {
        fiber: &'b mut StoreFiber<'a>,
        state: Option<PriorFiberResumeState>,
    }

    impl Drop for Restore<'_, '_> {
        fn drop(&mut self) {
            unsafe {
                self.fiber.state = Some(self.state.take().unwrap().replace());
            }
        }
    }
    unsafe {
        let _reset_executor = Reset(fiber.executor_ptr, *fiber.executor_ptr);
        *fiber.executor_ptr = &raw mut fiber.executor;
        let _reset_suspend = Reset(fiber.suspend, *fiber.suspend);
        let prev = fiber.state.take().unwrap().replace();
        let restore = Restore {
            fiber,
            state: Some(prev),
        };
        restore.fiber.fiber.as_ref().unwrap().resume(result)
    }
}

/// Create a new `StoreFiber` which runs the specified closure.
pub(crate) fn make_fiber<'a>(
    store: &mut dyn VMStore,
    fun: impl FnOnce(&mut dyn VMStore) -> Result<()> + 'a,
) -> Result<StoreFiber<'a>> {
    let engine = store.engine().clone();
    #[cfg(has_host_compiler_backend)]
    let executor = if cfg!(feature = "pulley") && engine.target().is_pulley() {
        Executor::Interpreter(Interpreter::new(&engine))
    } else {
        Executor::Native
    };
    #[cfg(not(has_host_compiler_backend))]
    let executor = {
        debug_assert!(engine.target().is_pulley());
        Executor::Interpreter(Interpreter::new(&engine))
    };
    let stack = store.store_opaque_mut().allocate_fiber_stack()?;
    let suspend = unsafe { &raw mut (*store.store_opaque_mut().async_state()).current_suspend };
    let executor_ptr =
        unsafe { &raw mut (*store.store_opaque_mut().async_state()).current_executor };
    let track_pkey_context_switch = store.has_pkey();
    let store = &raw mut *store;
    Ok(StoreFiber {
        fiber: Some(Fiber::new(stack, move |result: Result<()>, suspend| {
            if result.is_err() {
                result
            } else {
                // SAFETY: Per the documented contract for
                // `resume_fiber_raw`, we've been given exclusive access to
                // the store until we exit or yield it back to the resumer.
                let store_ref = unsafe { &mut *store };
                let suspend_ptr = unsafe {
                    &raw mut (*store_ref.store_opaque_mut().async_state()).current_suspend
                };
                // Configure our store's suspension context for the rest of the
                // execution of this fiber. Note that a raw pointer is stored here
                // which is only valid for the duration of this closure.
                // Consequently we at least replace it with the previous value when
                // we're done. This reset is also required for correctness because
                // otherwise our value will overwrite another active fiber's value.
                // There should be a test that segfaults in `async_functions.rs` if
                // this `Reset` is removed.
                //
                // SAFETY: The resumer is responsible for setting
                // `current_suspend` to a valid pointer.
                let _reset = Reset(suspend_ptr, unsafe { *suspend_ptr });
                unsafe { *suspend_ptr = suspend };
                fun(store_ref)
            }
        })?),
        state: Some(FiberResumeState {
            tls: crate::runtime::vm::AsyncWasmCallState::new(),
            mpk: if track_pkey_context_switch {
                Some(ProtectionMask::all())
            } else {
                None
            },
        }),
        engine,
        suspend,
        executor_ptr,
        executor,
    })
}

/// See `resume_fiber_raw`
pub(crate) unsafe fn resume_fiber(
    fiber: &mut StoreFiber,
    result: Result<()>,
) -> Result<Result<()>, StoreFiberYield> {
    match unsafe { resume_fiber_raw(fiber, result) } {
        Ok(result) => Ok(result),
        Err(yield_) => {
            // If `Err` is returned that means the fiber suspended, so we
            // propagate that here.
            //
            // An additional safety check is performed when leaving this
            // function to help bolster the guarantees of `unsafe impl Send`
            // above. Notably this future may get re-polled on a different
            // thread. Wasmtime's thread-local state points to the stack,
            // however, meaning that it would be incorrect to leave a pointer in
            // TLS when this function returns. This function performs a runtime
            // assert to verify that this is the case, notably that the one TLS
            // pointer Wasmtime uses is not pointing anywhere within the
            // stack. If it is then that's a bug indicating that TLS management
            // in Wasmtime is incorrect.
            if let Some(range) = fiber.fiber.as_ref().unwrap().stack().range() {
                AsyncWasmCallState::assert_current_state_not_in_range(range);
            }

            Err(yield_)
        }
    }
}

/// Suspend the current fiber, optionally returning exclusive access to the
/// specified store to the code which resumed the fiber.
///
/// SAFETY: `suspend` must be a valid pointer.  Additionally, if a store pointer
/// is provided, the fiber must give up access to the store until it is given
/// back access when next resumed.
unsafe fn suspend_fiber(
    suspend: *mut *mut Suspend<Result<()>, StoreFiberYield, Result<()>>,
    stack_limit: *mut usize,
    yield_: StoreFiberYield,
) -> Result<()> {
    // Take our current `Suspend` context which was configured as soon as our
    // fiber started. Note that we must load it at the front here and save it on
    // our stack frame. While we're polling the future other fibers may be
    // started for recursive computations, and the current suspend context is
    // only preserved at the edges of the fiber, not during the fiber itself.
    //
    // For a little bit of extra safety we also replace the current value with
    // null to try to catch any accidental bugs on our part early.  This is all
    // pretty unsafe so we're trying to be careful...
    //
    // Note that there should be a segfaulting test in `async_functions.rs` if
    // this `Reset` is removed.
    unsafe {
        let reset_suspend = Reset(suspend, *suspend);
        *suspend = ptr::null_mut();
        let _reset_stack_limit = Reset(stack_limit, *stack_limit);
        assert!(!(reset_suspend.1).is_null());
        (*reset_suspend.1).suspend(yield_)
    }
}

/// Run the specified function on a newly-created fiber and `.await` its
/// completion.
pub(crate) async fn on_fiber<R: Send>(
    store: &mut StoreOpaque,
    func: impl FnOnce(&mut StoreOpaque) -> R + Send,
) -> Result<R> {
    on_fiber_raw(store.traitobj_mut(), move |store| {
        func((*store).store_opaque_mut())
    })
    .await
}

/// Wrap the specified function in a fiber and return it.
fn prepare_fiber<'a, R: Send + 'a>(
    store: &mut dyn VMStore,
    func: impl FnOnce(&mut dyn VMStore) -> R + Send + 'a,
) -> Result<(StoreFiber<'a>, oneshot::Receiver<R>)> {
    let (tx, rx) = oneshot::channel();
    let fiber = make_fiber(store, {
        move |store| {
            _ = tx.send(func(store));
            Ok(())
        }
    })?;
    Ok((fiber, rx))
}

/// Run the specified function on a newly-created fiber and `.await` its
/// completion.
async fn on_fiber_raw<R: Send>(
    store: &mut StoreOpaque,
    func: impl FnOnce(&mut dyn VMStore) -> R + Send,
) -> Result<R> {
    let config = store.engine().config();
    debug_assert!(store.async_support());
    debug_assert!(config.async_stack_size > 0);

    let (fiber, mut rx) = prepare_fiber(store.traitobj_mut(), func)?;

    let guard_range = fiber.guard_range();
    let mut fiber = Some(fiber);
    let mut fiber = poll_fn(store, guard_range, move || {
        // SAFETY: We confer exclusive access to the store to the fiber here,
        // only taking it back when the fiber resolves.
        match unsafe { resume_fiber(fiber.as_mut().unwrap(), Ok(())) } {
            Ok(result) => Poll::Ready(result.map(|()| fiber.take().unwrap())),
            Err(_) => Poll::Pending,
        }
    })
    .await?;

    let stack = fiber.fiber.take().map(|f| f.into_stack());
    drop(fiber);
    if let Some(stack) = stack {
        store.deallocate_fiber_stack(stack);
    }

    Ok(rx.try_recv().unwrap().unwrap())
}

/// Wrap the specified function in a future which, when polled, will store a
/// pointer to the `Context` in the `AsyncState::current_poll_cx` field for the
/// specified store and then call the function.
///
/// This is intended for use with functions that resume fibers which may need to
/// poll futures using the stored `Context` pointer.
pub(crate) async fn poll_fn<R>(
    store: &mut StoreOpaque,
    guard_range: (Option<SendSyncPtr<u8>>, Option<SendSyncPtr<u8>>),
    mut fun: impl FnMut() -> Poll<R>,
) -> R {
    #[derive(Clone, Copy)]
    struct PollCx(*mut PollContext);

    unsafe impl Send for PollCx {}

    let poll_cx = PollCx(unsafe { &raw mut (*store.async_state()).current_poll_cx });
    future::poll_fn({
        move |cx| {
            let _reset = Reset(poll_cx.0, unsafe { *poll_cx.0 });
            let guard_range_start = guard_range.0.map(|v| v.as_ptr()).unwrap_or(ptr::null_mut());
            let guard_range_end = guard_range.1.map(|v| v.as_ptr()).unwrap_or(ptr::null_mut());
            // We need to carry over this `cx` into our fiber's runtime for when
            // it tries to poll sub-futures that are created. Doing this must be
            // done unsafely, however, since `cx` is only alive for this one
            // singular function call. Here we do a `transmute` to extend the
            // lifetime of `Context` so it can be stored in our `Store`, and
            // then we replace the current polling context with this one.
            //
            // Note that the replace is done for weird situations where futures
            // might be switching contexts and there's multiple wasmtime futures
            // in a chain of futures.
            //
            // On exit from this function, though, we reset the polling context
            // back to what it was to signify that `Store` no longer has access
            // to this pointer.
            //
            // SAFETY: We store the pointer to the `Context` only for the
            // duration of this call and then reset it to its previous value
            // afterward, thereby ensuring `fun` never sees a stale pointer.
            unsafe {
                *poll_cx.0 = PollContext {
                    future_context: mem::transmute::<&mut Context<'_>, *mut Context<'static>>(cx),
                    guard_range_start,
                    guard_range_end,
                };
            }
            #[allow(dropping_copy_types)]
            drop(poll_cx);

            fun()
        }
    })
    .await
}
