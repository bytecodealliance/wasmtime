#![deny(unsafe_op_in_unsafe_fn)]

use crate::Engine;
use crate::store::{Executor, StoreId, StoreOpaque};
use crate::vm::mpk::{self, ProtectionMask};
use crate::vm::{AsyncWasmCallState, VMStore};
use anyhow::{Result, anyhow};
use core::mem;
use core::ops::Range;
use core::pin::Pin;
use core::ptr;
use core::task::{Context, Poll};
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
    current_suspend: *mut Suspend<Result<()>, StoreFiberYield, ()>,

    /// See `PollContext`
    current_poll_cx: PollContext,

    /// The last fiber stack that was in use by the store.
    ///
    /// We use this to cache and reuse stacks as a performance optimization.
    // TODO: With stack switching and the Component Model Async ABI, there may
    // be multiple concurrent fibers in play; consider caching more than one
    // stack at a time and making the number tunable via `Config`.
    last_fiber_stack: Option<wasmtime_fiber::FiberStack>,
}

impl Default for AsyncState {
    fn default() -> Self {
        Self {
            current_suspend: ptr::null_mut(),
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

    pub(crate) fn last_fiber_stack(&mut self) -> &mut Option<wasmtime_fiber::FiberStack> {
        &mut self.last_fiber_stack
    }
}

// Lots of pesky unsafe cells and pointers in this structure. This means we need
// to declare explicitly that we use this in a threadsafe fashion.
unsafe impl Send for AsyncState {}
unsafe impl Sync for AsyncState {}

/// Used to "stackfully" poll a future by suspending the current fiber
/// repeatedly in a loop until the future completes.
pub(crate) struct AsyncCx {
    current_suspend: *mut *mut Suspend<Result<()>, StoreFiberYield, ()>,
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
                current_poll_cx,
            })
        }
    }

    /// Poll the specified future using `Self::current_poll_cx`.
    ///
    /// This will panic if called recursively using the same `AsyncState`.
    ///
    /// SAFETY: `self` contains pointers into the `Store` with which it was
    /// created and must not be used after that `Store` has been disposed.
    unsafe fn poll<U>(&self, mut future: Pin<&mut (dyn Future<Output = U> + Send)>) -> Poll<U> {
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
    ///
    /// SAFETY: `self` contains pointers into the `Store` with which it was
    /// created and must not be used after that `Store` has been disposed.
    pub(crate) unsafe fn block_on<U>(
        &self,
        mut future: Pin<&mut (dyn Future<Output = U> + Send)>,
    ) -> Result<U> {
        loop {
            match unsafe { self.poll(future.as_mut()) } {
                Poll::Ready(v) => break Ok(v),
                Poll::Pending => unsafe { self.suspend(StoreFiberYield::KeepStore)? },
            }
        }
    }

    /// Suspend the current fiber, optionally transfering exclusive access to
    /// the store back to the code which resumed it.
    ///
    /// SAFETY: `self` contains pointers into the `Store` with which it was
    /// created and must not be used after that `Store` has been disposed.
    pub(crate) unsafe fn suspend(&self, yield_: StoreFiberYield) -> Result<()> {
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
            let reset_suspend = Reset(self.current_suspend, *self.current_suspend);
            *self.current_suspend = ptr::null_mut();
            assert!(!(reset_suspend.1).is_null());
            (*reset_suspend.1).suspend(yield_)
        }
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
    fiber: Option<Fiber<'a, Result<()>, StoreFiberYield, ()>>,
    /// See `FiberResumeState`
    state: Option<FiberResumeState>,
    /// The Wasmtime `Engine` to which this fiber belongs.
    engine: Engine,
    /// The current `Suspend` for this fiber (or null if it's not currently
    /// running).
    suspend: *mut *mut Suspend<Result<()>, StoreFiberYield, ()>,
    /// The executor (e.g. the Pulley interpreter state) belonging to this
    /// fiber.
    ///
    /// This is swapped with `StoreOpaque::executor` whenever this fiber is
    /// resumed, suspended, or resolved.
    executor: Executor,
    /// The id of the store with which this fiber was created.
    ///
    /// Any attempt to resume a fiber with a different store than the one with
    /// which it was created will panic.
    id: StoreId,
}

