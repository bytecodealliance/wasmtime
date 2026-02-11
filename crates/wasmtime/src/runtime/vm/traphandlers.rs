//! WebAssembly trap handling, which is built on top of the lower-level
//! signalhandling mechanisms.

mod backtrace;

#[cfg(feature = "coredump")]
#[path = "traphandlers/coredump_enabled.rs"]
mod coredump;
#[cfg(not(feature = "coredump"))]
#[path = "traphandlers/coredump_disabled.rs"]
mod coredump;

#[cfg(all(has_native_signals))]
mod signals;
#[cfg(all(has_native_signals))]
pub use self::signals::*;

#[cfg(feature = "gc")]
use crate::ThrownException;
use crate::runtime::module::lookup_code;
use crate::runtime::store::{ExecutorRef, StoreOpaque};
use crate::runtime::vm::sys::traphandlers;
use crate::runtime::vm::{InterpreterRef, VMContext, VMStore, VMStoreContext, f32x4, f64x2, i8x16};
#[cfg(all(feature = "debug", feature = "gc"))]
use crate::store::AsStoreOpaque;
use crate::{EntryStoreContext, prelude::*};
use crate::{StoreContextMut, WasmBacktrace};
use core::cell::Cell;
use core::num::NonZeroU32;
use core::ptr::{self, NonNull};
use wasmtime_unwinder::Handler;

#[cfg(feature = "debug")]
pub(crate) use self::backtrace::Activation;
pub use self::backtrace::Backtrace;
#[cfg(feature = "gc")]
pub use wasmtime_unwinder::Frame;

pub use self::coredump::CoreDumpStack;
pub use self::tls::tls_eager_initialize;
#[cfg(feature = "async")]
pub use self::tls::{AsyncWasmCallState, PreviousAsyncWasmCallState};

pub use traphandlers::SignalHandler;

pub(crate) struct TrapRegisters {
    pub pc: usize,
    pub fp: usize,
}

/// Return value from `test_if_trap`.
pub(crate) enum TrapTest {
    /// Not a wasm trap, need to delegate to whatever process handler is next.
    NotWasm,
    /// This trap was handled by the embedder via custom embedding APIs.
    #[cfg(has_host_compiler_backend)]
    #[cfg_attr(miri, expect(dead_code, reason = "using #[cfg] too unergonomic"))]
    HandledByEmbedder,
    /// This is a wasm trap, it needs to be handled.
    Trap(Handler),
}

fn lazy_per_thread_init() {
    traphandlers::lazy_per_thread_init();
}

/// Raises a preexisting trap or exception and unwinds.
///
/// If the preexisting state has registered a trap, this function will execute
/// the `Handler::resume` to make its way back to the original exception
/// handler created when Wasm was entered. If the state has registered an
/// exception, this function will perform the unwind action registered: either
/// resetting PC, FP, and SP to the handler in the middle of the Wasm
/// activation on the stack, or the entry trampoline back to the the host, if
/// the exception is uncaught.
///
/// This is currently only called from the `raise` builtin of
/// Wasmtime. This builtin is only used when the host returns back to
/// wasm and indicates that a trap or exception should be raised. In
/// this situation the host has already stored trap or exception
/// information within the `CallThreadState` and this is the low-level
/// operation to actually perform an unwind.
///
/// Note that this function is used both for Pulley and for native execution.
/// For Pulley this function will return and the interpreter will be
/// responsible for handling the control-flow transfer. For native this
/// function will not return as the control flow transfer will be handled
/// internally.
///
/// # Safety
///
/// Only safe to call when wasm code is on the stack, aka `catch_traps` must
/// have been previously called. Additionally no Rust destructors can be on the
/// stack. They will be skipped and not executed.
pub(super) unsafe fn raise_preexisting_trap(store: &mut dyn VMStore) {
    tls::with(|info| unsafe { info.unwrap().unwind(store) })
}

/// Invokes the closure `f` and handles any error/panic/trap that happens
/// within.
///
/// This will invoke the closure `f` with the provided `store` and the closure
/// will return a value that implements `HostResult`. This trait abstracts over
/// how host values are translated to ABI values when going back into wasm.
/// Some examples are:
///
/// * `T` - bare return types (not results) are simply returned as-is. No
///   `catch_unwind` happens as if a trap can't happen then the host shouldn't
///   be panicking or invoking user code.
///
/// * `Result<(), E>` - this represents an ABI return value of `bool` which
///   indicates whether the call succeeded. This return value will catch panics
///   and record trap information as `E`.
///
/// * `Result<u32, E>` - the ABI return value here is `u64` where on success
///   the 32-bit result is zero-extended and `u64::MAX` as a return value
///   indicates that a trap or panic happened.
///
/// This is primarily used in conjunction with the Cranelift-and-host boundary.
/// This function acts as a bridge between the two to appropriately handle
/// encoding host values to Cranelift-understood ABIs via the `HostResult`
/// trait.
pub fn catch_unwind_and_record_trap<R>(
    store: &mut dyn VMStore,
    f: impl FnOnce(&mut dyn VMStore) -> R,
) -> R::Abi
where
    R: HostResult,
{
    // Invoke the closure `f`, optionally catching unwinds depending on `R`. The
    // return value is always provided and if unwind information is provided
    // (e.g. `ret` is a "false"-y value) then it's recorded in TLS for the
    // unwind operation that's about to happen from Cranelift-generated code.
    let (ret, unwind) = R::maybe_catch_unwind(store, |store| f(store));
    if let Some(unwind) = unwind {
        tls::with(|info| info.unwrap().record_unwind(store, unwind));
    }
    ret
}

/// A trait used in conjunction with `catch_unwind_and_record_trap` to convert a
/// Rust-based type to a specific ABI while handling traps/unwinds.
///
/// This type is implemented for return values from host function calls and
/// libcalls. The `Abi` value of this trait represents either a successful
/// execution with some payload state or that a failed execution happened. In
/// the event of a failed execution the state of the failure itself is stored
/// within `CallThreadState::unwind`. Cranelift-compiled code is expected to
/// test for this failure sentinel and process it accordingly.
///
/// See `catch_unwind_and_record_trap` for some more information as well.
pub trait HostResult {
    /// The type of the value that's returned to Cranelift-compiled code. Needs
    /// to be ABI-safe to pass through an `extern "C"` return value.
    type Abi: Copy;

