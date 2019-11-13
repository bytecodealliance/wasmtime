extern crate alloc;

use alloc::rc::Rc;
use core::cell::{Ref, RefCell};
use std::fs::read;
use wasmtime_api::*;

#[test]
fn test_import_calling_export() {
    struct Callback {
        pub other: RefCell<Option<HostRef<Func>>>,
    }

    impl Callable for Callback {
        fn call(&self, _params: &[Val], _results: &mut [Val]) -> Result<(), HostRef<Trap>> {
            self.other
                .borrow()
                .as_ref()
                .expect("expected a function ref")
                .borrow()
                .call(&[])
                .expect("expected function not to trap");
            Ok(())
        }
    }

    let engine = HostRef::new(Engine::default());
    let store = HostRef::new(Store::new(&engine));
    let module = HostRef::new(
        Module::new(
            &store,
            &read("tests/import_calling_export.wasm").expect("failed to read wasm file"),
        )
        .expect("failed to create module"),
    );

    let callback = Rc::new(Callback {
        other: RefCell::new(None),
    });

    let callback_func = HostRef::new(Func::new(
        &store,
        FuncType::new(Box::new([]), Box::new([])),
        callback.clone(),
    ));

    let imports = vec![callback_func.into()];
    let instance = HostRef::new(
        Instance::new(&store, &module, imports.as_slice()).expect("failed to instantiate module"),
    );

    let exports = Ref::map(instance.borrow(), |instance| instance.exports());
    assert!(!exports.is_empty());

    let run_func = exports[0]
        .func()
        .expect("expected a run func in the module");

    *callback.other.borrow_mut() = Some(
        exports[1]
            .func()
            .expect("expected an other func in the module")
            .clone(),
    );

    run_func
        .borrow()
        .call(&[])
        .expect("expected function not to trap");
}
