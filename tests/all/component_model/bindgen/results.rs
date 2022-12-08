use super::engine;
use anyhow::{anyhow, Error};
use wasmtime::{
    component::{Component, Linker},
    Store,
};

mod empty_error {
    use super::*;
    wasmtime::component::bindgen!({
        inline: "
        world result-playground {
            import imports: interface {
                empty-error: func(a: float64) -> result<float64>
            }

            default export interface {
                empty-error: func(a: float64) -> result<float64>
            }
        }"
    });

    #[test]
    fn run() -> Result<(), Error> {
        let engine = engine();
        let component = Component::new(
            &engine,
            r#"
            (component
                (import "imports" (instance $i
                    (export "empty-error" (func (param "a" float64) (result (result float64))))
                ))
                (core module $libc
                    (memory (export "memory") 1)
                    (func (export "realloc") (param i32 i32 i32 i32) (result i32)
                        unreachable)
                )
                (core instance $libc (instantiate $libc))
                (core module $m
                    (import "" "core_empty_error" (func (param f64) (result i32)))
                    (export "core_empty_error" (func 0))
                )
                (core func $core_empty_error
                    (canon lower (func $i "empty-error") (memory $libc "memory"))
                )
                (core instance $i (instantiate $m
                    (with "" (instance (export "core_empty_error" (func $core_empty_error))))
                ))
                (func $f_empty_error
                    (export "empty-error")
                    (param "a" float64)
                    (result (result float64))
                    (canon lift (core func $i "empty_error") (memory $libc "memory"))
                )
            )
        "#,
        )?;

        #[derive(Default)]
        struct MyImports {}

        impl imports::Imports for MyImports {
            fn empty_error(&mut self, a: f64) -> Result<Result<f64, ()>, Error> {
                if a == 0.0 {
                    Ok(Ok(a))
                } else if a == 1.0 {
                    Ok(Err(()))
                } else {
                    Err(anyhow!("empty_error: trap"))
                }
            }
        }

        let mut linker = Linker::new(&engine);
        imports::add_to_linker(&mut linker, |f: &mut MyImports| f)?;

        let mut store = Store::new(&engine, MyImports::default());
        let (results, _) = ResultPlayground::instantiate(&mut store, &component, &linker)?;

        assert_eq!(
            results
                .empty_error(&mut store, 0.0)
                .expect("no trap")
                .expect("no error returned"),
            0.0
        );

        results
            .empty_error(&mut store, 1.0)
            .expect("no trap")
            .err()
            .expect("() error returned");

        results.empty_error(&mut store, 2.0).err().expect("trap");

        Ok(())
    }
}
