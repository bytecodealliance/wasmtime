//! Tests to check that keywords in `witx` files are escaped.
//!
//! No `#[test]` functions are defined below because the `wiggle::from_witx!` macro expanding into
//! syntactically correct Rust code at compile time is the subject under test.

/// Test that an enum variant that conflicts with a Rust keyword can be compiled properly.
mod enum_test {
    wiggle::from_witx!({
        witx_literal:
            "(typename $self
                 (enum (@witx tag u8)
                     $self
                     $2big
                 )
             )",
    });
}

/// Test module, trait, function, and function parameter names conflicting with Rust keywords.
///
/// We use `self` because the camel-cased trait name `Self` is *also* a strict keyword. This lets
/// us simultaneously test the name of the module and the generated trait.
mod module_trait_fn_and_arg_test {
    use wiggle_test::WasiCtx;
    wiggle::from_witx!({
        witx_literal:
            "(module $self
                 (@interface func (export \"fn\")
                     (param $use u32)
                     (param $virtual u32)
                 )
             )",
    });
    impl<'a> self_::Self_ for WasiCtx<'a> {
        fn fn_(&mut self, _memory: &mut wiggle::GuestMemory<'_>, _use_: u32, _virtual_: u32) {
            unimplemented!();
        }
    }
}

/// Test that a struct and member names conflicting with Rust keywords can be compiled properly.
mod struct_test {
    wiggle::from_witx!({
        witx_literal:
            "(typename $self
                 (record
                     (field $become s32)
                     (field $mut s32)
                 )
             )",
    });
}

/// Test that a union variant that conflicts with a Rust keyword can be compiled properly.
mod union_test {
    wiggle::from_witx!({
        witx: ["$CARGO_MANIFEST_DIR/tests/keywords_union.witx"],
    });
}
