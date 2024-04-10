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

mod with_key_and_resources {
    use anyhow::Result;
    use wasmtime::component::Resource;

    wasmtime::component::bindgen!({
        inline: "
            package demo:pkg;

            interface bar {
                resource a;
                resource b;
            }

            world foo {
                resource a;
                resource b;

                import foo: interface {
                    resource a;
                    resource b;
                }

                import bar;
            }
        ",
        with: {
            "a": MyA,
            "b": MyA,
            "foo/a": MyA,
            "foo/b": MyA,
            "demo:pkg/bar/a": MyA,
            "demo:pkg/bar/b": MyA,
        },
    });

    pub struct MyA;

    struct MyComponent;

    impl FooImports for MyComponent {}

    impl HostA for MyComponent {
        fn drop(&mut self, _: Resource<MyA>) -> Result<()> {
            loop {}
        }
    }

    impl HostB for MyComponent {
        fn drop(&mut self, _: Resource<MyA>) -> Result<()> {
            loop {}
        }
    }

    impl foo::Host for MyComponent {}

    impl foo::HostA for MyComponent {
        fn drop(&mut self, _: Resource<MyA>) -> Result<()> {
            loop {}
        }
    }

    impl foo::HostB for MyComponent {
        fn drop(&mut self, _: Resource<MyA>) -> Result<()> {
            loop {}
        }
    }

    impl demo::pkg::bar::Host for MyComponent {}

    impl demo::pkg::bar::HostA for MyComponent {
        fn drop(&mut self, _: Resource<MyA>) -> Result<()> {
            loop {}
        }
    }

    impl demo::pkg::bar::HostB for MyComponent {
        fn drop(&mut self, _: Resource<MyA>) -> Result<()> {
            loop {}
        }
    }
}

mod trappable_errors_with_versioned_and_unversioned_packages {
    wasmtime::component::bindgen!({
        inline: "
            package foo:foo@0.1.0;

            interface a {
                variant error {
                    other(string),
                }

                f: func() -> result<_, error>;
            }

            world foo {
                import a;
            }
        ",
        path: "tests/codegen/unversioned-foo.wit",
        trappable_error_type: {
            "foo:foo/a@0.1.0/error" => MyX,
        },
    });

    #[allow(dead_code)]
    type MyX = u64;
}

mod trappable_errors {
    wasmtime::component::bindgen!({
        inline: "
            package demo:pkg;

            interface a {
                type b = u64;

                z1: func() -> result<_, b>;
                z2: func() -> result<_, b>;
            }

            interface b {
                use a.{b};
                z: func() -> result<_, b>;
            }

            interface c {
                type b = u64;
            }

            interface d {
                use c.{b};
                z: func() -> result<_, b>;
            }

            world foo {
                import a;
                import b;
                import d;
            }
        ",
        trappable_error_type: {
            "demo:pkg/a/b" => MyX,
            "demo:pkg/c/b" => MyX,
        },
    });

    #[allow(dead_code)]
    type MyX = u32;
}

mod interface_name_with_rust_keyword {
    wasmtime::component::bindgen!({
        inline: "
            package foo:foo;

            interface crate { }

            world foo {
                export crate;
            }
        "
    });
}

mod with_works_with_hierarchy {
    mod bindings {
        wasmtime::component::bindgen!({
            inline: "
                package foo:foo;

                interface a {
                    record t {
                        x: u32,
                    }
                    x: func() -> t;
                }

                interface b {
                    use a.{t};
                    x: func() -> t;
                }

                interface c {
                    use b.{t};
                    x: func() -> t;
                }

                world foo {
                    import c;
                }
            "
        });
    }

    mod with_just_one_interface {
        wasmtime::component::bindgen!({
            inline: "
                package foo:foo;

                interface a {
                    record t {
                        x: u32,
                    }
                    x: func() -> t;
                }

                interface b {
                    use a.{t};
                    x: func() -> t;
                }

                interface c {
                    use b.{t};
                    x: func() -> t;
                }

                world foo {
                    use c.{t};

                    import x: func() -> t;
                }
            ",
            with: { "foo:foo/a": super::bindings::foo::foo::a }
        });

        struct X;

        impl FooImports for X {
            fn x(&mut self) -> super::bindings::foo::foo::a::T {
                loop {}
            }
        }
    }

