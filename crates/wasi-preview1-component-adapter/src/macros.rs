//! Minimal versions of standard-library panicking and printing macros.
//!
//! We're avoiding static initializers, so we can't have things like string
//! literals. Replace the standard assert macros with simpler implementations.

#[allow(dead_code)]
#[doc(hidden)]
pub fn println(message: &[u8]) {
    // FIXME: We need some way to print a message.
    let _ = message;
}

/// A minimal `eprintln` for debugging.
#[allow(unused_macros)]
macro_rules! eprintln {
    ($arg:tt) => {{
        // We have to expand string literals into byte arrays to prevent them
        // from getting statically initialized.
        // We use `str` instead of `str_nl` because we're calling the logging
        // API which expects lines.
        let message = byte_array_literals::str!($arg);
        $crate::macros::println(&message);
    }};
}

pub(crate) fn eprint_u32(x: u32) {
    if x == 0 {
        eprintln!("0");
    } else {
        eprint_u32_impl(x)
    }

    fn eprint_u32_impl(x: u32) {
        if x != 0 {
            eprint_u32_impl(x / 10);

            let digit = [b'0' + ((x % 10) as u8)];
            crate::macros::println(&digit);
        }
    }
}

/// A minimal `unreachable`.
macro_rules! unreachable {
    () => {{
        eprintln!("unreachable executed at adapter line:");
        crate::macros::eprint_u32(line!());
        #[cfg(target_arch = "wasm32")]
        core::arch::wasm32::unreachable();
        // This is here to keep rust-analyzer happy when building for native:
        #[cfg(not(target_arch = "wasm32"))]
        std::process::abort();
    }};

    ($arg:tt) => {{
        eprintln!("unreachable executed at adapter line:");
        crate::macros::eprint_u32(line!());
        eprintln!($arg);
        #[cfg(target_arch = "wasm32")]
        core::arch::wasm32::unreachable();
        // This is here to keep rust-analyzer happy when building for native:
        #[cfg(not(target_arch = "wasm32"))]
        std::process::abort();
    }};
}

/// A minimal `assert`.
macro_rules! assert {
    ($cond:expr $(,)?) => {
        if !$cond {
            unreachable!("assertion failed")
        }
    };
}

/// A minimal `assert_eq`.
macro_rules! assert_eq {
    ($left:expr, $right:expr $(,)?) => {
        assert!($left == $right);
    };
}
