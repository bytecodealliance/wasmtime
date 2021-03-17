//! WebAssembly trap handling, which is built on top of the lower-level
//! signalhandling mechanisms.

use crate::VMInterrupts;
use backtrace::Backtrace;
use std::any::Any;
use std::cell::Cell;
use std::error::Error;
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use std::sync::Once;
use wasmtime_environ::ir;

pub use self::tls::TlsRestore;

extern "C" {
    fn RegisterSetjmp(
        jmp_buf: *mut *const u8,
        callback: extern "C" fn(*mut u8),
        payload: *mut u8,
    ) -> i32;
    fn Unwind(jmp_buf: *const u8) -> !;
}

cfg_if::cfg_if! {
    if #[cfg(target_os = "macos")] {
        mod macos;
        use macos as sys;
    } else if #[cfg(unix)] {
        mod unix;
        use unix as sys;
    } else if #[cfg(target_os = "windows")] {
        mod windows;
        use windows as sys;
    }
}

pub use sys::SignalHandler;

/// This function performs the low-overhead platform-specific initialization
/// that we want to do eagerly to ensure a more-deterministic global process
/// state.
///
/// This is especially relevant for signal handlers since handler ordering
/// depends on installation order: the wasm signal handler must run *before*
/// the other crash handlers and since POSIX signal handlers work LIFO, this
/// function needs to be called at the end of the startup process, after other
/// handlers have been installed. This function can thus be called multiple
/// times, having no effect after the first call.
pub fn init_traps() {
    static INIT: Once = Once::new();
    INIT.call_once(|| unsafe { sys::platform_init() });
}

/// Raises a user-defined trap immediately.
///
/// This function performs as-if a wasm trap was just executed, only the trap
/// has a dynamic payload associated with it which is user-provided. This trap
/// payload is then returned from `catch_traps` below.
///
/// # Safety
///
/// Only safe to call when wasm code is on the stack, aka `catch_traps` must
/// have been previously called. Additionally no Rust destructors can be on the
/// stack. They will be skipped and not executed.
pub unsafe fn raise_user_trap(data: Box<dyn Error + Send + Sync>) -> ! {
    tls::with(|info| info.unwrap().unwind_with(UnwindReason::UserTrap(data)))
}

/// Raises a trap from inside library code immediately.
///
/// This function performs as-if a wasm trap was just executed. This trap
/// payload is then returned from `catch_traps` below.
///
/// # Safety
///
/// Only safe to call when wasm code is on the stack, aka `catch_traps` must
/// have been previously called. Additionally no Rust destructors can be on the
/// stack. They will be skipped and not executed.
pub unsafe fn raise_lib_trap(trap: Trap) -> ! {
    tls::with(|info| info.unwrap().unwind_with(UnwindReason::LibTrap(trap)))
}

/// Carries a Rust panic across wasm code and resumes the panic on the other
/// side.
///
/// # Safety
///
/// Only safe to call when wasm code is on the stack, aka `catch_traps` must
/// have been previously called. Additionally no Rust destructors can be on the
/// stack. They will be skipped and not executed.
pub unsafe fn resume_panic(payload: Box<dyn Any + Send>) -> ! {
    tls::with(|info| info.unwrap().unwind_with(UnwindReason::Panic(payload)))
}

/// Stores trace message with backtrace.
#[derive(Debug)]
pub enum Trap {
    /// A user-raised trap through `raise_user_trap`.
    User(Box<dyn Error + Send + Sync>),

    /// A trap raised from jit code
    Jit {
        /// The program counter in JIT code where this trap happened.
        pc: usize,
        /// Native stack backtrace at the time the trap occurred
        backtrace: Backtrace,
        /// An indicator for whether this may have been a trap generated from an
        /// interrupt, used for switching what would otherwise be a stack
        /// overflow trap to be an interrupt trap.
        maybe_interrupted: bool,
    },

    /// A trap raised from a wasm libcall
    Wasm {
        /// Code of the trap.
        trap_code: ir::TrapCode,
        /// Native stack backtrace at the time the trap occurred
        backtrace: Backtrace,
    },

