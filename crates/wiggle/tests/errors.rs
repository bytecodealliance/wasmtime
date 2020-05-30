/// Execute the wiggle guest conversion code to exercise it
mod convert_just_errno {
    use wiggle_test::{impl_errno, HostMemory, WasiCtx};

    /// The `errors` argument to the wiggle gives us a hook to map a rich error
    /// type like this one (typical of wiggle use cases in wasi-common and beyond)
    /// down to the flat error enums that witx can specify.
    #[derive(Debug, thiserror::Error)]
    pub enum RichError {
        #[error("Invalid argument: {0}")]
        InvalidArg(String),
        #[error("Won't cross picket line: {0}")]
        PicketLine(String),
    }

    // Define an errno with variants corresponding to RichError. Use it in a
    // trivial function.
    wiggle::from_witx!({
        witx_literal: "
(typename $errno (enum u8 $ok $invalid_arg $picket_line))
(module $one_error_conversion
  (@interface func (export \"foo\")
     (param $strike u32)
     (result $err $errno)))
    ",
        ctx: WasiCtx,
        errors: { errno => RichError },
    });

    // The impl of GuestErrorConversion works just like in every other test where
    // we have a single error type with witx `$errno` with the success called `$ok`
    impl_errno!(types::Errno, types::GuestErrorConversion);

    /// When the `errors` mapping in witx is non-empty, we need to impl the
    /// types::UserErrorConversion trait that wiggle generates from that mapping.
    impl<'a> types::UserErrorConversion for WasiCtx<'a> {
        fn errno_from_rich_error(&self, e: RichError) -> types::Errno {
            // WasiCtx can collect a Vec<String> log so we can test this. We're
            // logging the Display impl that `thiserror::Error` provides us.
            self.log.borrow_mut().push(e.to_string());
            // Then do the trivial mapping down to the flat enum.
            match e {
                RichError::InvalidArg { .. } => types::Errno::InvalidArg,
                RichError::PicketLine { .. } => types::Errno::PicketLine,
            }
        }
    }

    impl<'a> one_error_conversion::OneErrorConversion for WasiCtx<'a> {
        fn foo(&self, strike: u32) -> Result<(), RichError> {
            // We use the argument to this function to exercise all of the
            // possible error cases we could hit here
            match strike {
                0 => Ok(()),
                1 => Err(RichError::PicketLine(format!("I'm not a scab"))),
                _ => Err(RichError::InvalidArg(format!("out-of-bounds: {}", strike))),
            }
        }
    }

    #[test]
    fn one_error_conversion_test() {
        let ctx = WasiCtx::new();
        let host_memory = HostMemory::new();

        // Exercise each of the branches in `foo`.
        // Start with the success case:
        let r0 = one_error_conversion::foo(&ctx, &host_memory, 0);
        assert_eq!(
            r0,
            i32::from(types::Errno::Ok),
            "Expected return value for strike=0"
        );
        assert!(ctx.log.borrow().is_empty(), "No error log for strike=0");

        // First error case:
        let r1 = one_error_conversion::foo(&ctx, &host_memory, 1);
        assert_eq!(
            r1,
            i32::from(types::Errno::PicketLine),
            "Expected return value for strike=1"
        );
        assert_eq!(
            ctx.log.borrow_mut().pop().expect("one log entry"),
            "Won't cross picket line: I'm not a scab",
            "Expected log entry for strike=1",
        );

        // Second error case:
        let r2 = one_error_conversion::foo(&ctx, &host_memory, 2);
        assert_eq!(
            r2,
            i32::from(types::Errno::InvalidArg),
            "Expected return value for strike=2"
        );
        assert_eq!(
            ctx.log.borrow_mut().pop().expect("one log entry"),
            "Invalid argument: out-of-bounds: 2",
            "Expected log entry for strike=2",
        );
    }
}

/// Type-check the wiggle guest conversion code against a more complex case where
/// we use two distinct error types.
mod convert_multiple_error_types {
    pub use super::convert_just_errno::RichError;
    use wiggle_test::WasiCtx;

    /// Test that we can map multiple types of errors.
    #[derive(Debug, thiserror::Error)]
    #[allow(dead_code)]
    pub enum AnotherRichError {
        #[error("I've had this many cups of coffee and can't even think straight: {0}")]
        TooMuchCoffee(usize),
    }

    // Just like the other error, except that we have a second errno type:
    // trivial function.
    wiggle::from_witx!({
        witx_literal: "
(typename $errno (enum u8 $ok $invalid_arg $picket_line))
(typename $errno2 (enum u8 $ok $too_much_coffee))
(module $two_error_conversions
  (@interface func (export \"foo\")
     (param $strike u32)
     (result $err $errno))
  (@interface func (export \"bar\")
     (param $drink u32)
     (result $err $errno2)))
    ",
        ctx: WasiCtx,
        errors: { errno => RichError, errno2 => AnotherRichError },
    });

    // Can't use the impl_errno! macro as usual here because the conversion
    // trait ends up having two methods.
    // We aren't going to execute this code, so the bodies are elided.
    impl<'a> types::GuestErrorConversion for WasiCtx<'a> {
        fn into_errno(&self, _e: wiggle::GuestError) -> types::Errno {
            unimplemented!()
        }
        fn into_errno2(&self, _e: wiggle::GuestError) -> types::Errno2 {
            unimplemented!()
        }
    }
    impl wiggle::GuestErrorType for types::Errno {
        fn success() -> types::Errno {
            <types::Errno>::Ok
        }
    }
    impl wiggle::GuestErrorType for types::Errno2 {
        fn success() -> types::Errno2 {
            <types::Errno2>::Ok
        }
    }

    // The UserErrorConversion trait will also have two methods for this test. They correspond to
    // each member of the `errors` mapping.
    // Bodies elided.
    impl<'a> types::UserErrorConversion for WasiCtx<'a> {
        fn errno_from_rich_error(&self, _e: RichError) -> types::Errno {
            unimplemented!()
        }
        fn errno2_from_another_rich_error(&self, _e: AnotherRichError) -> types::Errno2 {
            unimplemented!()
        }
    }

    // And here's the witx module trait impl, bodies elided
    impl<'a> two_error_conversions::TwoErrorConversions for WasiCtx<'a> {
        fn foo(&self, _: u32) -> Result<(), RichError> {
            unimplemented!()
        }
        fn bar(&self, _: u32) -> Result<(), AnotherRichError> {
            unimplemented!()
        }
    }
}
