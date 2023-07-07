macro_rules! gentest {
    ($id:ident $name:tt $path:tt) => {
        mod $id {
            mod sugar {
                wasmtime::component::bindgen!(in $path);
            }
            mod async_ {
                wasmtime::component::bindgen!({
                    path: $path,
                    async: true,
                });
            }
            mod tracing {
                wasmtime::component::bindgen!({
                    path: $path,
                    tracing: true,
                    ownership: Borrowing {
                        duplicate_if_necessary: true
                    }
                });
            }
        }
    };
}

component_macro_test_helpers::foreach!(gentest);
