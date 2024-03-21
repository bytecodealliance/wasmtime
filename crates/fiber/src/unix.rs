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

#![allow(unused_macros)]

use crate::{RunResult, RuntimeFiberStack};
use std::cell::Cell;
use std::io;
use std::ops::Range;
use std::ptr;

pub enum FiberStack {
    Default {
        // The top of the stack; for stacks allocated by the fiber implementation itself,
        // the base address of the allocation will be `top.sub(len.unwrap())`
        top: *mut u8,
        // The length of the stack
        len: usize,
        mmap: bool,
    },
    Custom(Box<dyn RuntimeFiberStack>),
}

impl FiberStack {
    pub fn new(size: usize) -> io::Result<Self> {
        // Round up our stack size request to the nearest multiple of the
        // page size.
        let page_size = rustix::param::page_size();
        let size = if size == 0 {
            page_size
        } else {
            (size + (page_size - 1)) & (!(page_size - 1))
        };

        unsafe {
            // Add in one page for a guard page and then ask for some memory.
            let mmap_len = size + page_size;
            let mmap = rustix::mm::mmap_anonymous(
                ptr::null_mut(),
                mmap_len,
                rustix::mm::ProtFlags::empty(),
                rustix::mm::MapFlags::PRIVATE,
            )?;

            rustix::mm::mprotect(
                mmap.cast::<u8>().add(page_size).cast(),
                size,
                rustix::mm::MprotectFlags::READ | rustix::mm::MprotectFlags::WRITE,
            )?;

            Ok(Self::Default {
                top: mmap.cast::<u8>().add(mmap_len),
                len: mmap_len,
                mmap: true,
            })
        }
    }

    pub unsafe fn from_raw_parts(base: *mut u8, len: usize) -> io::Result<Self> {
        Ok(Self::Default {
            top: base.add(len),
            len,
            mmap: false,
        })
    }

    pub fn from_custom(custom: Box<dyn RuntimeFiberStack>) -> io::Result<Self> {
        Ok(Self::Custom(custom))
    }

    pub fn top(&self) -> Option<*mut u8> {
        Some(match self {
            FiberStack::Default {
                top,
                len: _,
                mmap: _,
            } => *top,
            FiberStack::Custom(r) => {
                let top = r.top();
                let page_size = rustix::param::page_size();
                assert!(
                    top.align_offset(page_size) == 0,
                    "expected fiber stack top ({}) to be page aligned ({})",
                    top as usize,
                    page_size
                );
                top
            }
        })
    }

    pub fn range(&self) -> Option<Range<usize>> {
        Some(match self {
            FiberStack::Default { top, len, mmap: _ } => {
                let base = unsafe { top.sub(*len) as usize };
                base..base + len
            }
            FiberStack::Custom(s) => {
                let range = s.range();
                let page_size = rustix::param::page_size();
                let start_ptr = range.start as *const u8;
                assert!(
                    start_ptr.align_offset(page_size) == 0,
                    "expected fiber stack end ({}) to be page aligned ({})",
                    range.start,
                    page_size
                );
                let end_ptr = range.end as *const u8;
                assert!(
                    end_ptr.align_offset(page_size) == 0,
                    "expected fiber stack start ({}) to be page aligned ({})",
                    range.end,
                    page_size
                );
                range
            }
        })
    }
}

impl Drop for FiberStack {
    fn drop(&mut self) {
        unsafe {
            if let FiberStack::Default {
                top,
                len,
                mmap: true,
            } = self
            {
                let ret = rustix::mm::munmap(top.sub(*len) as _, *len);
                debug_assert!(ret.is_ok());
            }
        }
    }
}

pub struct Fiber;

pub struct Suspend(*mut u8);

extern "C" {
    #[wasmtime_versioned_export_macros::versioned_link]
    fn wasmtime_fiber_init(
        top_of_stack: *mut u8,
        entry: extern "C" fn(*mut u8, *mut u8),
        entry_arg0: *mut u8,
    );
    #[wasmtime_versioned_export_macros::versioned_link]
    fn wasmtime_fiber_switch(top_of_stack: *mut u8);
    #[allow(dead_code)] // only used in inline assembly for some platforms
    #[wasmtime_versioned_export_macros::versioned_link]
    fn wasmtime_fiber_start();
}

extern "C" fn fiber_start<F, A, B, C>(arg0: *mut u8, top_of_stack: *mut u8)
where
    F: FnOnce(A, &super::Suspend<A, B, C>) -> C,
{
    unsafe {
        let inner = Suspend(top_of_stack);
        let initial = inner.take_resume::<A, B, C>();
        super::Suspend::<A, B, C>::execute(inner, initial, Box::from_raw(arg0.cast::<F>()))
    }
}

impl Fiber {
    pub fn new<F, A, B, C>(stack: &FiberStack, func: F) -> io::Result<Self>
    where
        F: FnOnce(A, &super::Suspend<A, B, C>) -> C,
    {
        unsafe {
            let data = Box::into_raw(Box::new(func)).cast();
            wasmtime_fiber_init(stack.top().unwrap(), fiber_start::<F, A, B, C>, data);
        }

        Ok(Self)
    }

    pub(crate) fn resume<A, B, C>(&self, stack: &FiberStack, result: &Cell<RunResult<A, B, C>>) {
        unsafe {
            // Store where our result is going at the very tip-top of the
            // stack, otherwise known as our reserved slot for this information.
            //
            // In the diagram above this is updating address 0xAff8
            let addr = stack.top().unwrap().cast::<usize>().offset(-1);
            addr.write(result as *const _ as usize);

            wasmtime_fiber_switch(stack.top().unwrap());

            // null this out to help catch use-after-free
            addr.write(0);
        }
    }
}

impl Suspend {
    pub(crate) fn switch<A, B, C>(&self, result: RunResult<A, B, C>) -> A {
        unsafe {
            // Calculate 0xAff8 and then write to it
            (*self.result_location::<A, B, C>()).set(result);
            wasmtime_fiber_switch(self.0);
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
        let ret = self.0.cast::<*const u8>().offset(-1).read();
        assert!(!ret.is_null());
        ret.cast()
    }
}

cfg_if::cfg_if! {
    if #[cfg(target_arch = "aarch64")] {
        mod aarch64;
    } else if #[cfg(target_arch = "x86_64")] {
        mod x86_64;
    } else if #[cfg(target_arch = "x86")] {
        mod x86;
    } else if #[cfg(target_arch = "arm")] {
        mod arm;
    } else if #[cfg(target_arch = "s390x")] {
        // currently `global_asm!` isn't stable on s390x so this is an external
        // assembler file built with the `build.rs`.
    } else if #[cfg(target_arch = "riscv64")]  {
        mod riscv64;
    }else {
        compile_error!("fibers are not supported on this CPU architecture");
    }
}