    /// Executes `f` and returns the ABI/unwind information as a result.
    ///
    /// This may optionally catch unwinds during execution depending on this
    /// implementation. The ABI return value is unconditionally provided. If an
    /// unwind was detected (e.g. a host panic or a wasm trap) then that's
    /// additionally returned as well.
    ///
    /// If an unwind is returned then it's expected that when the host returns
    /// back to wasm (which should be soon after calling this through
    /// `catch_unwind_and_record_trap`) then wasm will very quickly turn around
    /// and initiate an unwind (currently through `raise_preexisting_trap`).
    fn maybe_catch_unwind(
        store: &mut dyn VMStore,
        f: impl FnOnce(&mut dyn VMStore) -> Self,
    ) -> (Self::Abi, Option<UnwindReason>);
}

// Base case implementations that do not catch unwinds. These are for libcalls
// that neither trap nor execute user code. The raw value is the ABI itself.
//
// Panics in these libcalls will result in a process abort as unwinding is not
// allowed via Rust through `extern "C"` function boundaries.
macro_rules! host_result_no_catch {
    ($($t:ty,)*) => {
        $(
            impl HostResult for $t {
                type Abi = $t;
                #[allow(unreachable_code, reason = "some types uninhabited on some platforms")]
                fn maybe_catch_unwind(
                    store: &mut dyn VMStore,
                    f: impl FnOnce(&mut dyn VMStore) -> $t,
                ) -> ($t, Option<UnwindReason>) {
                    (f(store), None)
                }
            }
        )*
    }
}

host_result_no_catch! {
    (),
    bool,
    u32,
    *mut u8,
    u64,
    f32,
    f64,
    i8x16,
    f32x4,
    f64x2,
}

impl HostResult for NonNull<u8> {
    type Abi = *mut u8;
    fn maybe_catch_unwind(
        store: &mut dyn VMStore,
        f: impl FnOnce(&mut dyn VMStore) -> Self,
    ) -> (*mut u8, Option<UnwindReason>) {
        (f(store).as_ptr(), None)
    }
}

/// Implementation of `HostResult` for `Result<T, E>`.
///
/// This is where things get interesting for `HostResult`. This is generically
/// defined to allow many shapes of the `Result` type to be returned from host
/// calls or libcalls. To do this an extra trait requirement is placed on the
/// successful result `T`: `HostResultHasUnwindSentinel`.
///
/// The general requirement is that `T` says what ABI it has, and the ABI must
/// have a sentinel value which indicates that an unwind in wasm should happen.
/// For example if `T = ()` then `true` means that the call succeeded and
/// `false` means that an unwind happened. Here the sentinel is `false` and the
/// ABI is `bool`.
///
/// This is the only implementation of `HostResult` which actually catches
/// unwinds as there's a sentinel to encode.
impl<T, E> HostResult for Result<T, E>
where
    T: HostResultHasUnwindSentinel,
    E: Into<TrapReason>,
{
    type Abi = T::Abi;

    fn maybe_catch_unwind(
        store: &mut dyn VMStore,
        f: impl FnOnce(&mut dyn VMStore) -> Result<T, E>,
    ) -> (T::Abi, Option<UnwindReason>) {
        // First prepare the closure `f` as something that'll be invoked to
        // generate the return value of this function. This is the
        // conditionally, below, passed to `catch_unwind`.
        let f = move || match f(store) {
            Ok(ret) => (ret.into_abi(), None),
            Err(reason) => (T::SENTINEL, Some(UnwindReason::Trap(reason.into()))),
        };

        // With `panic=unwind` use `std::panic::catch_unwind` to catch possible
        // panics to rethrow.
        #[cfg(all(feature = "std", panic = "unwind"))]
        {
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
                Ok(result) => result,
                Err(err) => (T::SENTINEL, Some(UnwindReason::Panic(err))),
            }
        }

        // With `panic=abort` there's no use in using `std::panic::catch_unwind`
        // since it won't actually catch anything. Note that
        // `std::panic::catch_unwind` will technically optimize to this but having
        // this branch avoids using the `std::panic` module entirely.
        #[cfg(not(all(feature = "std", panic = "unwind")))]
        {
            f()
        }
    }
}

/// Trait used in conjunction with `HostResult for Result<T, E>` where this is
/// the trait bound on `T`.
///
/// This is for values in the "ok" position of a `Result` return value. Each
/// value can have a separate ABI from itself (e.g. `type Abi`) and must be
/// convertible to the ABI. Additionally all implementations of this trait have
/// a "sentinel value" which indicates that an unwind happened. This means that
/// no valid instance of `Self` should generate the `SENTINEL` via the
/// `into_abi` function.
pub unsafe trait HostResultHasUnwindSentinel {
    /// The Cranelift-understood ABI of this value (should not be `Self`).
    type Abi: Copy;

    /// A value that indicates that an unwind should happen and is tested for in
    /// Cranelift-generated code.
    const SENTINEL: Self::Abi;

    /// Converts this value into the ABI representation. Should never returned
    /// the `SENTINEL` value.
    fn into_abi(self) -> Self::Abi;
}

/// No return value from the host is represented as a `bool` in the ABI. Here
/// `true` means that execution succeeded while `false` is the sentinel used to
/// indicate an unwind.
unsafe impl HostResultHasUnwindSentinel for () {
    type Abi = bool;
    const SENTINEL: bool = false;
    fn into_abi(self) -> bool {
        true
    }
}

unsafe impl HostResultHasUnwindSentinel for NonZeroU32 {
    type Abi = u32;
    const SENTINEL: Self::Abi = 0;
    fn into_abi(self) -> Self::Abi {
        self.get()
    }
}

/// A 32-bit return value can be inflated to a 64-bit return value in the ABI.
/// In this manner a successful result is a zero-extended 32-bit value and the
/// failure sentinel is `u64::MAX` or -1 as a signed integer.
unsafe impl HostResultHasUnwindSentinel for u32 {
    type Abi = u64;
    const SENTINEL: u64 = u64::MAX;
    fn into_abi(self) -> u64 {
        self.into()
    }
}

/// If there is not actual successful result (e.g. an empty enum) then the ABI
/// can be `()`, or nothing, because there's no successful result and it's
/// always a failure.
unsafe impl HostResultHasUnwindSentinel for core::convert::Infallible {
    type Abi = ();
    const SENTINEL: () = ();
    fn into_abi(self) {
        match self {}
    }
}

unsafe impl HostResultHasUnwindSentinel for bool {
    type Abi = u32;
    const SENTINEL: Self::Abi = u32::MAX;
    fn into_abi(self) -> Self::Abi {
        u32::from(self)
    }
}

