//! WebAssembly trap handling, which is built on top of the lower-level
//! signalhandling mechanisms.

use crate::{VMContext, VMInterrupts};
use backtrace::Backtrace;
use std::any::Any;
use std::cell::{Cell, UnsafeCell};
use std::error::Error;
use std::mem::MaybeUninit;
use std::ptr;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::Once;
use wasmtime_environ::ir;

pub use self::tls::TlsRestore;

extern "C" {
    #[allow(improper_ctypes)]
    fn RegisterSetjmp(
        jmp_buf: *mut *const u8,
        callback: extern "C" fn(*mut u8, *mut VMContext),
        payload: *mut u8,
        callee: *mut VMContext,
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

/// Globally-set callback to determine whether a program counter is actually a
/// wasm trap.
///
/// This is initialized during `init_traps` below. The definition lives within
/// `wasmtime` currently.
static mut IS_WASM_PC: fn(usize) -> bool = |_| false;

/// This function is required to be called before any WebAssembly is entered.
/// This will configure global state such as signal handlers to prepare the
/// process to receive wasm traps.
///
/// This function must not only be called globally once before entering
/// WebAssembly but it must also be called once-per-thread that enters
/// WebAssembly. Currently in wasmtime's integration this function is called on
/// creation of a `Engine`.
///
/// The `is_wasm_pc` argument is used when a trap happens to determine if a
/// program counter is the pc of an actual wasm trap or not. This is then used
/// to disambiguate faults that happen due to wasm and faults that happen due to
/// bugs in Rust or elsewhere.
pub fn init_traps(is_wasm_pc: fn(usize) -> bool) {
    static INIT: Once = Once::new();
    INIT.call_once(|| unsafe {
        IS_WASM_PC = is_wasm_pc;
        sys::platform_init();
    });
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
pub unsafe fn catch_traps<'a, F>(
    vminterrupts: *mut VMInterrupts,
    signal_handler: Option<*const SignalHandler<'static>>,
    callee: *mut VMContext,
    mut closure: F,
) -> Result<(), Trap>
where
    F: FnMut(*mut VMContext),
{
    return CallThreadState::new(signal_handler).with(vminterrupts, |cx| {
        RegisterSetjmp(
            cx.jmp_buf.as_ptr(),
            call_closure::<F>,
            &mut closure as *mut F as *mut u8,
            callee,
        )
    });

    extern "C" fn call_closure<F>(payload: *mut u8, callee: *mut VMContext)
    where
        F: FnMut(*mut VMContext),
    {
        unsafe { (*(payload as *mut F))(callee) }
    }
}

/// Temporary state stored on the stack which is registered in the `tls` module
/// below for calls into wasm.
pub struct CallThreadState {
    unwind: UnsafeCell<MaybeUninit<UnwindReason>>,
    jmp_buf: Cell<*const u8>,
    handling_trap: Cell<bool>,
    signal_handler: Option<*const SignalHandler<'static>>,
    prev: Cell<tls::Ptr>,
}

enum UnwindReason {
    Panic(Box<dyn Any + Send>),
    UserTrap(Box<dyn Error + Send + Sync>),
    LibTrap(Trap),
    JitTrap { backtrace: Backtrace, pc: usize },
}

impl CallThreadState {
    #[inline]
    fn new(signal_handler: Option<*const SignalHandler<'static>>) -> CallThreadState {
        CallThreadState {
            unwind: UnsafeCell::new(MaybeUninit::uninit()),
            jmp_buf: Cell::new(ptr::null()),
            handling_trap: Cell::new(false),
            signal_handler,
            prev: Cell::new(ptr::null()),
        }
    }

    fn with(
        self,
        interrupts: *mut VMInterrupts,
        closure: impl FnOnce(&CallThreadState) -> i32,
    ) -> Result<(), Trap> {
        let ret = tls::set(&self, || closure(&self))?;
        if ret != 0 {
            return Ok(());
        }
        match unsafe { (*self.unwind.get()).as_ptr().read() } {
            UnwindReason::UserTrap(data) => Err(Trap::User(data)),
            UnwindReason::LibTrap(trap) => Err(trap),
            UnwindReason::JitTrap { backtrace, pc } => {
                let maybe_interrupted = unsafe {
                    (*interrupts).stack_limit.load(SeqCst) == wasmtime_environ::INTERRUPTED
                };
                Err(Trap::Jit {
                    pc,
                    backtrace,
                    maybe_interrupted,
                })
            }
            UnwindReason::Panic(panic) => std::panic::resume_unwind(panic),
        }
    }

    fn unwind_with(&self, reason: UnwindReason) -> ! {
        unsafe {
            (*self.unwind.get()).as_mut_ptr().write(reason);
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
    #[cfg_attr(target_os = "macos", allow(dead_code))] // macOS is more raw and doesn't use this
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
        if let Some(handler) = self.signal_handler {
            if unsafe { call_handler(&*handler) } {
                return 1 as *const _;
            }
        }

        // If this fault wasn't in wasm code, then it's not our problem
        if unsafe { !IS_WASM_PC(pc as usize) } {
            return ptr::null();
        }

        // If all that passed then this is indeed a wasm trap, so return the
        // `jmp_buf` passed to `Unwind` to resume.
        self.jmp_buf.get()
    }

    fn capture_backtrace(&self, pc: *const u8) {
        let backtrace = Backtrace::new_unresolved();
        unsafe {
            (*self.unwind.get())
                .as_mut_ptr()
                .write(UnwindReason::JitTrap {
                    backtrace,
                    pc: pc as usize,
                });
        }
    }
}

struct ResetCell<'a, T: Copy>(&'a Cell<T>, T);

impl<T: Copy> Drop for ResetCell<'_, T> {
    #[inline]
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
    use crate::Trap;
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
        use crate::Trap;
        use std::cell::Cell;
        use std::ptr;

        pub type Ptr = *const CallThreadState;

        // The first entry here is the `Ptr` which is what's used as part of the
        // public interface of this module. The second entry is a boolean which
        // allows the runtime to perform per-thread initialization if necessary
        // for handling traps (e.g. setting up ports on macOS and sigaltstack on
        // Unix).
        thread_local!(static PTR: Cell<(Ptr, bool)> = Cell::new((ptr::null(), false)));

        #[inline(never)] // see module docs for why this is here
        pub fn replace(val: Ptr) -> Result<Ptr, Trap> {
            PTR.with(|p| {
                // When a new value is configured that means that we may be
                // entering WebAssembly so check to see if this thread has
                // performed per-thread initialization for traps.
                let (prev, mut initialized) = p.get();
                if !initialized {
                    super::super::sys::lazy_per_thread_init()?;
                    initialized = true;
                }
                p.set((val, initialized));
                Ok(prev)
            })
        }

        #[inline(never)] // see module docs for why this is here
        pub fn get() -> Ptr {
            PTR.with(|p| p.get().0)
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
        pub unsafe fn take() -> Result<TlsRestore, Trap> {
            // Our tls pointer must be set at this time, and it must not be
            // null. We need to restore the previous pointer since we're
            // removing ourselves from the call-stack, and in the process we
            // null out our own previous field for safety in case it's
            // accidentally used later.
            let raw = raw::get();
            assert!(!raw.is_null());
            let prev = (*raw).prev.replace(ptr::null());
            raw::replace(prev)?;
            Ok(TlsRestore(raw))
        }

        /// Restores a previous tls state back into this thread's TLS.
        ///
        /// This is unsafe because it's intended to only be used within the
        /// context of stack switching within wasmtime.
        pub unsafe fn replace(self) -> Result<(), super::Trap> {
            // We need to configure our previous TLS pointer to whatever is in
            // TLS at this time, and then we set the current state to ourselves.
            let prev = raw::get();
            assert!((*self.0).prev.get().is_null());
            (*self.0).prev.set(prev);
            raw::replace(self.0)?;
            Ok(())
        }
    }

    /// Configures thread local state such that for the duration of the
    /// execution of `closure` any call to `with` will yield `ptr`, unless this
    /// is recursively called again.
    #[inline]
    pub fn set<R>(state: &CallThreadState, closure: impl FnOnce() -> R) -> Result<R, Trap> {
        struct Reset<'a>(&'a CallThreadState);

        impl Drop for Reset<'_> {
            #[inline]
            fn drop(&mut self) {
                raw::replace(self.0.prev.replace(ptr::null()))
                    .expect("tls should be previously initialized");
            }
        }

        let prev = raw::replace(state)?;
        state.prev.set(prev);
        let _reset = Reset(state);
        Ok(closure())
    }

    /// Returns the last pointer configured with `set` above. Panics if `set`
    /// has not been previously called.
    pub fn with<R>(closure: impl FnOnce(Option<&CallThreadState>) -> R) -> R {
        let p = raw::get();
        unsafe { closure(if p.is_null() { None } else { Some(&*p) }) }
    }
}
