macro_rules! gentest {
    ($id:ident $name:tt $path:tt) => {
        mod $id {
            mod sugar {
                wasmtime::component::bindgen!(in $path);
            }
            mod normal {
                wasmtime::component::bindgen!($name in $path);
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
                    world: $name,
                    tracing: true,
                    duplicate_if_necessary: true,
                });
            }
            mod interfaces_only {
                wasmtime::component::bindgen!({
                    path: $path,
                    world: $name,
                    only_interfaces: true,
                });
            }
        }
        // ...
    };

}

component_macro_test_helpers::foreach!(gentest);
