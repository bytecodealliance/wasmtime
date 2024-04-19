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

use crate::{RunResult, RuntimeFiberStack};
use std::cell::Cell;
use std::io;
use std::ops::Range;
use std::ptr;

pub struct FiberStack {
    base: *mut u8,
    len: usize,

    /// Stored here to ensure that when this `FiberStack` the backing storage,
    /// if any, is additionally dropped.
    _storage: FiberStackStorage,
}

enum FiberStackStorage {
    Mmap(#[allow(dead_code)] MmapFiberStack),
    Unmanaged,
    Custom(#[allow(dead_code)] Box<dyn RuntimeFiberStack>),
}

impl FiberStack {
    pub fn new(size: usize) -> io::Result<Self> {
        // See comments in `mod asan` below for why asan has a different stack
        // allocation strategy.
        if cfg!(asan) {
            return Self::from_custom(asan::new_fiber_stack(size)?);
        }
        let page_size = rustix::param::page_size();
        let stack = MmapFiberStack::new(size)?;

        Ok(FiberStack {
            base: stack.mapping_base.wrapping_byte_add(page_size),
            len: stack.mapping_len - page_size,
            _storage: FiberStackStorage::Mmap(stack),
        })
    }

    pub unsafe fn from_raw_parts(base: *mut u8, len: usize) -> io::Result<Self> {
        // See comments in `mod asan` below for why asan has a different stack
        // allocation strategy.
        if cfg!(asan) {
            return Self::from_custom(asan::new_fiber_stack(len)?);
        }
        Ok(FiberStack {
            base,
            len,
            _storage: FiberStackStorage::Unmanaged,
        })
    }

    pub fn from_custom(custom: Box<dyn RuntimeFiberStack>) -> io::Result<Self> {
        let range = custom.range();
        let page_size = rustix::param::page_size();
        let start_ptr = range.start as *mut u8;
        assert!(
            start_ptr.align_offset(page_size) == 0,
            "expected fiber stack base ({start_ptr:?}) to be page aligned ({page_size:#x})",
        );
        let end_ptr = range.end as *const u8;
        assert!(
            end_ptr.align_offset(page_size) == 0,
            "expected fiber stack end ({end_ptr:?}) to be page aligned ({page_size:#x})",
        );
        Ok(FiberStack {
            base: start_ptr,
            len: range.len(),
            _storage: FiberStackStorage::Custom(custom),
        })
    }

    pub fn top(&self) -> Option<*mut u8> {
        Some(self.base.wrapping_byte_add(self.len))
    }

    pub fn range(&self) -> Option<Range<usize>> {
        let base = self.base as usize;
        Some(base..base + self.len)
    }
}

struct MmapFiberStack {
    mapping_base: *mut u8,
    mapping_len: usize,
}

unsafe impl Send for MmapFiberStack {}
unsafe impl Sync for MmapFiberStack {}

impl MmapFiberStack {
    fn new(size: usize) -> io::Result<Self> {
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
                mmap.byte_add(page_size),
                size,
                rustix::mm::MprotectFlags::READ | rustix::mm::MprotectFlags::WRITE,
            )?;

            Ok(MmapFiberStack {
                mapping_base: mmap.cast(),
                mapping_len: mmap_len,
            })
        }
    }
}

impl Drop for MmapFiberStack {
    fn drop(&mut self) {
        unsafe {
            let ret = rustix::mm::munmap(self.mapping_base.cast(), self.mapping_len);
            debug_assert!(ret.is_ok());
        }
    }
}

pub struct Fiber;

pub struct Suspend {
    top_of_stack: *mut u8,
    previous: asan::PreviousStack,
}

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
    F: FnOnce(A, &mut super::Suspend<A, B, C>) -> C,
{
    unsafe {
        // Complete the `start_switch` AddressSanitizer handshake which would
        // have been started in `Fiber::resume`.
        let previous = asan::fiber_start_complete();

        let inner = Suspend {
            top_of_stack,
            previous,
        };
        let initial = inner.take_resume::<A, B, C>();
        super::Suspend::<A, B, C>::execute(inner, initial, Box::from_raw(arg0.cast::<F>()))
    }
}

