//! macOS implementation of `outband_fuel`.

use mach::{
    kern_return::kern_return_t,
    mach_init::mach_thread_self,
    mach_types::thread_port_t,
    message::mach_msg_type_number_t,
    thread_act::{thread_get_state, thread_resume, thread_suspend},
    thread_status::{thread_state_flavor_t, thread_state_t},
};
use std::mem;

pub static ARM_THREAD_STATE64: thread_state_flavor_t = 6;

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
pub static THREAD_STATE_NONE: thread_state_flavor_t = 13;
#[cfg(target_arch = "aarch64")]
pub static THREAD_STATE_NONE: thread_state_flavor_t = 5;

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, Hash, PartialOrd, PartialEq, Eq, Ord)]
pub struct arm_thread_state64_t {
    pub __x: [u64; 29],
    pub __fp: u64, // frame pointer x29
    pub __lr: u64, // link register x30
    pub __sp: u64, // stack pointer x31
    pub __pc: u64,
    pub __cpsr: u32,
    pub __pad: u32,
}

impl arm_thread_state64_t {
    pub fn count() -> mach_msg_type_number_t {
        (mem::size_of::<Self>() / mem::size_of::<u32>()) as mach_msg_type_number_t
    }
}

extern "C" {
    pub fn thread_set_state(
        target_act: thread_port_t,
        flavor: thread_state_flavor_t,
        new_state: thread_state_t,
        new_stateCnt: mach_msg_type_number_t,
    ) -> kern_return_t;
}

pub struct CheckHandle {
    target: thread_port_t,
}

impl CheckHandle {
    pub fn from_current_thread() -> Self {
        Self {
            // TODO: mach_thread_self requires deallocation to balance.
            // also, chromium uses pthread_mach_thread_np(pthread_self()) which is two
            // libc calls and no system calls.
            // https://codereview.chromium.org/276043002/
            target: unsafe { mach_thread_self() },
        }
    }

    pub fn check(&self) {
        // TODO:
        // This function should:
        //
        // - check if the thread is the same as the current one. If it is then bail.
        // - suspend the thread.
        // - get the current state of the thread.
        // - check the fuel register in the state.
        // - if the fuel register is overflown, then modify the state to point to the trap shim,
        //   and set the current state of the thread.
        // - resume the thread.
        unimplemented!()
    }
}

// TODO: The trap shim. Among other things it should flush the fuel reg into the runtime limits.

pub fn platform_init() {}
