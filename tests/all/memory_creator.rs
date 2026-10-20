#[cfg(all(not(target_os = "windows"), not(miri)))]
mod not_for_windows {
    use wasmtime::*;

    use rustix::mm::{MapFlags, MprotectFlags, ProtFlags, mmap_anonymous, mprotect, munmap};

    use std::ptr::null_mut;
    use std::sync::{Arc, Mutex};

    /// What `CustomMemory` does when asked to initialize itself from a CoW
    /// memory image.
    #[derive(Clone, Copy, PartialEq)]
    enum ImageMode {
        /// Do not map, return false, let Wasmtime copy the data.
        ReturnFalse,
        /// Map the image with `MemoryImage::map_at`, return true.
        Map,
        /// Fail initialization.
        Fail,
    }

    struct CustomMemory {
        mem: usize,
        size: usize,
        guard_size: usize,
        used_wasm_bytes: usize,
        glob_bytes_counter: Arc<Mutex<usize>>,
        image_mode: ImageMode,
        images_mapped: Arc<Mutex<usize>>,
    }

    impl CustomMemory {
        unsafe fn new(
            minimum: usize,
            maximum: usize,
            glob_counter: Arc<Mutex<usize>>,
            image_mode: ImageMode,
            images_mapped: Arc<Mutex<usize>>,
        ) -> Self {
            let page_size = rustix::param::page_size();
            let guard_size = page_size;
            let size = maximum + guard_size;
            // We rely on the Wasm page size being multiple of host page size.
            assert_eq!(size % page_size, 0);

            let mem = unsafe {
                mmap_anonymous(null_mut(), size, ProtFlags::empty(), MapFlags::PRIVATE)
                    .expect("mmap failed")
            };

            // NOTE: mmap_anonymous returns zero initialized memory, which is relied upon by this
            // API.

            unsafe {
                mprotect(mem, minimum, MprotectFlags::READ | MprotectFlags::WRITE)
                    .expect("mprotect failed");
            }
            *glob_counter.lock().unwrap() += minimum;

            Self {
                mem: mem as usize,
                size,
                guard_size,
                used_wasm_bytes: minimum,
                glob_bytes_counter: glob_counter,
                image_mode,
                images_mapped,
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

        fn init_from_image(&mut self, image: &MemoryImage) -> wasmtime::Result<bool> {
            match self.image_mode {
                ImageMode::ReturnFalse => Ok(false),
                ImageMode::Map => {
                    assert!(image.linear_memory_offset() + image.len() <= self.used_wasm_bytes);
                    // SAFETY: this memory owns its whole mapping and nothing
                    // has accessed it yet.
                    unsafe {
                        image.map_at(self.as_ptr())?;
                    }
                    *self.images_mapped.lock().unwrap() += 1;
                    Ok(true)
                }
                ImageMode::Fail => wasmtime::bail!("custom memory refused its image"),
            }
        }
    }

    struct CustomMemoryCreator {
        pub num_created_memories: Mutex<usize>,
        pub num_total_bytes: Arc<Mutex<usize>>,
        pub num_images_mapped: Arc<Mutex<usize>>,
        image_mode: ImageMode,
    }

    impl CustomMemoryCreator {
        fn with_image_mode(image_mode: ImageMode) -> Self {
            Self {
                num_created_memories: Mutex::new(0),
                num_total_bytes: Arc::new(Mutex::new(0)),
                num_images_mapped: Arc::new(Mutex::new(0)),
                image_mode,
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
                // Cap the maximum at 10MiB to reduce the virtual memory
                // allocated by this test to execute on 32-bit platforms.
                let mem = Box::new(CustomMemory::new(
                    minimum,
                    maximum.unwrap_or(10 << 20),
                    self.num_total_bytes.clone(),
                    self.image_mode,
                    self.num_images_mapped.clone(),
                ));
                *self.num_created_memories.lock().unwrap() += 1;
                Ok(mem)
            }
        }
    }

    fn config() -> (Store<()>, Arc<CustomMemoryCreator>) {
        config_with(ImageMode::ReturnFalse, true)
    }

    fn config_with(image_mode: ImageMode, cow: bool) -> (Store<()>, Arc<CustomMemoryCreator>) {
        let mem_creator = Arc::new(CustomMemoryCreator::with_image_mode(image_mode));
        let mut config = Config::new();
        config
            .with_host_memory(mem_creator.clone())
            .memory_reservation(0)
            .memory_guard_size(0)
            .memory_init_cow(cow);
        (Store::new(&Engine::new(&config).unwrap(), ()), mem_creator)
    }

    /// Whether Wasmtime builds CoW images for modules compiled in memory on
    /// this platform, i.e. whether `LinearMemory::init_from_image` is called.
    fn cow_images_supported() -> bool {
        cfg!(target_os = "linux")
    }

    /// A module whose memory is initialized by a data segment, which is
    /// turned into a CoW image when `memory_init_cow` is enabled.
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

    fn check_data_module(store: &mut Store<()>, instance: Instance) -> wasmtime::Result<()> {
        let load = instance.get_typed_func::<i32, i32>(&mut *store, "load")?;
        let grow = instance.get_typed_func::<i32, i32>(&mut *store, "grow")?;
        assert_eq!(load.call(&mut *store, 0)?, i32::from(b'h'));
        assert_eq!(load.call(&mut *store, 1)?, i32::from(b'e'));
        assert_eq!(load.call(&mut *store, 2)?, i32::from(b'l'));
        assert_eq!(load.call(&mut *store, 3)?, i32::from(b'l'));
        assert_eq!(load.call(&mut *store, 4)?, i32::from(b'o'));
        assert_eq!(load.call(&mut *store, 5)?, 0);
        assert_eq!(grow.call(&mut *store, 1)?, 1);
        assert_eq!(load.call(&mut *store, 0)?, i32::from(b'h'));
        assert_eq!(load.call(&mut *store, 1)?, i32::from(b'e'));
        assert_eq!(load.call(&mut *store, 2)?, i32::from(b'l'));
        assert_eq!(load.call(&mut *store, 3)?, i32::from(b'l'));
        assert_eq!(load.call(&mut *store, 4)?, i32::from(b'o'));
        assert_eq!(load.call(&mut *store, 65536)?, 0);
        Ok(())
    }

    // Regression test for [#10740](https://github.com/bytecodealliance/wasmtime/issues/10740):
    // a custom memory used with CoW.
    #[test]
    fn host_memory_with_cow_image_when_impl_fails_mapping() -> wasmtime::Result<()> {
        let (mut store, mem_creator) = config();
        let module = Module::new(store.engine(), DATA_MODULE)?;
        let instance = Instance::new(&mut store, &module, &[])?;
        check_data_module(&mut store, instance)?;
        assert_eq!(*mem_creator.num_created_memories.lock().unwrap(), 1);
        assert_eq!(*mem_creator.num_images_mapped.lock().unwrap(), 0);
        Ok(())
    }

    // Note that this test doesn't actually test the CoW if cow_images_supported is false, in
    // which case the runtime should fully copy the data.
    #[test]
    fn host_memory_with_cow_image_when_impl_maps() -> wasmtime::Result<()> {
        let (mut store, mem_creator) = config_with(ImageMode::Map, true);
        let module = Module::new(store.engine(), DATA_MODULE)?;

        let first = Instance::new(&mut store, &module, &[])?;
        check_data_module(&mut store, first)?;

        // Writes to one instance's memory must not leak into the image that
        // later instances are initialized from.
        let store_byte = first.get_typed_func::<(i32, i32), ()>(&mut store, "store")?;
        store_byte.call(&mut store, (0, i32::from(b'x')))?;
        let second = Instance::new(&mut store, &module, &[])?;
        let load = second.get_typed_func::<i32, i32>(&mut store, "load")?;
        assert_eq!(load.call(&mut store, 0)?, i32::from(b'h'));
        let load_first = first.get_typed_func::<i32, i32>(&mut store, "load")?;
        assert_eq!(load_first.call(&mut store, 0)?, i32::from(b'x'));

        let expected_mapped = if cow_images_supported() { 2 } else { 0 };
        assert_eq!(*mem_creator.num_created_memories.lock().unwrap(), 2);
        assert_eq!(
            *mem_creator.num_images_mapped.lock().unwrap(),
            expected_mapped
        );
        Ok(())
    }

    #[test]
    fn host_memory_without_cow_init_from_image_is_not_called() -> wasmtime::Result<()> {
        let (mut store, mem_creator) = config_with(ImageMode::Fail, false);
        let module = Module::new(store.engine(), DATA_MODULE)?;
        let instance = Instance::new(&mut store, &module, &[])?;
        check_data_module(&mut store, instance)?;
        assert_eq!(*mem_creator.num_images_mapped.lock().unwrap(), 0);
        Ok(())
    }

    #[test]
    fn host_memory_init_from_image_error_fails_instantiation() -> wasmtime::Result<()> {
        if !cow_images_supported() {
            return Ok(());
        }
        let (mut store, _mem_creator) = config_with(ImageMode::Fail, true);
        let module = Module::new(store.engine(), DATA_MODULE)?;
        let err = Instance::new(&mut store, &module, &[]).unwrap_err();
        assert!(
            format!("{err:?}").contains("custom memory refused its image"),
            "unexpected error: {err:?}"
        );
        Ok(())
    }

    #[test]
    fn host_memory() -> wasmtime::Result<()> {
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
    fn host_memory_grow() -> wasmtime::Result<()> {
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

    // Regression test: memories created by the host with `Memory::new` used to
    // ignore the configured `MemoryCreator`.
    #[test]
    fn host_created_memory_uses_creator() -> wasmtime::Result<()> {
        let page_size = wasmtime_environ::Memory::DEFAULT_PAGE_SIZE as usize;
        let (mut store, mem_creator) = config();

        let memory = Memory::new(&mut store, MemoryType::new(1, Some(3))?)?;
        assert_eq!(*mem_creator.num_created_memories.lock().unwrap(), 1);
        assert_eq!(*mem_creator.num_total_bytes.lock().unwrap(), page_size);

        // Growing from the host goes through `CustomMemory::grow_to`...
        memory.grow(&mut store, 1)?;
        assert_eq!(*mem_creator.num_total_bytes.lock().unwrap(), 2 * page_size);

        // ... and so does growing from Wasm.
        memory.data_mut(&mut store)[page_size] = 42;
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

        assert_eq!(load.call(&mut store, page_size as i32)?, 42);
        assert_eq!(grow.call(&mut store, ())?, 2);
        assert_eq!(*mem_creator.num_total_bytes.lock().unwrap(), 3 * page_size);

        // The importing instance didn't create a memory of its own.
        assert_eq!(*mem_creator.num_created_memories.lock().unwrap(), 1);

        drop(store);
        assert_eq!(*mem_creator.num_total_bytes.lock().unwrap(), 0);

        Ok(())
    }
}
