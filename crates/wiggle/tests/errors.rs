/// Execute the wiggle guest conversion code to exercise it
mod convert_just_errno {
    use anyhow::Result;
    use wiggle::GuestMemory;
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
(typename $errno (enum (@witx tag u8) $ok $invalid_arg $picket_line))
(module $one_error_conversion
  (@interface func (export \"foo\")
     (param $strike u32)
     (result $err (expected (error $errno)))))
    ",
        errors: { errno => trappable ErrnoT },
    });

    impl_errno!(types::Errno);

    impl From<RichError> for types::ErrnoT {
        fn from(rich: RichError) -> types::ErrnoT {
            match rich {
                RichError::InvalidArg(s) => {
                    types::ErrnoT::from(types::Errno::InvalidArg).context(s)
                }
                RichError::PicketLine(s) => {
                    types::ErrnoT::from(types::Errno::PicketLine).context(s)
                }
            }
        }
    }

    impl<'a> one_error_conversion::OneErrorConversion for WasiCtx<'a> {
        fn foo(&mut self, _memory: &mut GuestMemory<'_>, strike: u32) -> Result<(), types::ErrnoT> {
            // We use the argument to this function to exercise all of the
            // possible error cases we could hit here
            match strike {
                0 => Ok(()),
                1 => Err(RichError::PicketLine(format!("I'm not a scab")))?,
                _ => Err(RichError::InvalidArg(format!("out-of-bounds: {strike}")))?,
            }
        }
    }

    #[test]
    fn one_error_conversion_test() {
        let mut ctx = WasiCtx::new();
        let mut host_memory = HostMemory::new();
        let mut memory = host_memory.guest_memory();

        // Exercise each of the branches in `foo`.
        // Start with the success case:
        let r0 = one_error_conversion::foo(&mut ctx, &mut memory, 0).unwrap();
        assert_eq!(
            r0,
            types::Errno::Ok as i32,
            "Expected return value for strike=0"
        );
        assert!(ctx.log.borrow().is_empty(), "No error log for strike=0");

        // First error case:
        let r1 = one_error_conversion::foo(&mut ctx, &mut memory, 1).unwrap();
        assert_eq!(
            r1,
            types::Errno::PicketLine as i32,
            "Expected return value for strike=1"
        );

        // Second error case:
        let r2 = one_error_conversion::foo(&mut ctx, &mut memory, 2).unwrap();
        assert_eq!(
            r2,
            types::Errno::InvalidArg as i32,
            "Expected return value for strike=2"
        );
    }
}

/// Type-check the wiggle guest conversion code against a more complex case where
/// we use two distinct error types.
mod convert_multiple_error_types {
    pub use super::convert_just_errno::RichError;
    use anyhow::Result;
    use wiggle::GuestMemory;
    use wiggle_test::{impl_errno, WasiCtx};

    /// Test that we can map multiple types of errors.
    #[derive(Debug, thiserror::Error)]
    #[expect(dead_code, reason = "testing codegen below")]
    pub enum AnotherRichError {
        #[error("I've had this many cups of coffee and can't even think straight: {0}")]
        TooMuchCoffee(usize),
    }

    // Just like the prior test, except that we have a second errno type. This should mean there
    // are two functions in UserErrorConversion.
    // Additionally, test that the function "baz" marked noreturn always returns a wasmtime::Trap.
    wiggle::from_witx!({
        witx_literal: "
(typename $errno (enum (@witx tag u8) $ok $invalid_arg $picket_line))
(typename $errno2 (enum (@witx tag u8) $ok $too_much_coffee))
(module $two_error_conversions
  (@interface func (export \"foo\")
     (param $strike u32)
     (result $err (expected (error $errno))))
  (@interface func (export \"bar\")
     (param $drink u32)
     (result $err (expected (error $errno2))))
  (@interface func (export \"baz\")
     (param $drink u32)
     (@witx noreturn)))
    ",
        errors: { errno => RichError, errno2 => AnotherRichError },
    });

    impl_errno!(types::Errno);
    impl_errno!(types::Errno2);

    // The UserErrorConversion trait will also have two methods for this test. They correspond to
    // each member of the `errors` mapping.
    // Bodies elided.
    impl<'a> types::UserErrorConversion for WasiCtx<'a> {
        fn errno_from_rich_error(&mut self, _e: RichError) -> Result<types::Errno> {
            unimplemented!()
        }
        fn errno2_from_another_rich_error(
            &mut self,
            _e: AnotherRichError,
        ) -> Result<types::Errno2> {
            unimplemented!()
        }
    }

    // And here's the witx module trait impl, bodies elided
    impl<'a> two_error_conversions::TwoErrorConversions for WasiCtx<'a> {
        fn foo(&mut self, _: &mut GuestMemory<'_>, _: u32) -> Result<(), RichError> {
            unimplemented!()
        }
        fn bar(&mut self, _: &mut GuestMemory<'_>, _: u32) -> Result<(), AnotherRichError> {
            unimplemented!()
        }
        fn baz(&mut self, _: &mut GuestMemory<'_>, _: u32) -> anyhow::Error {
            unimplemented!()
        }
    }
}
