use rayon::prelude::*;
use std::sync::atomic::{AtomicU32, Ordering::SeqCst};
use std::time::Duration;
use wasmtime::*;

fn module(engine: &Engine) -> Result<Module> {
    let mut wat = format!("(module\n");
    wat.push_str("(import \"\" \"\" (memory 0))\n");
    for i in 0..=33 {
        let offset = if i == 0 {
            0
        } else if i == 33 {
            !0
        } else {
            1u32 << (i - 1)
        };

        for (width, instr) in [
            (1, &["i32.load8_s"][..]),
            (2, &["i32.load16_s"]),
            (4, &["i32.load" /*, "f32.load"*/]),
            (8, &["i64.load" /*, "f64.load"*/]),
            #[cfg(not(any(target_arch = "s390x", target_arch = "riscv64")))]
            (16, &["v128.load"]),
        ]
        .iter()
        {
            for (j, instr) in instr.iter().enumerate() {
                wat.push_str(&format!(
                    "(func (export \"{} {} v{}\") (param i32)\n",
                    width, offset, j
                ));
                wat.push_str("local.get 0\n");
                wat.push_str(instr);
                wat.push_str(&format!(" offset={}\n", offset));
                wat.push_str("drop\n)");
            }
        }
    }
    wat.push_str(")");
    Module::new(engine, &wat)
}

struct TestFunc {
    width: u32,
    offset: u32,
    func: TypedFunc<u32, ()>,
}

fn find_funcs(store: &mut Store<()>, instance: &Instance) -> Vec<TestFunc> {
    let list = instance
        .exports(&mut *store)
        .map(|export| {
            let name = export.name();
            let mut parts = name.split_whitespace();
            (
                parts.next().unwrap().parse().unwrap(),
                parts.next().unwrap().parse().unwrap(),
                export.into_func().unwrap(),
            )
        })
        .collect::<Vec<_>>();
    list.into_iter()
        .map(|(width, offset, func)| TestFunc {
            width,
            offset,
            func: func.typed(&store).unwrap(),
        })
        .collect()
}

fn test_traps(store: &mut Store<()>, funcs: &[TestFunc], addr: u32, mem: &Memory) {
    let mem_size = mem.data_size(&store) as u64;
    for func in funcs {
        let result = func.func.call(&mut *store, addr);
        let base = u64::from(func.offset) + u64::from(addr);
        let range = base..base + u64::from(func.width);
        if range.start >= mem_size || range.end >= mem_size {
            assert!(
                result.is_err(),
                "access at {}+{}+{} succeeded but should have failed when memory has {} bytes",
                addr,
                func.offset,
                func.width,
                mem_size
            );
        } else {
            assert!(result.is_ok());
        }
    }
}