    /// A trap indicating that the runtime was unable to allocate sufficient memory.
    OOM {
        /// Native stack backtrace at the time the OOM occurred
        backtrace: Backtrace,
    },
}

impl Trap {
    /// Construct a new Wasm trap with the given source location and trap code.
    ///
    /// Internally saves a backtrace when constructed.
    pub fn wasm(trap_code: ir::TrapCode) -> Self {
        let backtrace = Backtrace::new_unresolved();
        Trap::Wasm {
            trap_code,
            backtrace,
        }
    }

    /// Construct a new OOM trap with the given source location and trap code.
    ///
    /// Internally saves a backtrace when constructed.
    pub fn oom() -> Self {
        let backtrace = Backtrace::new_unresolved();
        Trap::OOM { backtrace }
    }
}

/// Catches any wasm traps that happen within the execution of `closure`,
/// returning them as a `Result`.
///
/// Highly unsafe since `closure` won't have any dtors run.
pub unsafe fn catch_traps<F>(trap_info: &impl TrapInfo, mut closure: F) -> Result<(), Trap>
where
    F: FnMut(),
{
    sys::lazy_per_thread_init()?;

    return CallThreadState::new(trap_info).with(|cx| {
        RegisterSetjmp(
            cx.jmp_buf.as_ptr(),
            call_closure::<F>,
            &mut closure as *mut F as *mut u8,
        )
    });

    extern "C" fn call_closure<F>(payload: *mut u8)
    where
        F: FnMut(),
    {
        unsafe { (*(payload as *mut F))() }
    }
}

/// Runs `func` with the last `trap_info` object registered by `catch_traps`.
///
/// Calls `func` with `None` if `catch_traps` wasn't previously called from this
/// stack frame.
pub fn with_last_info<R>(func: impl FnOnce(Option<&dyn Any>) -> R) -> R {
    tls::with(|state| func(state.map(|s| s.trap_info.as_any())))
}

/// Invokes the contextually-defined context's out-of-gas function.
///
/// (basically delegates to `wasmtime::Store::out_of_gas`)
pub fn out_of_gas() {
    tls::with(|state| state.unwrap().trap_info.out_of_gas())
}

/// Temporary state stored on the stack which is registered in the `tls` module
/// below for calls into wasm.
pub struct CallThreadState<'a> {
    unwind: Cell<UnwindReason>,
    jmp_buf: Cell<*const u8>,
    handling_trap: Cell<bool>,
    trap_info: &'a (dyn TrapInfo + 'a),
    prev: Cell<tls::Ptr>,
}

/// A package of functionality needed by `catch_traps` to figure out what to do
/// when handling a trap.
///
/// Note that this is an `unsafe` trait at least because it's being run in the
/// context of a synchronous signal handler, so it needs to be careful to not
/// access too much state in answering these queries.
pub unsafe trait TrapInfo {
    /// Converts this object into an `Any` to dynamically check its type.
    fn as_any(&self) -> &dyn Any;

    /// Returns whether the given program counter lies within wasm code,
    /// indicating whether we should handle a trap or not.
    fn is_wasm_trap(&self, pc: usize) -> bool;

    /// Uses `call` to call a custom signal handler, if one is specified.
    ///
    /// Returns `true` if `call` returns true, otherwise returns `false`.
    fn custom_signal_handler(&self, call: &dyn Fn(&SignalHandler) -> bool) -> bool;

    /// Returns the maximum size, in bytes, the wasm native stack is allowed to
    /// grow to.
    fn max_wasm_stack(&self) -> usize;

    /// Callback invoked whenever WebAssembly has entirely consumed the fuel
    /// that it was allotted.
    ///
    /// This function may return, and it may also `raise_lib_trap`.
    fn out_of_gas(&self);

    /// Returns the VM interrupts to use for interrupting Wasm code.
    fn interrupts(&self) -> &VMInterrupts;
}

enum UnwindReason {
    None,
    Panic(Box<dyn Any + Send>),
    UserTrap(Box<dyn Error + Send + Sync>),
    LibTrap(Trap),
    JitTrap { backtrace: Backtrace, pc: usize },
}

