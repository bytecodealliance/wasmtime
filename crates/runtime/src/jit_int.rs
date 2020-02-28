//! The GDB's JIT compilation interface. The low level module that exposes
//! the __jit_debug_register_code() and __jit_debug_descriptor to register
//! or unregister generated object images with debuggers.

use std::ptr;

#[repr(C)]
struct JITCodeEntry {
    next_entry: *mut JITCodeEntry,
    prev_entry: *mut JITCodeEntry,
    symfile_addr: *const u8,
    symfile_size: u64,
}

const JIT_NOACTION: u32 = 0;
const JIT_REGISTER_FN: u32 = 1;
const JIT_UNREGISTER_FN: u32 = 2;

#[repr(C)]
struct JITDescriptor {
    version: u32,
    action_flag: u32,
    relevant_entry: *mut JITCodeEntry,
    first_entry: *mut JITCodeEntry,
}

#[no_mangle]
#[used]
static mut __jit_debug_descriptor: JITDescriptor = JITDescriptor {
    version: 1,
    action_flag: JIT_NOACTION,
    relevant_entry: ptr::null_mut(),
    first_entry: ptr::null_mut(),
};

#[no_mangle]
#[inline(never)]
extern "C" fn __jit_debug_register_code() {
    // Hack to not allow inlining even when Rust wants to do it in release mode.
    let x = 3;
    unsafe {
        std::ptr::read_volatile(&x);
    }
}

/// Registeration for JIT image
pub struct GdbJitImageRegistration {
    entry: *mut JITCodeEntry,
    file: Vec<u8>,
}

impl GdbJitImageRegistration {
    /// Registers JIT image using __jit_debug_register_code
    pub fn register(file: Vec<u8>) -> Self {
        Self {
            entry: unsafe { register_gdb_jit_image(&file) },
            file,
        }
    }

    /// JIT image used in registration
    pub fn file(&self) -> &[u8] {
        &self.file
    }
}

impl Drop for GdbJitImageRegistration {
    fn drop(&mut self) {
        unsafe {
            unregister_gdb_jit_image(self.entry);
        }
    }
}

unsafe fn register_gdb_jit_image(file: &[u8]) -> *mut JITCodeEntry {
    // Create a code entry for the file, which gives the start and size of the symbol file.
    let entry = Box::into_raw(Box::new(JITCodeEntry {
        next_entry: __jit_debug_descriptor.first_entry,
        prev_entry: ptr::null_mut(),
        symfile_addr: file.as_ptr(),
        symfile_size: file.len() as u64,
    }));
    // Add it to the linked list in the JIT descriptor.
    if !__jit_debug_descriptor.first_entry.is_null() {
        (*__jit_debug_descriptor.first_entry).prev_entry = entry;
    }
    __jit_debug_descriptor.first_entry = entry;
    // Point the relevant_entry field of the descriptor at the entry.
    __jit_debug_descriptor.relevant_entry = entry;
    // Set action_flag to JIT_REGISTER and call __jit_debug_register_code.
    __jit_debug_descriptor.action_flag = JIT_REGISTER_FN;
    __jit_debug_register_code();

    __jit_debug_descriptor.action_flag = JIT_NOACTION;
    __jit_debug_descriptor.relevant_entry = ptr::null_mut();
    entry
}

unsafe fn unregister_gdb_jit_image(entry: *mut JITCodeEntry) {
    // Remove the code entry corresponding to the code from the linked list.
    if !(*entry).prev_entry.is_null() {
        (*(*entry).prev_entry).next_entry = (*entry).next_entry;
    } else {
        __jit_debug_descriptor.first_entry = (*entry).next_entry;
    }
    if !(*entry).next_entry.is_null() {
        (*(*entry).next_entry).prev_entry = (*entry).prev_entry;
    }
    // Point the relevant_entry field of the descriptor at the code entry.
    __jit_debug_descriptor.relevant_entry = entry;
    // Set action_flag to JIT_UNREGISTER and call __jit_debug_register_code.
    __jit_debug_descriptor.action_flag = JIT_UNREGISTER_FN;
    __jit_debug_register_code();

    __jit_debug_descriptor.action_flag = JIT_NOACTION;
    __jit_debug_descriptor.relevant_entry = ptr::null_mut();
    let _box = Box::from_raw(entry);
}
