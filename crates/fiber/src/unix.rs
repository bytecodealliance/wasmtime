//! The unix fiber implementation has some platform-specific details
//! (naturally) but there's a few details of the stack layout which are common
//! amongst all platforms using this file. Remember that none of this applies to
//! Windows, which is entirely separate.
//!
//! The stack is expected to look pretty standard with a guard page at the end.
//! Currently allocation happens in this file but this is probably going to be
//! refactored to happen somewhere else. Otherwise though the stack layout is
//! expected to look like so:
//!
//!
//! ```text
//! 0xB000 +-----------------------+   <- top of stack
//!        | &Cell<RunResult>      |   <- where to store results
//! 0xAff8 +-----------------------+
//!        | *const u8             |   <- last sp to resume from
//! 0xAff0 +-----------------------+   <- 16-byte aligned
//!        |                       |
//!        ~        ...            ~   <- actual native stack space to use
//!        |                       |
//! 0x1000 +-----------------------+
//!        |  guard page           |
//! 0x0000 +-----------------------+
//! ```
//!
//! Here `0xAff8` is filled in temporarily while `resume` is running. The fiber
//! started with 0xB000 as a parameter so it knows how to find this.
//! Additionally `resumes` stores state at 0xAff0 to restart execution, and
//! `suspend`, which has 0xB000 so it can find this, will read that and write
//! its own resumption information into this slot as well.

use crate::RunResult;
use std::cell::Cell;
use std::io;
use std::ptr;

pub struct Fiber {
    // The top of the stack; for stacks allocated by the fiber implementation itself,
    // the base address of the allocation will be `top_of_stack.sub(alloc_len.unwrap())`
    top_of_stack: *mut u8,
    alloc_len: Option<usize>,
}

pub struct Suspend {
    top_of_stack: *mut u8,
}

extern "C" {
    fn wasmtime_fiber_init(
        top_of_stack: *mut u8,
        entry: extern "C" fn(*mut u8, *mut u8),
        entry_arg0: *mut u8,
    );
    fn wasmtime_fiber_switch(top_of_stack: *mut u8);
}

extern "C" fn fiber_start<F, A, B, C>(arg0: *mut u8, top_of_stack: *mut u8)
where
    F: FnOnce(A, &super::Suspend<A, B, C>) -> C,
{
    unsafe {
        let inner = Suspend { top_of_stack };
        let initial = inner.take_resume::<A, B, C>();
        super::Suspend::<A, B, C>::execute(inner, initial, Box::from_raw(arg0.cast::<F>()))
    }
}

impl Fiber {
    pub fn new<F, A, B, C>(stack_size: usize, func: F) -> io::Result<Self>
    where
        F: FnOnce(A, &super::Suspend<A, B, C>) -> C,
    {
        let fiber = Self::alloc_with_stack(stack_size)?;
        fiber.init(func);
        Ok(fiber)
    }

    pub fn new_with_stack<F, A, B, C>(top_of_stack: *mut u8, func: F) -> io::Result<Self>
    where
        F: FnOnce(A, &super::Suspend<A, B, C>) -> C,
    {
        let fiber = Self {
            top_of_stack,
            alloc_len: None,
        };

        fiber.init(func);

        Ok(fiber)
    }

    fn init<F, A, B, C>(&self, func: F)
    where
        F: FnOnce(A, &super::Suspend<A, B, C>) -> C,
    {
        unsafe {
            let data = Box::into_raw(Box::new(func)).cast();
            wasmtime_fiber_init(self.top_of_stack, fiber_start::<F, A, B, C>, data);
        }
    }

    fn alloc_with_stack(stack_size: usize) -> io::Result<Self> {
        unsafe {
            // Round up our stack size request to the nearest multiple of the
            // page size.
            let page_size = libc::sysconf(libc::_SC_PAGESIZE) as usize;
            let stack_size = if stack_size == 0 {
                page_size
            } else {
                (stack_size + (page_size - 1)) & (!(page_size - 1))
            };

            // Add in one page for a guard page and then ask for some memory.
            let mmap_len = stack_size + page_size;
            let mmap = libc::mmap(
                ptr::null_mut(),
                mmap_len,
                libc::PROT_NONE,
                libc::MAP_ANON | libc::MAP_PRIVATE,
                -1,
                0,
            );
            if mmap == libc::MAP_FAILED {
                return Err(io::Error::last_os_error());
            }
            let ret = Self {
                top_of_stack: mmap.cast::<u8>().add(mmap_len),
                alloc_len: Some(mmap_len),
            };
            let res = libc::mprotect(
                mmap.cast::<u8>().add(page_size).cast(),
                stack_size,
                libc::PROT_READ | libc::PROT_WRITE,
            );
            if res != 0 {
                Err(io::Error::last_os_error())
            } else {
                Ok(ret)
            }
        }
    }

    pub(crate) fn resume<A, B, C>(&self, result: &Cell<RunResult<A, B, C>>) {
        unsafe {
            // Store where our result is going at the very tip-top of the
            // stack, otherwise known as our reserved slot for this information.
            //
            // In the diagram above this is updating address 0xAff8
            let addr = self.top_of_stack.cast::<usize>().offset(-1);
            addr.write(result as *const _ as usize);

            wasmtime_fiber_switch(self.top_of_stack);

            // null this out to help catch use-after-free
            addr.write(0);
        }
    }
}

impl Drop for Fiber {
    fn drop(&mut self) {
        unsafe {
            if let Some(alloc_len) = self.alloc_len {
                let ret = libc::munmap(self.top_of_stack.sub(alloc_len) as _, alloc_len);
                debug_assert!(ret == 0);
            }
        }
    }
}

impl Suspend {
    pub(crate) fn switch<A, B, C>(&self, result: RunResult<A, B, C>) -> A {
        unsafe {
            // Calculate 0xAff8 and then write to it
            (*self.result_location::<A, B, C>()).set(result);
            wasmtime_fiber_switch(self.top_of_stack);
            self.take_resume::<A, B, C>()
        }
    }

    unsafe fn take_resume<A, B, C>(&self) -> A {
        match (*self.result_location::<A, B, C>()).replace(RunResult::Executing) {
            RunResult::Resuming(val) => val,
            _ => panic!("not in resuming state"),
        }
    }

    unsafe fn result_location<A, B, C>(&self) -> *const Cell<RunResult<A, B, C>> {
        let ret = self.top_of_stack.cast::<*const u8>().offset(-1).read();
        assert!(!ret.is_null());
        return ret.cast();
    }
}