impl<'a> CallThreadState<'a> {
    fn new(trap_info: &'a (dyn TrapInfo + 'a)) -> CallThreadState<'a> {
        CallThreadState {
            unwind: Cell::new(UnwindReason::None),
            jmp_buf: Cell::new(ptr::null()),
            handling_trap: Cell::new(false),
            trap_info,
            prev: Cell::new(ptr::null()),
        }
    }

    fn with(self, closure: impl FnOnce(&CallThreadState) -> i32) -> Result<(), Trap> {
        let _reset = self.update_stack_limit()?;
        let ret = tls::set(&self, || closure(&self));
        match self.unwind.replace(UnwindReason::None) {
            UnwindReason::None => {
                debug_assert_eq!(ret, 1);
                Ok(())
            }
            UnwindReason::UserTrap(data) => {
                debug_assert_eq!(ret, 0);
                Err(Trap::User(data))
            }
            UnwindReason::LibTrap(trap) => Err(trap),
            UnwindReason::JitTrap { backtrace, pc } => {
                debug_assert_eq!(ret, 0);
                let interrupts = self.trap_info.interrupts();
                let maybe_interrupted =
                    interrupts.stack_limit.load(SeqCst) == wasmtime_environ::INTERRUPTED;
                Err(Trap::Jit {
                    pc,
                    backtrace,
                    maybe_interrupted,
                })
            }
            UnwindReason::Panic(panic) => {
                debug_assert_eq!(ret, 0);
                std::panic::resume_unwind(panic)
            }
        }
    }

    /// Checks and/or initializes the wasm native call stack limit.
    ///
    /// This function will inspect the current state of the stack and calling
    /// context to determine which of three buckets we're in:
    ///
    /// 1. We are the first wasm call on the stack. This means that we need to
    ///    set up a stack limit where beyond which if the native wasm stack
    ///    pointer goes beyond forces a trap. For now we simply reserve an
    ///    arbitrary chunk of bytes (1 MB from roughly the current native stack
    ///    pointer). This logic will likely get tweaked over time.
    ///
    /// 2. We aren't the first wasm call on the stack. In this scenario the wasm
    ///    stack limit is already configured. This case of wasm -> host -> wasm
    ///    we assume that the native stack consumed by the host is accounted for
    ///    in the initial stack limit calculation. That means that in this
    ///    scenario we do nothing.
    ///
    /// 3. We were previously interrupted. In this case we consume the interrupt
    ///    here and return a trap, clearing the interrupt and allowing the next
    ///    wasm call to proceed.
    ///
    /// The return value here is a trap for case 3, a noop destructor in case 2,
    /// and a meaningful destructor in case 1
    ///
    /// For more information about interrupts and stack limits see
    /// `crates/environ/src/cranelift.rs`.
    ///
    /// Note that this function must be called with `self` on the stack, not the
    /// heap/etc.
    fn update_stack_limit(&self) -> Result<impl Drop + '_, Trap> {
        // Determine the stack pointer where, after which, any wasm code will
        // immediately trap. This is checked on the entry to all wasm functions.
        //
        // Note that this isn't 100% precise. We are requested to give wasm
        // `max_wasm_stack` bytes, but what we're actually doing is giving wasm
        // probably a little less than `max_wasm_stack` because we're
        // calculating the limit relative to this function's approximate stack
        // pointer. Wasm will be executed on a frame beneath this one (or next
        // to it). In any case it's expected to be at most a few hundred bytes
        // of slop one way or another. When wasm is typically given a MB or so
        // (a million bytes) the slop shouldn't matter too much.
        let wasm_stack_limit = psm::stack_pointer() as usize - self.trap_info.max_wasm_stack();

        let interrupts = self.trap_info.interrupts();
        let reset_stack_limit = match interrupts.stack_limit.compare_exchange(
            usize::max_value(),
            wasm_stack_limit,
            SeqCst,
            SeqCst,
        ) {
            Ok(_) => {
                // We're the first wasm on the stack so we've now reserved the
                // `max_wasm_stack` bytes of native stack space for wasm.
                // Nothing left to do here now except reset back when we're
                // done.
                true
            }
            Err(n) if n == wasmtime_environ::INTERRUPTED => {
                // This means that an interrupt happened before we actually
                // called this function, which means that we're now
                // considered interrupted. Be sure to consume this interrupt
                // as part of this process too.
                interrupts.stack_limit.store(usize::max_value(), SeqCst);
                return Err(Trap::Wasm {
                    trap_code: ir::TrapCode::Interrupt,
                    backtrace: Backtrace::new_unresolved(),
                });
            }
            Err(_) => {
                // The stack limit was previously set by a previous wasm
                // call on the stack. We leave the original stack limit for
                // wasm in place in that case, and don't reset the stack
                // limit when we're done.
                false
            }
        };

        struct Reset<'a>(bool, &'a AtomicUsize);

        impl Drop for Reset<'_> {
            fn drop(&mut self) {
                if self.0 {
                    self.1.store(usize::max_value(), SeqCst);
                }
            }
        }

        Ok(Reset(reset_stack_limit, &interrupts.stack_limit))
    }

    fn unwind_with(&self, reason: UnwindReason) -> ! {
        self.unwind.replace(reason);
        unsafe {
            Unwind(self.jmp_buf.get());
        }
    }

    /// Trap handler using our thread-local state.
    ///
    /// * `pc` - the program counter the trap happened at
    /// * `call_handler` - a closure used to invoke the platform-specific
    ///   signal handler for each instance, if available.
    ///
    /// Attempts to handle the trap if it's a wasm trap. Returns a few
    /// different things:
    ///
    /// * null - the trap didn't look like a wasm trap and should continue as a
    ///   trap
    /// * 1 as a pointer - the trap was handled by a custom trap handler on an
    ///   instance, and the trap handler should quickly return.
    /// * a different pointer - a jmp_buf buffer to longjmp to, meaning that
    ///   the wasm trap was succesfully handled.
    fn jmp_buf_if_trap(
        &self,
        pc: *const u8,
        call_handler: impl Fn(&SignalHandler) -> bool,
    ) -> *const u8 {
        // If we hit a fault while handling a previous trap, that's quite bad,
        // so bail out and let the system handle this recursive segfault.
        //
        // Otherwise flag ourselves as handling a trap, do the trap handling,
        // and reset our trap handling flag.
        if self.handling_trap.replace(true) {
            return ptr::null();
        }
        let _reset = ResetCell(&self.handling_trap, false);

        // If we haven't even started to handle traps yet, bail out.
        if self.jmp_buf.get().is_null() {
            return ptr::null();
        }

        // First up see if any instance registered has a custom trap handler,
        // in which case run them all. If anything handles the trap then we
        // return that the trap was handled.
        if self.trap_info.custom_signal_handler(&call_handler) {
            return 1 as *const _;
        }

        // If this fault wasn't in wasm code, then it's not our problem
        if !self.trap_info.is_wasm_trap(pc as usize) {
            return ptr::null();
        }

        // If all that passed then this is indeed a wasm trap, so return the
        // `jmp_buf` passed to `Unwind` to resume.
        self.jmp_buf.get()
    }

    fn capture_backtrace(&self, pc: *const u8) {
        let backtrace = Backtrace::new_unresolved();
        self.unwind.replace(UnwindReason::JitTrap {
            backtrace,
            pc: pc as usize,
        });
    }
}

