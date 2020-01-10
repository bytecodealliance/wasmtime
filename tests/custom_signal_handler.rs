#[cfg(not(target_os = "windows"))]
mod tests {
    use std::rc::Rc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use wasmtime::*;
    use wasmtime_interface_types::{ModuleData, Value};

    const WAT1: &str = r#"
(module
  (func $read (export "read") (result i32)
    (i32.load (i32.const 0))
  )
  (func $read_out_of_bounds (export "read_out_of_bounds") (result i32)
    (i32.load
      (i32.mul
        ;; memory size in Wasm pages
        (memory.size)
        ;; Wasm page size
        (i32.const 65536)
      )
    )
  )
  (func $start
    (i32.store (i32.const 0) (i32.const 123))
  )
  (start $start)
  (memory (export "memory") 1 4)
)
"#;

    const WAT2: &str = r#"
(module
  (import "other_module" "read" (func $other_module.read (result i32)))
  (func $run (export "run") (result i32)
      call $other_module.read)
)
"#;

    fn invoke_export(
        instance: &Instance,
        data: &[u8],
        func_name: &str,
    ) -> Result<Vec<Value>, anyhow::Error> {
        ModuleData::new(&data)?.invoke_export(instance, func_name, &[])
    }

    // Locate "memory" export, get base address and size and set memory protection to PROT_NONE
    fn set_up_memory(instance: &Instance) -> (*mut u8, usize) {
        let mem_export = instance.get_wasmtime_memory().expect("memory");

        let (base, length) = if let wasmtime_runtime::Export::Memory {
            definition,
            vmctx: _,
            memory: _,
        } = mem_export
        {
            unsafe {
                let definition = std::ptr::read(definition);
                (definition.base, definition.current_length)
            }
        } else {
            panic!("expected memory");
        };

        // So we can later trigger SIGSEGV by performing a read
        unsafe {
            libc::mprotect(base as *mut libc::c_void, length, libc::PROT_NONE);
        }

        println!("memory: base={:?}, length={}", base, length);

        (base, length)
    }

    fn handle_sigsegv(
        base: *mut u8,
        length: usize,
        signum: libc::c_int,
        siginfo: *const libc::siginfo_t,
    ) -> bool {
        println!("Hello from instance signal handler!");
        // SIGSEGV on Linux, SIGBUS on Mac
        if libc::SIGSEGV == signum || libc::SIGBUS == signum {
            let si_addr: *mut libc::c_void = unsafe { (*siginfo).si_addr() };
            // Any signal from within module's memory we handle ourselves
            let result = (si_addr as u64) < (base as u64) + (length as u64);
            // Remove protections so the execution may resume
            unsafe {
                libc::mprotect(
                    base as *mut libc::c_void,
                    length,
                    libc::PROT_READ | libc::PROT_WRITE,
                );
            }
            println!("signal handled: {}", result);
            result
        } else {
            // Otherwise, we forward to wasmtime's signal handler.
            false
        }
    }

    #[test]
    fn test_custom_signal_handler_single_instance() -> anyhow::Result<()> {
        let engine = Engine::new(&Config::default());
        let store = Store::new(&engine);
        let data = wat::parse_str(WAT1)?;
        let module = Module::new(&store, &data)?;
        let instance = Instance::new(&store, &module, &[])?;

        let (base, length) = set_up_memory(&instance);
        instance.set_signal_handler(move |signum, siginfo, _| {
            handle_sigsegv(base, length, signum, siginfo)
        });

        let exports = instance.exports();
        assert!(!exports.is_empty());

        // these invoke wasmtime_call_trampoline from action.rs
        {
            println!("calling read...");
            let result = invoke_export(&instance, &data, "read").expect("read succeeded");
            assert_eq!("123", result[0].clone().to_string());
        }

        {
            println!("calling read_out_of_bounds...");
            let trap = invoke_export(&instance, &data, "read_out_of_bounds").unwrap_err();
            assert!(trap.root_cause().to_string().starts_with(
                "trapped: Trap { message: \"call error: wasm trap: out of bounds memory access"
            ));
        }

        // these invoke wasmtime_call_trampoline from callable.rs
        {
            let read_func = exports[0]
                .func()
                .expect("expected a 'read' func in the module");
            println!("calling read...");
            let result = read_func.call(&[]).expect("expected function not to trap");
            assert_eq!(123i32, result[0].clone().unwrap_i32());
        }

        {
            let read_out_of_bounds_func = exports[1]
                .func()
                .expect("expected a 'read_out_of_bounds' func in the module");
            println!("calling read_out_of_bounds...");
            let trap = read_out_of_bounds_func.call(&[]).unwrap_err();
            assert!(trap
                .message()
                .starts_with("call error: wasm trap: out of bounds memory access"));
        }
        Ok(())
    }