/// Stores trace message with backtrace.
#[derive(Debug)]
pub struct Trap {
    /// Original reason from where this trap originated.
    pub reason: TrapReason,
    /// Wasm backtrace of the trap, if any.
    pub backtrace: Option<Backtrace>,
    /// The Wasm Coredump, if any.
    pub coredumpstack: Option<CoreDumpStack>,
}

/// Enumeration of different methods of raising a trap (or a sentinel
/// for an exception).
#[derive(Debug)]
pub enum TrapReason {
    /// A user-raised trap through `raise_user_trap`.
    User(Error),

    /// A trap raised from Cranelift-generated code.
    Jit {
        /// The program counter where this trap originated.
        ///
        /// This is later used with side tables from compilation to translate
        /// the trapping address to a trap code.
        pc: usize,

        /// If the trap was a memory-related trap such as SIGSEGV then this
        /// field will contain the address of the inaccessible data.
        ///
        /// Note that wasm loads/stores are not guaranteed to fill in this
        /// information. Dynamically-bounds-checked memories, for example, will
        /// not access an invalid address but may instead load from NULL or may
        /// explicitly jump to a `ud2` instruction. This is only available for
        /// fault-based traps which are one of the main ways, but not the only
        /// way, to run wasm.
        faulting_addr: Option<usize>,

        /// The trap code associated with this trap.
        trap: wasmtime_environ::Trap,
    },

    /// A trap raised from a wasm libcall
    Wasm(wasmtime_environ::Trap),

    /// An exception.
    ///
    /// Note that internally, exceptions are rooted on the Store, while
    /// when crossing the public API, exceptions are held in a
    /// `wasmtime::Exception` which contains a boxed root and implements
    /// `Error`. This choice is intentional, to keep the internal
    /// implementation lightweight and ensure the types represent only
    /// allowable states.
    #[cfg(feature = "gc")]
    Exception,
}

impl From<Error> for TrapReason {
    fn from(error: Error) -> Self {
        #[cfg(feature = "gc")]
        if error.is::<ThrownException>() {
            return TrapReason::Exception;
        }

        TrapReason::User(error)
    }
}

impl From<wasmtime_environ::Trap> for TrapReason {
    fn from(code: wasmtime_environ::Trap) -> Self {
        TrapReason::Wasm(code)
    }
}

