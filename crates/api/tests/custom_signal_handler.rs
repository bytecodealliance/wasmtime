use core::cell::Ref;
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

#[test]
fn test_custom_signal_handler() {
    let engine = HostRef::new(Engine::new(Config::default()));
    let store = HostRef::new(Store::new(engine));
    let data = std::fs::read("tests/custom_signal_handler.wasm").expect("failed to read wasm file");
    let module = HostRef::new(Module::new(store.clone(), &data).expect("failed to create module"));
    let instance = HostRef::new(
        Instance::new(store.clone(), module, &[]).expect("failed to instantiate module"),
    );

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
}
