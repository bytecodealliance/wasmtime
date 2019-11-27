use core::cell::Ref;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use wasmtime_api::*;
use wasmtime_interface_types::*;

fn invoke_export(
    store: &HostRef<Store>,
    instance: &HostRef<Instance>,
    data: &[u8],
    func_name: &str,
) -> Result<Vec<wasmtime_interface_types::Value>, failure::Error> {
    let mut handle = instance.borrow().handle().clone();
    let mut context = store.borrow().engine().borrow().create_wasmtime_context();
    ModuleData::new(&data)
        .expect("module data")
        .invoke(&mut context, &mut handle, func_name, &[])
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

#[test]
fn test_custom_signal_handler_single_instance() {
    let engine = HostRef::new(Engine::new(Config::default()));
    let store = HostRef::new(Store::new(engine));
    let data = std::fs::read("tests/custom_signal_handler.wasm").expect("failed to read wasm file");
    let module = HostRef::new(Module::new(store.clone(), &data).expect("failed to create module"));
    let instance = HostRef::new(
        Instance::new(store.clone(), module, &[]).expect("failed to instantiate module"),
    );

    let (base, length) = set_up_memory(&instance);

    instance
        .borrow_mut()
        .set_signal_handler(move |signum, siginfo, _context| {
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
        });

    let exports = Ref::map(instance.borrow(), |instance| instance.exports());
    assert!(!exports.is_empty());

    // these invoke wasmtime_call_trampoline from action.rs
    {
        println!("calling read...");
        let result = invoke_export(&store, &instance, &data, "read").expect("read succeeded");
        assert_eq!("123", result[0].clone().to_string());
    }

    {
        println!("calling read_out_of_bounds...");
        let trap = invoke_export(&store, &instance, &data, "read_out_of_bounds").unwrap_err();
        assert!(trap
            .find_root_cause()
            .to_string()
            .starts_with("trapped: wasm trap: out of bounds memory access"));
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
        assert_eq!(123i32, result[0].clone().into());
    }

    {
        let read_out_of_bounds_func = exports[1]
            .func()
            .expect("expected a 'read_out_of_bounds' func in the module");
        println!("calling read_out_of_bounds...");
        let trap = read_out_of_bounds_func.borrow().call(&[]).unwrap_err();
        assert!(trap
            .borrow()
            .message()
            .starts_with("wasm trap: out of bounds memory access"));
    }
}

#[test]
fn test_custom_signal_handler_multiple_instances() {
    let engine = HostRef::new(Engine::new(Config::default()));
    let store = HostRef::new(Store::new(engine));
    let data = std::fs::read("tests/custom_signal_handler.wasm").expect("failed to read wasm file");
    let module = HostRef::new(Module::new(store.clone(), &data).expect("failed to create module"));

    // Set up multiple instances

    let instance1 = HostRef::new(
        Instance::new(store.clone(), module.clone(), &[]).expect("failed to instantiate module"),
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
        Instance::new(store.clone(), module, &[]).expect("failed to instantiate module"),
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
        let result = invoke_export(&store, &instance1, &data, "read").expect("read succeeded");
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
        let result = invoke_export(&store, &instance2, &data, "read").expect("read succeeded");
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
    let engine = HostRef::new(Engine::new(Config::default()));
    let store = HostRef::new(Store::new(engine));

    // instance1 which defines 'read'
    let data1 =
        std::fs::read("tests/custom_signal_handler.wasm").expect("failed to read wasm file");
    let module1 =
        HostRef::new(Module::new(store.clone(), &data1).expect("failed to create module"));
    let instance1 = HostRef::new(
        Instance::new(store.clone(), module1.clone(), &[]).expect("failed to instantiate module"),
    );

    let instance1_exports = Ref::map(instance1.borrow(), |i| i.exports());
    assert!(!instance1_exports.is_empty());
    let instance1_read = instance1_exports[0].clone();

    // instance2 wich calls 'instance1.read'
    let data2 =
        std::fs::read("tests/custom_signal_handler_2.wasm").expect("failed to read wasm file");
    let module2 =
        HostRef::new(Module::new(store.clone(), &data2).expect("failed to create module"));
    let instance2 = HostRef::new(
        Instance::new(store.clone(), module2.clone(), &[instance1_read])
            .expect("failed to instantiate module"),
    );

    let result = invoke_export(&store, &instance2, &data2, "run").expect("instance1.run succeeded");
    assert_eq!("123", result[0].clone().to_string());
}