/// Catches any wasm traps that happen within the execution of `closure`,
/// returning them as a `Result`.
pub fn catch_traps<T, F>(
    store: &mut StoreContextMut<'_, T>,
    old_state: &mut EntryStoreContext,
    mut closure: F,
) -> Result<()>
where
    F: FnMut(NonNull<VMContext>, Option<InterpreterRef<'_>>) -> bool,
{
    let caller = store.0.default_caller();

    let result = CallThreadState::new(store.0, old_state).with(|_cx| match store.0.executor() {
        ExecutorRef::Interpreter(r) => closure(caller, Some(r)),
        #[cfg(has_host_compiler_backend)]
        ExecutorRef::Native => closure(caller, None),
    });

    match result {
        Ok(x) => Ok(x),
        #[cfg(feature = "gc")]
        Err(UnwindState::UnwindToHost {
            reason: UnwindReason::Trap(TrapReason::Exception),
            backtrace: _,
            coredump_stack: _,
        }) => Err(ThrownException.into()),
        Err(UnwindState::UnwindToHost {
            reason: UnwindReason::Trap(reason),
            backtrace,
            coredump_stack,
        }) => Err(crate::trap::from_runtime_box(
            store.0,
            Box::new(Trap {
                reason,
                backtrace,
                coredumpstack: coredump_stack,
            }),
        )),
        #[cfg(all(feature = "std", panic = "unwind"))]
        Err(UnwindState::UnwindToHost {
            reason: UnwindReason::Panic(panic),
            ..
        }) => std::panic::resume_unwind(panic),
        #[cfg(feature = "gc")]
        Err(UnwindState::UnwindToWasm { .. }) => {
            unreachable!("We should not have returned to the host with an UnwindToWasm state");
        }
        Err(UnwindState::None) => {
            unreachable!("We should not have gotten an error with no unwind state");
        }
    }
}

// Module to hide visibility of the `CallThreadState::prev` field and force
// usage of its accessor methods.
mod call_thread_state {
    use super::*;
    use crate::EntryStoreContext;
    use crate::runtime::vm::{Unwind, VMStackChain};

    /// Queued-up unwinding on the CallThreadState, ready to be
    /// enacted by `unwind()`.
    ///
    /// This represents either a request to unwind to the entry point
    /// from host, with associated data; or a request to
    /// unwind into the middle of the Wasm action, e.g. when an
    /// exception is caught.
    pub enum UnwindState {
        /// Unwind all the way to the entry from host to Wasm, using
        /// the handler configured in the entry trampoline.
        UnwindToHost {
            reason: UnwindReason,
            backtrace: Option<Backtrace>,
            coredump_stack: Option<CoreDumpStack>,
        },
        /// Unwind into Wasm. The exception destination has been
        /// resolved. Note that the payload value is still not
        /// specified, because it must remain rooted on the Store
        /// until `unwind()` actually takes the value. The first
        /// payload word in the underlying exception ABI is used to
        /// send the raw `VMExnRef`.
        #[cfg(feature = "gc")]
        UnwindToWasm(Handler),
        /// Do not unwind.
        None,
    }

    impl UnwindState {
        pub(super) fn is_none(&self) -> bool {
            match self {
                Self::None => true,
                _ => false,
            }
        }
    }

    /// Temporary state stored on the stack which is registered in the `tls`
    /// module below for calls into wasm.
    ///
    /// This structure is stored on the stack and allocated during the
    /// `catch_traps` function above. The purpose of this structure is to track
    /// the state of an "activation" or a sequence of 0-or-more contiguous
    /// WebAssembly call frames. A `CallThreadState` always lives on the stack
    /// and additionally maintains pointers to previous states to form a linked
    /// list of activations.
    ///
    /// One of the primary goals of `CallThreadState` is to store the state of
    /// various fields in `VMStoreContext` when it was created. This is done
    /// because calling WebAssembly will clobber these fields otherwise.
    ///
    /// Another major purpose of `CallThreadState` is to assist with unwinding
    /// and track state necessary when an unwind happens for the original
    /// creator of `CallThreadState` to determine why the unwind happened.
    ///
    /// Note that this structure is pointed-to from TLS, hence liberal usage of
    /// interior mutability here since that only gives access to
    /// `&CallThreadState`.
    pub struct CallThreadState {
        /// Unwind state set when initiating an unwind and read when
        /// the control transfer occurs (after the `raise` point is
        /// reached for host-code destinations and right when
        /// performing the jump for Wasm-code destinations).
        pub(super) unwind: Cell<UnwindState>,
        #[cfg(all(has_native_signals))]
        pub(super) signal_handler: Option<*const SignalHandler>,
        pub(super) capture_backtrace: bool,
        #[cfg(feature = "coredump")]
        pub(super) capture_coredump: bool,

        pub(crate) vm_store_context: NonNull<VMStoreContext>,
        pub(crate) unwinder: &'static dyn Unwind,

        pub(super) prev: Cell<tls::Ptr>,

        // The state of the runtime for the *previous* `CallThreadState` for
        // this same store. Our *current* state is saved in `self.vm_store_context`,
        // etc. We need access to the old values of these
        // fields because the `VMStoreContext` typically doesn't change across
        // nested calls into Wasm (i.e. they are typically calls back into the
        // same store and `self.vm_store_context == self.prev.vm_store_context`) and we must to
        // maintain the list of contiguous-Wasm-frames stack regions for
        // backtracing purposes.
        old_state: *mut EntryStoreContext,
    }

    impl Drop for CallThreadState {
        fn drop(&mut self) {
            // Unwind information should not be present as it should have
            // already been processed.
            debug_assert!(self.unwind.replace(UnwindState::None).is_none());
        }
    }

    impl CallThreadState {
        #[inline]
        pub(super) fn new(
            store: &mut StoreOpaque,
            old_state: *mut EntryStoreContext,
        ) -> CallThreadState {
            CallThreadState {
                unwind: Cell::new(UnwindState::None),
                unwinder: store.unwinder(),
                #[cfg(all(has_native_signals))]
                signal_handler: store.signal_handler(),
                capture_backtrace: store.engine().config().wasm_backtrace,
                #[cfg(feature = "coredump")]
                capture_coredump: store.engine().config().coredump_on_trap,
                vm_store_context: store.vm_store_context_ptr(),
                prev: Cell::new(ptr::null()),
                old_state,
            }
        }

        /// Get the saved FP upon exit from Wasm for the previous `CallThreadState`.
        ///
        /// # Safety
        ///
        /// Requires that the saved last Wasm trampoline FP points to
        /// a valid trampoline frame, or is null.
        pub unsafe fn old_last_wasm_exit_fp(&self) -> usize {
            let trampoline_fp = unsafe { (&*self.old_state).last_wasm_exit_trampoline_fp };
            // SAFETY: `trampoline_fp` is either a valid FP from an
            // active trampoline frame or is null.
            unsafe { VMStoreContext::wasm_exit_fp_from_trampoline_fp(trampoline_fp) }
        }

        /// Get the saved PC upon exit from Wasm for the previous `CallThreadState`.
        pub unsafe fn old_last_wasm_exit_pc(&self) -> usize {
            unsafe { (&*self.old_state).last_wasm_exit_pc }
        }

        /// Get the saved FP upon entry into Wasm for the previous `CallThreadState`.
        pub unsafe fn old_last_wasm_entry_fp(&self) -> usize {
            unsafe { (&*self.old_state).last_wasm_entry_fp }
        }

        /// Get the saved `VMStackChain` for the previous `CallThreadState`.
        pub unsafe fn old_stack_chain(&self) -> VMStackChain {
            unsafe { (&*self.old_state).stack_chain.clone() }
        }

        /// Get the previous `CallThreadState`.
        pub fn prev(&self) -> tls::Ptr {
            self.prev.get()
        }

        /// Pushes this `CallThreadState` activation on to the linked list
        /// stored in TLS.
        ///
        /// This method will take the current head of the linked list, stored in
        /// our TLS pointer, and move it into `prev`. The TLS pointer is then
        /// updated to `self`.
        ///
        /// # Panics
        ///
        /// Panics if this activation is already in a linked list (e.g.
        /// `self.prev` is set).
        #[inline]
        pub(crate) unsafe fn push(&self) {
            assert!(self.prev.get().is_null());
            self.prev.set(tls::raw::replace(self));
        }

        /// Pops this `CallThreadState` from the linked list stored in TLS.
        ///
        /// This method will restore `self.prev` into the head of the linked
        /// list stored in TLS and will additionally null-out `self.prev`.
        ///
        /// # Panics
        ///
        /// Panics if this activation isn't the head of the list.
        #[inline]
        pub(crate) unsafe fn pop(&self) {
            let prev = self.prev.replace(ptr::null());
            let head = tls::raw::replace(prev);
            assert!(core::ptr::eq(head, self));
        }

        /// Swaps the state in this `CallThreadState`'s `VMStoreContext` with
        /// the state in `EntryStoreContext` that was saved when this
        /// activation was created.
        ///
        /// This method is using during suspension of a fiber to restore the
        /// store back to what it originally was and prepare it to be resumed
        /// later on. This takes various fields of `VMStoreContext` and swaps
        /// them with what was saved in `EntryStoreContext`. That restores
        /// a store to just before this activation was called but saves off the
        /// fields of this activation to get restored/resumed at a later time.
        #[cfg(feature = "async")]
        pub(super) unsafe fn swap(&self) {
            unsafe fn swap<T>(a: &core::cell::UnsafeCell<T>, b: &mut T) {
                unsafe { core::mem::swap(&mut *a.get(), b) }
            }

            unsafe {
                let cx = self.vm_store_context.as_ref();
                swap(
                    &cx.last_wasm_exit_trampoline_fp,
                    &mut (*self.old_state).last_wasm_exit_trampoline_fp,
                );
                swap(
                    &cx.last_wasm_exit_pc,
                    &mut (*self.old_state).last_wasm_exit_pc,
                );
                swap(
                    &cx.last_wasm_entry_fp,
                    &mut (*self.old_state).last_wasm_entry_fp,
                );
                swap(
                    &cx.last_wasm_entry_sp,
                    &mut (*self.old_state).last_wasm_entry_sp,
                );
                swap(
                    &cx.last_wasm_entry_trap_handler,
                    &mut (*self.old_state).last_wasm_entry_trap_handler,
                );
                swap(&cx.stack_chain, &mut (*self.old_state).stack_chain);
            }
        }
    }
}
pub use call_thread_state::*;

#[cfg(feature = "gc")]
use super::compute_handler;

pub enum UnwindReason {
    #[cfg(all(feature = "std", panic = "unwind"))]
    Panic(Box<dyn std::any::Any + Send>),
    Trap(TrapReason),
}

impl<E> From<E> for UnwindReason
where
    E: Into<TrapReason>,
{
    fn from(value: E) -> UnwindReason {
        UnwindReason::Trap(value.into())
    }
}

impl CallThreadState {
    #[inline]
    fn with(mut self, closure: impl FnOnce(&CallThreadState) -> bool) -> Result<(), UnwindState> {
        let succeeded = tls::set(&mut self, |me| closure(me));
        if succeeded {
            Ok(())
        } else {
            Err(self.read_unwind())
        }
    }

    #[cold]
    fn read_unwind(&self) -> UnwindState {
        self.unwind.replace(UnwindState::None)
    }

    /// Records the unwind information provided within this `CallThreadState`,
    /// optionally capturing a backtrace at this time.
    ///
    /// This function is used to stash metadata for why an unwind is about to
    /// happen. The actual unwind is expected to happen after this function is
    /// called using, for example, the `unwind` function below.
    ///
    /// Note that this is a relatively low-level function and will panic if
    /// mis-used.
    ///
    /// # Panics
    ///
    /// Panics if unwind information has already been recorded as that should
    /// have been processed first.
    fn record_unwind(&self, store: &mut dyn VMStore, reason: UnwindReason) {
        if cfg!(debug_assertions) {
            let prev = self.unwind.replace(UnwindState::None);
            assert!(prev.is_none());
        }
        let state = match reason {
            #[cfg(all(feature = "std", panic = "unwind"))]
            UnwindReason::Panic(err) => {
                // Panics don't need backtraces. There is nowhere to attach the
                // hypothetical backtrace to and it doesn't really make sense to try
                // in the first place since this is a Rust problem rather than a
                // Wasm problem.
                UnwindState::UnwindToHost {
                    reason: UnwindReason::Panic(err),
                    backtrace: None,
                    coredump_stack: None,
                }
            }
            // An unwind due to an already-set pending exception
            // triggers the handler-search stack-walk. We store the
            // resolved handler if one exists. In either case, the
            // exception remains rooted in the Store until we actually
            // perform the unwind, and then gets taken and becomes the
            // payload at that point.
            #[cfg(feature = "gc")]
            UnwindReason::Trap(TrapReason::Exception) => {
                // SAFETY: we are invoking `compute_handler()` while
                // Wasm is on the stack and we have re-entered via a
                // trampoline, as required by its stack-walking logic.
                let handler = unsafe { compute_handler(store) };
                match handler {
                    Some(handler) => UnwindState::UnwindToWasm(handler),
                    None => UnwindState::UnwindToHost {
                        reason: UnwindReason::Trap(TrapReason::Exception),
                        backtrace: None,
                        coredump_stack: None,
                    },
                }
            }
            // And if we are just propagating an existing trap that already has
            // a backtrace attached to it, then there is no need to capture a
            // new backtrace either.
            UnwindReason::Trap(TrapReason::User(err))
                if err.downcast_ref::<WasmBacktrace>().is_some() =>
            {
                UnwindState::UnwindToHost {
                    reason: UnwindReason::Trap(TrapReason::User(err)),
                    backtrace: None,
                    coredump_stack: None,
                }
            }
            UnwindReason::Trap(trap) => {
                log::trace!("Capturing backtrace and coredump for {trap:?}");
                UnwindState::UnwindToHost {
                    reason: UnwindReason::Trap(trap),
                    backtrace: self.capture_backtrace(store.vm_store_context_mut(), None),
                    coredump_stack: self.capture_coredump(store.vm_store_context_mut(), None),
                }
            }
        };

        // Avoid unused-variable warning in non-exceptions/GC build.
        let _ = store;

        self.unwind.set(state);
    }

    /// Helper function to perform an actual unwinding operation.
    ///
    /// This must be preceded by a `record_unwind` operation above to be
    /// processed correctly on the other side.
    ///
    /// # Unsafety
    ///
    /// This function is not safe if a corresponding handler wasn't already
    /// setup in the entry trampoline. Additionally this isn't safe as it may
    /// skip all Rust destructors on the stack, if there are any, for native
    /// executors as `Handler::resume` will be used.
    unsafe fn unwind(&self, store: &mut dyn VMStore) {
        #[allow(unused_mut, reason = "only  mutated in `debug` configuration")]
        let mut unwind = self.unwind.replace(UnwindState::None);

        #[cfg(feature = "debug")]
        {
            let result = match &unwind {
                #[cfg(feature = "gc")]
                UnwindState::UnwindToWasm(_) => {
                    use wasmtime_core::alloc::PanicOnOom;

                    assert!(store.as_store_opaque().has_pending_exception());
                    let exn = store
                        .as_store_opaque()
                        .pending_exception_owned_rooted()
                        // TODO(#12069): handle allocation failure here
                        .panic_on_oom()
                        .expect("exception should be set when we are throwing");
                    store.block_on_debug_handler(crate::DebugEvent::CaughtExceptionThrown(exn))
                }
                #[cfg(feature = "gc")]
                UnwindState::UnwindToHost {
                    reason: UnwindReason::Trap(TrapReason::Exception),
                    ..
                } => {
                    use wasmtime_core::alloc::PanicOnOom;

                    let exn = store
                        .as_store_opaque()
                        .pending_exception_owned_rooted()
                        // TODO(#12069): handle allocation failure here
                        .panic_on_oom()
                        .expect("exception should be set when we are throwing");
                    store.block_on_debug_handler(crate::DebugEvent::UncaughtExceptionThrown(
                        exn.clone(),
                    ))
                }
                UnwindState::UnwindToHost {
                    reason: UnwindReason::Trap(TrapReason::Wasm(trap)),
                    ..
                } => store.block_on_debug_handler(crate::DebugEvent::Trap(*trap)),
                UnwindState::UnwindToHost {
                    reason: UnwindReason::Trap(TrapReason::User(err)),
                    ..
                } => store.block_on_debug_handler(crate::DebugEvent::HostcallError(err)),

                UnwindState::UnwindToHost {
                    reason: UnwindReason::Trap(TrapReason::Jit { .. }),
                    ..
                } => {
                    // JIT traps not handled yet.
                    Ok(())
                }
                #[cfg(all(feature = "std", panic = "unwind"))]
                UnwindState::UnwindToHost {
                    reason: UnwindReason::Panic(_),
                    ..
                } => {
                    // We don't invoke any debugger hook when we're
                    // unwinding due to a Rust (host-side) panic.
                    Ok(())
                }

                UnwindState::None => unreachable!(),
            };

            // If the debugger invocation itself resulted in an `Err`
            // (which can only come from the `block_on` hitting a
            // failure mode), we need to override our unwind as-if
            // were handling a host error.
            if let Err(err) = result {
                unwind = UnwindState::UnwindToHost {
                    reason: UnwindReason::Trap(TrapReason::User(err)),
                    backtrace: None,
                    coredump_stack: None,
                };
            }
        }

        match unwind {
            UnwindState::UnwindToHost { .. } => {
                self.unwind.set(unwind);
                let handler = self.entry_trap_handler();
                let payload1 = 0;
                let payload2 = 0;
                unsafe {
                    self.resume_to_exception_handler(
                        store.executor(),
                        &handler,
                        payload1,
                        payload2,
                    );
                }
            }
            #[cfg(feature = "gc")]
            UnwindState::UnwindToWasm(handler) => {
                // Take the pending exception at this time and use it as payload.
                let payload1 = usize::try_from(
                    store
                        .take_pending_exception()
                        .unwrap()
                        .as_gc_ref()
                        .as_raw_u32(),
                )
                .expect("GC ref does not fit in usize");
                // We only use one of the payload words.
                let payload2 = 0;
                unsafe {
                    self.resume_to_exception_handler(
                        store.executor(),
                        &handler,
                        payload1,
                        payload2,
                    );
                }
            }
            UnwindState::None => {
                panic!("Attempting to unwind with no unwind state set.");
            }
        }
    }

    pub(crate) fn entry_trap_handler(&self) -> Handler {
        unsafe {
            let vm_store_context = self.vm_store_context.as_ref();
            let fp = *vm_store_context.last_wasm_entry_fp.get();
            let sp = *vm_store_context.last_wasm_entry_sp.get();
            let pc = *vm_store_context.last_wasm_entry_trap_handler.get();
            Handler { pc, sp, fp }
        }
    }

    unsafe fn resume_to_exception_handler(
        &self,
        executor: ExecutorRef<'_>,
        handler: &Handler,
        payload1: usize,
        payload2: usize,
    ) {
        unsafe {
            match executor {
                ExecutorRef::Interpreter(mut r) => {
                    r.resume_to_exception_handler(handler, payload1, payload2)
                }
                #[cfg(has_host_compiler_backend)]
                ExecutorRef::Native => handler.resume_tailcc(payload1, payload2),
            }
        }
    }

    fn capture_backtrace(
        &self,
        limits: *const VMStoreContext,
        trap_pc_and_fp: Option<(usize, usize)>,
    ) -> Option<Backtrace> {
        if !self.capture_backtrace {
            return None;
        }

        Some(unsafe { Backtrace::new_with_trap_state(limits, self.unwinder, self, trap_pc_and_fp) })
    }

    pub(crate) fn iter<'a>(&'a self) -> impl Iterator<Item = &'a Self> + 'a {
        let mut state = Some(self);
        core::iter::from_fn(move || {
            let this = state?;
            state = unsafe { this.prev().as_ref() };
            Some(this)
        })
    }

    /// Trap handler using our thread-local state.
    ///
    /// * `regs` - some special program registers at the time that the trap
    ///   happened, for example `pc`.
    /// * `faulting_addr` - the system-provided address that the a fault, if
    ///   any, happened at. This is used when debug-asserting that all segfaults
    ///   are known to live within a `Store<T>` in a valid range.
    /// * `call_handler` - a closure used to invoke the platform-specific
    ///   signal handler for each instance, if available.
    ///
    /// Attempts to handle the trap if it's a wasm trap. Returns a `TrapTest`
    /// which indicates what this could be, such as:
    ///
    /// * `TrapTest::NotWasm` - not a wasm fault, this should get forwarded to
    ///   the next platform-specific fault handler.
    /// * `TrapTest::HandledByEmbedder` - the embedder `call_handler` handled
    ///   this signal, nothing else to do.
    /// * `TrapTest::Trap` - this is a wasm trap an the stack needs to be
    ///   unwound now.
    pub(crate) fn test_if_trap(
        &self,
        regs: TrapRegisters,
        faulting_addr: Option<usize>,
        call_handler: impl FnOnce(&SignalHandler) -> bool,
    ) -> TrapTest {
        // First up see if any instance registered has a custom trap handler,
        // in which case run them all. If anything handles the trap then we
        // return that the trap was handled.
        let _ = &call_handler;
        #[cfg(all(has_native_signals, not(miri)))]
        if let Some(handler) = self.signal_handler {
            if unsafe { call_handler(&*handler) } {
                return TrapTest::HandledByEmbedder;
            }
        }

        // If this fault wasn't in wasm code, then it's not our problem
        let Some((code, text_offset)) = lookup_code(regs.pc) else {
            return TrapTest::NotWasm;
        };

        // If the fault was at a location that was not marked as potentially
        // trapping, then that's a bug in Cranelift/Winch/etc. Don't try to
        // catch the trap and pretend this isn't wasm so the program likely
        // aborts.
        let Some(trap) = code.lookup_trap_code(text_offset) else {
            return TrapTest::NotWasm;
        };

        // If all that passed then this is indeed a wasm trap, so return the
        // `Handler` setup in the original wasm frame.
        self.set_jit_trap(regs, faulting_addr, trap);
        let entry_handler = self.entry_trap_handler();
        TrapTest::Trap(entry_handler)
    }

    pub(crate) fn set_jit_trap(
        &self,
        TrapRegisters { pc, fp, .. }: TrapRegisters,
        faulting_addr: Option<usize>,
        trap: wasmtime_environ::Trap,
    ) {
        let backtrace = self.capture_backtrace(self.vm_store_context.as_ptr(), Some((pc, fp)));
        let coredump_stack = self.capture_coredump(self.vm_store_context.as_ptr(), Some((pc, fp)));
        self.unwind.set(UnwindState::UnwindToHost {
            reason: UnwindReason::Trap(TrapReason::Jit {
                pc,
                faulting_addr,
                trap,
            }),
            backtrace,
            coredump_stack,
        });
    }
}

