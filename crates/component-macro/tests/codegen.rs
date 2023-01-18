macro_rules! gentest {
    ($id:ident $name:tt $path:tt) => {
        mod $id {
            mod normal {
                wasmtime::component::bindgen!($name in $path);
            }
            mod async_ {
                wasmtime::component::bindgen!({
                    world: $name,
                    path: $path,
                    async: true,
                });
            }
            mod tracing {
                wasmtime::component::bindgen!({
                    world: $name,
                    path: $path,
                    tracing: true,
                });
            }
        }
        // ...
    };
}

component_macro_test_helpers::foreach!(gentest);
