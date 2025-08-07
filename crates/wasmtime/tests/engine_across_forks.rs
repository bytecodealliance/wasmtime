use wasmtime::Result;

fn main() -> Result<()> {
    #[cfg(unix)]
    if true {
        use libtest_mimic::{Arguments, Trial};

        let mut trials = Vec::new();
        for (name, test) in linux::TESTS {
            trials.push(Trial::test(*name, || {
                test().map_err(|e| format!("{e:?}").into())
            }));
        }

        let mut args = Arguments::from_args();
        // I'll be honest, I'm scared of threads + fork, so I'm just
        // preemptively disabling threads here.
        args.test_threads = Some(1);
        libtest_mimic::run(&args, trials).exit()
    }

    Ok(())
}

mod linux {
    use rustix::fd::AsRawFd;
    use rustix::process::{Pid, WaitOptions, waitpid};
    use std::io::{self, BufRead, BufReader};
    use wasmtime::*;

    pub const TESTS: &[(&str, fn() -> Result<()>)] = &[
        ("smoke", smoke),
        ("pooling_allocator_reset", pooling_allocator_reset),
    ];

    fn smoke() -> Result<()> {
        let engine = Engine::default();
        let module = Module::new(&engine, r#"(module (func (export "")))"#)?;
        run_in_child(|| {
            let mut store = Store::new(&engine, ());
            let instance = Instance::new(&mut store, &module, &[])?;
            let export = instance.get_typed_func::<(), ()>(&mut store, "")?;
            export.call(&mut store, ())?;
            Ok(())
        })?;
        Ok(())
    }

    fn pooling_allocator_reset() -> Result<()> {
        let mut pooling = PoolingAllocationConfig::new();
        pooling.linear_memory_keep_resident(4096);
        let mut config = Config::new();
        config.allocation_strategy(pooling);
        let engine = Engine::new(&config)?;
        let module = Module::new(
            &engine,
            r#"
                (module
                    (memory (export "") 1 1)
                    (data (i32.const 0) "\0a")
                )
            "#,
        )?;

        let assert_pristine = || {
            let mut store = Store::new(&engine, ());
            let instance = Instance::new(&mut store, &module, &[])?;
            let memory = instance.get_memory(&mut store, "").unwrap();
            let data = memory.data(&store);
            assert_eq!(data[0], 0x0a);
            anyhow::Ok((store, memory))
        };
        run_in_child(|| {
            // Allocate a memory, and then mutate it.
            let (mut store, memory) = assert_pristine()?;
            let data = memory.data_mut(&mut store);
            data[0] = 0;
            drop(store);

            // Allocating the memory again should reuse the same pooling
            // allocator slot but it should be reset correctly.
            assert_pristine()?;
            assert_pristine()?;
            Ok(())
        })?;
        Ok(())
    }

    fn run_in_child(closure: impl FnOnce() -> Result<()>) -> Result<()> {
        let (read, write) = io::pipe()?;
        let child = match unsafe { libc::fork() } {
            -1 => return Err(io::Error::last_os_error().into()),

            0 => {
                // If a panic happens, don't let it go above this stack frame.
                let _bomb = Bomb;

                drop(read);
                unsafe {
                    assert!(libc::dup2(write.as_raw_fd(), 1) == 1);
                    assert!(libc::dup2(write.as_raw_fd(), 2) == 2);
                }
                drop(write);

                closure().unwrap();
                std::process::exit(0);
            }

            pid => pid,
        };

        drop(write);

        for line in BufReader::new(read).lines() {
            println!("CHILD: {}", line?);
        }

        let (_pid, status) =
            waitpid(Some(Pid::from_raw(child).unwrap()), WaitOptions::empty())?.unwrap();
        assert_eq!(status.as_raw(), 0);

        Ok(())
    }

    struct Bomb;

    impl Drop for Bomb {
        fn drop(&mut self) {
            std::process::exit(1);
        }
    }
}