impl StoreFiber<'_> {
    fn dispose(&mut self, store: &mut StoreOpaque) {
        if let Some(fiber) = &mut self.fiber {
            if !fiber.done() {
                let result = resume_fiber(store, self, Err(anyhow!("future dropped")));
                debug_assert!(result.is_ok());
            }
        }
    }
}

// Note that this implementation will panic if the fiber is in-progress, which
// will abort the process if there is already a panic being unwound.  That
// should only happen if we failed to call `StoreFiber::dispose` on the
// in-progress fiber prior to dropping it, which indicates a bug in this crate
// which must be fixed.
impl Drop for StoreFiber<'_> {
    fn drop(&mut self) {
        if self.fiber.is_none() {
            return;
        }

        assert!(
            self.fiber.as_ref().unwrap().done(),
            "attempted to drop in-progress fiber without first calling `StoreFiber::dispose`"
        );

        self.state.take().unwrap().dispose();

        unsafe {
            self.engine
                .allocator()
                .deallocate_fiber_stack(self.fiber.take().unwrap().into_stack());
        }
    }
}

// This is surely the most dangerous `unsafe impl Send` in the entire
// crate. There are two members in `StoreFiber` which cause it to not be
// `Send`. One is `suspend` and is entirely uninteresting.  This is just used to
// manage `Suspend` when resuming, and requires raw pointers to get it to happen
// easily.  Nothing too weird about the `Send`-ness, values aren't actually
// crossing threads.
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

    /// The current wasm stack limit, if in use.
    ///
    /// This field stores the old of `VMStoreContext::stack_limit` that this
    /// fiber should be using during its execution. This is saved/restored when
    /// a fiber is suspended/resumed to ensure that when there are multiple
    /// fibers within the store they all maintain an appropriate fiber-relative
    /// stack limit.
    stack_limit: usize,
}

impl FiberResumeState {
    unsafe fn replace(self, store: &mut StoreOpaque) -> PriorFiberResumeState {
        let tls = unsafe { self.tls.push() };
        let mpk = swap_mpk_states(self.mpk);
        PriorFiberResumeState {
            tls,
            mpk,
            stack_limit: store.replace_stack_limit(self.stack_limit),
        }
    }

    fn dispose(self) {
        self.tls.assert_null();
    }
}

impl StoreOpaque {
    /// Helper function to swap the `stack_limit` field in the `VMStoreContext`
    /// within this store.
    fn replace_stack_limit(&mut self, stack_limit: usize) -> usize {
        // SAFETY: the `VMStoreContext` points to within this store itself but
        // is accessed through raw pointers to assist with Miri. The `&mut
        // StoreOpaque` passed to this function shows that this has permission
        // to mutate state in the store, however.
        unsafe { mem::replace(&mut *self.vm_store_context().stack_limit.get(), stack_limit) }
    }
}

struct PriorFiberResumeState {
    tls: crate::runtime::vm::PreviousAsyncWasmCallState,
    mpk: Option<ProtectionMask>,
    stack_limit: usize,
}

