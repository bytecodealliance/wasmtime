//! Minimal versions of standard-library panicking and printing macros.
//!
//! We're avoiding static initializers, so we can't have things like string
//! literals. Replace the standard assert macros with simpler implementations.

use crate::bindings::wasi::cli::stderr::get_stderr;

#[allow(dead_code, reason = "useful for debugging")]
#[doc(hidden)]
pub fn print(message: &[u8]) {
    let _ = get_stderr().blocking_write_and_flush(message);
}

/// A minimal `eprint` for debugging.
#[allow(unused_macros, reason = "useful for debugging")]
macro_rules! eprint {
    ($arg:tt) => {{
        // We have to expand string literals into byte arrays to prevent them
        // from getting statically initialized.
        let message = byte_array_literals::str!($arg);
        $crate::macros::print(&message);
    }};
}

/// A minimal `eprintln` for debugging.
#[allow(unused_macros, reason = "useful for debugging")]
macro_rules! eprintln {
    ($arg:tt) => {{
        // We have to expand string literals into byte arrays to prevent them
        // from getting statically initialized.
        let message = byte_array_literals::str_nl!($arg);
        $crate::macros::print(&message);
    }};
}

#[allow(dead_code, reason = "useful for debugging")]
#[doc(hidden)]
pub fn eprint_unreachable(line: u32) {
    eprint!("unreachable executed at adapter line ");
    crate::macros::eprint_u32(line);
}

fn eprint_u32(x: u32) {
    if x == 0 {
        eprint!("0");
    } else {
        eprint_u32_impl(x)
    }

    fn eprint_u32_impl(x: u32) {
        if x != 0 {
            eprint_u32_impl(x / 10);

            let digit = [b'0' + ((x % 10) as u8)];
            crate::macros::print(&digit);
        }
    }
}

#[allow(dead_code, reason = "useful for debugging")]
#[doc(hidden)]
pub fn unreachable(line: u32) -> ! {
    crate::macros::eprint_unreachable(line);
    eprint!("\n");
    #[cfg(target_arch = "wasm32")]
    core::arch::wasm32::unreachable();
    // This is here to keep rust-analyzer happy when building for native:
    #[cfg(not(target_arch = "wasm32"))]
    std::process::abort();
}

/// A minimal `unreachable`.
macro_rules! unreachable {
    () => {{
        crate::macros::unreachable(line!());
    }};

    ($arg:tt) => {{
        crate::macros::eprint_unreachable(line!());
        eprint!(": ");
        eprintln!($arg);
        eprint!("\n");
        #[cfg(target_arch = "wasm32")]
        core::arch::wasm32::unreachable();
        // This is here to keep rust-analyzer happy when building for native:
        #[cfg(not(target_arch = "wasm32"))]
        std::process::abort();
    }};
}

#[allow(dead_code, reason = "useful for debugging")]
#[doc(hidden)]
pub fn assert_fail(line: u32) -> ! {
    eprint!("assertion failed at adapter line ");
    crate::macros::eprint_u32(line);
    #[cfg(target_arch = "wasm32")]
    core::arch::wasm32::unreachable();
    // This is here to keep rust-analyzer happy when building for native:
    #[cfg(not(target_arch = "wasm32"))]
    std::process::abort();
}

/// A minimal `assert`.
macro_rules! assert {
    ($cond:expr $(,)?) => {
        if !$cond {
            crate::macros::assert_fail(line!());
        }
    };
}

/// A minimal `assert_eq`.
macro_rules! assert_eq {
    ($left:expr, $right:expr $(,)?) => {
        assert!($left == $right);
    };
}
