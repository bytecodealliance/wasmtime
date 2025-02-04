mod bindings {
    wit_bindgen::generate!({
        path: "../misc/component-async-tests/wit",
        world: "round-trip-many",
    });

    use super::Component;
    export!(Component);
}

use bindings::{
    exports::local::local::many::{Guest, Stuff},
    local::local::many,
};

struct Component;

impl Guest for Component {
    fn foo(
        a: String,
        b: u32,
        c: Vec<u8>,
        d: (u64, u64),
        e: Stuff,
        f: Option<Stuff>,
        g: Result<Stuff, ()>,
    ) -> (
        String,
        u32,
        Vec<u8>,
        (u64, u64),
        Stuff,
        Option<Stuff>,
        Result<Stuff, ()>,
    ) {
        let into = |v: Stuff| many::Stuff {
            a: v.a,
            b: v.b,
            c: v.c,
        };
        let from = |v: many::Stuff| Stuff {
            a: v.a,
            b: v.b,
            c: v.c,
        };
        let (a, b, c, d, e, f, g) = many::foo(
            &format!("{a} - entered guest"),
            b,
            &c,
            d,
            &into(e),
            f.map(into).as_ref(),
            g.map(into).as_ref().map_err(drop),
        );
        (
            format!("{a} - exited guest",),
            b,
            c,
            d,
            from(e),
            f.map(from),
            g.map(from),
        )
    }
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
