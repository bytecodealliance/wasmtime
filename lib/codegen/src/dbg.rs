//! Debug tracing macros.
//!
//! This module defines the `dbg!` macro which works like `println!` except it writes to the
//! Cranelift tracing output file if enabled.
//!
//! Tracing can be enabled by setting the `CRANELIFT_DBG` environment variable to something
/// other than `0`.
///
/// The output will appear in files named `cranelift.dbg.*`, where the suffix is named after the
/// thread doing the logging.
#[cfg(feature = "std")]
use std::cell::RefCell;
#[cfg(feature = "std")]
use std::env;
#[cfg(feature = "std")]
use std::ffi::OsStr;
use std::fmt;
#[cfg(feature = "std")]
use std::fs::File;
#[cfg(feature = "std")]
use std::io::{self, Write};
#[cfg(feature = "std")]
use std::sync::atomic;
#[cfg(feature = "std")]
use std::thread;

#[cfg(feature = "std")]
static STATE: atomic::AtomicIsize = atomic::ATOMIC_ISIZE_INIT;

/// Is debug tracing enabled?
///
/// Debug tracing can be enabled by setting the `CRANELIFT_DBG` environment variable to something
/// other than `0`.
///
/// This inline function turns into a constant `false` when debug assertions are disabled.
#[cfg(feature = "std")]
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

/// Does nothing
#[cfg(not(feature = "std"))]
#[inline]
pub fn enabled() -> bool {
    false
}

/// Initialize `STATE` from the environment variable.
#[cfg(feature = "std")]
fn initialize() -> bool {
    let enable = match env::var_os("CRANELIFT_DBG") {
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

#[cfg(feature = "std")]
thread_local! {
    static WRITER : RefCell<io::BufWriter<File>> = RefCell::new(open_file());
}

/// Write a line with the given format arguments.
///
/// This is for use by the `dbg!` macro.
#[cfg(feature = "std")]
pub fn writeln_with_format_args(args: fmt::Arguments) -> io::Result<()> {
    WRITER.with(|rc| {
        let mut w = rc.borrow_mut();
        writeln!(*w, "{}", args)?;
        w.flush()
    })
}

/// Open the tracing file for the current thread.
#[cfg(feature = "std")]
fn open_file() -> io::BufWriter<File> {
    let curthread = thread::current();
    let tmpstr;
    let mut path = "cranelift.dbg.".to_owned();
    path.extend(
        match curthread.name() {
            Some(name) => name.chars(),
            // The thread is unnamed, so use the thread ID instead.
            None => {
                tmpstr = format!("{:?}", curthread.id());
                tmpstr.chars()
            }
        }.filter(|ch| ch.is_alphanumeric() || *ch == '-' || *ch == '_'),
    );
    let file = File::create(path).expect("Can't open tracing file");
    io::BufWriter::new(file)
}

/// Write a line to the debug trace file if tracing is enabled.
///
/// Arguments are the same as for `printf!`.
#[cfg(feature = "std")]
#[macro_export]
macro_rules! dbg {
    ($($arg:tt)+) => {
        if $crate::dbg::enabled() {
            // Drop the error result so we don't get compiler errors for ignoring it.
            // What are you going to do, log the error?
            $crate::dbg::writeln_with_format_args(format_args!($($arg)+)).ok();
        }
    }
}

/// `dbg!` isn't supported in `no_std` mode, so expand it into nothing.
#[cfg(not(feature = "std"))]
#[macro_export]
macro_rules! dbg {
    ($($arg:tt)+) => {};
}

/// Helper for printing lists.
pub struct DisplayList<'a, T>(pub &'a [T])
where
    T: 'a + fmt::Display;

impl<'a, T> fmt::Display for DisplayList<'a, T>
where
    T: 'a + fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0.split_first() {
            None => write!(f, "[]"),
            Some((first, rest)) => {
                write!(f, "[{}", first)?;
                for x in rest {
                    write!(f, ", {}", x)?;
                }
                write!(f, "]")
            }
        }
    }
}
