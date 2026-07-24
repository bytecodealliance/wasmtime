#![cfg(not(miri))]

use super::engine;
use wasmtime::Result;
use wasmtime::{
    Config, Engine, Store,
    component::{Component, Linker},
};

mod ownership;
mod results;

mod no_imports {
    use super::*;
    use std::rc::Rc;

    wasmtime::component::bindgen!({
        inline: "
            package foo:foo;

            world no-imports {
                export foo: interface {
                    foo: func();
                }

                export bar: func();
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
        let no_imports = NoImports::instantiate(&mut store, &component, &linker)?;
        no_imports.call_bar(&mut store)?;
        no_imports.foo().call_foo(&mut store)?;

        let linker = Linker::new(&engine);
        let mut non_send_store = Store::new(&engine, Rc::new(()));
        let no_imports = NoImports::instantiate(&mut non_send_store, &component, &linker)?;
        no_imports.call_bar(&mut non_send_store)?;
        no_imports.foo().call_foo(&mut non_send_store)?;
        Ok(())
    }
}

mod no_imports_concurrent {
    use super::*;
    use futures::{
        FutureExt,
        stream::{FuturesUnordered, TryStreamExt},
    };

    wasmtime::component::bindgen!({
        inline: "
            package foo:foo;

            world no-imports {
                export foo: interface {
                    foo: async func();
                }

                export bar: async func();
            }
        ",
    });

    #[tokio::test]
    async fn run() -> Result<()> {
        let mut config = Config::new();
        config.wasm_component_model_async(true);
        let engine = &Engine::new(&config)?;

        let component = Component::new(
            &engine,
            r#"
                (component
                    (core module $m
                        (import "" "task.return" (func $task-return))
                        (func (export "bar") (result i32)
                            call $task-return
                            i32.const 0
                        )
                        (func (export "callback") (param i32 i32 i32) (result i32) unreachable)
                    )
                    (core func $task-return (canon task.return))
                    (core instance $i (instantiate $m
                        (with "" (instance (export "task.return" (func $task-return))))
                    ))

                    (func $f (export "bar") async
                        (canon lift (core func $i "bar") async (callback (core func $i "callback")))
                    )

                    (instance $i (export "foo" (func $f)))
                    (export "foo" (instance $i))
                )
            "#,
        )?;

        let linker = Linker::new(&engine);
        let mut store = Store::new(&engine, ());
        let no_imports = NoImports::instantiate_async(&mut store, &component, &linker).await?;
        store
            .run_concurrent(async move |accessor| {
                let mut futures = FuturesUnordered::new();
                futures.push(no_imports.call_bar(accessor).boxed());
                futures.push(no_imports.foo().call_foo(accessor).boxed());
                assert!(futures.try_next().await?.is_some());
                assert!(futures.try_next().await?.is_some());
                Ok(())
            })
            .await?
    }
}

mod one_import {
    use super::*;
    use wasmtime::component::HasSelf;

    wasmtime::component::bindgen!({
        inline: "
            package foo:foo;

            world one-import {
                import foo: interface {
                    foo: func();
                }

                export bar: func();
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
            fn foo(&mut self) {
                self.hit = true;
            }
        }

        let mut linker = Linker::new(&engine);
        foo::add_to_linker::<_, HasSelf<_>>(&mut linker, |f| f)?;
        let mut store = Store::new(&engine, MyImports::default());
        let one_import = OneImport::instantiate(&mut store, &component, &linker)?;
        one_import.call_bar(&mut store)?;
        assert!(store.data().hit);
        Ok(())
    }
}

mod one_import_concurrent {
    use super::*;
    use wasmtime::component::{Accessor, HasData};

    wasmtime::component::bindgen!({
        inline: "
            package foo:foo;

            world no-imports {
                import foo: interface {
                    foo: async func();
                }

                export bar: async func();
            }
        "
    });

    #[tokio::test]
    async fn run() -> Result<()> {
        let mut config = Config::new();
        config.wasm_component_model_async(true);
        let engine = &Engine::new(&config)?;

        let component = Component::new(
            &engine,
            r#"
                (component
                    (import "foo" (instance $foo-instance
                        (export "foo" (func async))
                    ))
                    (core module $libc
                        (memory (export "memory") 1)
                    )
                    (core instance $libc-instance (instantiate $libc))
                    (core module $m
                        (import "" "foo" (func $foo (param) (result i32)))
                        (import "" "task.return" (func $task-return))
                        (func (export "bar") (result i32)
                            call $foo
                            drop
                            call $task-return
                            i32.const 0
                        )
                        (func (export "callback") (param i32 i32 i32) (result i32) unreachable)
                    )
                    (core func $foo (canon lower (func $foo-instance "foo") async (memory (core memory $libc-instance "memory"))))
                    (core func $task-return (canon task.return))
                    (core instance $i (instantiate $m
                        (with "" (instance
                            (export "task.return" (func $task-return))
                            (export "foo" (func $foo))
                        ))
                    ))

                    (func $f (export "bar") async
                        (canon lift (core func $i "bar") async (callback (core func $i "callback")))
                    )

                    (instance $i (export "foo" (func $f)))
                    (export "foo" (instance $i))
                )
            "#,
        )?;

        #[derive(Default)]
        struct MyImports {
            hit: bool,
        }

        impl HasData for MyImports {
            type Data<'a> = &'a mut MyImports;
        }

        impl<T> foo::HostWithStore<T> for MyImports {
            async fn foo(accessor: &Accessor<T, Self>) {
                accessor.with(|mut view| view.get().hit = true);
            }
        }

        impl foo::Host for MyImports {}

        let mut linker = Linker::new(&engine);
        foo::add_to_linker::<_, MyImports>(&mut linker, |x| x)?;
        let mut store = Store::new(&engine, MyImports::default());
        let no_imports = NoImports::instantiate_async(&mut store, &component, &linker).await?;
        store
            .run_concurrent(async move |accessor| no_imports.call_bar(accessor).await)
            .await??;
        assert!(store.data().hit);
        Ok(())
    }
}

mod resources_at_world_level {
    use super::*;
    use wasmtime::component::{HasSelf, Resource};

    wasmtime::component::bindgen!({
        inline: "
            package foo:foo;

            world resources {
                resource x {
                    constructor();
                }

                export y: func(x: x);
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
            fn new(&mut self) -> Resource<X> {
                self.ctor_hit = true;
                Resource::new_own(80)
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
        Resources::add_to_linker::<_, HasSelf<_>>(&mut linker, |f| f)?;
        let mut store = Store::new(&engine, MyImports::default());
        let one_import = Resources::instantiate(&mut store, &component, &linker)?;
        one_import.call_y(&mut store, Resource::new_own(40))?;
        assert!(store.data().ctor_hit);
        assert_eq!(store.data().drops, 2);
        Ok(())
    }
}

mod resources_at_interface_level {
    use super::*;
    use wasmtime::component::{HasSelf, Resource};

    wasmtime::component::bindgen!({
        inline: "
            package foo:foo;

            interface def {
                resource x {
                    constructor();
                }
            }

            interface user {
                use def.{x};

                y: func(x: x);
            }

            world resources {
                export user;
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
                        (export "x" (type $x (sub resource)))
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
            fn new(&mut self) -> Resource<X> {
                self.ctor_hit = true;
                Resource::new_own(80)
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
        Resources::add_to_linker::<_, HasSelf<_>>(&mut linker, |f| f)?;
        let mut store = Store::new(&engine, MyImports::default());
        let one_import = Resources::instantiate(&mut store, &component, &linker)?;
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
            package foo:foo;

            world t1 {
                import foo: interface {
                    foo: func();
                }
                import x: func();
                import y: func();
                export z: func();
            }
        ",
        imports: { default: async },
        exports: { default: async },
    });

    #[expect(dead_code, reason = "just here for bindings")]
    struct T;

    impl T1Imports for T {
        async fn x(&mut self) {}

        async fn y(&mut self) {}
    }

    async fn _test_t1(t1: &T1, store: &mut Store<()>) {
        let _ = t1.call_z(&mut *store).await;
    }

    wasmtime::component::bindgen!({
        inline: "
            package foo:foo;

            world t2 {
                import x: func();
                import y: func();
                export z: func();
            }
        ",
        imports: {
            "x": tracing,
            default: async,
        },
        exports: { default: async },
    });

    impl T2Imports for T {
        fn x(&mut self) {}

        async fn y(&mut self) {}
    }

    async fn _test_t2(t2: &T2, store: &mut Store<()>) {
        let _ = t2.call_z(&mut *store).await;
    }

    wasmtime::component::bindgen!({
        inline: "
            package foo:foo;

            world t3 {
                import x: func();
                import y: func();
                export z: func();
            }
        ",
        imports: { "x": async },
        exports: { default: async },
    });

    impl T3Imports for T {
        async fn x(&mut self) {}

        fn y(&mut self) {}
    }

    async fn _test_t3(t3: &T3, store: &mut Store<()>) {
        let _ = t3.call_z(&mut *store).await;
    }
}

mod exported_resources {
    use super::*;
    use std::mem;
    use wasmtime::component::{HasSelf, Resource};

    wasmtime::component::bindgen!({
        inline: "
            package foo:foo;

            interface a {
                resource x {
                    constructor();
                }
            }

            world resources {
                export b: interface {
                    use a.{x as y};

                    resource x {
                        constructor(y: y);
                        foo: func() -> u32;
                    }
                }

                resource x;

                export f: func(x1: x, x2: x) -> x;
            }
        ",
    });

    #[derive(Default)]
    struct MyImports {
        hostcalls: Vec<Hostcall>,
        next_a_x: u32,
    }

    #[derive(PartialEq, Debug)]
    enum Hostcall {
        DropRootX(u32),
        DropAX(u32),
        NewA,
    }

    use foo::foo::a;

    impl ResourcesImports for MyImports {}

    impl HostX for MyImports {
        fn drop(&mut self, val: Resource<X>) -> Result<()> {
            self.hostcalls.push(Hostcall::DropRootX(val.rep()));
            Ok(())
        }
    }

    impl a::HostX for MyImports {
        fn new(&mut self) -> Resource<a::X> {
            let rep = self.next_a_x;
            self.next_a_x += 1;
            self.hostcalls.push(Hostcall::NewA);
            Resource::new_own(rep)
        }

        fn drop(&mut self, val: Resource<a::X>) -> Result<()> {
            self.hostcalls.push(Hostcall::DropAX(val.rep()));
            Ok(())
        }
    }

    impl foo::foo::a::Host for MyImports {}

    #[test]
    fn run() -> Result<()> {
        let engine = engine();

        let component = Component::new(
            &engine,
            r#"
(component
  ;; setup the `foo:foo/a` import
  (import (interface "foo:foo/a") (instance $a
    (export "x" (type $x (sub resource)))
    (export "[constructor]x" (func (result (own $x))))
  ))
  (alias export $a "x" (type $a-x))
  (core func $a-x-drop (canon resource.drop $a-x))
  (core func $a-x-ctor (canon lower (func $a "[constructor]x")))

  ;; setup the root import of the `x` resource
  (import "x" (type $x (sub resource)))
  (core func $root-x-dtor (canon resource.drop $x))

  ;; setup and declare the `x` resource for the `b` export.
  (core module $indirect-dtor
    (func (export "b-x-dtor") (param i32)
      local.get 0
      i32.const 0
      call_indirect (param i32)
    )
    (table (export "$imports") 1 1 funcref)
  )
  (core instance $indirect-dtor (instantiate $indirect-dtor))
  (type $b-x (resource (rep i32) (dtor (core func $indirect-dtor "b-x-dtor"))))
  (core func $b-x-drop (canon resource.drop $b-x))
  (core func $b-x-rep (canon resource.rep $b-x))
  (core func $b-x-new (canon resource.new $b-x))

  ;; main module implementation
  (core module $main
    (import "foo:foo/a" "[constructor]x" (func $a-x-ctor (result i32)))
    (import "foo:foo/a" "[resource-drop]x" (func $a-x-dtor (param i32)))
    (import "$root" "[resource-drop]x" (func $x-dtor (param i32)))
    (import "[export]b" "[resource-drop]x" (func $b-x-dtor (param i32)))
    (import "[export]b" "[resource-new]x" (func $b-x-new (param i32) (result i32)))
    (import "[export]b" "[resource-rep]x" (func $b-x-rep (param i32) (result i32)))
    (func (export "b#[constructor]x") (param i32) (result i32)
      (call $a-x-dtor (local.get 0))
      (call $b-x-new (call $a-x-ctor))
    )
    (func (export "b#[method]x.foo") (param i32) (result i32)
      local.get 0)
    (func (export "b#[dtor]x") (param i32)
      (call $a-x-dtor (local.get 0))
    )
    (func (export "f") (param i32 i32) (result i32)
      (call $x-dtor (local.get 0))
      local.get 1
    )
  )
  (core instance $main (instantiate $main
    (with "foo:foo/a" (instance
      (export "[resource-drop]x" (func $a-x-drop))
      (export "[constructor]x" (func $a-x-ctor))
    ))
    (with "$root" (instance
      (export "[resource-drop]x" (func $root-x-dtor))
    ))
    (with "[export]b" (instance
      (export "[resource-drop]x" (func $b-x-drop))
      (export "[resource-rep]x" (func $b-x-rep))
      (export "[resource-new]x" (func $b-x-new))
    ))
  ))

  ;; fill in `$indirect-dtor`'s table with the actual destructor definition
  ;; now that it's available.
  (core module $fixup
    (import "" "b-x-dtor" (func $b-x-dtor (param i32)))
    (import "" "$imports" (table 1 1 funcref))
    (elem (i32.const 0) func $b-x-dtor)
  )
  (core instance (instantiate $fixup
    (with "" (instance
      (export "$imports" (table 0 "$imports"))
      (export "b-x-dtor" (func $main "b#[dtor]x"))
    ))
  ))

  ;; Create the `b` export through a subcomponent instantiation.
  (func $b-x-ctor (param "y" (own $a-x)) (result (own $b-x))
    (canon lift (core func $main "b#[constructor]x")))
  (func $b-x-foo (param "self" (borrow $b-x)) (result u32)
    (canon lift (core func $main "b#[method]x.foo")))
  (component $b
    (import "a-x" (type $y (sub resource)))
    (import "b-x" (type $x' (sub resource)))
    (import "ctor" (func $ctor (param "y" (own $y)) (result (own $x'))))
    (import "foo" (func $foo (param "self" (borrow $x')) (result u32)))
    (export $x "x" (type $x'))
    (export "[constructor]x"
      (func $ctor)
      (func (param "y" (own $y)) (result (own $x))))
    (export "[method]x.foo"
      (func $foo)
      (func (param "self" (borrow $x)) (result u32)))
  )
  (instance (export "b") (instantiate $b
    (with "ctor" (func $b-x-ctor))
    (with "foo" (func $b-x-foo))
    (with "a-x" (type 0 "x"))
    (with "b-x" (type $b-x))
  ))

  ;; Create the `f` export which is a bare function
  (func (export "f") (param "x1" (own $x)) (param "x2" (own $x)) (result (own $x))
    (canon lift (core func $main "f")))
)
            "#,
        )?;

        let mut linker = Linker::new(&engine);
        Resources::add_to_linker::<_, HasSelf<_>>(&mut linker, |f| f)?;
        let mut store = Store::new(&engine, MyImports::default());
        let i = Resources::instantiate(&mut store, &component, &linker)?;

        // call the root export `f` twice
        let ret = i.call_f(&mut store, Resource::new_own(1), Resource::new_own(2))?;
        assert_eq!(ret.rep(), 2);
        assert_eq!(
            mem::take(&mut store.data_mut().hostcalls),
            [Hostcall::DropRootX(1)]
        );
        let ret = i.call_f(&mut store, Resource::new_own(3), Resource::new_own(4))?;
        assert_eq!(ret.rep(), 4);
        assert_eq!(
            mem::take(&mut store.data_mut().hostcalls),
            [Hostcall::DropRootX(3)]
        );

        // interact with the `b` export
        let b = i.b();
        let b_x = b.x().call_constructor(&mut store, Resource::new_own(5))?;
        assert_eq!(
            mem::take(&mut store.data_mut().hostcalls),
            [Hostcall::DropAX(5), Hostcall::NewA]
        );
        b.x().call_foo(&mut store, b_x)?;
        assert_eq!(mem::take(&mut store.data_mut().hostcalls), []);
        b_x.resource_drop(&mut store)?;
        assert_eq!(
            mem::take(&mut store.data_mut().hostcalls),
            [Hostcall::DropAX(0)],
        );
        Ok(())
    }
}

mod unstable_import {
    use super::*;
    use wasmtime::component::HasSelf;

    wasmtime::component::bindgen!({
        inline: "
            package foo:foo;

            @unstable(feature = experimental-interface)
            interface my-interface {
                @unstable(feature = experimental-function)
                my-function: func();
            }

            world my-world {
                @unstable(feature = experimental-import)
                import my-interface;

                export bar: func();
            }
        ",
    });

    #[test]
    fn run() -> Result<()> {
        // In the example above, all features are required for `my-function` to be imported:
        assert_success(
            LinkOptions::default()
                .experimental_interface(true)
                .experimental_import(true)
                .experimental_function(true),
        );

        // And every other incomplete combination should fail:
        assert_failure(&LinkOptions::default());
        assert_failure(LinkOptions::default().experimental_function(true));
        assert_failure(LinkOptions::default().experimental_interface(true));
        assert_failure(
            LinkOptions::default()
                .experimental_interface(true)
                .experimental_function(true),
        );
        assert_failure(
            LinkOptions::default()
                .experimental_interface(true)
                .experimental_import(true),
        );
        assert_failure(LinkOptions::default().experimental_import(true));
        assert_failure(
            LinkOptions::default()
                .experimental_import(true)
                .experimental_function(true),
        );

        Ok(())
    }

    fn assert_success(link_options: &LinkOptions) {
        run_with_options(link_options).unwrap();
    }
    fn assert_failure(link_options: &LinkOptions) {
        let err = run_with_options(link_options).unwrap_err().to_string();
        assert_eq!(
            err,
            "component imports instance `foo:foo/my-interface`, but a matching implementation was not found in the linker"
        );
    }

    fn run_with_options(link_options: &LinkOptions) -> Result<()> {
        let engine = engine();

        let component = Component::new(
            &engine,
            r#"
                (component
                    (import "foo:foo/my-interface" (instance $i
                        (export "my-function" (func))
                    ))
                    (core module $m
                        (import "" "" (func))
                        (export "" (func 0))
                    )
                    (core func $f (canon lower (func $i "my-function")))
                    (core instance $r (instantiate $m
                        (with "" (instance (export "" (func $f))))
                    ))

                    (func $f (export "bar") (canon lift (core func $r "")))
                )
            "#,
        )?;

        #[derive(Default)]
        struct MyHost;

        impl foo::foo::my_interface::Host for MyHost {
            fn my_function(&mut self) {}
        }

        let mut linker = Linker::new(&engine);
        MyWorld::add_to_linker::<_, HasSelf<_>>(&mut linker, link_options, |h| h)?;
        let mut store = Store::new(&engine, MyHost::default());
        let one_import = MyWorld::instantiate(&mut store, &component, &linker)?;
        one_import.call_bar(&mut store)?;
        Ok(())
    }
}

mod anyhow_errors {
    use super::*;
    use crate::ErrorExt;
    use wasmtime::component::HasSelf;
    use wasmtime::error::Context as _;

    wasmtime::component::bindgen!({
        anyhow: true,
        imports: { default: trappable },
        inline: "
            package foo:foo;

            interface my-interface {
                ok: func() -> u32;
                trap: func() -> u32;
            }

            world my-world {
                import my-interface;
                export ok: func() -> u32;
                export trap: func() -> u32;
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
                    (import "foo:foo/my-interface" (instance $i
                        (export "ok" (func (result u32)))
                        (export "trap" (func (result u32)))
                    ))

                    (core module $m
                        (import "" "ok" (func (result i32)))
                        (import "" "trap" (func (result i32)))
                        (export "ok" (func 0))
                        (export "trap" (func 1))
                    )

                    (core func $ok (canon lower (func $i "ok")))
                    (core func $trap (canon lower (func $i "trap")))

                    (core instance $r (instantiate $m
                        (with "" (instance (export "ok" (func $ok))
                                           (export "trap" (func $trap))))
                    ))

                    (func (export "ok") (result u32) (canon lift (core func $r "ok")))
                    (func (export "trap") (result u32) (canon lift (core func $r "trap")))
                )
            "#,
        )?;

        #[derive(Default)]
        struct MyHost;

        impl foo::foo::my_interface::Host for MyHost {
            // NB: these must return an `anyhow::Result` since we `bindgen!`ed
            // with `anyhow: true`.
            fn ok(&mut self) -> anyhow_for_testing::Result<u32> {
                Ok(42)
            }
            fn trap(&mut self) -> anyhow_for_testing::Result<u32> {
                anyhow_for_testing::bail!("anyhow error")
            }
        }

        let mut linker = Linker::new(&engine);
        MyWorld::add_to_linker::<_, HasSelf<_>>(&mut linker, |h| h)
            .context("failed to add to linker")?;
        let mut store = Store::new(&engine, MyHost::default());
        let instance = MyWorld::instantiate(&mut store, &component, &linker)
            .context("failed to instantiate")?;

        let x = instance
            .call_ok(&mut store)
            .context("failed to call `ok` function")?;
        assert_eq!(x, 42);

        let result = instance.call_trap(&mut store);
        let error = result.unwrap_err();
        error.assert_contains("anyhow error");

        Ok(())
    }
}

mod implements {
    use super::*;
    use std::collections::HashMap;
    use wasmtime::component::HasSelf;

    wasmtime::component::bindgen!({
        inline: "
            package demo:pkg;

            interface store {
                get: func(key: u32) -> u32;
                set: func(key: u32, value: u32);
            }

            world cache {
                import hot-cache: store;
                import durable: store;

                export run: func();
            }
        ",
    });

    const DUMMY: &str = r#"
        (component
            (import "hot-cache" (instance $hot
                (export "get" (func (param "key" u32) (result u32)))
                (export "set" (func (param "key" u32) (param "value" u32)))
            ))
            (import "durable" (instance $dur
                (export "get" (func (param "key" u32) (result u32)))
                (export "set" (func (param "key" u32) (param "value" u32)))
            ))

            (core module $m
                (import "hot" "get" (func $hot-get (param i32) (result i32)))
                (import "hot" "set" (func $hot-set (param i32 i32)))
                (import "durable" "get" (func $durable-get (param i32) (result i32)))
                (import "durable" "set" (func $durable-set (param i32 i32)))

                (func (export "run")
                    (call $durable-set (i32.const 0) (i32.const 1))
                    (call $hot-set (i32.const 0) (i32.const 2))

                    (if (i32.ne (call $durable-get (i32.const 0)) (i32.const 1))
                        (then unreachable))
                    (if (i32.ne (call $hot-get (i32.const 0)) (i32.const 2))
                        (then unreachable))
                )
            )
            (core func $hot-get (canon lower (func $hot "get")))
            (core func $hot-set (canon lower (func $hot "set")))
            (core func $dur-get (canon lower (func $dur "get")))
            (core func $dur-set (canon lower (func $dur "set")))
            (core instance $i (instantiate $m
                (with "hot" (instance
                    (export "get" (func $hot-get))
                    (export "set" (func $hot-set))
                ))
                (with "durable" (instance
                    (export "get" (func $dur-get))
                    (export "set" (func $dur-set))
                ))
            ))

            (func (export "run") (canon lift (core func $i "run")))
        )
    "#;

    #[test]
    fn add_to_linker_can_instantiate() -> Result<()> {
        #[derive(Default)]
        struct MyHost {
            kv: HashMap<u32, u32>,
        }

        impl demo::pkg::store::Host for MyHost {
            fn get(&mut self, key: u32) -> u32 {
                *self.kv.get(&key).unwrap()
            }

            fn set(&mut self, key: u32, value: u32) {
                self.kv.insert(key, value);
            }
        }

        let engine = engine();
        let mut linker = Linker::new(&engine);
        Cache::add_to_linker::<_, HasSelf<MyHost>>(&mut linker, |s| s)?;
        let component = Component::new(&engine, DUMMY)?;
        let mut store = Store::new(&engine, MyHost::default());
        let instance = Cache::instantiate(&mut store, &component, &linker)?;

        // This is expected to fail since both interfaces are routed to the same
        // location, not different ones.
        assert!(instance.call_run(&mut store).is_err());
        Ok(())
    }

    impl demo::pkg::store::Host for HashMap<u32, u32> {
        fn get(&mut self, key: u32) -> u32 {
            *HashMap::get(self, &key).unwrap()
        }

        fn set(&mut self, key: u32, value: u32) {
            self.insert(key, value);
        }
    }

    #[test]
    fn add_to_linker_instance_can_instantiate() -> Result<()> {
        #[derive(Default)]
        struct MyHost {
            hot_cache: HashMap<u32, u32>,
            durable: HashMap<u32, u32>,
        }

        let engine = engine();
        let mut linker = Linker::new(&engine);
        demo::pkg::store::add_to_linker_instance::<MyHost, HasSelf<HashMap<u32, u32>>>(
            &mut linker.instance("hot-cache")?,
            |s| &mut s.hot_cache,
        )?;
        demo::pkg::store::add_to_linker_instance::<MyHost, HasSelf<HashMap<u32, u32>>>(
            &mut linker.instance("durable")?,
            |s| &mut s.durable,
        )?;
        let component = Component::new(&engine, DUMMY)?;
        let mut store = Store::new(&engine, MyHost::default());
        let instance = Cache::instantiate(&mut store, &component, &linker)?;
        instance.call_run(&mut store)?;
        Ok(())
    }
}

mod named_imports {
    use super::*;
    use std::collections::HashMap;
    use wasmtime::component::HasSelf;

    /// Host-chosen id type threaded into every method call.
    #[derive(Clone)]
    pub struct MyId(u32);

    wasmtime::component::bindgen!({
        inline: "
            package demo:pkg;

            interface store {
                get: func(key: u32) -> u32;
                set: func(key: u32, value: u32);
            }

            world cache {
                import store;

                export run: func();
            }
        ",
        named_imports: {
            "demo:pkg/store": MyId,
        },
    });

    // A component which imports the `store` interface twice, under arbitrary
    // names `a` and `b`, each annotated as implementing `demo:pkg/store`. The
    // exported `run` writes through each import and reads the value back, so a
    // mix-up in id routing would cause a trap.
    const COMPONENT: &str = r#"
        (component
            (import "a" (implements "demo:pkg/store") (instance $a
                (export "get" (func (param "key" u32) (result u32)))
                (export "set" (func (param "key" u32) (param "value" u32)))
            ))
            (import "b" (implements "demo:pkg/store") (instance $b
                (export "get" (func (param "key" u32) (result u32)))
                (export "set" (func (param "key" u32) (param "value" u32)))
            ))

            (core module $m
                (import "a" "get" (func $a-get (param i32) (result i32)))
                (import "a" "set" (func $a-set (param i32 i32)))
                (import "b" "get" (func $b-get (param i32) (result i32)))
                (import "b" "set" (func $b-set (param i32 i32)))

                (func (export "run")
                    (call $a-set (i32.const 0) (i32.const 10))
                    (call $b-set (i32.const 0) (i32.const 20))

                    (if (i32.ne (call $a-get (i32.const 0)) (i32.const 10))
                        (then unreachable))
                    (if (i32.ne (call $b-get (i32.const 0)) (i32.const 20))
                        (then unreachable))
                )
            )
            (core func $a-get-l (canon lower (func $a "get")))
            (core func $a-set-l (canon lower (func $a "set")))
            (core func $b-get-l (canon lower (func $b "get")))
            (core func $b-set-l (canon lower (func $b "set")))
            (core instance $i (instantiate $m
                (with "a" (instance
                    (export "get" (func $a-get-l))
                    (export "set" (func $a-set-l))
                ))
                (with "b" (instance
                    (export "get" (func $b-get-l))
                    (export "set" (func $b-set-l))
                ))
            ))

            (func (export "run") (canon lift (core func $i "run")))
        )
    "#;

    fn implements_engine() -> Engine {
        let mut config = Config::new();
        config.wasm_component_model(true);
        config.wasm_component_model_implements(true);
        Engine::new(&config).unwrap()
    }

    #[derive(Default)]
    struct MyHost {
        kv: HashMap<(u32, u32), u32>,
        calls: Vec<(u32, &'static str)>,
    }

    impl named_imports::demo::pkg::store::Host for MyHost {
        fn get(&mut self, id: MyId, key: u32) -> u32 {
            self.calls.push((id.0, "get"));
            *self.kv.get(&(id.0, key)).unwrap()
        }

        fn set(&mut self, id: MyId, key: u32, value: u32) {
            self.calls.push((id.0, "set"));
            self.kv.insert((id.0, key), value);
        }
    }

    #[test]
    fn ids_are_threaded_through() -> Result<()> {
        let engine = implements_engine();
        let component = Component::new(&engine, COMPONENT)?;
        let mut linker = Linker::new(&engine);
        named_imports::demo::pkg::store::add_to_linker::<_, HasSelf<MyHost>>(
            &mut linker,
            &component,
            |name| match name {
                "a" => Ok(MyId(1)),
                "b" => Ok(MyId(2)),
                other => wasmtime::bail!("unexpected import: {other}"),
            },
            |s| s,
        )?;
        let mut store = Store::new(&engine, MyHost::default());
        let cache = Cache::instantiate(&mut store, &component, &linker)?;

        cache.call_run(&mut store)?;

        let calls = &store.data().calls;
        assert!(calls.contains(&(1, "set")), "calls: {calls:?}");
        assert!(calls.contains(&(2, "set")), "calls: {calls:?}");
        assert!(calls.contains(&(1, "get")), "calls: {calls:?}");
        assert!(calls.contains(&(2, "get")), "calls: {calls:?}");
        Ok(())
    }

    #[test]
    fn lookup_error_is_propagated() -> Result<()> {
        let engine = implements_engine();
        let component = Component::new(&engine, COMPONENT)?;
        let mut linker = Linker::new(&engine);
        let result = named_imports::demo::pkg::store::add_to_linker::<_, HasSelf<MyHost>>(
            &mut linker,
            &component,
            |name| match name {
                "a" => Ok(MyId(1)),
                _ => wasmtime::bail!("bad import"),
            },
            |s| s,
        );
        let err = result.expect_err("expected lookup error to propagate");
        assert!(
            err.to_string().contains("bad import"),
            "unexpected error: {err:?}"
        );
        Ok(())
    }
}

/// A component shared by the sync and async resource named-import tests below.
///
/// It imports `demo:pkg/store` (which defines a `counter` resource plus a free
/// `label` function) twice, under arbitrary names `a` and `b`, each annotated
/// as implementing that interface. The exported `run` exercises, through *both*
/// imports: the constructor, an instance method (`value`/`bump`), a static
/// method (`combine`), the free function (`label`), and the destructor.
const RESOURCE_COMPONENT: &str = r#"
    (component
        (import "a" (implements "demo:pkg/store") (instance $a
            (export "counter" (type $counter_a (sub resource)))
            (export "[constructor]counter" (func (param "init" u32) (result (own $counter_a))))
            (export "[method]counter.value" (func (param "self" (borrow $counter_a)) (result u32)))
            (export "[method]counter.bump" (func (param "self" (borrow $counter_a)) (param "by" u32)))
            (export "[static]counter.combine" (func (param "a" u32) (param "b" u32) (result u32)))
            (export "label" (func (result u32)))
        ))
        (import "b" (implements "demo:pkg/store") (instance $b
            (export "counter" (type $counter_b (sub resource)))
            (export "[constructor]counter" (func (param "init" u32) (result (own $counter_b))))
            (export "[method]counter.value" (func (param "self" (borrow $counter_b)) (result u32)))
            (export "[method]counter.bump" (func (param "self" (borrow $counter_b)) (param "by" u32)))
            (export "[static]counter.combine" (func (param "a" u32) (param "b" u32) (result u32)))
            (export "label" (func (result u32)))
        ))

        (alias export $a "counter" (type $ca))
        (alias export $b "counter" (type $cb))

        (core func $a-ctor (canon lower (func $a "[constructor]counter")))
        (core func $a-value (canon lower (func $a "[method]counter.value")))
        (core func $a-bump (canon lower (func $a "[method]counter.bump")))
        (core func $a-combine (canon lower (func $a "[static]counter.combine")))
        (core func $a-label (canon lower (func $a "label")))
        (core func $a-drop (canon resource.drop $ca))
        (core func $b-ctor (canon lower (func $b "[constructor]counter")))
        (core func $b-value (canon lower (func $b "[method]counter.value")))
        (core func $b-bump (canon lower (func $b "[method]counter.bump")))
        (core func $b-combine (canon lower (func $b "[static]counter.combine")))
        (core func $b-label (canon lower (func $b "label")))
        (core func $b-drop (canon resource.drop $cb))

        (core module $m
            (import "" "a-ctor" (func $a-ctor (param i32) (result i32)))
            (import "" "a-value" (func $a-value (param i32) (result i32)))
            (import "" "a-bump" (func $a-bump (param i32 i32)))
            (import "" "a-combine" (func $a-combine (param i32 i32) (result i32)))
            (import "" "a-label" (func $a-label (result i32)))
            (import "" "a-drop" (func $a-drop (param i32)))
            (import "" "b-ctor" (func $b-ctor (param i32) (result i32)))
            (import "" "b-value" (func $b-value (param i32) (result i32)))
            (import "" "b-bump" (func $b-bump (param i32 i32)))
            (import "" "b-combine" (func $b-combine (param i32 i32) (result i32)))
            (import "" "b-label" (func $b-label (result i32)))
            (import "" "b-drop" (func $b-drop (param i32)))

            (func (export "run")
                (local $ha i32)
                (local $hb i32)

                (local.set $ha (call $a-ctor (i32.const 10)))
                (call $a-bump (local.get $ha) (i32.const 5))
                (if (i32.ne (call $a-value (local.get $ha)) (i32.const 15))
                    (then unreachable))

                (local.set $hb (call $b-ctor (i32.const 100)))
                (if (i32.ne (call $b-value (local.get $hb)) (i32.const 100))
                    (then unreachable))

                ;; static method through each import
                (if (i32.ne (call $a-combine (i32.const 3) (i32.const 4)) (i32.const 7))
                    (then unreachable))
                (if (i32.ne (call $b-combine (i32.const 5) (i32.const 6)) (i32.const 11))
                    (then unreachable))

                ;; free function in the same interface, id-dependent result
                (if (i32.ne (call $a-label) (i32.const 1000))
                    (then unreachable))
                (if (i32.ne (call $b-label) (i32.const 2000))
                    (then unreachable))

                (call $a-drop (local.get $ha))
                (call $b-drop (local.get $hb))
            )
        )
        (core instance $i (instantiate $m
            (with "" (instance
                (export "a-ctor" (func $a-ctor))
                (export "a-value" (func $a-value))
                (export "a-bump" (func $a-bump))
                (export "a-combine" (func $a-combine))
                (export "a-label" (func $a-label))
                (export "a-drop" (func $a-drop))
                (export "b-ctor" (func $b-ctor))
                (export "b-value" (func $b-value))
                (export "b-bump" (func $b-bump))
                (export "b-combine" (func $b-combine))
                (export "b-label" (func $b-label))
                (export "b-drop" (func $b-drop))
            ))
        ))

        (func (export "run") (canon lift (core func $i "run")))
    )
"#;

mod named_imports_resources {
    use super::*;
    use std::collections::HashMap;
    use wasmtime::component::{HasSelf, Resource};

    /// Host-chosen id threaded into every resource method.
    #[derive(Clone)]
    pub struct MyId(u32);

    wasmtime::component::bindgen!({
        inline: "
            package demo:pkg;

            interface store {
                resource counter {
                    constructor(init: u32);
                    value: func() -> u32;
                    bump: func(by: u32);
                    combine: static func(a: u32, b: u32) -> u32;
                }

                label: func() -> u32;
            }

            world runner {
                import store;

                export run: func();
            }
        ",
        named_imports: {
            "demo:pkg/store": MyId,
        },
    });

    use demo::pkg::store::Counter;

    fn implements_engine() -> Engine {
        let mut config = Config::new();
        config.wasm_component_model(true);
        config.wasm_component_model_implements(true);
        Engine::new(&config).unwrap()
    }

    #[derive(Default)]
    struct MyHost {
        /// Resource state keyed by `(id, rep)`.
        counters: HashMap<(u32, u32), u32>,
        /// Global rep allocator producing distinct reps across imports.
        next_rep: u32,
        /// Records `(id, op)` of every static/free-function call.
        calls: Vec<(u32, &'static str)>,
        /// Records `(id, rep)` of every destructor call.
        drops: Vec<(u32, u32)>,
    }

    impl named_imports::demo::pkg::store::HostCounter for MyHost {
        fn new(&mut self, id: MyId, init: u32) -> Resource<Counter> {
            self.next_rep += 1;
            let rep = self.next_rep;
            self.counters.insert((id.0, rep), init);
            Resource::new_own(rep)
        }

        fn value(&mut self, id: MyId, self_: Resource<Counter>) -> u32 {
            self.counters[&(id.0, self_.rep())]
        }

        fn bump(&mut self, id: MyId, self_: Resource<Counter>, by: u32) {
            *self.counters.get_mut(&(id.0, self_.rep())).unwrap() += by;
        }

        fn combine(&mut self, id: MyId, a: u32, b: u32) -> u32 {
            self.calls.push((id.0, "combine"));
            a + b
        }

        fn drop(&mut self, id: MyId, rep: Resource<Counter>) -> Result<()> {
            self.drops.push((id.0, rep.rep()));
            self.counters.remove(&(id.0, rep.rep()));
            Ok(())
        }
    }

    impl named_imports::demo::pkg::store::Host for MyHost {
        fn label(&mut self, id: MyId) -> u32 {
            self.calls.push((id.0, "label"));
            id.0 * 1000
        }
    }

    #[test]
    fn resource_ids_are_threaded_through() -> Result<()> {
        let engine = implements_engine();
        let component = Component::new(&engine, super::RESOURCE_COMPONENT)?;
        let mut linker = Linker::new(&engine);
        named_imports::demo::pkg::store::add_to_linker::<_, HasSelf<MyHost>>(
            &mut linker,
            &component,
            |name| match name {
                "a" => Ok(MyId(1)),
                "b" => Ok(MyId(2)),
                other => wasmtime::bail!("unexpected import: {other}"),
            },
            |s| s,
        )?;
        let mut store = Store::new(&engine, MyHost::default());
        let runner = Runner::instantiate(&mut store, &component, &linker)?;

        runner.call_run(&mut store)?;

        let data = store.data();
        // Destructors ran with the right id and distinct reps (a -> 1, b -> 2),
        // so a swapped drop id would fail these assertions.
        assert!(data.drops.contains(&(1, 1)), "drops: {:?}", data.drops);
        assert!(data.drops.contains(&(2, 2)), "drops: {:?}", data.drops);
        // The static method and the free function were each routed to the right
        // id through both imports.
        assert!(
            data.calls.contains(&(1, "combine")),
            "calls: {:?}",
            data.calls
        );
        assert!(
            data.calls.contains(&(2, "combine")),
            "calls: {:?}",
            data.calls
        );
        assert!(
            data.calls.contains(&(1, "label")),
            "calls: {:?}",
            data.calls
        );
        assert!(
            data.calls.contains(&(2, "label")),
            "calls: {:?}",
            data.calls
        );
        // Every counter was removed on drop.
        assert!(data.counters.is_empty(), "counters: {:?}", data.counters);
        Ok(())
    }
}

// The same resource exercise as above, but with the host bindings generated in
// async mode (`imports`/`exports` async). This drives the `resource_async`
// destructor closure and the async resource methods through the runtime, where
// `call_async` runs each host future. The WIT functions remain synchronous, so
// the component itself (`super::RESOURCE_COMPONENT`) is unchanged.
mod named_imports_resources_async {
    use super::*;
    use std::collections::HashMap;
    use wasmtime::component::{HasSelf, Resource};

    #[derive(Clone)]
    pub struct MyId(u32);

    wasmtime::component::bindgen!({
        inline: "
            package demo:pkg;

            interface store {
                resource counter {
                    constructor(init: u32);
                    value: func() -> u32;
                    bump: func(by: u32);
                    combine: static func(a: u32, b: u32) -> u32;
                }

                label: func() -> u32;
            }

            world runner {
                import store;

                export run: func();
            }
        ",
        named_imports: {
            "demo:pkg/store": MyId,
        },
        imports: { default: async },
        exports: { default: async },
    });

    use demo::pkg::store::Counter;

    fn implements_async_engine() -> Engine {
        let mut config = Config::new();
        config.wasm_component_model(true);
        config.wasm_component_model_implements(true);
        Engine::new(&config).unwrap()
    }

    #[derive(Default)]
    struct MyHost {
        counters: HashMap<(u32, u32), u32>,
        next_rep: u32,
        calls: Vec<(u32, &'static str)>,
        drops: Vec<(u32, u32)>,
    }

    impl named_imports::demo::pkg::store::HostCounter for MyHost {
        async fn new(&mut self, id: MyId, init: u32) -> Resource<Counter> {
            self.next_rep += 1;
            let rep = self.next_rep;
            self.counters.insert((id.0, rep), init);
            Resource::new_own(rep)
        }

        async fn value(&mut self, id: MyId, self_: Resource<Counter>) -> u32 {
            self.counters[&(id.0, self_.rep())]
        }

        async fn bump(&mut self, id: MyId, self_: Resource<Counter>, by: u32) {
            *self.counters.get_mut(&(id.0, self_.rep())).unwrap() += by;
        }

        async fn combine(&mut self, id: MyId, a: u32, b: u32) -> u32 {
            self.calls.push((id.0, "combine"));
            a + b
        }

        async fn drop(&mut self, id: MyId, rep: Resource<Counter>) -> Result<()> {
            self.drops.push((id.0, rep.rep()));
            self.counters.remove(&(id.0, rep.rep()));
            Ok(())
        }
    }

    impl named_imports::demo::pkg::store::Host for MyHost {
        async fn label(&mut self, id: MyId) -> u32 {
            self.calls.push((id.0, "label"));
            id.0 * 1000
        }
    }

    #[tokio::test]
    async fn resource_ids_are_threaded_through_async() -> Result<()> {
        let engine = implements_async_engine();
        let component = Component::new(&engine, super::RESOURCE_COMPONENT)?;
        let mut linker = Linker::new(&engine);
        named_imports::demo::pkg::store::add_to_linker::<_, HasSelf<MyHost>>(
            &mut linker,
            &component,
            |name| match name {
                "a" => Ok(MyId(1)),
                "b" => Ok(MyId(2)),
                other => wasmtime::bail!("unexpected import: {other}"),
            },
            |s| s,
        )?;
        let mut store = Store::new(&engine, MyHost::default());
        let runner = Runner::instantiate_async(&mut store, &component, &linker).await?;

        runner.call_run(&mut store).await?;

        let data = store.data();
        assert!(data.drops.contains(&(1, 1)), "drops: {:?}", data.drops);
        assert!(data.drops.contains(&(2, 2)), "drops: {:?}", data.drops);
        assert!(
            data.calls.contains(&(1, "combine")),
            "calls: {:?}",
            data.calls
        );
        assert!(
            data.calls.contains(&(2, "combine")),
            "calls: {:?}",
            data.calls
        );
        assert!(
            data.calls.contains(&(1, "label")),
            "calls: {:?}",
            data.calls
        );
        assert!(
            data.calls.contains(&(2, "label")),
            "calls: {:?}",
            data.calls
        );
        assert!(data.counters.is_empty(), "counters: {:?}", data.counters);
        Ok(())
    }
}
