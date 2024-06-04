use test_programs::rpc_sync::{foo, rpc_test};

fn main() {
    foo::foo("foo");

    let v = foo::f("foo");
    assert_eq!(v, 42);

    let v = rpc_test::sync::sync::fallible(true);
    assert_eq!(v, Ok(true));

    let v = rpc_test::sync::sync::fallible(false);
    assert_eq!(v, Err("test".to_string()));

    let v = rpc_test::sync::sync::numbers();
    assert_eq!(v, (1, 2, 3, 4, 5, 6, 7, 8, 9., 10.,));

    let v = rpc_test::sync::sync::with_flags(true, false, true);
    assert_eq!(
        v,
        rpc_test::sync::sync::Abc::A | rpc_test::sync::sync::Abc::C
    );

    let v = rpc_test::sync::sync::with_variant_option(false);
    assert_eq!(v, None);

    let v = rpc_test::sync::sync::with_variant_option(true);
    assert_eq!(
        v,
        Some(rpc_test::sync::sync::Var::Var(rpc_test::sync::sync::Rec {
            nested: rpc_test::sync::sync::RecNested {
                foo: "bar".to_string()
            }
        }))
    );

    let v = rpc_test::sync::sync::with_record();
    assert_eq!(
        v,
        rpc_test::sync::sync::Rec {
            nested: rpc_test::sync::sync::RecNested {
                foo: "foo".to_string()
            }
        },
    );

    let v = rpc_test::sync::sync::with_record_list(0);
    assert_eq!(v, []);

    let v = rpc_test::sync::sync::with_record_list(3);
    assert_eq!(
        v,
        [
            rpc_test::sync::sync::Rec {
                nested: rpc_test::sync::sync::RecNested {
                    foo: "0".to_string()
                }
            },
            rpc_test::sync::sync::Rec {
                nested: rpc_test::sync::sync::RecNested {
                    foo: "1".to_string()
                }
            },
            rpc_test::sync::sync::Rec {
                nested: rpc_test::sync::sync::RecNested {
                    foo: "2".to_string()
                }
            },
        ]
    );

    let v = rpc_test::sync::sync::with_record_tuple();
    assert_eq!(
        v,
        (
            rpc_test::sync::sync::Rec {
                nested: rpc_test::sync::sync::RecNested {
                    foo: "0".to_string()
                }
            },
            rpc_test::sync::sync::Rec {
                nested: rpc_test::sync::sync::RecNested {
                    foo: "1".to_string()
                }
            },
        )
    );

    let v = rpc_test::sync::sync::with_enum();
    assert_eq!(v, rpc_test::sync::sync::Foobar::Bar);
}
