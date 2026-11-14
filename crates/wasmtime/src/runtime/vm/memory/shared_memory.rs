use crate::Engine;
use crate::prelude::*;
use crate::runtime::vm::memory::{DefaultMemoryCreator, LocalMemory, validate_atomic_addr};
#[cfg(feature = "std")]
use crate::runtime::vm::parking_spot::{ParkingSpot, Waiter};
use crate::runtime::vm::{self, Memory, VMMemoryDefinition, WaitResult};
use crate::sync::RwLock;
use alloc::sync::Arc;
use core::ops::Range;
use core::ptr::NonNull;
#[cfg(target_has_atomic = "64")]
use core::sync::atomic::AtomicU64;
use core::sync::atomic::{AtomicU32, Ordering};
use core::time::Duration;
#[cfg(feature = "std")]
use std::{cell::RefCell, time::Instant};
use wasmtime_environ::Trap;

/// For shared memory (and only for shared memory), this lock-version restricts
/// access when growing the memory or checking its size. This is to conform with
/// the [thread proposal]: "When `IsSharedArrayBuffer(...)` is true, the return
/// value should be the result of an atomic read-modify-write of the new size to
/// the internal `length` slot."
///
/// [thread proposal]:
///     https://github.com/WebAssembly/threads/blob/master/proposals/threads/Overview.md#webassemblymemoryprototypegrow
#[derive(Clone)]
pub struct SharedMemory(Arc<SharedMemoryInner>);

struct SharedMemoryInner {
    memory: RwLock<LocalMemory>,
    #[cfg(feature = "std")]
    spot: ParkingSpot,
    ty: wasmtime_environ::Memory,
    def: LongTermVMMemoryDefinition,
}

impl SharedMemory {
    /// Construct a new [`SharedMemory`].
    pub fn new(engine: &Engine, ty: &wasmtime_environ::Memory) -> Result<Self> {
        let tunables = engine.tunables();
        let memory_tunables = wasmtime_environ::MemoryTunables::new(
            tunables,
            wasmtime_environ::MemoryKind::LinearMemory,
        );
        // Note that without a limiter being passed to `limit_new` this
        // `assert_ready` should never panic.
        let (minimum_bytes, maximum_bytes) = vm::assert_ready(Memory::limit_new(ty, None))?;
        let request = crate::MemoryRequest::new(ty, &memory_tunables, minimum_bytes, maximum_bytes);
        let creator = engine
            .config()
            .mem_creator
            .as_deref()
            .unwrap_or(&DefaultMemoryCreator);
        let boxed = creator.new_memory(&request)?;
        Self::wrap(
            engine,
            ty,
            LocalMemory::new(ty, &memory_tunables, boxed, None)?,
        )
    }

    /// Wrap an existing [Memory] with the locking provided by a [SharedMemory].
    pub fn wrap(
        engine: &Engine,
        ty: &wasmtime_environ::Memory,
        memory: LocalMemory,
    ) -> Result<Self> {
        if !engine.config().shared_memory {
            bail!(
                "shared memory support is disabled for this engine -- see `Config::shared_memory`"
            );
        }
        if !ty.shared {
            bail!("shared memory must have a `shared` memory type");
        }
        if !cfg!(target_has_atomic = "64") {
            bail!("shared memory requires a target with 64-bit atomics");
        }
        Ok(Self(try_new::<Arc<_>>(SharedMemoryInner {
            ty: *ty,
            #[cfg(feature = "std")]
            spot: ParkingSpot::default(),
            def: LongTermVMMemoryDefinition(memory.vmmemory()),
            memory: RwLock::new(memory),
        })?))
    }

    /// Return the memory type for this [`SharedMemory`].
    pub fn ty(&self) -> &wasmtime_environ::Memory {
        &self.0.ty
    }

    /// Convert this shared memory into a [`Memory`].
    pub fn as_memory(self) -> Memory {
        Memory::Shared(self)
    }

    /// Return a pointer to the shared memory's [VMMemoryDefinition].
    pub fn vmmemory_ptr(&self) -> NonNull<VMMemoryDefinition> {
        NonNull::from(&self.0.def.0)
    }

    /// Same as `LocalMemory::grow`, except with `&self`.
    pub fn grow(&self, delta_pages: u64) -> Result<Option<(usize, usize)>, Error> {
        let mut memory = self.0.memory.write();
        // Without a limiter being passed in this shouldn't have an await point,
        // so it should be safe to assert that it's ready.
        let result = vm::assert_ready(memory.grow(delta_pages, None))?;
        if let Some((_old_size_in_bytes, new_size_in_bytes)) = result {
            // Store the new size to the `VMMemoryDefinition` for JIT-generated
            // code (and runtime functions) to access. No other code can be
            // growing this memory due to the write lock, but code in other
            // threads could have access to this shared memory and we want them
            // to see the most consistent version of the `current_length`; a
            // weaker consistency is possible if we accept them seeing an older,
            // smaller memory size (assumption: memory only grows) but presently
            // we are aiming for accuracy.
            //
            // Note that it could be possible to access a memory address that is
            // now-valid due to changes to the page flags in `grow` above but
            // beyond the `memory.size` that we are about to assign to. In these
            // and similar cases, discussion in the thread proposal concluded
            // that: "multiple accesses in one thread racing with another
            // thread's `memory.grow` that are in-bounds only after the grow
            // commits may independently succeed or trap" (see
            // https://github.com/WebAssembly/threads/issues/26#issuecomment-433930711).
            // In other words, some non-determinism is acceptable when using
            // `memory.size` on work being done by `memory.grow`.
            self.0
                .def
                .0
                .current_length
                .store(new_size_in_bytes, Ordering::SeqCst);
        }
        Ok(result)
    }