#[test]
#[cfg_attr(miri, ignore)]
fn offsets_static_dynamic_oh_my() -> Result<()> {
    const GB: u64 = 1 << 30;

    let mut engines = Vec::new();
    let sizes = [0, 1 * GB, 4 * GB];
    for &static_memory_maximum_size in sizes.iter() {
        for &guard_size in sizes.iter() {
            for &guard_before_linear_memory in [true, false].iter() {
                let mut config = Config::new();
                config.wasm_simd(true);
                config.static_memory_maximum_size(static_memory_maximum_size);
                config.dynamic_memory_guard_size(guard_size);
                config.static_memory_guard_size(guard_size);
                config.guard_before_linear_memory(guard_before_linear_memory);
                config.cranelift_debug_verifier(true);
                engines.push(Engine::new(&config)?);
            }
        }
    }

    engines.par_iter().for_each(|engine| {
        let module = module(&engine).unwrap();

        for (min, max) in [(1, Some(2)), (1, None)].iter() {
            let mut store = Store::new(&engine, ());
            let mem = Memory::new(&mut store, MemoryType::new(*min, *max)).unwrap();
            let instance = Instance::new(&mut store, &module, &[mem.into()]).unwrap();
            let funcs = find_funcs(&mut store, &instance);

            test_traps(&mut store, &funcs, 0, &mem);
            test_traps(&mut store, &funcs, 65536, &mem);
            test_traps(&mut store, &funcs, u32::MAX, &mem);

            mem.grow(&mut store, 1).unwrap();

            test_traps(&mut store, &funcs, 0, &mem);
            test_traps(&mut store, &funcs, 65536, &mem);
            test_traps(&mut store, &funcs, u32::MAX, &mem);
        }
    });

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn guards_present() -> Result<()> {
    const GUARD_SIZE: u64 = 65536;

    let mut config = Config::new();
    config.static_memory_maximum_size(1 << 20);
    config.dynamic_memory_guard_size(GUARD_SIZE);
    config.static_memory_guard_size(GUARD_SIZE);
    config.guard_before_linear_memory(true);
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());
    let static_mem = Memory::new(&mut store, MemoryType::new(1, Some(2)))?;
    let dynamic_mem = Memory::new(&mut store, MemoryType::new(1, None))?;

    let assert_guards = |store: &Store<()>| unsafe {
        // guards before
        println!("check pre-static-mem");
        assert_faults(static_mem.data_ptr(&store).offset(-(GUARD_SIZE as isize)));
        println!("check pre-dynamic-mem");
        assert_faults(dynamic_mem.data_ptr(&store).offset(-(GUARD_SIZE as isize)));

        // guards after
        println!("check post-static-mem");
        assert_faults(
            static_mem
                .data_ptr(&store)
                .add(static_mem.data_size(&store)),
        );
        println!("check post-dynamic-mem");
        assert_faults(
            dynamic_mem
                .data_ptr(&store)
                .add(dynamic_mem.data_size(&store)),
        );
    };
    assert_guards(&store);
    // static memory should start with the second page unmapped
    unsafe {
        assert_faults(static_mem.data_ptr(&store).add(65536));
    }
    println!("growing");
    static_mem.grow(&mut store, 1).unwrap();
    dynamic_mem.grow(&mut store, 1).unwrap();
    assert_guards(&store);

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn guards_present_pooling() -> Result<()> {
    const GUARD_SIZE: u64 = 65536;

    let mut pool = crate::small_pool_config();
    pool.total_memories(2)
        .memory_pages(10)
        .memory_protection_keys(MpkEnabled::Disable);
    let mut config = Config::new();
    config.static_memory_maximum_size(1 << 20);
    config.dynamic_memory_guard_size(GUARD_SIZE);
    config.static_memory_guard_size(GUARD_SIZE);
    config.guard_before_linear_memory(true);
    config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool));
    let engine = Engine::new(&config)?;

    let mut store = Store::new(&engine, ());

    let mem1 = {
        let m = Module::new(&engine, "(module (memory (export \"\") 1 2))")?;
        Instance::new(&mut store, &m, &[])?
            .get_memory(&mut store, "")
            .unwrap()
    };
    let mem2 = {
        let m = Module::new(&engine, "(module (memory (export \"\") 1))")?;
        Instance::new(&mut store, &m, &[])?
            .get_memory(&mut store, "")
            .unwrap()
    };

    unsafe fn assert_guards(store: &Store<()>, mem: &Memory) {
        // guards before
        println!("check pre-mem");
        assert_faults(mem.data_ptr(&store).offset(-(GUARD_SIZE as isize)));

        // unmapped just after memory
        println!("check mem");
        assert_faults(mem.data_ptr(&store).add(mem.data_size(&store)));

        // guards after memory
        println!("check post-mem");
        assert_faults(mem.data_ptr(&store).add(1 << 20));
    }
    unsafe {
        assert_guards(&store, &mem1);
        assert_guards(&store, &mem2);
        println!("growing");
        mem1.grow(&mut store, 1).unwrap();
        mem2.grow(&mut store, 1).unwrap();
        assert_guards(&store, &mem1);
        assert_guards(&store, &mem2);
    }

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn guards_present_pooling_mpk() -> Result<()> {
    if !wasmtime::PoolingAllocationConfig::are_memory_protection_keys_available() {
        println!("skipping `guards_present_pooling_mpk` test; mpk is not supported");
        return Ok(());
    }

    const GUARD_SIZE: u64 = 65536;
    let mut pool = crate::small_pool_config();
    pool.total_memories(4)
        .memory_pages(10)
        .memory_protection_keys(MpkEnabled::Enable)
        .max_memory_protection_keys(2);
    let mut config = Config::new();
    config.static_memory_maximum_size(1 << 20);
    config.dynamic_memory_guard_size(GUARD_SIZE);
    config.static_memory_guard_size(GUARD_SIZE);
    config.guard_before_linear_memory(true);
    config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool));
    let engine = Engine::new(&config)?;

    let mut store = Store::new(&engine, ());

    let mem1 = {
        let m = Module::new(&engine, "(module (memory (export \"\") 1 2))")?;
        Instance::new(&mut store, &m, &[])?
            .get_memory(&mut store, "")
            .unwrap()
    };
    let mem2 = {
        let m = Module::new(&engine, "(module (memory (export \"\") 1))")?;
        Instance::new(&mut store, &m, &[])?
            .get_memory(&mut store, "")
            .unwrap()
    };

    unsafe fn assert_guards(store: &Store<()>, mem: &Memory) {
        // guards before
        println!("check pre-mem");
        assert_faults(mem.data_ptr(&store).offset(-(GUARD_SIZE as isize)));

        // unmapped just after memory
        println!("check mem");
        assert_faults(mem.data_ptr(&store).add(mem.data_size(&store)));

        // guards after memory
        println!("check post-mem");
        assert_faults(mem.data_ptr(&store).add(1 << 20));
    }
    unsafe {
        assert_guards(&store, &mem1);
        assert_guards(&store, &mem2);
        println!("growing");
        mem1.grow(&mut store, 1).unwrap();
        mem2.grow(&mut store, 1).unwrap();
        assert_guards(&store, &mem1);
        assert_guards(&store, &mem2);
    }

    Ok(())
}

