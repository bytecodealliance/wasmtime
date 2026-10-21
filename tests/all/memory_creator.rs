//! Tests of custom linear memories provided with `Config::with_host_memory`.
// None of this works on miri since we use actual system mmap utilities.
#![cfg(not(miri))]

use std::ptr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use wasmtime::*;

const WASM_PAGE: usize = 0x10000;
const GUARD_SIZE: usize = WASM_PAGE;
const MAP_CAPACITY: usize = 10 << 20; // 10MiB as max cap for mmap.

/// What `CustomMemory` does when asked to initialize itself from a CoW memory
/// image.
#[derive(Clone, Copy, PartialEq)]
#[cfg_attr(
    not(target_os = "linux"),
    allow(dead_code, reason = "CoW images are only built on Linux")
)]
enum ImageMode {
    /// Map the image with `MemoryImage::map_at`, return true.
    Map,
    /// Do not map, return false, let Wasmtime copy the data.
    ReturnFalse,
    /// Return true *without* initializing the memory.
    /// This is for tests to check that wasmtime does not manually copy anything.
    Lie,
    /// Fail initialization.
    Fail,
}

/// Observability of calls for tests.
#[derive(Default)]
struct Counters {
    num_memories_created: AtomicUsize,
    num_bytes_accessible: AtomicUsize,
    grow_to_calls: AtomicUsize,
    init_from_image_calls: AtomicUsize,
    images_mapped: AtomicUsize,
}

struct SendSyncPtr(*mut u8);
unsafe impl Send for SendSyncPtr {}
unsafe impl Sync for SendSyncPtr {}

/// A linear memory backed by an mmap.
/// `size` bytes are accessible, followed by inaccessible space up to `capacity` and a
/// guard region of `GUARD_SIZE` bytes.
struct CustomMemory {
    base: SendSyncPtr,
    capacity: usize,
    size: usize,
    /// Whether every `grow_to` moves the memory to a new reservation.
    /// When false, growing beyond capacity fails (static memory).
    move_on_grow: bool,
    image_mode: ImageMode,
    counters: Arc<Counters>,
}

impl CustomMemory {
    fn new(
        minimum: usize,
        capacity: usize,
        move_on_grow: bool,
        image_mode: ImageMode,
        counters: Arc<Counters>,
    ) -> Self {
        let capacity = capacity.max(minimum);
        // SAFETY: the reservation is new and `minimum` bytes fit in it.
        let base = unsafe {
            let base = sys::reserve(capacity + GUARD_SIZE);
            sys::make_available(base, minimum);
            base
        };
        counters.num_bytes_accessible.fetch_add(minimum, SeqCst);
        Self {
            base: SendSyncPtr(base),
            capacity,
            size: minimum,
            move_on_grow,
            image_mode,
            counters,
        }
    }

    fn base(&self) -> *mut u8 {
        self.base.0
    }
}

impl Drop for CustomMemory {
    fn drop(&mut self) {
        self.counters
            .num_bytes_accessible
            .fetch_sub(self.size, SeqCst);
        // SAFETY: this memory owns its reservation, which nothing uses anymore.
        unsafe { sys::release(self.base(), self.capacity + GUARD_SIZE) };
    }
}

// SAFETY: the first `size` bytes at `base` are committed, readable, writable
// and zero-initialized, and the rest of the reservation up to `capacity` plus
// the guard region is inaccessible. Reservations are aligned to at least the
// host page size. Without `move_on_grow` the memory never moves and refuses to
// grow beyond its capacity. With `move_on_grow` every growth moves it to a new
// reservation, which is only valid for growth beyond the capacity; growing
// within the capacity then deliberately violates the base pointer stability
// requirement, which one test uses to check that Wasmtime detects it.
unsafe impl LinearMemory for CustomMemory {
    fn byte_size(&self) -> usize {
        self.size
    }

    fn byte_capacity(&self) -> usize {
        self.capacity
    }

