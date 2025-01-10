#[cfg(all(not(target_os = "windows"), not(miri)))]
mod not_for_windows {
    use wasmtime::*;
    use wasmtime_environ::WASM32_MAX_SIZE;

    use rustix::mm::{MapFlags, MprotectFlags, ProtFlags, mmap_anonymous, mprotect, munmap};

    use std::ptr::null_mut;
    use std::sync::{Arc, Mutex};

    struct CustomMemory {
        mem: usize,
        size: usize,
        guard_size: usize,
        used_wasm_bytes: usize,
        glob_bytes_counter: Arc<Mutex<usize>>,
    }

    impl CustomMemory {
        unsafe fn new(minimum: usize, maximum: usize, glob_counter: Arc<Mutex<usize>>) -> Self {
            let page_size = rustix::param::page_size();
            let guard_size = page_size;
            let size = maximum + guard_size;
            // We rely on the Wasm page size being multiple of host page size.
            assert_eq!(size % page_size, 0);

            let mem = mmap_anonymous(null_mut(), size, ProtFlags::empty(), MapFlags::PRIVATE)
                .expect("mmap failed");

            // NOTE: mmap_anonymous returns zero initialized memory, which is relied upon by this
            // API.

            mprotect(mem, minimum, MprotectFlags::READ | MprotectFlags::WRITE)
                .expect("mprotect failed");
            *glob_counter.lock().unwrap() += minimum;

            Self {
                mem: mem as usize,
                size,
                guard_size,
                used_wasm_bytes: minimum,
                glob_bytes_counter: glob_counter,
            }
        }
    }

    impl Drop for CustomMemory {
        fn drop(&mut self) {
            *self.glob_bytes_counter.lock().unwrap() -= self.used_wasm_bytes;
            unsafe { munmap(self.mem as *mut _, self.size).expect("munmap failed") };
        }
    }

    unsafe impl LinearMemory for CustomMemory {
        fn byte_size(&self) -> usize {
            self.used_wasm_bytes
        }

        fn byte_capacity(&self) -> usize {
            self.size - self.guard_size
        }

        fn grow_to(&mut self, new_size: usize) -> wasmtime::Result<()> {
            println!("grow to {new_size:x}");
            let delta = new_size - self.used_wasm_bytes;
            unsafe {
                let start = (self.mem as *mut u8).add(self.used_wasm_bytes) as _;
                mprotect(start, delta, MprotectFlags::READ | MprotectFlags::WRITE)
                    .expect("mprotect failed");
            }

            *self.glob_bytes_counter.lock().unwrap() += delta;
            self.used_wasm_bytes = new_size;
            Ok(())
        }

        fn as_ptr(&self) -> *mut u8 {
            self.mem as *mut u8
        }
    }

    struct CustomMemoryCreator {
        pub num_created_memories: Mutex<usize>,
        pub num_total_bytes: Arc<Mutex<usize>>,
    }

    impl CustomMemoryCreator {
        pub fn new() -> Self {
            Self {
                num_created_memories: Mutex::new(0),
                num_total_bytes: Arc::new(Mutex::new(0)),
            }
        }
    }

    unsafe impl MemoryCreator for CustomMemoryCreator {
        fn new_memory(
            &self,
            ty: MemoryType,
            minimum: usize,
            maximum: Option<usize>,
            reserved_size: Option<usize>,
            guard_size: usize,
        ) -> Result<Box<dyn LinearMemory>, String> {
            assert_eq!(guard_size, 0);
            assert_eq!(reserved_size, Some(0));
            assert!(!ty.is_64());
            unsafe {
                let mem = Box::new(CustomMemory::new(
                    minimum,
                    maximum.unwrap_or(usize::try_from(WASM32_MAX_SIZE).unwrap()),
                    self.num_total_bytes.clone(),
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
            .memory_reservation(0)
            .memory_guard_size(0);
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
        let tot_pages = *mem_creator.num_total_bytes.lock().unwrap();
        assert_eq!(
            tot_pages,
            (4 * wasmtime_environ::Memory::DEFAULT_PAGE_SIZE) as usize
        );

        drop(store);
        let tot_pages = *mem_creator.num_total_bytes.lock().unwrap();
        assert_eq!(tot_pages, 0);

        Ok(())
    }
}