/// A private inner module managing the state of Wasmtime's thread-local storage
/// (TLS) state.
///
/// Wasmtime at this time has a single pointer of TLS. This single pointer of
/// TLS is the totality of all TLS required by Wasmtime. By keeping this as
/// small as possible it generally makes it easier to integrate with external
/// systems and implement features such as fiber context switches. This single
/// TLS pointer is declared in platform-specific modules to handle platform
/// differences, so this module here uses getters/setters which delegate to
/// platform-specific implementations.
///
/// The single TLS pointer used by Wasmtime is morally
/// `Option<&CallThreadState>` meaning that it's a possibly-present pointer to
/// some state. This pointer is a pointer to the most recent (youngest)
/// `CallThreadState` activation, or the most recent call into WebAssembly.
///
/// This TLS pointer is additionally the head of a linked list of activations
/// that are all stored on the stack for the current thread. Each time
/// WebAssembly is recursively invoked by an embedder will push a new entry into
/// this linked list. This singly-linked list is maintained with its head in TLS
/// node pointers are stored in `CallThreadState::prev`.
///
/// An example stack might look like this:
///
/// ```text
/// ┌─────────────────────┐◄───── highest, or oldest, stack address
/// │ native stack frames │
/// │         ...         │
/// │  ┌───────────────┐◄─┼──┐
/// │  │CallThreadState│  │  │
/// │  └───────────────┘  │  p
/// ├─────────────────────┤  r
/// │  wasm stack frames  │  e
/// │         ...         │  v
/// ├─────────────────────┤  │
/// │ native stack frames │  │
/// │         ...         │  │
/// │  ┌───────────────┐◄─┼──┼── TLS pointer
/// │  │CallThreadState├──┼──┘
/// │  └───────────────┘  │
/// ├─────────────────────┤
/// │  wasm stack frames  │
/// │         ...         │
/// ├─────────────────────┤
/// │ native stack frames │
/// │         ...         │
/// └─────────────────────┘◄───── smallest, or youngest, stack address
/// ```
///
/// # Fibers and async
///
/// Wasmtime supports stack-switching with fibers to implement async. This means
/// that Wasmtime will temporarily execute code on a separate stack and then
/// suspend from this stack back to the embedder for async operations. Doing
/// this safely requires manual management of the TLS pointer updated by
/// Wasmtime.
///
/// For example when a fiber is suspended that means that the TLS pointer needs
/// to be restored to whatever it was when the fiber was resumed. Additionally
/// this may need to pop multiple `CallThreadState` activations, one for each
/// one located on the fiber stack itself.
///
/// The `AsyncWasmCallState` and `PreviousAsyncWasmCallState` structures in this
/// module are used to manage this state, namely:
///
/// * The `AsyncWasmCallState` structure represents the state of a suspended
///   fiber. This is a linked list, in reverse order, from oldest activation on
///   the fiber to youngest activation on the fiber.
///
/// * The `PreviousAsyncWasmCallState` structure represents a pointer within our
///   thread's TLS linked list of activations when a fiber was resumed. This
///   pointer is used during fiber suspension to know when to stop popping
///   activations from the thread's linked list.
///
/// Note that this means that the directionality of linked list links is
/// opposite when stored in TLS vs when stored for a suspended fiber. The
/// thread's current list pointed to by TLS is youngest-to-oldest links, while a
/// suspended fiber stores oldest-to-youngest links.
pub(crate) mod tls {
    use super::CallThreadState;

