use super::engine;
use anyhow::{bail, Result};
use wasmtime::{
    component::{Component, Linker},
    Store,
};

mod results;

mod no_imports {
    use super::*;

    wasmtime::component::bindgen!({
        inline: "
            default world no-imports {
                export foo: interface {
                    foo: func()
                }

                export bar: func()
            }
        ",
    });

    #[test]
    fn run() -> Result<()> {
        let engine = engine();

        let component = Component::new(
            &engine,
            r#"
                (component
                    (core module $m
                        (func (export ""))
                    )
                    (core instance $i (instantiate $m))

                    (func $f (export "bar") (canon lift (core func $i "")))

                    (instance $i (export "foo" (func $f)))
                    (export "foo" (instance $i))
                )
            "#,
        )?;

        let linker = Linker::new(&engine);
        let mut store = Store::new(&engine, ());
        let (no_imports, _) = NoImports::instantiate(&mut store, &component, &linker)?;
        no_imports.call_bar(&mut store)?;
        no_imports.foo().call_foo(&mut store)?;
        Ok(())
    }
}

mod one_import {
    use super::*;

    wasmtime::component::bindgen!({
        inline: "
            default world one-import {
                import foo: interface {
                    foo: func()
                }

                export bar: func()
            }
        ",
    });

    #[test]
    fn run() -> Result<()> {
        let engine = engine();

        let component = Component::new(
            &engine,
            r#"
                (component
                    (import "foo" (instance $i
                        (export "foo" (func))
                    ))
                    (core module $m
                        (import "" "" (func))
                        (export "" (func 0))
                    )
                    (core func $f (canon lower (func $i "foo")))
                    (core instance $i (instantiate $m
                        (with "" (instance (export "" (func $f))))
                    ))

                    (func $f (export "bar") (canon lift (core func $i "")))
                )
            "#,
        )?;

        #[derive(Default)]
        struct MyImports {
            hit: bool,
        }

        impl foo::Host for MyImports {
            fn foo(&mut self) -> Result<()> {
                self.hit = true;
                Ok(())
            }
        }

        let mut linker = Linker::new(&engine);
        foo::add_to_linker(&mut linker, &component, |f: &mut MyImports| f)?;
        let mut store = Store::new(&engine, MyImports::default());
        let (one_import, _) = OneImport::instantiate(&mut store, &component, &linker)?;
        one_import.call_bar(&mut store)?;
        assert!(store.data().hit);
        Ok(())
    }
}

mod wildcards {
    use super::*;

    wasmtime::component::bindgen!({
        inline: "
            default world wildcards {
                import imports: interface {
                    *: func() -> u32
                }
                export exports: interface {
                    *: func() -> u32
                }
            }
        ",
    });

    #[test]
    fn run() -> Result<()> {
        let engine = engine();

        let component = Component::new(
            &engine,
            r#"
                (component
                    (import "imports" (instance $i
                        (export "a" (func (result u32)))
                        (export "b" (func (result u32)))
                        (export "c" (func (result u32)))
                    ))
                    (core module $m
                        (import "" "a" (func (result i32)))
                        (import "" "b" (func (result i32)))
                        (import "" "c" (func (result i32)))
                        (export "x" (func 0))
                        (export "y" (func 1))
                        (export "z" (func 2))
                    )
                    (core func $a (canon lower (func $i "a")))
                    (core func $b (canon lower (func $i "b")))
                    (core func $c (canon lower (func $i "c")))
                    (core instance $j (instantiate $m
                        (with "" (instance
                            (export "a" (func $a))
                            (export "b" (func $b))
                            (export "c" (func $c))
                        ))
                    ))
                    (func $x (result u32) (canon lift (core func $j "x")))
                    (func $y (result u32) (canon lift (core func $j "y")))
                    (func $z (export "z") (result u32) (canon lift (core func $j "z")))
                    (instance $k
                       (export "x" (func $x))
                       (export "y" (func $y))
                       (export "z" (func $z))
                    )
                    (export "exports" (instance $k))
                )
            "#,
        )?;

        #[derive(Default)]
        struct MyImports;

        impl imports::Host for MyImports {
            fn call(&mut self, name: &str) -> Result<u32> {
                Ok(match name {
                    "a" => 42,
                    "b" => 43,
                    "c" => 44,
                    _ => bail!("unexpected function name: \"{name}\""),
                })
            }
        }

        let mut linker = Linker::new(&engine);
        imports::add_to_linker(&mut linker, &component, |f: &mut MyImports| f)?;
        let mut store = Store::new(&engine, MyImports::default());
        let (wildcards, _) = Wildcards::instantiate(&mut store, &component, &linker)?;
        assert_eq!(42, wildcards.exports.call("x", &mut store)?);
        assert_eq!(43, wildcards.exports.call("y", &mut store)?);
        assert_eq!(44, wildcards.exports.call("z", &mut store)?);
        Ok(())
    }
}
