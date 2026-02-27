use crate::prelude::*;
use core::fmt;

/// Helper macro to `bail!` with a `WasmtimeBug` instance.
///
/// This is used in locations in lieu of panicking. The general idea when using
/// this is:
///
/// * The invocation of this cannot be refactored to be statically ruled out.
/// * The invocation cannot be reasoned about locally to determine that this is
///   dynamically not reachable.
///
/// This macro serves as an alternative to `panic!` which returns a
/// `WasmtimeBug` instead of panicking. This means that a trap is raised in the
/// guest and a store is poisoned for example (w.r.t. components). This
/// primarily serves as a DoS mitigation mechanism where if the panic were
/// actually hit at runtime it would be a CVE. The worst-case scenario of
/// raising a trap is that a guest is erroneously terminated, which is a much
/// more controlled failure mode.
///
/// The general guideline for using this is "don't" if you can avoid it because
/// it's best to either statically rule out these cases or make it verifiable
/// locally that it can't be hit. When this isn't possible, however, this is a
/// good alternative to panicking in the case that this is actually executed at
/// runtime.
macro_rules! bail_bug {
    ($($arg:tt)*) => {{
        // Minimize argument passing to the `new` function by placing the
        // file/line in a static which is passed by reference to just pass a
        // single extra pointer argument.
        static POS: (&'static str, u32) = (file!(), line!());
        $crate::bail!(crate::WasmtimeBug::new(format_args!($($arg)*), &POS))
    }}
}

pub(crate) use bail_bug;

/// Error which indicates a bug in Wasmtime.
///
/// This structure is used internally with Wasmtime for situations which are a
/// bug in Wasmtime but not serious enough to raise a panic and unwind the
/// current thread of execution. In these situations this is still considered a
/// bug and a trap is raised to terminate a guest, and it's considered something
/// that needs to be fixed in Wasmtime.
#[derive(Debug)]
pub struct WasmtimeBug {
    message: String,
    file: &'static str,
    line: u32,
}

impl WasmtimeBug {
    #[cold]
    pub(crate) fn new(message: fmt::Arguments<'_>, pos: &'static (&'static str, u32)) -> Self {
        if cfg!(debug_assertions) {
            panic!("BUG: {message}");
        }
        Self {
            message: message.to_string(),
            file: pos.0,
            line: pos.1,
        }
    }
}

impl fmt::Display for WasmtimeBug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "\
BUG: {}
location: {}:{}
version: {}

This is a bug in Wasmtime that was not thought to be reachable. A panic is
not happening to avoid taking down the thread, but this trap is being injected
into WebAssembly guests to prevent their execution. The Wasmtime project would
appreciate a bug report with a copy of this message to help investigate what
happened. If you're able to provide a reproduction, that would be appreciated,
but it is not necessary to do so and instead indicating that this is reachable
is a sufficiently actionable bug for maintainers to investigate.

",
            self.message,
            self.file,
            self.line,
            env!("CARGO_PKG_VERSION"),
        )
    }
}

impl core::error::Error for WasmtimeBug {}
