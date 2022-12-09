use super::{super::REALLOC_AND_FREE, engine};
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
        }",
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
                )
                (core instance $libc (instantiate $libc))
                (core module $m
                    (import "" "core_empty_error" (func $f (param f64 i32)))
                    (import "libc" "memory" (memory 0))
                    (func (export "core_empty_error_export") (param f64) (result i32)
                        (call $f (local.get 0) (i32.const 8))
                        (i32.const 8)
                    )
                )
                (core func $core_empty_error
                    (canon lower (func $i "empty-error") (memory $libc "memory"))
                )
                (core instance $i (instantiate $m
                    (with "" (instance (export "core_empty_error" (func $core_empty_error))))
                    (with "libc" (instance $libc))
                ))
                (func $f_empty_error
                    (export "empty-error")
                    (param "a" float64)
                    (result (result float64))
                    (canon lift (core func $i "core_empty_error_export") (memory $libc "memory"))
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

        let e = results.empty_error(&mut store, 2.0).err().expect("trap");
        assert_eq!(
            format!("{}", e.source().expect("trap message is stored in source")),
            "empty_error: trap"
        );

        Ok(())
    }
}

mod string_error {
    use super::*;
    wasmtime::component::bindgen!({
        inline: "
        world result-playground {
            import imports: interface {
                string-error: func(a: float64) -> result<float64, string>
            }

            default export interface {
                string-error: func(a: float64) -> result<float64, string>
            }
        }",
    });

    #[test]
    fn run() -> Result<(), Error> {
        let engine = engine();
        let component = Component::new(
            &engine,
            format!(
                r#"
            (component
                (import "imports" (instance $i
                    (export "string-error" (func (param "a" float64) (result (result float64 (error string)))))
                ))
                (core module $libc
                    (memory (export "memory") 1)
                    {REALLOC_AND_FREE}
                )
                (core instance $libc (instantiate $libc))
                (core module $m
                    (import "" "core_string_error" (func $f (param f64 i32)))
                    (import "libc" "memory" (memory 0))
                    (import "libc" "realloc" (func $realloc (param i32 i32 i32 i32) (result i32)))
                    (func (export "core_string_error_export") (param f64) (result i32)
                        (local $retptr i32)
                        (local.set $retptr
                            (call $realloc
                                (i32.const 0)
                                (i32.const 0)
                                (i32.const 4)
                                (i32.const 16)))
                        (call $f (local.get 0) (local.get $retptr))
                        (local.get $retptr)
                    )
                )
                (core func $core_string_error
                    (canon lower (func $i "string-error") (memory $libc "memory") (realloc (func $libc "realloc")))
                )
                (core instance $i (instantiate $m
                    (with "" (instance (export "core_string_error" (func $core_string_error))))
                    (with "libc" (instance $libc))
                ))
                (func $f_string_error
                    (export "string-error")
                    (param "a" float64)
                    (result (result float64 (error string)))
                    (canon lift (core func $i "core_string_error_export") (memory $libc "memory"))
                )
            )
        "#
            ),
        )?;

        #[derive(Default)]
        struct MyImports {}

        impl imports::Imports for MyImports {
            fn string_error(&mut self, a: f64) -> Result<Result<f64, String>, Error> {
                if a == 0.0 {
                    Ok(Ok(a))
                } else if a == 1.0 {
                    Ok(Err("string_error: error".to_owned()))
                } else {
                    Err(anyhow!("string_error: trap"))
                }
            }
        }

        let mut linker = Linker::new(&engine);
        imports::add_to_linker(&mut linker, |f: &mut MyImports| f)?;

        let mut store = Store::new(&engine, MyImports::default());
        let (results, _) = ResultPlayground::instantiate(&mut store, &component, &linker)?;

        assert_eq!(
            results
                .string_error(&mut store, 0.0)
                .expect("no trap")
                .expect("no error returned"),
            0.0
        );

        let e = results
            .string_error(&mut store, 1.0)
            .expect("no trap")
            .err()
            .expect("error returned");
        assert_eq!(e, "string_error: error");

        let e = results.string_error(&mut store, 2.0).err().expect("trap");
        assert_eq!(
            format!("{}", e.source().expect("trap message is stored in source")),
            "string_error: trap"
        );

        Ok(())
    }
}