    mod with_whole_package {
        wasmtime::component::bindgen!({
            inline: "
                package foo:foo;

                interface a {
                    record t {
                        x: u32,
                    }
                    x: func() -> t;
                }

                interface b {
                    use a.{t};
                    x: func() -> t;
                }

                interface c {
                    use b.{t};
                    x: func() -> t;
                }

                world foo {
                    use c.{t};

                    import x: func() -> t;
                }
            ",
            with: { "foo:foo": super::bindings::foo::foo }
        });

        struct X;

        impl FooImports for X {
            fn x(&mut self) -> super::bindings::foo::foo::a::T {
                loop {}
            }
        }
    }

    mod with_whole_namespace {
        wasmtime::component::bindgen!({
            inline: "
                package foo:foo;

                interface a {
                    record t {
                        x: u32,
                    }
                    x: func() -> t;
                }

                interface b {
                    use a.{t};
                    x: func() -> t;
                }

                interface c {
                    use b.{t};
                    x: func() -> t;
                }

                world foo {
                    use c.{t};

                    import x: func() -> t;
                }
            ",
            with: { "foo": super::bindings::foo }
        });

        struct X;

        impl FooImports for X {
            fn x(&mut self) -> super::bindings::foo::foo::a::T {
                loop {}
            }
        }
    }
}

mod trappable_imports {
    mod none {
        wasmtime::component::bindgen!({
            inline: "
                package foo:foo;

                world foo {
                    import foo: func();
                }
            ",
            trappable_imports: false,
        });
        struct X;

        impl FooImports for X {
            fn foo(&mut self) {}
        }
    }

    mod all {
        wasmtime::component::bindgen!({
            inline: "
                package foo:foo;

                world foo {
                    import foo: func();
                }
            ",
            trappable_imports: true,
        });
        struct X;

        impl FooImports for X {
            fn foo(&mut self) -> wasmtime::Result<()> {
                Ok(())
            }
        }
    }

    mod some {
        wasmtime::component::bindgen!({
            inline: "
                package foo:foo;

                world foo {
                    import foo: func();
                    import bar: func();
                }
            ",
            trappable_imports: ["foo"],
        });
        struct X;

        impl FooImports for X {
            fn foo(&mut self) -> wasmtime::Result<()> {
                Ok(())
            }
            fn bar(&mut self) {}
        }
    }

    mod across_interfaces {
        use wasmtime::component::Resource;

        wasmtime::component::bindgen!({
            inline: "
                package foo:foo;

                interface a {
                    foo: func();
                    bar: func();

                    resource r {
                        constructor();
                        foo: func();
                        bar: static func();
                    }
                }

                world foo {
                    import a;
                    import foo: func();
                    import bar: func();
                    import i: interface {
                        foo: func();
                        bar: func();
                    }

                }
            ",
            trappable_imports: ["foo"],
            with: { "foo:foo/a/r": R },
        });

        struct X;
        pub struct R;

        impl FooImports for X {
            fn foo(&mut self) -> wasmtime::Result<()> {
                Ok(())
            }
            fn bar(&mut self) {}
        }

        impl i::Host for X {
            fn foo(&mut self) -> wasmtime::Result<()> {
                Ok(())
            }
            fn bar(&mut self) {}
        }

        impl foo::foo::a::Host for X {
            fn foo(&mut self) -> wasmtime::Result<()> {
                Ok(())
            }
            fn bar(&mut self) {}
        }

        impl foo::foo::a::HostR for X {
            fn new(&mut self) -> Resource<R> {
                loop {}
            }
            fn foo(&mut self, _: Resource<R>) {}
            fn bar(&mut self) {}
            fn drop(&mut self, _: Resource<R>) -> wasmtime::Result<()> {
                Ok(())
            }
        }
    }

    mod resources {
        use wasmtime::component::Resource;

        wasmtime::component::bindgen!({
            inline: "
                package foo:foo;

                interface a {
                    resource r {
                        constructor();
                        foo: func();
                        bar: static func();
                    }
                }

                world foo {
                    import a;

                }
            ",
            trappable_imports: [
                "[constructor]r",
                "[method]r.foo",
                "[static]r.bar",
            ],
            with: { "foo:foo/a/r": R },
        });

        struct X;
        pub struct R;

        impl foo::foo::a::Host for X {}

        impl foo::foo::a::HostR for X {
            fn new(&mut self) -> wasmtime::Result<Resource<R>> {
                loop {}
            }
            fn foo(&mut self, _: Resource<R>) -> wasmtime::Result<()> {
                Ok(())
            }
            fn bar(&mut self) -> wasmtime::Result<()> {
                Ok(())
            }
            fn drop(&mut self, _: Resource<R>) -> wasmtime::Result<()> {
                Ok(())
            }
        }
    }
}