struct ResetCell<'a, T: Copy>(&'a Cell<T>, T);

impl<T: Copy> Drop for ResetCell<'_, T> {
    fn drop(&mut self) {
        self.0.set(self.1);
    }
}

// A private inner module for managing the TLS state that we require across
// calls in wasm. The WebAssembly code is called from C++ and then a trap may
// happen which requires us to read some contextual state to figure out what to
// do with the trap. This `tls` module is used to persist that information from
// the caller to the trap site.
mod tls {
    use super::CallThreadState;
    use std::mem;
    use std::ptr;

    pub use raw::Ptr;

    // An even *more* inner module for dealing with TLS. This actually has the
    // thread local variable and has functions to access the variable.
    //
    // Note that this is specially done to fully encapsulate that the accessors
    // for tls must not be inlined. Wasmtime's async support employs stack
    // switching which can resume execution on different OS threads. This means
    // that borrows of our TLS pointer must never live across accesses because
    // otherwise the access may be split across two threads and cause unsafety.
    //
    // This also means that extra care is taken by the runtime to save/restore
    // these TLS values when the runtime may have crossed threads.
    mod raw {
        use super::CallThreadState;
        use std::cell::Cell;
        use std::ptr;

        pub type Ptr = *const CallThreadState<'static>;

        thread_local!(static PTR: Cell<Ptr> = Cell::new(ptr::null()));

