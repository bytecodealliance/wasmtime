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
(typename $s (record (field $f1 (@witx usize)) (field $f2 (@witx pointer u8))))
(typename $t (record (field $f1 u32) (field $f2 f32)))
(module $one_error_conversion
  (@interface func (export \"foo\")
     (param $strike u32)
     (param $s $s)
     (result $err (expected $t (error $errno)))))
    ",
    errors: { errno => RichError },
});

/// When the `errors` mapping in witx is non-empty, we need to impl the
/// types::UserErrorConversion trait that wiggle generates from that mapping.
impl<'a> types::UserErrorConversion for WasiCtx<'a> {
    fn errno_from_rich_error(&self, e: RichError) -> Result<types::Errno, wiggle::Trap> {
        wiggle::tracing::debug!(
            rich_error = wiggle::tracing::field::debug(&e),
            "error conversion"
        );
        // WasiCtx can collect a Vec<String> log so we can test this. We're
        // logging the Display impl that `thiserror::Error` provides us.
        self.log.borrow_mut().push(e.to_string());
        // Then do the trivial mapping down to the flat enum.
        match e {
            RichError::InvalidArg { .. } => Ok(types::Errno::InvalidArg),
            RichError::PicketLine { .. } => Ok(types::Errno::PicketLine),
        }
    }
}

impl<'a> one_error_conversion::OneErrorConversion for WasiCtx<'a> {
    fn foo(&self, strike: u32, _s: &types::S) -> Result<types::T, RichError> {
        // We use the argument to this function to exercise all of the
        // possible error cases we could hit here
        match strike {
            0 => Ok(types::T {
                f1: 123,
                f2: 456.78,
            }),
            1 => Err(RichError::PicketLine(format!("I'm not a scab"))),
            _ => Err(RichError::InvalidArg(format!("out-of-bounds: {}", strike))),
        }
    }
}

fn main() {
    if std::env::var("RUST_LOG").is_err() {
        // with no RUST_LOG env variable: use the tracing subscriber.
        let subscriber = tracing_subscriber::fmt()
            // all spans/events with a level equal to or higher than TRACE (e.g, trace, debug, info, warn, etc.)
            // will be written to stdout.
            .with_max_level(tracing::Level::TRACE)
            // builds the subscriber.
            .finish();
        tracing::subscriber::set_global_default(subscriber).expect("set global tracing subscriber");
    } else {
        // with RUST_LOG set: use the env_logger backend to tracing.
        env_logger::init();
    }

    let ctx = WasiCtx::new();
    let host_memory = HostMemory::new();

    // Exercise each of the branches in `foo`.
    // Start with the success case:
    let r0 = one_error_conversion::foo(&ctx, &host_memory, 0, 0, 8);
    assert_eq!(
        r0,
        Ok(types::Errno::Ok as i32),
        "Expected return value for strike=0"
    );
    assert!(ctx.log.borrow().is_empty(), "No error log for strike=0");

    // First error case:
    let r1 = one_error_conversion::foo(&ctx, &host_memory, 1, 0, 8);
    assert_eq!(
        r1,
        Ok(types::Errno::PicketLine as i32),
        "Expected return value for strike=1"
    );
    assert_eq!(
        ctx.log.borrow_mut().pop().expect("one log entry"),
        "Won't cross picket line: I'm not a scab",
        "Expected log entry for strike=1",
    );

    // Second error case:
    let r2 = one_error_conversion::foo(&ctx, &host_memory, 2, 0, 8);
    assert_eq!(
        r2,
        Ok(types::Errno::InvalidArg as i32),
        "Expected return value for strike=2"
    );
    assert_eq!(
        ctx.log.borrow_mut().pop().expect("one log entry"),
        "Invalid argument: out-of-bounds: 2",
        "Expected log entry for strike=2",
    );
}
