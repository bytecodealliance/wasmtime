#[cfg(not(target_os = "windows"))]
mod tests {
    use core::cell::Ref;
    use std::rc::Rc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use wasmtime::*;
    use wasmtime_interface_types::{ModuleData, Value};

    fn invoke_export(
        instance: &HostRef<Instance>,
        data: &[u8],
        func_name: &str,
    ) -> Result<Vec<Value>, anyhow::Error> {
        ModuleData::new(&data)
            .expect("module data")
            .invoke_export(instance, func_name, &[])
    }

    // Locate "memory" export, get base address and size and set memory protection to PROT_NONE
    fn set_up_memory(instance: &HostRef<Instance>) -> (*mut u8, usize) {
        let mem_export = instance.borrow().get_wasmtime_memory().expect("memory");

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
    fn test_custom_signal_handler_single_instance() {
        let engine = Engine::new(&Config::default());
        let store = Store::new(&engine);
        let data =
            std::fs::read("tests/custom_signal_handler.wasm").expect("failed to read wasm file");
        let module = Module::new(&store, &data).expect("failed to create module");
        let instance = HostRef::new(
            Instance::new(&store, &module, &[]).expect("failed to instantiate module"),
        );

        let (base, length) = set_up_memory(&instance);
        instance
            .borrow_mut()
            .set_signal_handler(move |signum, siginfo, _| {
                handle_sigsegv(base, length, signum, siginfo)
            });

        let exports = Ref::map(instance.borrow(), |instance| instance.exports());
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
            let result = read_func
                .borrow()
                .call(&[])
                .expect("expected function not to trap");
            assert_eq!(123i32, result[0].clone().unwrap_i32());
        }

        {
            let read_out_of_bounds_func = exports[1]
                .func()
                .expect("expected a 'read_out_of_bounds' func in the module");
            println!("calling read_out_of_bounds...");
            let trap = read_out_of_bounds_func.borrow().call(&[]).unwrap_err();
            assert!(trap
                .message()
                .starts_with("call error: wasm trap: out of bounds memory access"));
        }
    }

    #[test]
    fn test_custom_signal_handler_multiple_instances() {
        let engine = Engine::new(&Config::default());
        let store = Store::new(&engine);
        let data =
            std::fs::read("tests/custom_signal_handler.wasm").expect("failed to read wasm file");
        let module = Module::new(&store, &data).expect("failed to create module");

        // Set up multiple instances

        let instance1 = HostRef::new(
            Instance::new(&store, &module, &[]).expect("failed to instantiate module"),
        );
        let instance1_handler_triggered = Rc::new(AtomicBool::new(false));

        {
            let (base1, length1) = set_up_memory(&instance1);

            instance1.borrow_mut().set_signal_handler({
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

        let instance2 = HostRef::new(
            Instance::new(&store, &module, &[]).expect("failed to instantiate module"),
        );
        let instance2_handler_triggered = Rc::new(AtomicBool::new(false));

        {
            let (base2, length2) = set_up_memory(&instance2);

            instance2.borrow_mut().set_signal_handler({
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
            let exports1 = Ref::map(instance1.borrow(), |i| i.exports());
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
            let exports2 = Ref::map(instance2.borrow(), |i| i.exports());
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
    }

    #[test]
    fn test_custom_signal_handler_instance_calling_another_instance() {
        let engine = Engine::new(&Config::default());
        let store = Store::new(&engine);

        // instance1 which defines 'read'
        let data1 =
            std::fs::read("tests/custom_signal_handler.wasm").expect("failed to read wasm file");
        let module1 = Module::new(&store, &data1).expect("failed to create module");
        let instance1: HostRef<Instance> = HostRef::new(
            Instance::new(&store, &module1, &[]).expect("failed to instantiate module"),
        );
        let (base1, length1) = set_up_memory(&instance1);
        instance1
            .borrow_mut()
            .set_signal_handler(move |signum, siginfo, _| {
                println!("instance1");
                handle_sigsegv(base1, length1, signum, siginfo)
            });

        let instance1_exports = Ref::map(instance1.borrow(), |i| i.exports());
        assert!(!instance1_exports.is_empty());
        let instance1_read = instance1_exports[0].clone();

        // instance2 wich calls 'instance1.read'
        let data2 =
            std::fs::read("tests/custom_signal_handler_2.wasm").expect("failed to read wasm file");
        let module2 = Module::new(&store, &data2).expect("failed to create module");
        let instance2 = HostRef::new(
            Instance::new(&store, &module2, &[instance1_read])
                .expect("failed to instantiate module"),
        );
        // since 'instance2.run' calls 'instance1.read' we need to set up the signal handler to handle
        // SIGSEGV originating from within the memory of instance1
        instance2
            .borrow_mut()
            .set_signal_handler(move |signum, siginfo, _| {
                handle_sigsegv(base1, length1, signum, siginfo)
            });

        println!("calling instance2.run");
        let result = invoke_export(&instance2, &data2, "run").expect("instance2.run succeeded");
        assert_eq!("123", result[0].clone().to_string());
    }
}
