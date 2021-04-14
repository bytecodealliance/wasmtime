//! This module implements user space page fault handling with the `userfaultfd` ("uffd") system call on Linux.
//!
//! Handling page faults for memory accesses in regions relating to WebAssembly instances
//! enables the runtime to protect guard pages in user space rather than kernel space (i.e. without `mprotect`).
//!
//! Additionally, linear memories can be lazy-initialized upon first access.
//!
//! Handling faults in user space is slower than handling faults in the kernel. However,
//! in use cases where there is a high number of concurrently executing instances, handling the faults
//! in user space requires rarely changing memory protection levels.  This can improve concurrency
//! by not taking kernel memory manager locks and may decrease TLB shootdowns as fewer page table entries need
//! to continually change.
//!
//! Here's how the `uffd` feature works:
//!
//! 1. A user fault file descriptor is created to monitor specific areas of the address space.
//! 2. A thread is spawned to continually read events from the user fault file descriptor.
//! 3. When a page fault event is received, the handler thread calculates where the fault occurred:
//!    a) If the fault occurs on a linear memory page, it is handled by either copying the page from
//!       initialization data or zeroing it.
//!    b) If the fault occurs on a guard page, the protection level of the guard page is changed to
//!       force the kernel to signal SIGBUS on the next retry. The faulting page is recorded so the
//!       protection level can be reset in the future.
//! 4. Faults to address space relating to an instance may occur from both Wasmtime (e.g. instance
//!    initialization) or from WebAssembly code (e.g. reading from or writing to linear memory),
//!    therefore the user fault handling must do as little work as possible to handle the fault.
//! 5. When the pooling allocator is dropped, it will drop the memory mappings relating to the pool; this
//!    generates unmap events for the fault handling thread, which responds by decrementing the mapping
//!    count. When the count reaches zero, the user fault handling thread will gracefully terminate.
//!
//! This feature requires a Linux kernel 4.11 or newer to use.

use super::{InstancePool, MemoryPool};
use crate::instance::Instance;
use anyhow::{bail, Context, Result};
use std::thread;
use userfaultfd::{Event, FeatureFlags, IoctlFlags, Uffd, UffdBuilder};
use wasmtime_environ::{entity::EntityRef, wasm::DefinedMemoryIndex, MemoryInitialization};

const WASM_PAGE_SIZE: usize = wasmtime_environ::WASM_PAGE_SIZE as usize;

fn decommit(addr: *mut u8, len: usize) -> Result<()> {
    if len == 0 {
        return Ok(());
    }

    unsafe {
        // On Linux, this tells the kernel to discard the backing of the pages in the range.
        // If the discarded pages are part of a uffd region, then the next access will fault
        // and the user fault handler will receive the event.
        // If the pages are not monitored by uffd, the kernel will zero the page on next access,
        // as if it were mmap'd for the first time.
        if libc::madvise(addr as _, len, libc::MADV_DONTNEED) != 0 {
            bail!(
                "madvise failed to decommit: {}",
                std::io::Error::last_os_error()
            );
        }
    }

    Ok(())
}

pub fn commit_memory_pages(_addr: *mut u8, _len: usize) -> Result<()> {
    // A no-op as memory pages remain READ|WRITE with uffd
    Ok(())
}

pub fn decommit_memory_pages(addr: *mut u8, len: usize) -> Result<()> {
    decommit(addr, len)
}

pub fn commit_table_pages(_addr: *mut u8, _len: usize) -> Result<()> {
    // A no-op as table pages remain READ|WRITE
    Ok(())
}

pub fn decommit_table_pages(addr: *mut u8, len: usize) -> Result<()> {
    decommit(addr, len)
}

#[cfg(feature = "async")]
pub fn commit_stack_pages(_addr: *mut u8, _len: usize) -> Result<()> {
    // A no-op as stack pages remain READ|WRITE
    Ok(())
}

#[cfg(feature = "async")]
pub fn decommit_stack_pages(addr: *mut u8, len: usize) -> Result<()> {
    decommit(addr, len)
}