        #[inline(never)] // see module docs for why this is here
        pub fn replace(val: Ptr) -> Ptr {
            // Mark the current thread as handling interrupts for this specific
            // CallThreadState: may clobber the previous entry.
            super::super::sys::register_tls(val);

            PTR.with(|p| p.replace(val))
        }

        #[inline(never)] // see module docs for why this is here
        pub fn get() -> Ptr {
            PTR.with(|p| p.get())
        }
    }

    /// Opaque state used to help control TLS state across stack switches for
    /// async support.
    pub struct TlsRestore(raw::Ptr);

    impl TlsRestore {
        /// Takes the TLS state that is currently configured and returns a
        /// token that is used to replace it later.
        ///
        /// This is not a safe operation since it's intended to only be used
        /// with stack switching found with fibers and async wasmtime.
        pub unsafe fn take() -> TlsRestore {
            // Our tls pointer must be set at this time, and it must not be
            // null. We need to restore the previous pointer since we're
            // removing ourselves from the call-stack, and in the process we
            // null out our own previous field for safety in case it's
            // accidentally used later.
            let raw = raw::get();
            assert!(!raw.is_null());
            let prev = (*raw).prev.replace(ptr::null());
            raw::replace(prev);
            TlsRestore(raw)
        }

        /// Restores a previous tls state back into this thread's TLS.
        ///
        /// This is unsafe because it's intended to only be used within the
        /// context of stack switching within wasmtime.
        pub unsafe fn replace(self) -> Result<(), super::Trap> {
            // When replacing to the previous value of TLS, we might have
            // crossed a thread: make sure the trap-handling lazy initializer
            // runs.
            super::sys::lazy_per_thread_init()?;

            // We need to configure our previous TLS pointer to whatever is in
            // TLS at this time, and then we set the current state to ourselves.
            let prev = raw::get();
            assert!((*self.0).prev.get().is_null());
            (*self.0).prev.set(prev);
            raw::replace(self.0);
            Ok(())
        }
    }

    /// Configures thread local state such that for the duration of the
    /// execution of `closure` any call to `with` will yield `ptr`, unless this
    /// is recursively called again.
    pub fn set<R>(state: &CallThreadState<'_>, closure: impl FnOnce() -> R) -> R {
        struct Reset<'a, 'b>(&'a CallThreadState<'b>);

        impl Drop for Reset<'_, '_> {
            fn drop(&mut self) {
                raw::replace(self.0.prev.replace(ptr::null()));
            }
        }

        // Note that this extension of the lifetime to `'static` should be
        // safe because we only ever access it below with an anonymous
        // lifetime, meaning `'static` never leaks out of this module.
        let ptr = unsafe {
            mem::transmute::<*const CallThreadState<'_>, *const CallThreadState<'static>>(state)
        };
        let prev = raw::replace(ptr);
        state.prev.set(prev);
        let _reset = Reset(state);
        closure()
    }

    /// Returns the last pointer configured with `set` above. Panics if `set`
    /// has not been previously called.
    pub fn with<R>(closure: impl FnOnce(Option<&CallThreadState<'_>>) -> R) -> R {
        let p = raw::get();
        unsafe { closure(if p.is_null() { None } else { Some(&*p) }) }
    }
}
