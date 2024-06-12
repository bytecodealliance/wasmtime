use super::{super::REALLOC_AND_FREE, engine};
use anyhow::Result;
use wasmtime::{
    component::{Component, Linker},
    Store,
};

fn component() -> String {
    format!(
        r#"
        (component
            (core module $libc
                (memory (export "memory") 1)
                {REALLOC_AND_FREE}
            )
            (core instance $libc (instantiate $libc))
            (core module $m
                (import "libc" "memory" (memory 0))
                (import "libc" "realloc" (func $realloc (param i32 i32 i32 i32) (result i32)))
                (func (export "core_foo_export") (param i32 i32) (result i32)
                    (local $retptr i32)
                    (local.set $retptr
                        (call $realloc
                            (i32.const 0)
                            (i32.const 0)
                            (i32.const 4)
                            (i32.const 8)))
                    (i32.store offset=0 (local.get $retptr) (local.get 0))
                    (i32.store offset=4 (local.get $retptr) (local.get 1))
                    (local.get $retptr)
                )
                (func (export "core_bar_export") (param i32 i32 i32 i32))
                (func (export "core_baz_export") (param i32 i32 i32 i32) (result i32)
                    (local $retptr i32)
                    (local.set $retptr
                        (call $realloc
                            (i32.const 0)
                            (i32.const 0)
                            (i32.const 4)
                            (i32.const 16)))
                    (i32.store offset=0 (local.get $retptr) (local.get 0))
                    (i32.store offset=4 (local.get $retptr) (local.get 1))
                    (i32.store offset=8 (local.get $retptr) (local.get 2))
                    (i32.store offset=12 (local.get $retptr) (local.get 3))
                    (local.get $retptr)
                )
            )
            (core instance $i (instantiate $m
                (with "libc" (instance $libc))
            ))

            (func $f_foo
                (param "a" (list (list string)))
                (result (list (list string)))
                (canon lift (core func $i "core_foo_export") (memory $libc "memory")
                    (realloc (func $libc "realloc"))
                )
            )

            (type $thing (record (field "name" string) (field "value" (list string))))

            (func $f_bar
                (param "a" $thing)
                (canon lift (core func $i "core_bar_export") (memory $libc "memory")
                    (realloc (func $libc "realloc"))
                )
            )

            (func $f_baz
                (param "a" $thing)
                (result $thing)
                (canon lift (core func $i "core_baz_export") (memory $libc "memory")
                    (realloc (func $libc "realloc"))
                )
            )

            (component $c_lists
                (import "import-foo" (func $f
                    (param "a" (list (list string)))
                    (result (list (list string)))
                ))
                (export "foo" (func $f))
            )
            (instance $i_lists (instantiate $c_lists
                (with "import-foo" (func $f_foo))
            ))
            (export "lists" (instance $i_lists))

            (component $c_thing_in
                (import "import-thing" (type $import-thing (eq $thing)))
                (import "import-bar" (func $f (param "a" $import-thing)))
                (export $export-thing "thing" (type $thing))
                (export "bar" (func $f) (func (param "a" $export-thing)))
            )
            (instance $i_thing_in (instantiate $c_thing_in
                (with "import-thing" (type $thing))
                (with "import-bar" (func $f_bar))
            ))
            (export "thing-in" (instance $i_thing_in))

            (component $c_thing_in_and_out
                (import "import-thing" (type $import-thing (eq $thing)))
                (import "import-baz" (func $f (param "a" $import-thing) (result $import-thing)))
                (export $export-thing "thing" (type $thing))
                (export "baz" (func $f) (func (param "a" $export-thing) (result $export-thing)))
            )
            (instance $i_thing_in_and_out (instantiate $c_thing_in_and_out
                (with "import-thing" (type $thing))
                (with "import-baz" (func $f_baz))
            ))
            (export "thing-in-and-out" (instance $i_thing_in_and_out))
        )
        "#
    )
}

mod owning {
    use super::*;

    wasmtime::component::bindgen!({
        inline: "
        package inline:inline;
        world test {
            export lists: interface {
                foo: func(a: list<list<string>>) -> list<list<string>>;
            }

            export thing-in: interface {
                record thing {
                    name: string,
                    value: list<string>
                }

                bar: func(a: thing);
            }

            export thing-in-and-out: interface {
                record thing {
                    name: string,
                    value: list<string>
                }

                baz: func(a: thing) -> thing;
            }
        }",
        ownership: Owning
    });

    impl PartialEq for exports::thing_in::Thing {
        fn eq(&self, other: &Self) -> bool {
            self.name == other.name && self.value == other.value
        }
    }

    impl PartialEq for exports::thing_in_and_out::Thing {
        fn eq(&self, other: &Self) -> bool {
            self.name == other.name && self.value == other.value
        }
    }