/// This is used to initialize the memory pool when uffd is enabled.
///
/// Without uffd, all of the memory pool's pages are initially protected with `NONE` to treat the entire
/// range as guard pages. When an instance is created, the initial pages of the memory are
/// changed to `READ_WRITE`.
///
/// With uffd, however, the potentially accessible pages of the each linear memory are made `READ_WRITE` and
/// the page fault handler will detect an out of bounds access and treat the page, temporarily,
/// as a guard page.
pub(super) fn initialize_memory_pool(pool: &MemoryPool) -> Result<()> {
    if pool.memory_size == 0 || pool.max_wasm_pages == 0 {
        return Ok(());
    }

    for i in 0..pool.max_instances {
        for base in pool.get(i) {
            unsafe {
                region::protect(
                    base as _,
                    pool.max_wasm_pages as usize * WASM_PAGE_SIZE,
                    region::Protection::READ_WRITE,
                )
                .context("failed to initialize memory pool for uffd")?;
            }
        }
    }

    Ok(())
}

/// This is used to reset a linear memory's guard page back to read-write as the page might be accessible
/// again in the future depending on how the linear memory grows.
fn reset_guard_page(addr: *mut u8, len: usize) -> Result<()> {
    unsafe {
        region::protect(addr, len, region::Protection::READ_WRITE)
            .context("failed to reset guard page")
    }
}

/// Represents a location of a page fault within monitored regions of memory.
enum FaultLocation<'a> {
    /// The address location is in a WebAssembly linear memory page.
    /// The fault handler will copy the pages from initialization data if necessary.
    MemoryPage {
        /// The address of the page being accessed.
        page_addr: *mut u8,
        /// The length of the page being accessed.
        len: usize,
        /// The instance related to the memory page that was accessed.
        instance: &'a Instance,
        /// The index of the memory that was accessed.
        memory_index: DefinedMemoryIndex,
        /// The Wasm page index to initialize if the access was not a guard page.
        page_index: Option<usize>,
    },
}

/// Used to resolve fault addresses to a location.
///
/// This implementation relies heavily on how the linear memory pool organizes its memory.
///
/// `usize` is used here instead of pointers to keep this `Send` as it gets sent to the handler thread.
struct FaultLocator {
    instances_start: usize,
    instance_size: usize,
    max_instances: usize,
    memories_start: usize,
    memories_end: usize,
    memory_size: usize,
    max_memories: usize,
}

impl FaultLocator {
    fn new(instances: &InstancePool) -> Self {
        let instances_start = instances.mapping.as_ptr() as usize;
        let memories_start = instances.memories.mapping.as_ptr() as usize;
        let memories_end = memories_start + instances.memories.mapping.len();

        // Should always have instances
        debug_assert!(instances_start != 0);

        Self {
            instances_start,
            instance_size: instances.instance_size,
            max_instances: instances.max_instances,
            memories_start,
            memories_end,
            memory_size: instances.memories.memory_size,
            max_memories: instances.memories.max_memories,
        }
    }

    /// This is super-duper unsafe as it is used from the handler thread
    /// to access instance data without any locking primitives.
    ///
    /// It is assumed that the thread that owns the instance being accessed is
    /// currently suspended waiting on a fault to be handled.
    ///
    /// Of course a stray faulting memory access from a thread that does not own
    /// the instance might introduce a race, but this implementation considers
    /// such to be a serious soundness bug not originating in this code.
    ///
    /// If the assumption holds true, accessing the instance data from the handler thread
    /// should, in theory, be safe.
    unsafe fn get_instance(&self, index: usize) -> &Instance {
        debug_assert!(index < self.max_instances);
        &*((self.instances_start + (index * self.instance_size)) as *const Instance)
    }