    pub use raw::Ptr;

    // An even *more* inner module for dealing with TLS. This actually has the
    // thread local variable and has functions to access the variable.
    //
    // Note that this is specially done to fully encapsulate that the accessors
    // for tls may or may not be inlined. Wasmtime's async support employs stack
    // switching which can resume execution on different OS threads. This means
    // that borrows of our TLS pointer must never live across accesses because
    // otherwise the access may be split across two threads and cause unsafety.
    //
    // This also means that extra care is taken by the runtime to save/restore
    // these TLS values when the runtime may have crossed threads.
    //
    // Note, though, that if async support is disabled at compile time then
    // these functions are free to be inlined.
    pub(super) mod raw {
        use super::CallThreadState;

        pub type Ptr = *const CallThreadState;

        const _: () = {
            assert!(core::mem::align_of::<CallThreadState>() > 1);
        };

        fn tls_get() -> (Ptr, bool) {
            let mut initialized = false;
            let p = crate::runtime::vm::sys::tls_get().map_addr(|a| {
                initialized = (a & 1) != 0;
                a & !1
            });
            (p.cast(), initialized)
        }

        fn tls_set(ptr: Ptr, initialized: bool) {
            let encoded = ptr.map_addr(|a| a | usize::from(initialized));
            crate::runtime::vm::sys::tls_set(encoded.cast_mut().cast::<u8>());
        }