    #[test]
    fn owning() -> Result<()> {
        let engine = engine();
        let component = Component::new(&engine, component())?;

        let linker = Linker::new(&engine);
        let mut store = Store::new(&engine, ());
        let test = Test::instantiate(&mut store, &component, &linker)?;

        let value = vec![vec!["a".to_owned(), "b".to_owned()]];
        assert_eq!(value, test.lists().call_foo(&mut store, &value)?);

        let value = exports::thing_in::Thing {
            name: "thing 1".to_owned(),
            value: vec!["some value".to_owned(), "another value".to_owned()],
        };
        test.thing_in().call_bar(&mut store, &value)?;

        let value = exports::thing_in_and_out::Thing {
            name: "thing 1".to_owned(),
            value: vec!["some value".to_owned(), "another value".to_owned()],
        };
        assert_eq!(value, test.thing_in_and_out().call_baz(&mut store, &value)?);

        Ok(())
    }
}

mod borrowing_no_duplication {
    use super::*;
    wasmtime::component::bindgen!({
        inline: "
        package inline:inline;
        world test {
            export lists: interface {
                foo: func(a: list<list<string>>) -> list<list<string>>;
            }

            export thing-in: interface {
                record thing {
                    name: string,
                    value: list<string>
                }

                bar: func(a: thing);
            }

            export thing-in-and-out: interface {
                record thing {
                    name: string,
                    value: list<string>
                }

                baz: func(a: thing) -> thing;
            }
        }",
        ownership: Borrowing {
            duplicate_if_necessary: false
        }
    });

    impl PartialEq for exports::thing_in::Thing<'_> {
        fn eq(&self, other: &Self) -> bool {
            self.name == other.name && self.value == other.value
        }
    }

    impl PartialEq for exports::thing_in_and_out::Thing {
        fn eq(&self, other: &Self) -> bool {
            self.name == other.name && self.value == other.value
        }
    }

    #[test]
    fn borrowing_no_duplication() -> Result<()> {
        let engine = engine();
        let component = Component::new(&engine, component())?;

        let linker = Linker::new(&engine);
        let mut store = Store::new(&engine, ());
        let test = Test::instantiate(&mut store, &component, &linker)?;

        let value = &[&["a", "b"] as &[_]] as &[_];
        assert_eq!(value, test.lists().call_foo(&mut store, value)?);

        let value = exports::thing_in::Thing {
            name: "thing 1",
            value: &["some value", "another value"],
        };
        test.thing_in().call_bar(&mut store, value)?;

        let value = exports::thing_in_and_out::Thing {
            name: "thing 1".to_owned(),
            value: vec!["some value".to_owned(), "another value".to_owned()],
        };
        assert_eq!(value, test.thing_in_and_out().call_baz(&mut store, &value)?);

        Ok(())
    }
}

mod borrowing_with_duplication {
    use super::*;

    wasmtime::component::bindgen!({
        inline: "
        package inline:inline;
        world test {
            export lists: interface {
                foo: func(a: list<list<string>>) -> list<list<string>>;
            }

            export thing-in: interface {
                record thing {
                    name: string,
                    value: list<string>
                }

                bar: func(a: thing);
            }

            export thing-in-and-out: interface {
                record thing {
                    name: string,
                    value: list<string>
                }

                baz: func(a: thing) -> thing;
            }
        }",
        ownership: Borrowing {
            duplicate_if_necessary: true
        }
    });

    impl PartialEq for exports::thing_in::Thing<'_> {
        fn eq(&self, other: &Self) -> bool {
            self.name == other.name && self.value == other.value
        }
    }

    impl PartialEq for exports::thing_in_and_out::ThingResult {
        fn eq(&self, other: &Self) -> bool {
            self.name == other.name && self.value == other.value
        }
    }

    #[test]
    fn borrowing_with_duplication() -> Result<()> {
        let engine = engine();
        let component = Component::new(&engine, component())?;

        let linker = Linker::new(&engine);
        let mut store = Store::new(&engine, ());
        let test = Test::instantiate(&mut store, &component, &linker)?;

        let value = &[&["a", "b"] as &[_]] as &[_];
        assert_eq!(value, test.lists().call_foo(&mut store, value)?);

        let value = exports::thing_in::Thing {
            name: "thing 1",
            value: &["some value", "another value"],
        };
        test.thing_in().call_bar(&mut store, value)?;

        let value = exports::thing_in_and_out::ThingParam {
            name: "thing 1",
            value: &["some value", "another value"],
        };
        assert_eq!(
            exports::thing_in_and_out::ThingResult {
                name: "thing 1".to_owned(),
                value: vec!["some value".to_owned(), "another value".to_owned()],
            },
            test.thing_in_and_out().call_baz(&mut store, value)?
        );

        Ok(())
    }
}