    fn grow_to(&mut self, new_size: usize) -> Result<()> {
        self.counters.grow_to_calls.fetch_add(1, SeqCst);
        if self.move_on_grow {
            let capacity = new_size.max(self.capacity);
            // SAFETY: the new reservation is fresh, and the old one is owned
            // by this memory and released after copying its contents.
            unsafe {
                let base = sys::reserve(capacity + GUARD_SIZE);
                sys::make_available(base, new_size);
                ptr::copy_nonoverlapping(self.base(), base, self.size);
                sys::release(self.base(), self.capacity + GUARD_SIZE);
                self.base = SendSyncPtr(base);
            }
            self.capacity = capacity;
        } else {
            if new_size > self.capacity {
                bail!("custom memory cannot grow beyond its capacity");
            }
            // SAFETY: the new bytes lie within this memory's reservation. The
            // size is a multiple of 64K, so the start is page aligned.
            unsafe { sys::make_available(self.base().add(self.size), new_size - self.size) };
        }
        self.counters
            .num_bytes_accessible
            .fetch_add(new_size - self.size, SeqCst);
        self.size = new_size;
        Ok(())
    }

    fn as_ptr(&self) -> *mut u8 {
        self.base()
    }

    fn init_from_image(&mut self, image: &MemoryImage) -> Result<bool> {
        self.counters.init_from_image_calls.fetch_add(1, SeqCst);
        match self.image_mode {
            ImageMode::Map => {
                assert!(image.linear_memory_offset() + image.len() <= self.size);
                // SAFETY: this memory owns its whole reservation and nothing
                // has accessed it yet.
                unsafe { image.map_at(self.base())? };
                self.counters.images_mapped.fetch_add(1, SeqCst);
                Ok(true)
            }
            ImageMode::ReturnFalse => Ok(false),
            ImageMode::Lie => Ok(true),
            ImageMode::Fail => bail!("custom memory refused its image"),
        }
    }
}

struct CustomMemoryCreator {
    image_mode: ImageMode,
    capacity_of_each: usize,
    move_on_grow: bool,
    counters: Arc<Counters>,
}

impl CustomMemoryCreator {
    fn new(image_mode: ImageMode) -> Self {
        Self {
            image_mode,
            capacity_of_each: MAP_CAPACITY,
            move_on_grow: false,
            counters: Arc::default(),
        }
    }

    fn with_capacity(mut self, capacity: usize) -> Self {
        self.capacity_of_each = capacity;
        self
    }

    fn moving(mut self) -> Self {
        self.move_on_grow = true;
        self
    }
}

unsafe impl MemoryCreator for CustomMemoryCreator {
    fn new_memory(
        &self,
        ty: MemoryType,
        minimum: usize,
        _maximum: Option<usize>,
        reserved_size: Option<usize>,
        guard_size: usize,
    ) -> Result<Box<dyn LinearMemory>, String> {
        assert_eq!(guard_size, 0);
        assert_eq!(reserved_size, Some(0));
        assert!(!ty.is_64());
        let capacity = self.capacity_of_each;
        self.counters.num_memories_created.fetch_add(1, SeqCst);
        Ok(Box::new(CustomMemory::new(
            minimum,
            capacity,
            self.move_on_grow,
            self.image_mode,
            self.counters.clone(),
        )))
    }
}

/// Returns a store whose memories are created by `creator`, and the creator's
/// counters. `configure` can further adjust the engine's configuration.
fn store(
    creator: CustomMemoryCreator,
    configure: impl FnOnce(&mut Config),
) -> (Store<()>, Arc<Counters>) {
    let counters = creator.counters.clone();
    let mut config = Config::new();
    config
        .with_host_memory(Arc::new(creator))
        .memory_reservation(0)
        .memory_guard_size(0);
    configure(&mut config);
    (Store::new(&Engine::new(&config).unwrap(), ()), counters)
}

/// A module whose memory is initialized by a data segment.
const DATA_MODULE: &str = r#"
    (module
        (memory (export "memory") 1 4)
        (data (i32.const 0) "hello")
        (func (export "load") (param i32) (result i32)
            (i32.load8_u (local.get 0)))
        (func (export "store") (param i32 i32)
            (i32.store8 (local.get 0) (local.get 1)))
        (func (export "grow") (param i32) (result i32)
            (memory.grow (local.get 0)))
    )
"#;

/// This function tests that:
/// - "hello" is accessible in the Wasm memory where it's expected to (in static data);
/// - this stays true after growing the memory by a page.
fn check_data_module(store: &mut Store<()>, instance: Instance) -> Result<()> {
    let load = instance.get_typed_func::<i32, i32>(&mut *store, "load")?;
    let grow = instance.get_typed_func::<i32, i32>(&mut *store, "grow")?;
    for (i, byte) in b"hello\0".iter().enumerate() {
        assert_eq!(load.call(&mut *store, i as i32)?, i32::from(*byte));
    }
    assert_eq!(grow.call(&mut *store, 1)?, 1);
    for (i, byte) in b"hello\0".iter().enumerate() {
        assert_eq!(load.call(&mut *store, i as i32)?, i32::from(*byte));
    }
    assert_eq!(load.call(&mut *store, WASM_PAGE as i32)?, 0);
    Ok(())
}