impl PriorFiberResumeState {
    unsafe fn replace(self, store: &mut StoreOpaque) -> FiberResumeState {
        let tls = unsafe { self.tls.restore() };
        let mpk = swap_mpk_states(self.mpk);
        FiberResumeState {
            tls,
            mpk,
            stack_limit: store.replace_stack_limit(self.stack_limit),
        }
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
pub(crate) fn resume_fiber<'a>(
    store: &mut StoreOpaque,
    fiber: &mut StoreFiber<'a>,
    result: Result<()>,
) -> Result<(), StoreFiberYield> {
    assert_eq!(store.id(), fiber.id);

    struct Restore<'a, 'b> {
        store: &'b mut StoreOpaque,
        fiber: &'b mut StoreFiber<'a>,
        state: Option<PriorFiberResumeState>,
    }

    impl Drop for Restore<'_, '_> {
        fn drop(&mut self) {
            self.fiber.state = Some(unsafe { self.state.take().unwrap().replace(self.store) });
            self.store.swap_executor(&mut self.fiber.executor);
        }
    }
    let result = unsafe {
        let _reset_suspend = Reset(fiber.suspend, *fiber.suspend);
        let prev = fiber.state.take().unwrap().replace(store);
        store.swap_executor(&mut fiber.executor);
        let restore = Restore {
            store,
            fiber,
            state: Some(prev),
        };
        restore.fiber.fiber.as_ref().unwrap().resume(result)
    };

    match &result {
        // The fiber has finished, so recycle its stack by disposing of the
        // underlying fiber itself.
        Ok(_) => {
            let stack = fiber.fiber.take().map(|f| f.into_stack());
            if let Some(stack) = stack {
                store.deallocate_fiber_stack(stack);
            }
        }

        // The fiber has not yet finished, so it stays as-is.
        Err(_) => {
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
        }
    }

    result
}

/// Create a new `StoreFiber` which runs the specified closure.
pub(crate) fn make_fiber<'a>(
    store: &mut dyn VMStore,
    fun: impl FnOnce(&mut dyn VMStore) + Send + Sync + 'a,
) -> Result<StoreFiber<'a>> {
    let engine = store.engine().clone();
    let executor = Executor::new(&engine);
    let id = store.store_opaque().id();
    let stack = store.store_opaque_mut().allocate_fiber_stack()?;
    let suspend = unsafe { &raw mut (*store.store_opaque_mut().async_state()).current_suspend };
    let track_pkey_context_switch = store.has_pkey();
    let store = &raw mut *store;
    Ok(StoreFiber {
        fiber: Some(Fiber::new(stack, move |result: Result<()>, suspend| {
            // Cancelled before we started? Just return.
            if result.is_err() {
                return;
            }

            // SAFETY: Per the documented contract for
            // `resume_fiber`, we've been given exclusive access to
            // the store until we exit or yield it back to the resumer.
            let store_ref = unsafe { &mut *store };
            let suspend_ptr =
                unsafe { &raw mut (*store_ref.store_opaque_mut().async_state()).current_suspend };
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
        })?),
        state: Some(FiberResumeState {
            tls: crate::runtime::vm::AsyncWasmCallState::new(),
            mpk: if track_pkey_context_switch {
                Some(ProtectionMask::all())
            } else {
                None
            },
            stack_limit: usize::MAX,
        }),
        engine,
        suspend,
        executor,
        id,
    })
}

/// Run the specified function on a newly-created fiber and `.await` its
/// completion.
pub(crate) async fn on_fiber<R: Send + Sync>(
    store: &mut StoreOpaque,
    func: impl FnOnce(&mut StoreOpaque) -> R + Send + Sync,
) -> Result<R> {
    let config = store.engine().config();
    debug_assert!(store.async_support());
    debug_assert!(config.async_stack_size > 0);

    let mut result = None;
    let fiber = make_fiber(store.traitobj_mut(), |store| result = Some(func(store)))?;

    FiberFuture { store, fiber }.await;

    Ok(result.unwrap())
}

/// A `Future` implementation for running a `StoreFiber` to completion, giving
/// it exclusive access to its store until it resolves.
///
/// This is used to implement `on_fiber`, where the returned `Future` closes
/// over the `&mut StoreOpaque`.  It is not appropriate for use with fibers
/// which might need to release access to the store when suspending.
struct FiberFuture<'a, 'b> {
    store: &'a mut StoreOpaque,
    fiber: StoreFiber<'b>,
}

impl<'b> Future for FiberFuture<'_, 'b> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let me = self.get_mut();

        let poll_cx = unsafe { &raw mut (*me.store.async_state()).current_poll_cx };
        let _reset = Reset(poll_cx, unsafe { *poll_cx });
        let (guard_range_start, guard_range_end) = me
            .fiber
            .fiber
            .as_ref()
            .unwrap()
            .stack()
            .guard_range()
            .map(|r| (r.start, r.end))
            .unwrap_or((ptr::null_mut(), ptr::null_mut()));

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
            *poll_cx = PollContext {
                future_context: mem::transmute::<&mut Context<'_>, *mut Context<'static>>(cx),
                guard_range_start,
                guard_range_end,
            };
        }

        match resume_fiber(me.store, &mut me.fiber, Ok(())) {
            Ok(()) => Poll::Ready(()),
            Err(_) => Poll::Pending,
        }
    }
}

impl Drop for FiberFuture<'_, '_> {
    fn drop(&mut self) {
        self.fiber.dispose(self.store);
    }
}