unsafe fn assert_faults(ptr: *mut u8) {
    use std::io::Error;
    #[cfg(unix)]
    {
        // There's probably a faster way to do this here, but, uh, when in rome?
        match libc::fork() {
            0 => {
                *ptr = 4;
                std::process::exit(0);
            }
            -1 => panic!("failed to fork: {}", Error::last_os_error()),
            n => {
                let mut status = 0;
                assert!(
                    libc::waitpid(n, &mut status, 0) == n,
                    "failed to wait: {}",
                    Error::last_os_error()
                );
                assert!(libc::WIFSIGNALED(status));
            }
        }
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Memory::*;

        let mut info = std::mem::MaybeUninit::uninit();
        let r = VirtualQuery(
            ptr as *const _,
            info.as_mut_ptr(),
            std::mem::size_of_val(&info),
        );
        if r == 0 {
            panic!("failed to VirtualAlloc: {}", Error::last_os_error());
        }
        let info = info.assume_init();
        assert_eq!(info.AllocationProtect, PAGE_NOACCESS);
    }
}

#[test]
fn massive_64_bit_still_limited() -> Result<()> {
    // Creating a 64-bit memory which exceeds the limits of the address space
    // should still send a request to the `ResourceLimiter` to ensure that it
    // gets at least some chance to see that oom was requested.
    let mut config = Config::new();
    config.wasm_memory64(true);
    let engine = Engine::new(&config)?;

    let mut store = Store::new(&engine, MyLimiter { hit: false });
    store.limiter(|x| x);
    let ty = MemoryType::new64(1 << 48, None);
    assert!(Memory::new(&mut store, ty).is_err());
    assert!(store.data().hit);

    return Ok(());

    struct MyLimiter {
        hit: bool,
    }

    impl ResourceLimiter for MyLimiter {
        fn memory_growing(
            &mut self,
            _current: usize,
            _request: usize,
            _max: Option<usize>,
        ) -> Result<bool> {
            self.hit = true;
            Ok(true)
        }
        fn table_growing(
            &mut self,
            _current: u32,
            _request: u32,
            _max: Option<u32>,
        ) -> Result<bool> {
            unreachable!()
        }
    }
}