// CoW of in-memory data requires memfd, which is linux-only.
#[cfg(target_os = "linux")]
mod linux_only {
    use super::*;
    // Regression test for [#10740](https://github.com/bytecodealliance/wasmtime/issues/10740):
    // a custom memory that does not support CoW forces a copy from Wasmtime.
    #[test]
    #[cfg(target_os = "linux")]
    fn host_memory_with_cow_image_when_impl_refuses_mapping() -> Result<()> {
        let (mut store, counters) =
            store(CustomMemoryCreator::new(ImageMode::ReturnFalse), |cfg| {
                cfg.memory_init_cow(true);
            });
        let module = Module::new(store.engine(), DATA_MODULE)?;
        let instance = Instance::new(&mut store, &module, &[])?;
        check_data_module(&mut store, instance)?;
        assert_eq!(counters.num_memories_created.load(SeqCst), 1);
        assert_eq!(counters.init_from_image_calls.load(SeqCst), 1);
        assert_eq!(counters.images_mapped.load(SeqCst), 0);
        Ok(())
    }

    // When `init_from_image` returns `true`, Wasmtime trusts it and doesn't copy
    // the module's data segments into the memory.
    // This is relying on our CustomMemory breaking the contract (ImageMode::Lie) and not actually performing the mapping.
    #[test]
    #[cfg(target_os = "linux")]
    fn host_memory_init_from_image_true_skips_data_segments() -> Result<()> {
        let (mut store, counters) = store(CustomMemoryCreator::new(ImageMode::Lie), |cfg| {
            cfg.memory_init_cow(true);
        });
        let module = Module::new(store.engine(), DATA_MODULE)?;
        let instance = Instance::new(&mut store, &module, &[])?;
        let load = instance.get_typed_func::<i32, i32>(&mut store, "load")?;
        // The memory claimed to hold the image but left itself zeroed.
        assert_eq!(load.call(&mut store, 0)?, 0); // Would be 'h' if data was copied.
        assert_eq!(counters.init_from_image_calls.load(SeqCst), 1);
        Ok(())
    }

    // This checks the CoW semantics: writing to one instantiated memory does not leak into memories
    // later instantiated from the same image.
    #[test]
    #[cfg(target_os = "linux")]
    fn host_memory_with_cow_image_when_impl_maps() -> Result<()> {
        let (mut store, counters) = store(CustomMemoryCreator::new(ImageMode::Map), |cfg| {
            cfg.memory_init_cow(true);
        });
        let module = Module::new(store.engine(), DATA_MODULE)?;

        let first_instance = Instance::new(&mut store, &module, &[])?;
        check_data_module(&mut store, first_instance)?;

        let first_store = first_instance.get_typed_func::<(i32, i32), ()>(&mut store, "store")?;
        first_store.call(&mut store, (0, i32::from(b'x')))?;

        let second_instance = Instance::new(&mut store, &module, &[])?;

        let first_load = first_instance.get_typed_func::<i32, i32>(&mut store, "load")?;
        assert_eq!(first_load.call(&mut store, 0)?, i32::from(b'x'));
        let second_load = second_instance.get_typed_func::<i32, i32>(&mut store, "load")?;
        assert_eq!(second_load.call(&mut store, 0)?, i32::from(b'h'));

        assert_eq!(counters.num_memories_created.load(SeqCst), 2);
        assert_eq!(counters.init_from_image_calls.load(SeqCst), 2);
        assert_eq!(counters.images_mapped.load(SeqCst), 2);
        Ok(())
    }

    // Check that failures from implementations are propagated as-is (part of the public contract).
    #[test]
    #[cfg(target_os = "linux")]
    fn host_memory_init_from_image_error_fails_instantiation() -> Result<()> {
        let (mut store, counters) = store(CustomMemoryCreator::new(ImageMode::Fail), |cfg| {
            cfg.memory_init_cow(true);
        });
        let module = Module::new(store.engine(), DATA_MODULE)?;
        let err = Instance::new(&mut store, &module, &[]).unwrap_err();
        assert_eq!(counters.init_from_image_calls.load(SeqCst), 1);
        assert!(
            format!("{err:?}").contains("custom memory refused its image"),
            "unexpected error: {err:?}"
        );
        Ok(())
    }
}

