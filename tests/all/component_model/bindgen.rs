#![cfg(not(miri))]

use super::engine;
use anyhow::Result;
use wasmtime::{
    component::{Component, Linker},
    Store,
};

mod ownership;
mod results;

mod no_imports {
    use super::*;

    wasmtime::component::bindgen!({
        inline: "
            package foo:foo

            world no-imports {
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
            package foo:foo

            world one-import {
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
        foo::add_to_linker(&mut linker, |f: &mut MyImports| f)?;
        let mut store = Store::new(&engine, MyImports::default());
        let (one_import, _) = OneImport::instantiate(&mut store, &component, &linker)?;
        one_import.call_bar(&mut store)?;
        assert!(store.data().hit);
        Ok(())
    }
}

mod resources_at_world_level {
    use super::*;
    use wasmtime::component::Resource;

    wasmtime::component::bindgen!({
        inline: "
            package foo:foo

            world resources {
                resource x {
                    constructor()
                }

                export y: func(x: x)
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
                    (import "x" (type $x (sub resource)))
                    (import "[constructor]x" (func $ctor (result (own $x))))

                    (core func $dtor (canon resource.drop $x))
                    (core func $ctor (canon lower (func $ctor)))

                    (core module $m
                        (import "" "ctor" (func $ctor (result i32)))
                        (import "" "dtor" (func $dtor (param i32)))

                        (func (export "x") (param i32)
                            (call $dtor (local.get 0))
                            (call $dtor (call $ctor))
                        )
                    )
                    (core instance $i (instantiate $m
                        (with "" (instance
                            (export "ctor" (func $ctor))
                            (export "dtor" (func $dtor))
                        ))
                    ))
                    (func (export "y") (param "x" (own $x))
                        (canon lift (core func $i "x")))
                )
            "#,
        )?;

        #[derive(Default)]
        struct MyImports {
            ctor_hit: bool,
            drops: usize,
        }

        impl HostX for MyImports {
            fn new(&mut self) -> Result<Resource<X>> {
                self.ctor_hit = true;
                Ok(Resource::new_own(80))
            }

            fn drop(&mut self, val: Resource<X>) -> Result<()> {
                match self.drops {
                    0 => assert_eq!(val.rep(), 40),
                    1 => assert_eq!(val.rep(), 80),
                    _ => unreachable!(),
                }
                self.drops += 1;
                Ok(())
            }
        }

        impl ResourcesImports for MyImports {}

        let mut linker = Linker::new(&engine);
        Resources::add_to_linker(&mut linker, |f: &mut MyImports| f)?;
        let mut store = Store::new(&engine, MyImports::default());
        let (one_import, _) = Resources::instantiate(&mut store, &component, &linker)?;
        one_import.call_y(&mut store, Resource::new_own(40))?;
        assert!(store.data().ctor_hit);
        assert_eq!(store.data().drops, 2);
        Ok(())
    }
}

mod resources_at_interface_level {
    use super::*;
    use wasmtime::component::Resource;

    wasmtime::component::bindgen!({
        inline: "
            package foo:foo

            interface def {
                resource x {
                    constructor()
                }
            }

            interface user {
                use def.{x}

                y: func(x: x)
            }

            world resources {
                export user
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
                    (import (interface "foo:foo/def") (instance $i
                        (export $x "x" (type (sub resource)))
                        (export "[constructor]x" (func (result (own $x))))
                    ))
                    (alias export $i "x" (type $x))
                    (core func $dtor (canon resource.drop $x))
                    (core func $ctor (canon lower (func $i "[constructor]x")))

                    (core module $m
                        (import "" "ctor" (func $ctor (result i32)))
                        (import "" "dtor" (func $dtor (param i32)))

                        (func (export "x") (param i32)
                            (call $dtor (local.get 0))
                            (call $dtor (call $ctor))
                        )
                    )
                    (core instance $i (instantiate $m
                        (with "" (instance
                            (export "ctor" (func $ctor))
                            (export "dtor" (func $dtor))
                        ))
                    ))
                    (func $y (param "x" (own $x))
                        (canon lift (core func $i "x")))

                    (instance (export (interface "foo:foo/user"))
                        (export "y" (func $y))
                    )
                )
            "#,
        )?;

        #[derive(Default)]
        struct MyImports {
            ctor_hit: bool,
            drops: usize,
        }

        use foo::foo::def::X;

        impl foo::foo::def::HostX for MyImports {
            fn new(&mut self) -> Result<Resource<X>> {
                self.ctor_hit = true;
                Ok(Resource::new_own(80))
            }

            fn drop(&mut self, val: Resource<X>) -> Result<()> {
                match self.drops {
                    0 => assert_eq!(val.rep(), 40),
                    1 => assert_eq!(val.rep(), 80),
                    _ => unreachable!(),
                }
                self.drops += 1;
                Ok(())
            }
        }

        impl foo::foo::def::Host for MyImports {}

        let mut linker = Linker::new(&engine);
        Resources::add_to_linker(&mut linker, |f: &mut MyImports| f)?;
        let mut store = Store::new(&engine, MyImports::default());
        let (one_import, _) = Resources::instantiate(&mut store, &component, &linker)?;
        one_import
            .foo_foo_user()
            .call_y(&mut store, Resource::new_own(40))?;
        assert!(store.data().ctor_hit);
        assert_eq!(store.data().drops, 2);
        Ok(())
    }
}

mod async_config {
    use super::*;

    wasmtime::component::bindgen!({
        inline: "
            package foo:foo

            world t1 {
                import x: func()
                import y: func()
                export z: func()
            }
        ",
        async: true,
    });

    struct T;

    #[async_trait::async_trait]
    impl T1Imports for T {
        async fn x(&mut self) -> Result<()> {
            Ok(())
        }

        async fn y(&mut self) -> Result<()> {
            Ok(())
        }
    }

    async fn _test_t1(t1: &T1, store: &mut Store<()>) {
        let _ = t1.call_z(&mut *store).await;
    }

    wasmtime::component::bindgen!({
        inline: "
            package foo:foo

            world t2 {
                import x: func()
                import y: func()
                export z: func()
            }
        ",
        async: {
            except_imports: ["x"],
        },
    });

    #[async_trait::async_trait]
    impl T2Imports for T {
        fn x(&mut self) -> Result<()> {
            Ok(())
        }

        async fn y(&mut self) -> Result<()> {
            Ok(())
        }
    }

    async fn _test_t2(t2: &T2, store: &mut Store<()>) {
        let _ = t2.call_z(&mut *store).await;
    }

    wasmtime::component::bindgen!({
        inline: "
            package foo:foo

            world t3 {
                import x: func()
                import y: func()
                export z: func()
            }
        ",
        async: {
            only_imports: ["x"],
        },
    });

    #[async_trait::async_trait]
    impl T3Imports for T {
        async fn x(&mut self) -> Result<()> {
            Ok(())
        }

        fn y(&mut self) -> Result<()> {
            Ok(())
        }
    }

    async fn _test_t3(t3: &T3, store: &mut Store<()>) {
        let _ = t3.call_z(&mut *store).await;
    }
}