        #[cfg_attr(feature = "async", inline(never))] // see module docs
        #[cfg_attr(not(feature = "async"), inline)]
        pub fn replace(val: Ptr) -> Ptr {
            // When a new value is configured that means that we may be
            // entering WebAssembly so check to see if this thread has
            // performed per-thread initialization for traps.
            let (prev, initialized) = tls_get();
            if !initialized {
                super::super::lazy_per_thread_init();
            }
            tls_set(val, true);
            prev
        }

        /// Eagerly initialize thread-local runtime functionality. This will be performed
        /// lazily by the runtime if users do not perform it eagerly.
        #[cfg_attr(feature = "async", inline(never))] // see module docs
        #[cfg_attr(not(feature = "async"), inline)]
        pub fn initialize() {
            let (state, initialized) = tls_get();
            if initialized {
                return;
            }
            super::super::lazy_per_thread_init();
            tls_set(state, true);
        }

        #[cfg_attr(feature = "async", inline(never))] // see module docs
        #[cfg_attr(not(feature = "async"), inline)]
        pub fn get() -> Ptr {
            tls_get().0
        }
    }

    pub use raw::initialize as tls_eager_initialize;

    /// Opaque state used to persist the state of the `CallThreadState`
    /// activations associated with a fiber stack that's used as part of an
    /// async wasm call.
    #[cfg(feature = "async")]
    pub struct AsyncWasmCallState {
        // The head of a linked list of activations that are currently present
        // on an async call's fiber stack. This pointer points to the oldest
        // activation frame where the `prev` links internally link to younger
        // activation frames.
        //
        // When pushed onto a thread this linked list is traversed to get pushed
        // onto the current thread at the time.
        //
        // If this pointer is null then that means that the fiber this state is
        // associated with has no activations.
        state: raw::Ptr,
    }

    // SAFETY: This is a relatively unsafe unsafe block and not really all that
    // well audited. The general idea is that the linked list of activations
    // owned by `self.state` are safe to send to other threads, but that relies
    // on everything internally being safe as well as stack variables and such.
    // This is more-or-less tied to the very large comment in `fiber.rs` about
    // `unsafe impl Send` there.
    #[cfg(feature = "async")]
    unsafe impl Send for AsyncWasmCallState {}