    unsafe fn locate(&self, addr: usize) -> Option<FaultLocation> {
        // Check for a linear memory location
        if addr >= self.memories_start && addr < self.memories_end {
            let index = (addr - self.memories_start) / self.memory_size;
            let memory_index = DefinedMemoryIndex::new(index % self.max_memories);
            let memory_start = self.memories_start + (index * self.memory_size);
            let page_index = (addr - memory_start) / WASM_PAGE_SIZE;
            let instance = self.get_instance(index / self.max_memories);

            let init_page_index = instance.memories.get(memory_index).and_then(|m| {
                if page_index < m.size() as usize {
                    Some(page_index)
                } else {
                    None
                }
            });

            return Some(FaultLocation::MemoryPage {
                page_addr: (memory_start + page_index * WASM_PAGE_SIZE) as _,
                len: WASM_PAGE_SIZE,
                instance,
                memory_index,
                page_index: init_page_index,
            });
        }

        None
    }
}

/// This is called following a fault on a guard page.
///
/// Because the region being monitored is protected read-write, this needs to set the
/// protection level to `NONE` before waking the page.
///
/// This will cause the kernel to raise a SIGBUS when retrying the fault.
unsafe fn wake_guard_page_access(uffd: &Uffd, page_addr: *const u8, len: usize) -> Result<()> {
    // Set the page to NONE to induce a SIGBUS for the access on the next retry
    region::protect(page_addr, len, region::Protection::NONE)
        .context("failed to change guard page protection")?;

    uffd.wake(page_addr as _, len)
        .context("failed to wake guard page access")?;

    Ok(())
}

/// This is called to initialize a linear memory page (64 KiB).
///
/// If paged initialization is used for the module, then we can instruct the kernel to back the page with
/// what is already stored in the initialization data; if the page isn't in the initialization data,
/// it will be zeroed instead.
///
/// If paged initialization isn't being used, we zero the page. Initialization happens
/// at module instantiation in this case and the segment data will be then copied to the zeroed page.
unsafe fn initialize_wasm_page(
    uffd: &Uffd,
    instance: &Instance,
    page_addr: *const u8,
    memory_index: DefinedMemoryIndex,
    page_index: usize,
) -> Result<()> {
    // Check for paged initialization and copy the page if present in the initialization data
    if let MemoryInitialization::Paged { map, .. } = &instance.module.memory_initialization {
        let pages = &map[memory_index];

        if let Some(Some(data)) = pages.get(page_index) {
            debug_assert_eq!(data.len(), WASM_PAGE_SIZE);

            log::trace!(
                "copying linear memory page from {:p} to {:p}",
                data.as_ptr(),
                page_addr
            );

            uffd.copy(data.as_ptr() as _, page_addr as _, WASM_PAGE_SIZE, true)
                .context("failed to copy linear memory page")?;

            return Ok(());
        }
    }

    log::trace!("zeroing linear memory page at {:p}", page_addr);

    uffd.zeropage(page_addr as _, WASM_PAGE_SIZE, true)
        .context("failed to zero linear memory page")?;

    Ok(())
}

unsafe fn handle_page_fault(
    uffd: &Uffd,
    locator: &FaultLocator,
    addr: *mut std::ffi::c_void,
) -> Result<()> {
    match locator.locate(addr as usize) {
        Some(FaultLocation::MemoryPage {
            page_addr,
            len,
            instance,
            memory_index,
            page_index,
        }) => {
            log::trace!(
                "handling fault in linear memory at address {:p} on page {:p}",
                addr,
                page_addr
            );

            match page_index {
                Some(page_index) => {
                    initialize_wasm_page(&uffd, instance, page_addr, memory_index, page_index)?;
                }
                None => {
                    log::trace!("out of bounds memory access at {:p}", addr);

                    // Record the guard page fault so the page protection level can be reset later
                    instance.memories[memory_index].record_guard_page_fault(
                        page_addr,
                        len,
                        reset_guard_page,
                    );
                    wake_guard_page_access(&uffd, page_addr, len)?;
                }
            }
        }
        None => {
            bail!(
                "failed to locate fault address {:p} in registered memory regions",
                addr
            );
        }
    }

    Ok(())
}

