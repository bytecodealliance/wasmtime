//! Linux specific implementation of `outband_fuel`.
//!
//! The implementation here is built around signal handlers. We register a custom signal handler
//! for SIGUSR1. When a check handle is created it captures the current thread and process id.
//!
//! For performing the out-of-band check, a SIGUSR1 signal is sent using the `tgkill` system call.

use crate::outband_fuel::IS_WASM_PC;
use crate::traphandlers::{raise_lib_trap, tls};
use std::io;
use std::mem::{self, MaybeUninit};
use std::ptr;
use wasmtime_environ::TrapCode;

#[inline]
unsafe fn tgkill(tgid: libc::pid_t, tid: libc::pid_t, sig: libc::c_int) -> libc::c_int {
    libc::syscall(libc::SYS_tgkill, tgid, tid, sig) as libc::c_int
}

pub struct CheckHandle {
    /// The PID of the current process.
    my_pid: libc::pid_t,
    /// The TID of the target process.
    target_tid: libc::pid_t,
}

impl CheckHandle {
    pub fn from_current_thread() -> Self {
        unsafe {
            Self {
                my_pid: libc::getpid(),
                target_tid: libc::gettid(),
            }
        }
    }

    pub fn check(&self) {
        unsafe {
            // Send SIGUSR1 signal to the thread of interest.  Ignore the return value of the
            // syscall deliberately.
            let _ = tgkill(self.my_pid, self.target_tid, libc::SIGUSR1);
        }
    }
}

static mut PREV_SIGUSR1: MaybeUninit<libc::sigaction> = MaybeUninit::uninit();

pub fn platform_init() {
    unsafe {
        let mut handler: libc::sigaction = mem::zeroed();
        handler.sa_flags = libc::SA_SIGINFO;
        handler.sa_sigaction = trap_handler as usize;
        libc::sigemptyset(&mut handler.sa_mask);
        if libc::sigaction(libc::SIGUSR1, &handler, PREV_SIGUSR1.as_mut_ptr()) != 0 {
            panic!(
                "unable to install signal handler for async fuel: {}",
                io::Error::last_os_error(),
            );
        }
    }
}

unsafe extern "C" fn trap_handler(
    signum: libc::c_int,
    siginfo: *mut libc::siginfo_t,
    context: *mut libc::c_void,
) {
    let handled = tls::with(|info| {
        // we don't check if the tls info is set, because it's may not be immediatelly after
        // the pthread_t is initialized.
        let info = match info {
            Some(info) => info,
            None => return true,
        };

        let thread_state = get_thread_state(context);
        if IS_WASM_PC(thread_state.pc) {
            // at this point the fuel register is defined.

            // flush it into the VMRuntimeLimits.
            *(*info.runtime_limits()).fuel_consumed.get() = thread_state.fuel;

            // check if the fuel ran out and if so interrupt.
            if thread_state.fuel > 0 {
                raise_lib_trap(TrapCode::Interrupt);
            }
        }

        true
    });

    if handled {
        return;
    }

    // This signal is not for any compiled wasm code we expect, so we
    // need to forward the signal to the next handler. If there is no
    // next handler (SIG_IGN or SIG_DFL), then it's time to crash. To do
    // this, we set the signal back to its original disposition and
    // return. This will cause the faulting op to be re-executed which
    // will crash in the normal way. If there is a next handler, call
    // it. It will either crash synchronously, fix up the instruction
    // so that execution can continue and return, or trigger a crash by
    // returning the signal to it's original disposition and returning.
    let previous = &*PREV_SIGUSR1.as_ptr();
    if previous.sa_flags & libc::SA_SIGINFO != 0 {
        mem::transmute::<usize, extern "C" fn(libc::c_int, *mut libc::siginfo_t, *mut libc::c_void)>(
            previous.sa_sigaction,
        )(signum, siginfo, context)
    } else if previous.sa_sigaction == libc::SIG_DFL || previous.sa_sigaction == libc::SIG_IGN {
        libc::sigaction(signum, previous, ptr::null_mut());
    } else {
        mem::transmute::<usize, extern "C" fn(libc::c_int)>(previous.sa_sigaction)(signum)
    }
}

struct ThreadState {
    pc: usize,
    /// The contents of the fuel register.
    ///
    /// Only defined if `pc` points to a code compiled from wasm.
    fuel: i64,
}

unsafe fn get_thread_state(cx: *mut libc::c_void) -> ThreadState {
    cfg_if::cfg_if! {
        if #[cfg(all(target_os = "linux", target_arch = "x86_64"))] {
            let cx = &*(cx as *const libc::ucontext_t);
            let rip = cx.uc_mcontext.gregs[libc::REG_RIP as usize] as *const u8;
            let r15 = cx.uc_mcontext.gregs[libc::REG_R15 as usize];
            ThreadState {
                pc: rip as usize,
                fuel: r15,
            }
        } else if #[cfg(all(target_os = "macos", target_arch = "aarch64"))] {
            let cx = &*(cx as *const libc::ucontext_t);
            let pc = (*cx.uc_mcontext).__ss.__pc as *const u8;
            let x21 = (*cx.uc_mcontext).__ss.__x[21] as i64;
            ThreadState {
                pc: pc as usize,
                fuel: x21,
            }
        } else {
            panic!("unsupported platform")
        }
    }
}
