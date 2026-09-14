//! Runtime pointer-authentication state on macOS.

use mach2::kern_return::KERN_SUCCESS;
use mach2::mach_init::mach_thread_self;
use mach2::mach_port::mach_port_deallocate;
use mach2::structs::arm_thread_state64_t;
use mach2::thread_act::thread_get_state;
use mach2::thread_status::ARM_THREAD_STATE64;
use mach2::traps::mach_task_self;

pub(super) fn pointer_authentication_enabled() -> Result<bool, &'static str> {
    // macOS disables pointer authentication for ordinary arm64 executables and
    // some arm64e plugin hosts. Read the current thread's state rather than
    // inferring this from CPU features or the executable's Mach-O header.
    let mut state = arm_thread_state64_t::default();
    let mut count = arm_thread_state64_t::count();
    // SAFETY: mach_thread_self supplies an owned send right to this thread.
    // The buffer and count match ARM_THREAD_STATE64, and the right is released
    // after the query, including when the query fails.
    let result = unsafe {
        let thread = mach_thread_self();
        let result = thread_get_state(
            thread,
            ARM_THREAD_STATE64,
            (&mut state as *mut arm_thread_state64_t).cast(),
            &mut count,
        );
        mach_port_deallocate(mach_task_self(), thread);
        result
    };
    if result != KERN_SUCCESS || count != arm_thread_state64_t::count() {
        return Err("failed to query macOS pointer authentication state");
    }

    // mach/arm/_structs.h calls this field __opaque_flags when pointer
    // authentication is enabled at compile time, and __pad otherwise.
    const ARM_THREAD_STATE64_FLAGS_NO_PTRAUTH: u32 = 0x1;
    Ok(state.__pad & ARM_THREAD_STATE64_FLAGS_NO_PTRAUTH == 0)
}