fn fault_handler_thread(uffd: Uffd, locator: FaultLocator) -> Result<()> {
    loop {
        match uffd.read_event().expect("failed to read event") {
            Some(Event::Unmap { start, end }) => {
                log::trace!("memory region unmapped: {:p}-{:p}", start, end);

                let (start, end) = (start as usize, end as usize);

                if start == locator.memories_start && end == locator.memories_end {
                    break;
                } else {
                    panic!("unexpected memory region unmapped");
                }
            }
            Some(Event::Pagefault { addr, .. }) => unsafe {
                handle_page_fault(&uffd, &locator, addr as _)?
            },
            Some(_) => continue,
            None => bail!("no event was read from the user fault descriptor"),
        }
    }

    log::trace!("fault handler thread has successfully terminated");

    Ok(())
}

#[derive(Debug)]
pub struct PageFaultHandler {
    thread: Option<thread::JoinHandle<Result<()>>>,
}

impl PageFaultHandler {
    pub(super) fn new(instances: &InstancePool) -> Result<Self> {
        let uffd = UffdBuilder::new()
            .close_on_exec(true)
            .require_features(FeatureFlags::EVENT_UNMAP)
            .create()
            .context("failed to create user fault descriptor")?;

        // Register the linear memory pool with the userfault fd
        let start = instances.memories.mapping.as_ptr();
        let len = instances.memories.mapping.len();

        let thread = if !start.is_null() && len > 0 {
            let ioctls = uffd
                .register(start as _, len)
                .context("failed to register user fault range")?;

            if !ioctls.contains(IoctlFlags::WAKE | IoctlFlags::COPY | IoctlFlags::ZEROPAGE) {
                bail!(
                    "required user fault ioctls not supported by the kernel; found: {:?}",
                    ioctls,
                );
            }

            log::trace!(
                "user fault handling enabled on linear memory pool at {:p} with size {}",
                start,
                len
            );

            let locator = FaultLocator::new(&instances);

            Some(
                thread::Builder::new()
                    .name("page fault handler".into())
                    .spawn(move || fault_handler_thread(uffd, locator))
                    .context("failed to spawn page fault handler thread")?,
            )
        } else {
            log::trace!("user fault handling disabled as there is no linear memory pool");
            None
        };

        Ok(Self { thread })
    }
}