impl Fiber {
    pub fn new<F, A, B, C>(stack: &FiberStack, func: F) -> io::Result<Self>
    where
        F: FnOnce(A, &mut super::Suspend<A, B, C>) -> C,
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

            asan::fiber_switch(
                stack.top().unwrap(),
                false,
                &mut asan::PreviousStack::new(stack),
            );

            // null this out to help catch use-after-free
            addr.write(0);
        }
    }
}

impl Suspend {
    pub(crate) fn switch<A, B, C>(&mut self, result: RunResult<A, B, C>) -> A {
        unsafe {
            let is_finishing = match &result {
                RunResult::Returned(_) | RunResult::Panicked(_) => true,
                RunResult::Executing | RunResult::Resuming(_) | RunResult::Yield(_) => false,
            };
            // Calculate 0xAff8 and then write to it
            (*self.result_location::<A, B, C>()).set(result);

            asan::fiber_switch(self.top_of_stack, is_finishing, &mut self.previous);

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
    } else {
        compile_error!("fibers are not supported on this CPU architecture");
    }
}

/// Support for AddressSanitizer to support stack manipulations we do in this
/// fiber implementation.
///
/// This module uses, when fuzzing is enabled, special intrinsics provided by
/// the sanitizer runtime called `__sanitizer_{start,finish}_switch_fiber`.
/// These aren't really super heavily documented and the current implementation
/// is inspired by googling the functions and looking at Boost & Julia's usage
/// of them as well as the documentation for these functions in their own
/// header file in the LLVM source tree. The general idea is that they're
/// called around every stack switch with some other fiddly bits as well.
#[cfg(asan)]
mod asan {
    use super::{FiberStack, MmapFiberStack, RuntimeFiberStack};
    use rustix::param::page_size;
    use std::mem::ManuallyDrop;
    use std::ops::Range;
    use std::sync::Mutex;

    /// State for the "previous stack" maintained by asan itself and fed in for
    /// custom stacks.
    pub struct PreviousStack {
        bottom: *const u8,
        size: usize,
    }

    impl PreviousStack {
        pub fn new(stack: &FiberStack) -> PreviousStack {
            let range = stack.range().unwrap();
            PreviousStack {
                bottom: range.start as *const u8,
                // Discount the two pointers we store at the top of the stack,
                // so subtract two pointers.
                size: range.len() - 2 * std::mem::size_of::<*const u8>(),
            }
        }
    }

    impl Default for PreviousStack {
        fn default() -> PreviousStack {
            PreviousStack {
                bottom: std::ptr::null(),
                size: 0,
            }
        }
    }

    /// Switches the current stack to `top_of_stack`
    ///
    /// * `top_of_stack` - for going to fibers this is calculated and for
    ///   restoring back to the original stack this was saved during the initial
    ///   transition.
    /// * `is_finishing` - whether or not we're switching off a fiber for the
    ///   final time; customizes how asan intrinsics are invoked.
    /// * `prev` - the stack we're switching to initially and saves the
    ///   stack to return to upon resumption.
    pub unsafe fn fiber_switch(
        top_of_stack: *mut u8,
        is_finishing: bool,
        prev: &mut PreviousStack,
    ) {
        let mut private_asan_pointer = std::ptr::null_mut();

        // If this fiber is finishing then NULL is passed to asan to let it know
        // that it can deallocate the "fake stack" that it's tracking for this
        // fiber.
        let private_asan_pointer_ref = if is_finishing {
            None
        } else {
            Some(&mut private_asan_pointer)
        };

        // NB: in fiddling with asan an optimizations and such it appears that
        // these functions need to be "very close to each other". If other Rust
        // functions are invoked or added as an abstraction here that appears to
        // trigger false positives in ASAN. That leads to the design of this
        // module as-is where this function exists to have these three
        // functions very close to one another.
        __sanitizer_start_switch_fiber(private_asan_pointer_ref, prev.bottom, prev.size);
        super::wasmtime_fiber_switch(top_of_stack);
        __sanitizer_finish_switch_fiber(private_asan_pointer, &mut prev.bottom, &mut prev.size);
    }

    /// Hook for when a fiber first starts, used to configure ASAN.
    pub unsafe fn fiber_start_complete() -> PreviousStack {
        let mut ret = PreviousStack::default();
        __sanitizer_finish_switch_fiber(std::ptr::null_mut(), &mut ret.bottom, &mut ret.size);
        ret
    }