    #[test]
    fn test_custom_signal_handler_multiple_instances() -> anyhow::Result<()> {
        let engine = Engine::new(&Config::default());
        let store = Store::new(&engine);
        let data = wat::parse_str(WAT1)?;
        let module = Module::new(&store, &data)?;

        // Set up multiple instances

        let instance1 = Instance::new(&store, &module, &[])?;
        let instance1_handler_triggered = Rc::new(AtomicBool::new(false));

        {
            let (base1, length1) = set_up_memory(&instance1);

            instance1.set_signal_handler({
                let instance1_handler_triggered = instance1_handler_triggered.clone();
                move |_signum, _siginfo, _context| {
                    // Remove protections so the execution may resume
                    unsafe {
                        libc::mprotect(
                            base1 as *mut libc::c_void,
                            length1,
                            libc::PROT_READ | libc::PROT_WRITE,
                        );
                    }
                    instance1_handler_triggered.store(true, Ordering::SeqCst);
                    println!(
                        "Hello from instance1 signal handler! {}",
                        instance1_handler_triggered.load(Ordering::SeqCst)
                    );
                    true
                }
            });
        }

        let instance2 = Instance::new(&store, &module, &[]).expect("failed to instantiate module");
        let instance2_handler_triggered = Rc::new(AtomicBool::new(false));

        {
            let (base2, length2) = set_up_memory(&instance2);

            instance2.set_signal_handler({
                let instance2_handler_triggered = instance2_handler_triggered.clone();
                move |_signum, _siginfo, _context| {
                    // Remove protections so the execution may resume
                    unsafe {
                        libc::mprotect(
                            base2 as *mut libc::c_void,
                            length2,
                            libc::PROT_READ | libc::PROT_WRITE,
                        );
                    }
                    instance2_handler_triggered.store(true, Ordering::SeqCst);
                    println!(
                        "Hello from instance2 signal handler! {}",
                        instance2_handler_triggered.load(Ordering::SeqCst)
                    );
                    true
                }
            });
        }

        // Invoke both instances and trigger both signal handlers

        // First instance1
        {
            let exports1 = instance1.exports();
            assert!(!exports1.is_empty());

            println!("calling instance1.read...");
            let result = invoke_export(&instance1, &data, "read").expect("read succeeded");
            assert_eq!("123", result[0].clone().to_string());
            assert_eq!(
                instance1_handler_triggered.load(Ordering::SeqCst),
                true,
                "instance1 signal handler has been triggered"
            );
        }

        // And then instance2
        {
            let exports2 = instance2.exports();
            assert!(!exports2.is_empty());

            println!("calling instance2.read...");
            let result = invoke_export(&instance2, &data, "read").expect("read succeeded");
            assert_eq!("123", result[0].clone().to_string());
            assert_eq!(
                instance2_handler_triggered.load(Ordering::SeqCst),
                true,
                "instance1 signal handler has been triggered"
            );
        }
        Ok(())
    }

    #[test]
    fn test_custom_signal_handler_instance_calling_another_instance() -> anyhow::Result<()> {
        let engine = Engine::new(&Config::default());
        let store = Store::new(&engine);

        // instance1 which defines 'read'
        let data1 = wat::parse_str(WAT1)?;
        let module1 = Module::new(&store, &data1)?;
        let instance1 = Instance::new(&store, &module1, &[])?;
        let (base1, length1) = set_up_memory(&instance1);
        instance1.set_signal_handler(move |signum, siginfo, _| {
            println!("instance1");
            handle_sigsegv(base1, length1, signum, siginfo)
        });

        let instance1_exports = instance1.exports();
        assert!(!instance1_exports.is_empty());
        let instance1_read = instance1_exports[0].clone();

        // instance2 wich calls 'instance1.read'
        let data2 = wat::parse_str(WAT2)?;
        let module2 = Module::new(&store, &data2)?;
        let instance2 = Instance::new(&store, &module2, &[instance1_read])?;
        // since 'instance2.run' calls 'instance1.read' we need to set up the signal handler to handle
        // SIGSEGV originating from within the memory of instance1
        instance2.set_signal_handler(move |signum, siginfo, _| {
            handle_sigsegv(base1, length1, signum, siginfo)
        });

        println!("calling instance2.run");
        let result = invoke_export(&instance2, &data2, "run")?;
        assert_eq!("123", result[0].clone().to_string());
        Ok(())
    }
}