#[test]
#[cfg_attr(miri, ignore)]
fn tiny_static_heap() -> Result<()> {
    // The size of the memory in the module below is the exact same size as
    // the static memory size limit in the configuration. This is intended to
    // specifically test that a load of all the valid addresses of the memory
    // all pass bounds-checks in cranelift to help weed out any off-by-one bugs.
    let mut config = Config::new();
    config.static_memory_maximum_size(65536);
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());

    let module = Module::new(
        &engine,
        r#"
            (module
                (memory 1 1)
                (func (export "run")
                    (local $i i32)

                    (loop
                        (if (i32.eq (local.get $i) (i32.const 65536))
                            (then (return)))
                        (drop (i32.load8_u (local.get $i)))
                        (local.set $i (i32.add (local.get $i) (i32.const 1)))
                        br 0
                    )
                )
            )
        "#,
    )?;

    let i = Instance::new(&mut store, &module, &[])?;
    let f = i.get_typed_func::<(), ()>(&mut store, "run")?;
    f.call(&mut store, ())?;
    Ok(())
}

#[test]
fn static_forced_max() -> Result<()> {
    let mut config = Config::new();
    config.static_memory_maximum_size(5 * 65536);
    config.static_memory_forced(true);
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());

    let mem = Memory::new(&mut store, MemoryType::new(0, None))?;
    mem.grow(&mut store, 5).unwrap();
    assert!(mem.grow(&mut store, 1).is_err());
    Ok(())
}

#[test]
fn dynamic_extra_growth_unchanged_pointer() -> Result<()> {
    const EXTRA_PAGES: u64 = 5;
    let mut config = Config::new();
    config.static_memory_maximum_size(0);
    // 5 wasm pages extra
    config.dynamic_memory_reserved_for_growth(EXTRA_PAGES * (1 << 16));
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());

    fn assert_behaves_well(store: &mut Store<()>, mem: &Memory) -> Result<()> {
        let ptr = mem.data_ptr(&store);

        // Each growth here should retain the same linear pointer in memory and the
        // memory shouldn't get moved.
        for _ in 0..EXTRA_PAGES {
            mem.grow(&mut *store, 1)?;
            assert_eq!(ptr, mem.data_ptr(&store));
        }

        // Growth afterwards though will be forced to move the pointer
        mem.grow(&mut *store, 1)?;
        let new_ptr = mem.data_ptr(&store);
        assert_ne!(ptr, new_ptr);

        for _ in 0..EXTRA_PAGES - 1 {
            mem.grow(&mut *store, 1)?;
            assert_eq!(new_ptr, mem.data_ptr(&store));
        }
        Ok(())
    }

    let mem = Memory::new(&mut store, MemoryType::new(10, None))?;
    assert_behaves_well(&mut store, &mem)?;

    let module = Module::new(&engine, r#"(module (memory (export "mem") 10))"#)?;
    let instance = Instance::new(&mut store, &module, &[])?;
    let mem = instance.get_memory(&mut store, "mem").unwrap();
    assert_behaves_well(&mut store, &mem)?;

    let module = Module::new(
        &engine,
        r#"
            (module
                (memory (export "mem") 10)
                (data (i32.const 0) ""))
        "#,
    )?;
    let instance = Instance::new(&mut store, &module, &[])?;
    let mem = instance.get_memory(&mut store, "mem").unwrap();
    assert_behaves_well(&mut store, &mem)?;

    Ok(())
}

// This test exercises trying to create memories of the maximum 64-bit memory
// size of `1 << 48` pages. This should always fail but in the process of
// determining this failure we shouldn't hit any overflows or anything like that
// (checked via debug-mode tests).
#[test]
fn memory64_maximum_minimum() -> Result<()> {
    let mut config = Config::new();
    config.wasm_memory64(true);
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());

    assert!(Memory::new(&mut store, MemoryType::new64(1 << 48, None)).is_err());

    let module = Module::new(
        &engine,
        &format!(
            r#"
                (module
                    (memory i64 {})
                )
            "#,
            1u64 << 48,
        ),
    )?;
    assert!(Instance::new(&mut store, &module, &[]).is_err());

    let module = Module::new(
        &engine,
        &format!(
            r#"
                (module
                    (memory i64 {})
                    (data (i64.const 0) "")
                )
            "#,
            1u64 << 48,
        ),
    )?;
    assert!(Instance::new(&mut store, &module, &[]).is_err());

    Ok(())
}