    /// Implementation of `memory.atomic.notify` for this shared memory.
    pub fn atomic_notify(&self, addr_index: u64, count: u32) -> Result<u32, Trap> {
        let ptr = validate_atomic_addr(&self.0.def.0, addr_index, 4, 4)?;
        log::trace!("memory.atomic.notify(addr={addr_index:#x}, count={count})");
        #[cfg(feature = "std")]
        {
            let ptr = unsafe { &*ptr };
            Ok(self.0.spot.notify(ptr, count))
        }
        #[cfg(not(feature = "std"))]
        {
            let _ = ptr;
            Ok(0)
        }
    }

    /// Implementation of `memory.atomic.wait32` for this shared memory.
    pub fn atomic_wait32(
        &self,
        addr_index: u64,
        expected: u32,
        timeout: Option<Duration>,
    ) -> Result<WaitResult, Trap> {
        let addr = validate_atomic_addr(&self.0.def.0, addr_index, 4, 4)?;
        log::trace!(
            "memory.atomic.wait32(addr={addr_index:#x}, expected={expected}, timeout={timeout:?})"
        );

        // SAFETY: `addr_index` was validated by `validate_atomic_addr` above.
        assert!(core::mem::size_of::<AtomicU32>() == 4);
        assert!(core::mem::align_of::<AtomicU32>() <= 4);
        let atomic = unsafe { AtomicU32::from_ptr(addr.cast()) };
        // Wasm linear memory is always little-endian, but `AtomicU32` uses the
        // host's native endianness.
        let expected = expected.to_le();

        #[cfg(feature = "std")]
        {
            // Note that `checked_add` is used such that when `timeout` is too large
            // it'll cause there to be no timeout at all if we can't represent the
            // deadline. That effectively maps to the requested timeout since if we
            // can't represent the deadline we'll be here awhile.
            let deadline = timeout.and_then(|d| Instant::now().checked_add(d));
            WAITER.with(|waiter| {
                let mut waiter = waiter.borrow_mut();
                Ok(self.0.spot.wait32(atomic, expected, deadline, &mut waiter))
            })
        }
        #[cfg(not(feature = "std"))]
        {
            Ok(nonblocking_wait(
                atomic.load(Ordering::SeqCst) == expected,
                timeout,
            ))
        }
    }

    /// Implementation of `memory.atomic.wait64` for this shared memory.
    pub fn atomic_wait64(
        &self,
        addr_index: u64,
        expected: u64,
        timeout: Option<Duration>,
    ) -> Result<WaitResult, Trap> {
        let addr = validate_atomic_addr(&self.0.def.0, addr_index, 8, 8)?;
        log::trace!(
            "memory.atomic.wait64(addr={addr_index:#x}, expected={expected}, timeout={timeout:?})"
        );

        #[cfg(target_has_atomic = "64")]
        {
            // SAFETY: `addr_index` was validated by `validate_atomic_addr` above.
            assert!(core::mem::size_of::<AtomicU64>() == 8);
            assert!(core::mem::align_of::<AtomicU64>() <= 8);
            let atomic = unsafe { AtomicU64::from_ptr(addr.cast()) };
            // Wasm linear memory is always little-endian, but `AtomicU64` uses the
            // host's native endianness.
            let expected = expected.to_le();

            #[cfg(feature = "std")]
            {
                // See `atomic_wait32` for why this is using `checked_add`.
                let deadline = timeout.and_then(|d| Instant::now().checked_add(d));
                WAITER.with(|waiter| {
                    let mut waiter = waiter.borrow_mut();
                    Ok(self.0.spot.wait64(atomic, expected, deadline, &mut waiter))
                })
            }
            #[cfg(not(feature = "std"))]
            {
                Ok(nonblocking_wait(
                    atomic.load(Ordering::SeqCst) == expected,
                    timeout,
                ))
            }
        }
        #[cfg(not(target_has_atomic = "64"))]
        {
            let _ = (addr, expected, timeout);
            unreachable!("`SharedMemory` cannot be constructed without 64-bit atomics")
        }
    }

    pub(crate) fn byte_size(&self) -> usize {
        self.0.memory.read().byte_size()
    }

    pub(crate) fn needs_init(&self) -> bool {
        self.0.memory.read().needs_init()
    }

    pub(crate) fn wasm_accessible(&self) -> Range<usize> {
        self.0.memory.read().wasm_accessible()
    }
}

#[cfg(not(feature = "std"))]
fn nonblocking_wait(matches: bool, timeout: Option<Duration>) -> WaitResult {
    if !matches {
        WaitResult::Mismatch
    } else if timeout == Some(Duration::ZERO) {
        WaitResult::TimedOut
    } else {
        panic!("blocking `memory.atomic.wait` requires the `std` feature")
    }
}

#[cfg(feature = "std")]
thread_local! {
    /// Structure used in conjunction with `ParkingSpot` to block the current
    /// thread if necessary. Note that this is lazily initialized.
    static WAITER: RefCell<Waiter> = const { RefCell::new(Waiter::new()) };
}

/// Shared memory needs some representation of a `VMMemoryDefinition` for
/// JIT-generated code to access. This structure owns the base pointer and
/// length to the actual memory and we share this definition across threads by:
/// - never changing the base pointer; according to the specification, shared
///   memory must be created with a known maximum size so it can be allocated
///   once and never moved
/// - carefully changing the length, using atomic accesses in both the runtime
///   and JIT-generated code.
struct LongTermVMMemoryDefinition(VMMemoryDefinition);
unsafe impl Send for LongTermVMMemoryDefinition {}
unsafe impl Sync for LongTermVMMemoryDefinition {}
