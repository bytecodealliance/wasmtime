#[cfg(not(target_os = "windows"))]
mod not_for_windows {
    use wasmtime::*;
    use wasmtime_environ::{WASM_MAX_PAGES, WASM_PAGE_SIZE};

    use libc::MAP_FAILED;
    use libc::{mmap, mprotect, munmap};
    use libc::{sysconf, _SC_PAGESIZE};
    use libc::{MAP_ANON, MAP_PRIVATE, PROT_NONE, PROT_READ, PROT_WRITE};

    use std::io::Error;
    use std::ptr::null_mut;
    use std::sync::{Arc, Mutex};

    struct CustomMemory {
        mem: usize,
        size: usize,
        guard_size: usize,
        used_wasm_pages: u32,
        glob_page_counter: Arc<Mutex<u64>>,
    }

    impl CustomMemory {
        unsafe fn new(
            num_wasm_pages: u32,
            max_wasm_pages: u32,
            glob_counter: Arc<Mutex<u64>>,
        ) -> Self {
            let page_size = sysconf(_SC_PAGESIZE) as usize;
            let guard_size = page_size;
            let size = max_wasm_pages as usize * WASM_PAGE_SIZE as usize + guard_size;
            let used_size = num_wasm_pages as usize * WASM_PAGE_SIZE as usize;
            assert_eq!(size % page_size, 0); // we rely on WASM_PAGE_SIZE being multiple of host page size

            let mem = mmap(null_mut(), size, PROT_NONE, MAP_PRIVATE | MAP_ANON, -1, 0);
            assert_ne!(mem, MAP_FAILED, "mmap failed: {}", Error::last_os_error());

            let r = mprotect(mem, used_size, PROT_READ | PROT_WRITE);
            assert_eq!(r, 0, "mprotect failed: {}", Error::last_os_error());
            *glob_counter.lock().unwrap() += num_wasm_pages as u64;

            Self {
                mem: mem as usize,
                size,
                guard_size,
                used_wasm_pages: num_wasm_pages,
                glob_page_counter: glob_counter,
            }
        }
    }

    impl Drop for CustomMemory {
        fn drop(&mut self) {
            let n = self.used_wasm_pages as u64;
            *self.glob_page_counter.lock().unwrap() -= n;
            let r = unsafe { munmap(self.mem as *mut _, self.size) };
            assert_eq!(r, 0, "munmap failed: {}", Error::last_os_error());
        }
    }

    unsafe impl LinearMemory for CustomMemory {
        fn size(&self) -> u32 {
            self.used_wasm_pages
        }

        fn maximum(&self) -> Option<u32> {
            Some((self.size as u32 - self.guard_size as u32) / WASM_PAGE_SIZE)
        }

        fn grow(&mut self, delta: u32) -> Option<u32> {
            let delta_size = (delta as usize).checked_mul(WASM_PAGE_SIZE as usize)?;

            let prev_pages = self.used_wasm_pages;
            let prev_size = (prev_pages as usize).checked_mul(WASM_PAGE_SIZE as usize)?;

            let new_pages = prev_pages.checked_add(delta)?;

            if new_pages > self.maximum().unwrap() {
                return None;
            }
            unsafe {
                let start = (self.mem as *mut u8).add(prev_size) as _;
                let r = mprotect(start, delta_size, PROT_READ | PROT_WRITE);
                assert_eq!(r, 0, "mprotect failed: {}", Error::last_os_error());
            }

            *self.glob_page_counter.lock().unwrap() += delta as u64;
            self.used_wasm_pages = new_pages;
            Some(prev_pages)
        }

        fn as_ptr(&self) -> *mut u8 {
            self.mem as *mut u8
        }
    }

    struct CustomMemoryCreator {
        pub num_created_memories: Mutex<usize>,
        pub num_total_pages: Arc<Mutex<u64>>,
    }

    impl CustomMemoryCreator {
        pub fn new() -> Self {
            Self {
                num_created_memories: Mutex::new(0),
                num_total_pages: Arc::new(Mutex::new(0)),
            }
        }
    }

    unsafe impl MemoryCreator for CustomMemoryCreator {
        fn new_memory(
            &self,
            ty: MemoryType,
            reserved_size: Option<u64>,
            guard_size: u64,
        ) -> Result<Box<dyn LinearMemory>, String> {
            assert_eq!(guard_size, 0);
            assert!(reserved_size.is_none());
            let max = ty.limits().max().unwrap_or(WASM_MAX_PAGES);
            unsafe {
                let mem = Box::new(CustomMemory::new(
                    ty.limits().min(),
                    max,
                    self.num_total_pages.clone(),
                ));
                *self.num_created_memories.lock().unwrap() += 1;
                Ok(mem)
            }
        }
    }

    fn config() -> (Store<()>, Arc<CustomMemoryCreator>) {
        let mem_creator = Arc::new(CustomMemoryCreator::new());
        let mut config = Config::new();
        config
            .with_host_memory(mem_creator.clone())
            .static_memory_maximum_size(0)
            .dynamic_memory_guard_size(0);
        (Store::new(&Engine::new(&config).unwrap(), ()), mem_creator)
    }

    #[test]
    fn host_memory() -> anyhow::Result<()> {
        let (mut store, mem_creator) = config();
        let module = Module::new(
            store.engine(),
            r#"
            (module
                (memory (export "memory") 1)
            )
        "#,
        )?;
        Instance::new(&mut store, &module, &[])?;

        assert_eq!(*mem_creator.num_created_memories.lock().unwrap(), 1);

        Ok(())
    }

    #[test]
    fn host_memory_grow() -> anyhow::Result<()> {
        let (mut store, mem_creator) = config();
        let module = Module::new(
            store.engine(),
            r#"
            (module
                (func $f (drop (memory.grow (i32.const 1))))
                (memory (export "memory") 1 2)
                (start $f)
            )
        "#,
        )?;

        Instance::new(&mut store, &module, &[])?;
        let instance2 = Instance::new(&mut store, &module, &[])?;

        assert_eq!(*mem_creator.num_created_memories.lock().unwrap(), 2);

        assert_eq!(
            instance2
                .get_memory(&mut store, "memory")
                .unwrap()
                .size(&store),
            2
        );

        // we take the lock outside the assert, so it won't get poisoned on assert failure
        let tot_pages = *mem_creator.num_total_pages.lock().unwrap();
        assert_eq!(tot_pages, 4);

        drop(store);
        let tot_pages = *mem_creator.num_total_pages.lock().unwrap();
        assert_eq!(tot_pages, 0);

        Ok(())
    }
}