// With CoW disabled `init_from_image` should not be called,
// and the data segments should be copied into the custom memory.
#[test]
fn host_memory_without_cow_does_not_call_init_from_image() -> Result<()> {
    let (mut store, counters) = store(CustomMemoryCreator::new(ImageMode::Fail), |cfg| {
        cfg.memory_init_cow(false);
    });
    let module = Module::new(store.engine(), DATA_MODULE)?;
    let instance = Instance::new(&mut store, &module, &[])?;
    check_data_module(&mut store, instance)?;
    assert_eq!(counters.init_from_image_calls.load(SeqCst), 0);
    Ok(())
}

// Regression test: memories created by the host with `Memory::new` used to
// ignore the configured `MemoryCreator`.
#[test]
fn host_created_memory_uses_creator() -> Result<()> {
    let (mut store, counters) = store(CustomMemoryCreator::new(ImageMode::Fail), |_| {});

    let memory = Memory::new(&mut store, MemoryType::new(1, Some(3))?)?;
    assert_eq!(counters.num_memories_created.load(SeqCst), 1);
    assert_eq!(counters.num_bytes_accessible.load(SeqCst), WASM_PAGE);

    // Growing from the host goes through `CustomMemory::grow_to`...
    memory.grow(&mut store, 1)?;
    assert_eq!(counters.num_bytes_accessible.load(SeqCst), 2 * WASM_PAGE);

    // ... and so does growing from Wasm.
    memory.data_mut(&mut store)[WASM_PAGE] = 42;
    let module = Module::new(
        store.engine(),
        r#"
        (module
            (memory (import "" "memory") 1)
            (func (export "grow") (result i32) (memory.grow (i32.const 1)))
            (func (export "load") (param i32) (result i32)
                (i32.load8_u (local.get 0)))
        )
    "#,
    )?;
    let instance = Instance::new(&mut store, &module, &[memory.into()])?;
    let load = instance.get_typed_func::<i32, i32>(&mut store, "load")?;
    let grow = instance.get_typed_func::<(), i32>(&mut store, "grow")?;

    assert_eq!(load.call(&mut store, WASM_PAGE as i32)?, 42);
    assert_eq!(grow.call(&mut store, ())?, 2);
    assert_eq!(counters.num_bytes_accessible.load(SeqCst), 3 * WASM_PAGE);

    // The importing instance didn't create a memory of its own.
    assert_eq!(counters.num_memories_created.load(SeqCst), 1);

    drop(store);
    assert_eq!(counters.num_bytes_accessible.load(SeqCst), 0);

    Ok(())
}

// A memory that moves when growing beyond its capacity keeps its contents,
// and wasm code uses the new location afterwards.
#[test]
fn growth_that_moves_memory() -> Result<()> {
    let creator = CustomMemoryCreator::new(ImageMode::ReturnFalse)
        .with_capacity(WASM_PAGE)
        .moving();
    let (mut store, counters) = store(creator, |_| {});
    let module = Module::new(store.engine(), DATA_MODULE)?;
    let instance = Instance::new(&mut store, &module, &[])?;
    let memory = instance.get_memory(&mut store, "memory").unwrap();
    let load = instance.get_typed_func::<i32, i32>(&mut store, "load")?;
    let store_byte = instance.get_typed_func::<(i32, i32), ()>(&mut store, "store")?;
    let grow = instance.get_typed_func::<i32, i32>(&mut store, "grow")?;

    store_byte.call(&mut store, (7, 42))?;
    let base_before = memory.data_ptr(&store);

    assert_eq!(grow.call(&mut store, 1)?, 1);
    assert_eq!(counters.grow_to_calls.load(SeqCst), 1);
    // Sanity check: memory moved somewhere else.
    assert_ne!(memory.data_ptr(&store), base_before);
    assert_eq!(memory.data_size(&store), 2 * WASM_PAGE);

    assert_eq!(load.call(&mut store, 7)?, 42);
    store_byte.call(&mut store, (WASM_PAGE as i32 + 7, 43))?;
    assert_eq!(load.call(&mut store, WASM_PAGE as i32 + 7)?, 43);
    assert_eq!(memory.data(&store)[WASM_PAGE + 7], 43);
    Ok(())
}

