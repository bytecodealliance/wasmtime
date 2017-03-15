//! Debug tracing macros.
//!
//! This module defines the `dbg!` macro which works like `println!` except it writes to the
//! Cretonne tracing output file if enabled.
//!
//! Tracing can be enabled by setting the `CRETONNE_DBG` environment variable to something
/// other than `0`.
///
/// The output will appear in files named `cretonne.dbg.*`, where the suffix is named after the
/// thread doing the logging.

use std::ascii::AsciiExt;
use std::cell::RefCell;
use std::env;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{Write, BufWriter};
use std::sync::atomic;
use std::thread;

static STATE: atomic::AtomicIsize = atomic::ATOMIC_ISIZE_INIT;

/// Is debug tracing enabled?
///
/// Debug tracing can be enabled by setting the `CRETONNE_DBG` environment variable to something
/// other than `0`.
///
/// This inline function turns into a constant `false` when debug assertions are disabled.
#[inline]
pub fn enabled() -> bool {
    if cfg!(debug_assertions) {
        match STATE.load(atomic::Ordering::Relaxed) {
            0 => initialize(),
            s => s > 0,
        }
    } else {
        false
    }
}

/// Initialize `STATE` from the environment variable.
fn initialize() -> bool {
    let enable = match env::var_os("CRETONNE_DBG") {
        Some(s) => s != OsStr::new("0"),
        None => false,
    };

    if enable {
        STATE.store(1, atomic::Ordering::Relaxed);
    } else {
        STATE.store(-1, atomic::Ordering::Relaxed);
    }

    enable
}

thread_local! {
    static WRITER : RefCell<BufWriter<File>> = RefCell::new(open_file());
}

/// Execute a closure with mutable access to the tracing file writer.
///
/// This is for use by the `dbg!` macro.
pub fn with_writer<F, R>(f: F) -> R
    where F: FnOnce(&mut Write) -> R
{
    WRITER.with(|rc| f(&mut *rc.borrow_mut()))
}

/// Open the tracing file for the current thread.
fn open_file() -> BufWriter<File> {
    let file = match thread::current().name() {
            None => File::create("cretonne.dbg"),
            Some(name) => {
                let mut path = "cretonne.dbg.".to_owned();
                for ch in name.chars() {
                    if ch.is_ascii() && ch.is_alphanumeric() {
                        path.push(ch);
                    }
                }
                File::create(path)
            }
        }
        .expect("Can't open tracing file");
    BufWriter::new(file)
}

/// Write a line to the debug trace file if tracing is enabled.
///
/// Arguments are the same as for `printf!`.
#[macro_export]
macro_rules! dbg {
    ($($arg:tt)+) => {
        if $crate::dbg::enabled() {
            // Drop the error result so we don't get compiler errors for ignoring it.
            // What are you going to do, log the error?
            $crate::dbg::with_writer(|w| { writeln!(w, $($arg)+).ok(); })
        }
    }
}