impl Drop for PageFaultHandler {
    fn drop(&mut self) {
        // The handler thread should terminate once all monitored regions of memory are unmapped.
        // The pooling instance allocator ensures that the regions are unmapped prior to dropping
        // the page fault handler.
        if let Some(thread) = self.thread.take() {
            thread
                .join()
                .expect("failed to join page fault handler thread")
                .expect("fault handler thread failed");
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        Imports, InstanceAllocationRequest, InstanceLimits, ModuleLimits,
        PoolingAllocationStrategy, VMSharedSignatureIndex,
    };
    use std::ptr;
    use std::sync::Arc;
    use wasmtime_environ::{entity::PrimaryMap, wasm::Memory, MemoryPlan, MemoryStyle, Module};

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_address_locator() {
        let module_limits = ModuleLimits {
            imported_functions: 0,
            imported_tables: 0,
            imported_memories: 0,
            imported_globals: 0,
            types: 0,
            functions: 0,
            tables: 0,
            memories: 2,
            globals: 0,
            table_elements: 0,
            memory_pages: 2,
        };
        let instance_limits = InstanceLimits {
            count: 3,
            memory_reservation_size: (WASM_PAGE_SIZE * 10) as u64,
        };

        let instances =
            InstancePool::new(&module_limits, &instance_limits).expect("should allocate");

        let locator = FaultLocator::new(&instances);

        assert_eq!(locator.instances_start, instances.mapping.as_ptr() as usize);
        assert_eq!(locator.instance_size, 4096);
        assert_eq!(locator.max_instances, 3);
        assert_eq!(
            locator.memories_start,
            instances.memories.mapping.as_ptr() as usize
        );
        assert_eq!(
            locator.memories_end,
            locator.memories_start + instances.memories.mapping.len()
        );
        assert_eq!(locator.memory_size, WASM_PAGE_SIZE * 10);
        assert_eq!(locator.max_memories, 2);

        unsafe {
            assert!(locator.locate(0).is_none());
            assert!(locator.locate(locator.memories_end).is_none());

            let mut module = Module::new();

            for _ in 0..module_limits.memories {
                module.memory_plans.push(MemoryPlan {
                    memory: Memory {
                        minimum: 2,
                        maximum: Some(2),
                        shared: false,
                    },
                    style: MemoryStyle::Static { bound: 1 },
                    offset_guard_size: 0,
                });
            }

            module_limits.validate(&module).expect("should validate");

            let mut handles = Vec::new();
            let module = Arc::new(module);
            let finished_functions = &PrimaryMap::new();

            // Allocate the maximum number of instances with the maximum number of memories
            for _ in 0..instances.max_instances {
                handles.push(
                    instances
                        .allocate(
                            PoolingAllocationStrategy::Random,
                            InstanceAllocationRequest {
                                module: module.clone(),
                                finished_functions,
                                imports: Imports {
                                    functions: &[],
                                    tables: &[],
                                    memories: &[],
                                    globals: &[],
                                },
                                shared_signatures: VMSharedSignatureIndex::default().into(),
                                host_state: Box::new(()),
                                interrupts: ptr::null(),
                                externref_activations_table: ptr::null_mut(),
                                stack_map_lookup: None,
                            },
                        )
                        .expect("instance should allocate"),
                );
            }

            // Validate memory locations
            for instance_index in 0..instances.max_instances {
                for memory_index in 0..instances.memories.max_memories {
                    let memory_start = locator.memories_start
                        + (instance_index * locator.memory_size * locator.max_memories)
                        + (memory_index * locator.memory_size);

                    // Test for access to first page
                    match locator.locate(memory_start + 10000) {
                        Some(FaultLocation::MemoryPage {
                            page_addr,
                            len,
                            instance: _,
                            memory_index: mem_index,
                            page_index,
                        }) => {
                            assert_eq!(page_addr, memory_start as _);
                            assert_eq!(len, WASM_PAGE_SIZE);
                            assert_eq!(mem_index, DefinedMemoryIndex::new(memory_index));
                            assert_eq!(page_index, Some(0));
                        }
                        _ => panic!("expected a memory page location"),
                    }

                    // Test for access to second page
                    match locator.locate(memory_start + 1024 + WASM_PAGE_SIZE) {
                        Some(FaultLocation::MemoryPage {
                            page_addr,
                            len,
                            instance: _,
                            memory_index: mem_index,
                            page_index,
                        }) => {
                            assert_eq!(page_addr, (memory_start + WASM_PAGE_SIZE) as _);
                            assert_eq!(len, WASM_PAGE_SIZE);
                            assert_eq!(mem_index, DefinedMemoryIndex::new(memory_index));
                            assert_eq!(page_index, Some(1));
                        }
                        _ => panic!("expected a memory page location"),
                    }

                    // Test for guard page
                    match locator.locate(memory_start + 10 + 9 * WASM_PAGE_SIZE) {
                        Some(FaultLocation::MemoryPage {
                            page_addr,
                            len,
                            instance: _,
                            memory_index: mem_index,
                            page_index,
                        }) => {
                            assert_eq!(page_addr, (memory_start + (9 * WASM_PAGE_SIZE)) as _);
                            assert_eq!(len, WASM_PAGE_SIZE);
                            assert_eq!(mem_index, DefinedMemoryIndex::new(memory_index));
                            assert_eq!(page_index, None);
                        }
                        _ => panic!("expected a memory page location"),
                    }
                }
            }

            for handle in handles.drain(..) {
                instances.deallocate(&handle);
            }
        }
    }
}