// With `Config::memory_may_move(false)` a memory still grows within its
// capacity, but growth beyond it is refused without asking the memory.
#[test]
fn growth_limited_to_capacity_when_memory_may_not_move() -> Result<()> {
    let creator = CustomMemoryCreator::new(ImageMode::ReturnFalse).with_capacity(2 * WASM_PAGE);
    let (mut store, counters) = store(creator, |config| {
        config.memory_may_move(false);
    });
    let module = Module::new(store.engine(), DATA_MODULE)?;
    let instance = Instance::new(&mut store, &module, &[])?;
    let memory = instance.get_memory(&mut store, "memory").unwrap();
    let grow = instance.get_typed_func::<i32, i32>(&mut store, "grow")?;
    let base = memory.data_ptr(&store);
    assert_eq!(memory.data_size(&store), WASM_PAGE);

    assert_eq!(grow.call(&mut store, 1)?, 1);
    assert_eq!(counters.grow_to_calls.load(SeqCst), 1);
    assert_eq!(memory.data_size(&store), 2 * WASM_PAGE);
    assert_eq!(memory.data_ptr(&store), base);

    assert_eq!(grow.call(&mut store, 1)?, -1);
    assert_eq!(counters.grow_to_calls.load(SeqCst), 1);
    assert_eq!(memory.data_size(&store), 2 * WASM_PAGE);
    assert_eq!(memory.data_ptr(&store), base);
    Ok(())
}

// A memory that moves while growing within its capacity violates the `LinearMemory` contract.
// Wasmtime does not guarantee that this is detected, but it is an easy check that prevents
// potentially catastrophic errors, so we make sure it leads to a panic.
#[test]
#[should_panic(expected = "linear memory moved on grow when it must not have")]
fn growth_within_capacity_that_moves_memory_panics() {
    let creator = CustomMemoryCreator::new(ImageMode::ReturnFalse)
        .with_capacity(2 * WASM_PAGE)
        .moving();
    let (mut store, _counters) = store(creator, |_| {});
    let memory = Memory::new(&mut store, MemoryType::new(1, None).unwrap()).unwrap();
    let _ = memory.grow(&mut store, 1);
}

// Mmap providers for unix and windows.
mod sys {
    #[cfg(unix)]
    pub use unix::{make_available, release, reserve};
    #[cfg(windows)]
    pub use windows::{make_available, release, reserve};

    #[cfg(unix)]
    mod unix {
        use rustix::mm::{MapFlags, MprotectFlags, ProtFlags, mmap_anonymous, mprotect, munmap};

        pub unsafe fn reserve(size: usize) -> *mut u8 {
            let ptr = unsafe {
                mmap_anonymous(
                    std::ptr::null_mut(),
                    size,
                    ProtFlags::empty(),
                    MapFlags::PRIVATE,
                )
            };
            ptr.expect("mmap failed").cast()
        }

        pub unsafe fn make_available(ptr: *mut u8, len: usize) {
            unsafe { mprotect(ptr.cast(), len, MprotectFlags::READ | MprotectFlags::WRITE) }
                .expect("mprotect failed");
        }

        pub unsafe fn release(ptr: *mut u8, size: usize) {
            unsafe { munmap(ptr.cast(), size) }.expect("munmap failed");
        }
    }

    #[cfg(windows)]
    mod windows {
        use windows_sys::Win32::System::Memory::{
            MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_NOACCESS, PAGE_READWRITE, VirtualAlloc,
            VirtualFree,
        };

        pub unsafe fn reserve(size: usize) -> *mut u8 {
            let ptr = unsafe { VirtualAlloc(std::ptr::null(), size, MEM_RESERVE, PAGE_NOACCESS) };
            assert!(!ptr.is_null(), "VirtualAlloc(MEM_RESERVE) failed");
            ptr.cast()
        }

        pub unsafe fn make_available(ptr: *mut u8, len: usize) {
            if len == 0 {
                return;
            }
            let ret = unsafe { VirtualAlloc(ptr.cast(), len, MEM_COMMIT, PAGE_READWRITE) };
            assert!(!ret.is_null(), "VirtualAlloc(MEM_COMMIT) failed");
        }

        pub unsafe fn release(ptr: *mut u8, _size: usize) {
            let ret = unsafe { VirtualFree(ptr.cast(), 0, MEM_RELEASE) };
            assert_ne!(ret, 0, "VirtualFree failed");
        }
    }
}