#[test]
fn shared_memory_basics() -> Result<()> {
    let engine = Engine::default();
    assert!(SharedMemory::new(&engine, MemoryType::new(1, None)).is_err());
    assert!(SharedMemory::new(&engine, MemoryType::new(1, Some(1))).is_err());
    assert!(SharedMemory::new(&engine, MemoryType::new64(1, None)).is_err());
    assert!(SharedMemory::new(&engine, MemoryType::new64(1, Some(1))).is_err());
    assert!(SharedMemory::new(&engine, MemoryType::shared(1, 0)).is_err());

    let memory = SharedMemory::new(&engine, MemoryType::shared(1, 1))?;
    assert!(memory.ty().is_shared());
    assert_eq!(memory.ty().minimum(), 1);
    assert_eq!(memory.ty().maximum(), Some(1));
    assert_eq!(memory.size(), 1);
    assert_eq!(memory.data_size(), 65536);
    assert_eq!(memory.data().len(), 65536);
    assert!(memory.grow(1).is_err());

    // misaligned
    assert_eq!(memory.atomic_notify(1, 100), Err(Trap::HeapMisaligned));
    assert_eq!(
        memory.atomic_wait32(1, 100, None),
        Err(Trap::HeapMisaligned)
    );
    assert_eq!(
        memory.atomic_wait64(1, 100, None),
        Err(Trap::HeapMisaligned)
    );

    // oob
    assert_eq!(
        memory.atomic_notify(1 << 20, 100),
        Err(Trap::MemoryOutOfBounds)
    );
    assert_eq!(
        memory.atomic_wait32(1 << 20, 100, None),
        Err(Trap::MemoryOutOfBounds)
    );
    assert_eq!(
        memory.atomic_wait64(1 << 20, 100, None),
        Err(Trap::MemoryOutOfBounds)
    );

    // ok
    assert_eq!(memory.atomic_notify(8, 100), Ok(0));
    assert_eq!(memory.atomic_wait32(8, 1, None), Ok(WaitResult::Mismatch));
    assert_eq!(memory.atomic_wait64(8, 1, None), Ok(WaitResult::Mismatch));

    // timeout
    let near_future = Duration::new(0, 100);
    assert_eq!(
        memory.atomic_wait32(8, 0, Some(near_future)),
        Ok(WaitResult::TimedOut)
    );
    assert_eq!(
        memory.atomic_wait64(8, 0, Some(near_future)),
        Ok(WaitResult::TimedOut)
    );

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn shared_memory_wait_notify() -> Result<()> {
    const THREADS: usize = 8;
    const COUNT: usize = 100_000;

    let engine = Engine::default();
    let memory = SharedMemory::new(&engine, MemoryType::shared(1, 1))?;
    let data = unsafe { AtomicU32::from_ptr(memory.data().as_ptr().cast_mut().cast()) };
    let locked = unsafe { AtomicU32::from_ptr(memory.data().as_ptr().add(4).cast_mut().cast()) };

    // Note that `SeqCst` is used here to not think much about the orderings
    // here, and it also somewhat more closely mirrors what's happening in wasm.
    let lock = || {
        while locked.swap(1, SeqCst) == 1 {
            memory.atomic_wait32(0, 1, None).unwrap();
        }
    };
    let unlock = || {
        locked.store(0, SeqCst);
        memory.atomic_notify(0, 1).unwrap();
    };

    std::thread::scope(|s| {
        for _ in 0..THREADS {
            s.spawn(|| {
                for _ in 0..COUNT {
                    lock();
                    let next = data.load(SeqCst) + 1;
                    data.store(next, SeqCst);
                    unlock();
                }
            });
        }
    });

    assert_eq!(data.load(SeqCst), (THREADS * COUNT) as u32);

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn init_with_negative_segment() -> Result<()> {
    let engine = Engine::default();
    let module = Module::new(
        &engine,
        r#"
            (module
                (memory 65536)
                (data (i32.const 0x8000_0000) "x")
            )
        "#,
    )?;
    let mut store = Store::new(&engine, ());
    Instance::new(&mut store, &module, &[])?;
    Ok(())
}