    #[cfg(feature = "async")]
    impl AsyncWasmCallState {
        /// Creates new state that initially starts as null.
        pub fn new() -> AsyncWasmCallState {
            AsyncWasmCallState {
                state: core::ptr::null_mut(),
            }
        }

        /// Pushes the saved state of this wasm's call onto the current thread's
        /// state.
        ///
        /// This will iterate over the linked list of states stored within
        /// `self` and push them sequentially onto the current thread's
        /// activation list.
        ///
        /// The returned `PreviousAsyncWasmCallState` captures the state of this
        /// thread just before this operation, and it must have its `restore`
        /// method called to restore the state when the async wasm is suspended
        /// from.
        ///
        /// # Unsafety
        ///
        /// Must be carefully coordinated with
        /// `PreviousAsyncWasmCallState::restore` and fiber switches to ensure
        /// that this doesn't push stale data and the data is popped
        /// appropriately.
        pub unsafe fn push(self) -> PreviousAsyncWasmCallState {
            // First save the state of TLS as-is so when this state is popped
            // off later on we know where to stop.
            let ret = PreviousAsyncWasmCallState { state: raw::get() };

            // The oldest activation, if present, has various `VMStoreContext`
            // fields saved within it. These fields were the state for the
            // *youngest* activation when a suspension previously happened. By
            // swapping them back into the store this is an O(1) way of
            // restoring the state of a store's metadata fields at the time of
            // the suspension.
            //
            // The store's previous values before this function will all get
            // saved in the oldest activation's state on the stack. The store's
            // current state then describes the youngest activation which is
            // restored via the loop below.
            unsafe {
                if let Some(state) = self.state.as_ref() {
                    state.swap();
                }
            }

            // Our `state` pointer is a linked list of oldest-to-youngest so by
            // pushing in order of the list we restore the youngest-to-oldest
            // list as stored in the state of this current thread.
            let mut ptr = self.state;
            unsafe {
                while let Some(state) = ptr.as_ref() {
                    ptr = state.prev.replace(core::ptr::null_mut());
                    state.push();
                }
            }
            ret
        }

        /// Performs a runtime check that this state is indeed null.
        pub fn assert_null(&self) {
            assert!(self.state.is_null());
        }

        /// Asserts that the current CallThreadState pointer, if present, is not
        /// in the `range` specified.
        ///
        /// This is used when exiting a future in Wasmtime to assert that the
        /// current CallThreadState pointer does not point within the stack
        /// we're leaving (e.g. allocated for a fiber).
        pub fn assert_current_state_not_in_range(range: core::ops::Range<usize>) {
            let p = raw::get() as usize;
            assert!(p < range.start || range.end < p);
        }
    }

    /// Opaque state used to help control TLS state across stack switches for
    /// async support.
    ///
    /// This structure is returned from [`AsyncWasmCallState::push`] and
    /// represents the state of this thread's TLS variable prior to the push
    /// operation.
    #[cfg(feature = "async")]
    pub struct PreviousAsyncWasmCallState {
        // The raw value of this thread's TLS pointer when this structure was
        // created. This is not dereferenced or inspected but is used to halt
        // linked list traversal in [`PreviousAsyncWasmCallState::restore`].
        state: raw::Ptr,
    }

    #[cfg(feature = "async")]
    impl PreviousAsyncWasmCallState {
        /// Pops a fiber's linked list of activations and stores them in
        /// `AsyncWasmCallState`.
        ///
        /// This will pop the top activation of this current thread continuously
        /// until it reaches whatever the current activation was when
        /// [`AsyncWasmCallState::push`] was originally called.
        ///
        /// # Unsafety
        ///
        /// Must be paired with a `push` and only performed at a time when a
        /// fiber is being suspended.
        pub unsafe fn restore(self) -> AsyncWasmCallState {
            let thread_head = self.state;
            core::mem::forget(self);
            let mut ret = AsyncWasmCallState::new();
            loop {
                // If the current TLS state is as we originally found it, then
                // this loop is finished.
                //
                // Note, though, that before exiting, if the oldest
                // `CallThreadState` is present, the current state of
                // `VMStoreContext` is saved off within it. This will save the
                // current state, before this function, of `VMStoreContext`
                // into the `EntryStoreContext` stored with the oldest
                // activation. This is a bit counter-intuitive where the state
                // for the youngest activation is stored in the "old" state
                // of the oldest activation.
                //
                // What this does is restores the state of the store to just
                // before this async fiber was started. The fiber's state will
                // be entirely self-contained in the fiber itself and the
                // returned `AsyncWasmCallState`. Resumption above in
                // `AsyncWasmCallState::push` will perform the swap back into
                // the store to hook things up again.
                let ptr = raw::get();
                if ptr == thread_head {
                    unsafe {
                        if let Some(state) = ret.state.as_ref() {
                            state.swap();
                        }
                    }

                    break ret;
                }

                // Pop this activation from the current thread's TLS state, and
                // then afterwards push it onto our own linked list within this
                // `AsyncWasmCallState`. Note that the linked list in
                // `AsyncWasmCallState` is stored in reverse order so a
                // subsequent `push` later on pushes everything in the right
                // order.
                unsafe {
                    (*ptr).pop();
                    if let Some(state) = ret.state.as_ref() {
                        (*ptr).prev.set(state);
                    }
                }
                ret.state = ptr;
            }
        }
    }

    #[cfg(feature = "async")]
    impl Drop for PreviousAsyncWasmCallState {
        fn drop(&mut self) {
            panic!("must be consumed with `restore`");
        }
    }

    /// Configures thread local state such that for the duration of the
    /// execution of `closure` any call to `with` will yield `state`, unless
    /// this is recursively called again.
    #[inline]
    pub fn set<R>(state: &mut CallThreadState, closure: impl FnOnce(&CallThreadState) -> R) -> R {
        struct Reset<'a> {
            state: &'a CallThreadState,
        }

        impl Drop for Reset<'_> {
            #[inline]
            fn drop(&mut self) {
                unsafe {
                    self.state.pop();
                }
            }
        }

        unsafe {
            state.push();
            let reset = Reset { state };
            closure(reset.state)
        }
    }

    /// Returns the last pointer configured with `set` above, if any.
    pub fn with<R>(closure: impl FnOnce(Option<&CallThreadState>) -> R) -> R {
        let p = raw::get();
        unsafe { closure(if p.is_null() { None } else { Some(&*p) }) }
    }
}