    // These intrinsics are provided by the address sanitizer runtime. Their C
    // signatures were translated into Rust-isms here with `Option` and `&mut`.
    extern "C" {
        fn __sanitizer_start_switch_fiber(
            private_asan_pointer_save: Option<&mut *mut u8>,
            bottom: *const u8,
            size: usize,
        );
        fn __sanitizer_finish_switch_fiber(
            private_asan_pointer: *mut u8,
            bottom_old: &mut *const u8,
            size_old: &mut usize,
        );
    }

    /// This static is a workaround for llvm/llvm-project#53891, notably this is
    /// a global cache of all fiber stacks.
    ///
    /// The problem with ASAN is that if we allocate memory for a stack, use it
    /// as a stack, deallocate the stack, and then when that memory is later
    /// mapped as normal heap memory. This is possible due to `mmap` reusing
    /// addresses and it ends up confusing ASAN. In this situation ASAN will
    /// have false positives about stack overflows saying that writes to
    /// freshly-allocated memory, which just happened to historically be a
    /// stack, are a stack overflow.
    ///
    /// This static works around the issue by ensuring that, only when asan is
    /// enabled, all stacks are cached globally. Stacks are never deallocated
    /// and forever retained here. This only works if the number of stacks
    /// retained here is relatively small to prevent OOM from continuously
    /// running programs. That's hopefully the case as ASAN is mostly used in
    /// OSS-Fuzz and our fuzzers only fuzz one thing at a time per thread
    /// meaning that this should only ever be a relatively small set of stacks.
    static FIBER_STACKS: Mutex<Vec<MmapFiberStack>> = Mutex::new(Vec::new());

    pub fn new_fiber_stack(size: usize) -> std::io::Result<Box<dyn RuntimeFiberStack>> {
        let needed_size = size + page_size();
        let mut stacks = FIBER_STACKS.lock().unwrap();

        let stack = match stacks.iter().position(|i| needed_size <= i.mapping_len) {
            // If an appropriately sized stack was already allocated, then use
            // that one.
            Some(i) => stacks.remove(i),
            // ... otherwise allocate a brand new stack.
            None => MmapFiberStack::new(size)?,
        };
        let stack = AsanFiberStack(ManuallyDrop::new(stack));
        Ok(Box::new(stack))
    }

    /// Custom structure used to prevent the interior mmap-allocated stack from
    /// actually getting unmapped.
    ///
    /// On drop this stack will return the interior stack to the global
    /// `FIBER_STACKS` list.
    struct AsanFiberStack(ManuallyDrop<MmapFiberStack>);

    unsafe impl RuntimeFiberStack for AsanFiberStack {
        fn top(&self) -> *mut u8 {
            self.0.mapping_base.wrapping_byte_add(self.0.mapping_len)
        }

        fn range(&self) -> Range<usize> {
            let base = self.0.mapping_base as usize;
            let end = base + self.0.mapping_len;
            base + page_size()..end
        }
    }

    impl Drop for AsanFiberStack {
        fn drop(&mut self) {
            let stack = unsafe { ManuallyDrop::take(&mut self.0) };
            FIBER_STACKS.lock().unwrap().push(stack);
        }
    }
}

// Shim module that's the same as above but only has stubs.
#[cfg(not(asan))]
mod asan_disabled {
    use super::{FiberStack, RuntimeFiberStack};

    #[derive(Default)]
    pub struct PreviousStack;

    impl PreviousStack {
        #[inline]
        pub fn new(_stack: &FiberStack) -> PreviousStack {
            PreviousStack
        }
    }

    pub unsafe fn fiber_switch(
        top_of_stack: *mut u8,
        _is_finishing: bool,
        _prev: &mut PreviousStack,
    ) {
        super::wasmtime_fiber_switch(top_of_stack);
    }

    #[inline]
    pub unsafe fn fiber_start_complete() -> PreviousStack {
        PreviousStack
    }

    pub fn new_fiber_stack(_size: usize) -> std::io::Result<Box<dyn RuntimeFiberStack>> {
        unimplemented!()
    }
}

#[cfg(not(asan))]
use asan_disabled as asan;
