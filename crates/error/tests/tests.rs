extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

use alloc::{
    format,
    string::{String, ToString},
    sync::Arc,
};
use core::{
    fmt,
    sync::atomic::{AtomicU32, Ordering::SeqCst},
};
#[cfg(feature = "std")]
use std::backtrace::BacktraceStatus;
use wasmtime_internal_error::{Context, Error, OutOfMemory, Result, anyhow, bail, ensure};

#[derive(Debug)]
struct TestError(u32);

impl fmt::Display for TestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl core::error::Error for TestError {}

#[derive(Debug)]
struct CountDrops(Arc<AtomicU32>);

impl CountDrops {
    fn new(drops: &Arc<AtomicU32>) -> Self {
        CountDrops(drops.clone())
    }
}

impl Drop for CountDrops {
    fn drop(&mut self) {
        self.0.fetch_add(1, SeqCst);
    }
}

impl fmt::Display for CountDrops {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl core::error::Error for CountDrops {}

#[derive(Debug)]
struct ChainError {
    message: String,
    source: Option<Box<dyn core::error::Error + Send + Sync + 'static>>,
}

impl fmt::Display for ChainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl core::error::Error for ChainError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        let source = self.source.as_ref()?;
        Some(&**source)
    }
}

impl ChainError {
    fn new(
        message: impl Into<String>,
        source: Option<Box<dyn core::error::Error + Send + Sync + 'static>>,
    ) -> Self {
        let message = message.into();
        Self { message, source }
    }
}

#[test]
fn new() {
    let mut error = Error::new(TestError(42));
    assert!(error.is::<TestError>());
    assert_eq!(error.downcast_ref::<TestError>().unwrap().0, 42);
    error.downcast_mut::<TestError>().unwrap().0 += 1;
    assert_eq!(error.downcast_ref::<TestError>().unwrap().0, 43);
}

#[test]
fn new_drops() {
    let drops = Arc::new(AtomicU32::new(0));
    let error = Error::new(CountDrops::new(&drops));
    assert_eq!(drops.load(SeqCst), 0);
    drop(error);
    assert_eq!(drops.load(SeqCst), 1);
}

#[test]
fn from_error_with_large_align() {
    // The `{ConcreteError,DynError}::error` fields are not at the same
    // offset when the concrete error's type requires greater-than-pointer
    // alignment. Exercise our various conversions and accesses to make sure
    // that we do the right thing in this case (that is morally cast `*mut
    // DynError` to `*mut ConcreteError<E>` rather than casting `*mut
    // TypeErasedError` to `*mut E`).
    #[derive(Debug)]
    #[repr(align(16))]
    struct LargeAlign {
        value: u128,
    }

    impl fmt::Display for LargeAlign {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            fmt::Debug::fmt(self, f)
        }
    }

    impl core::error::Error for LargeAlign {}

    let value = 0x1234_6578_1234_5678_1234_6578_1234_5678;
    let error = Error::from(LargeAlign { value });
    assert!(error.is::<LargeAlign>());
    assert_eq!(error.downcast_ref::<LargeAlign>().unwrap().value, value);
}

#[test]
fn msg() {
    let error = Error::msg("uh oh!");
    assert!(error.is::<&str>());
    let msg = error.to_string();
    assert_eq!(msg, "uh oh!");
}

#[test]
fn msg_drops() {
    let drops = Arc::new(AtomicU32::new(0));
    let error = Error::msg(CountDrops::new(&drops));
    assert_eq!(drops.load(SeqCst), 0);
    drop(error);
    assert_eq!(drops.load(SeqCst), 1);
}

#[test]
fn from_error() {
    let error = Error::from(TestError(42));
    assert!(error.is::<TestError>());
    assert_eq!(error.downcast_ref::<TestError>().unwrap().0, 42);
}

#[test]
fn into_boxed_dyn_error() {
    let error = ChainError::new("ouch", None);
    let error = ChainError::new("whoops", Some(Box::new(error)));
    let error = Error::new(error).context("yikes");

    let error = error.into_boxed_dyn_error();
    assert_eq!(error.to_string(), "yikes");

    let error = error.source().unwrap();
    assert_eq!(error.to_string(), "whoops");

    let error = error.source().unwrap();
    assert_eq!(error.to_string(), "ouch");

    assert!(error.source().is_none());
}

#[test]
fn is() {
    // `is<T>` is true for `Error::msg(T)`
    let e = Error::msg("str");
    assert!(e.is::<&str>());
    assert!(!e.is::<TestError>());
    assert!(!e.is::<OutOfMemory>());

    // `is<T>` is true for `T` in the context chain.
    let e = e.context(TestError(42));
    assert!(e.is::<&str>());
    assert!(e.is::<TestError>());
    assert!(!e.is::<OutOfMemory>());

    // `is<T>` is still true when there are multiple `T`s and `U`s in the
    // chain.
    let e = e.context(TestError(36)).context("another str");
    assert!(e.is::<&str>());
    assert!(e.is::<TestError>());
    assert!(!e.is::<OutOfMemory>());

    // `is<T>` is true for `Error::from(T)`.
    let e = Error::from(TestError(36));
    assert!(e.is::<TestError>());
    assert!(!e.is::<&str>());
    assert!(!e.is::<OutOfMemory>());

    // `is<T>` is true for `Error::from(OutOfMemory)`.
    let e = Error::from(OutOfMemory::new());
    assert!(e.is::<OutOfMemory>());
    assert!(!e.is::<TestError>());
    assert!(!e.is::<&str>());
}

#[test]
#[cfg(feature = "backtrace")]
fn backtrace() {
    // Backtrace on OOM.
    let e = Error::from(OutOfMemory::new());
    assert_eq!(e.backtrace().status(), BacktraceStatus::Disabled);

    let backtraces_enabled =
        match std::env::var("RUST_LIB_BACKTRACE").or_else(|_| std::env::var("RUST_BACKTRACE")) {
            Err(_) => false,
            Ok(s) if s == "0" => false,
            Ok(_) => true,
        };

    let assert_backtrace = |e: Error| {
        if backtraces_enabled {
            assert!(matches!(
                e.backtrace().status(),
                BacktraceStatus::Unsupported | BacktraceStatus::Captured
            ));
        } else {
            assert_eq!(e.backtrace().status(), BacktraceStatus::Disabled);
        }
    };

    // Backtrace on `Error::msg`.
    assert_backtrace(Error::msg("whoops"));

    // Backtrace on `Error::new`.
    assert_backtrace(Error::new(TestError(42)));

    // Backtrace on context chain.
    assert_backtrace(Error::new(TestError(42)).context("yikes"));
}

#[test]
fn anyhow_macro_string_literal() {
    let error = anyhow!("literal");
    assert_eq!(error.to_string(), "literal");
}

#[test]
fn anyhow_macro_format_implicit_args() {
    let x = 42;
    let y = 36;
    let error = anyhow!("implicit args {x} {y}");
    assert_eq!(error.to_string(), "implicit args 42 36");
}

#[test]
fn anyhow_macro_format_explicit_args() {
    let a = 84;
    let b = 72;
    let error = anyhow!("explicit args {x} {y}", x = a / 2, y = b / 2);
    assert_eq!(error.to_string(), "explicit args 42 36");
}

#[test]
fn anyhow_macro_core_error() {
    let error = TestError(42);
    let error = anyhow!(error);
    assert!(error.is::<TestError>());
    assert_eq!(error.to_string(), "TestError(42)");
}

#[test]
fn anyhow_macro_core_error_chain() {
    let error = ChainError::new("ouch", None);
    let error = ChainError::new("yikes", Some(Box::new(error)));
    let error = ChainError::new("whoops", Some(Box::new(error)));
    let error = anyhow!(error);

    let mut chain = error.chain();

    let e = chain.next().unwrap();
    assert_eq!(e.to_string(), "whoops");

    let e = chain.next().unwrap();
    assert_eq!(e.to_string(), "yikes");

    let e = chain.next().unwrap();
    assert_eq!(e.to_string(), "ouch");

    assert!(chain.next().is_none());
}

#[test]
fn anyhow_macro_msg() {
    let error = 42;
    let error = anyhow!(error);
    assert_eq!(error.to_string(), "42");
}

#[test]
fn bail_macro() {
    fn bail_string_literal() -> Result<()> {
        bail!("whoops")
    }
    assert_eq!(bail_string_literal().unwrap_err().to_string(), "whoops");

    fn bail_format_implicit(x: u32) -> Result<()> {
        bail!("yikes {x}")
    }
    assert_eq!(
        bail_format_implicit(42).unwrap_err().to_string(),
        "yikes 42"
    );

    fn bail_format_explicit(y: u32) -> Result<()> {
        bail!("ouch {}", y + 1)
    }
    assert_eq!(bail_format_explicit(35).unwrap_err().to_string(), "ouch 36");

    fn bail_core_error() -> Result<()> {
        bail!(TestError(13))
    }
    assert_eq!(bail_core_error().unwrap_err().to_string(), "TestError(13)");

    fn bail_display() -> Result<()> {
        let x = 1337;
        bail!(x)
    }
    assert_eq!(bail_display().unwrap_err().to_string(), "1337");
}

#[test]
fn ensure_macro() {
    fn ensure_string_literal(c: bool) -> Result<()> {
        ensure!(c, "whoops");
        Ok(())
    }
    assert!(ensure_string_literal(true).is_ok());
    assert_eq!(
        ensure_string_literal(false).unwrap_err().to_string(),
        "whoops"
    );

    fn ensure_format_implicit(c: bool, x: u32) -> Result<()> {
        ensure!(c, "yikes {x}");
        Ok(())
    }
    assert!(ensure_format_implicit(true, 42).is_ok());
    assert_eq!(
        ensure_format_implicit(false, 42).unwrap_err().to_string(),
        "yikes 42"
    );

    fn ensure_format_explicit(c: bool, y: u32) -> Result<()> {
        ensure!(c, "ouch {}", y + 1);
        Ok(())
    }
    assert!(ensure_format_explicit(true, 35).is_ok());
    assert_eq!(
        ensure_format_explicit(false, 35).unwrap_err().to_string(),
        "ouch 36"
    );

    fn ensure_core_error(c: bool) -> Result<()> {
        ensure!(c, TestError(13));
        Ok(())
    }
    assert!(ensure_core_error(true).is_ok());
    assert_eq!(
        ensure_core_error(false).unwrap_err().to_string(),
        "TestError(13)"
    );

    fn ensure_display(c: bool) -> Result<()> {
        let x = 1337;
        ensure!(c, x);
        Ok(())
    }
    assert!(ensure_display(true).is_ok());
    assert_eq!(ensure_display(false).unwrap_err().to_string(), "1337");

    fn ensure_bool_ref(c: &bool) -> Result<()> {
        ensure!(c, "whoops");
        Ok(())
    }
    assert!(ensure_bool_ref(&true).is_ok());
    assert_eq!(ensure_bool_ref(&false).unwrap_err().to_string(), "whoops");
}

#[test]
fn downcast() {
    // Error::msg(T)
    let error = Error::msg("uh oh");
    let error = error.downcast::<TestError>().unwrap_err();
    let error = error.downcast::<OutOfMemory>().unwrap_err();
    assert_eq!(error.downcast::<&str>().unwrap(), "uh oh");

    // Error::new()
    let error = Error::new(TestError(42));
    let error = error.downcast::<&str>().unwrap_err();
    let error = error.downcast::<OutOfMemory>().unwrap_err();
    assert_eq!(error.downcast::<TestError>().unwrap().0, 42);

    // Error::from(oom)
    let error = Error::from(OutOfMemory::new());
    let error = error.downcast::<&str>().unwrap_err();
    let error = error.downcast::<TestError>().unwrap_err();
    assert!(error.downcast::<OutOfMemory>().is_ok());

    // First in context chain.
    let error = Error::new(TestError(42))
        .context("yikes")
        .context(OutOfMemory::new());
    let error = error.downcast::<String>().unwrap_err();
    assert!(error.downcast::<OutOfMemory>().is_ok());

    // Middle in context chain.
    let error = Error::new(TestError(42))
        .context("yikes")
        .context(OutOfMemory::new());
    let error = error.downcast::<String>().unwrap_err();
    assert_eq!(error.downcast::<&str>().unwrap(), "yikes");

    // Last in context chain.
    let error = Error::new(TestError(42))
        .context("yikes")
        .context(OutOfMemory::new());
    let error = error.downcast::<String>().unwrap_err();
    assert_eq!(error.downcast::<TestError>().unwrap().0, 42);

    // Multiple `T`s in the context chain gives the first one.
    let error = Error::new(TestError(42)).context(TestError(36));
    assert_eq!(error.downcast::<TestError>().unwrap().0, 36);
}

#[test]
fn downcast_drops_everything() {
    // Error::new
    let drops = Arc::new(AtomicU32::new(0));
    let error = Error::new(CountDrops::new(&drops))
        .context(CountDrops::new(&drops))
        .context(CountDrops::new(&drops));
    assert_eq!(drops.load(SeqCst), 0);
    let c = error.downcast::<CountDrops>().unwrap();
    assert_eq!(drops.load(SeqCst), 2);
    drop(c);
    assert_eq!(drops.load(SeqCst), 3);

    // Error::msg
    let drops = Arc::new(AtomicU32::new(0));
    let error = Error::msg(CountDrops(drops.clone()))
        .context(CountDrops(drops.clone()))
        .context(CountDrops(drops.clone()));
    assert_eq!(drops.load(SeqCst), 0);
    let c = error.downcast::<CountDrops>().unwrap();
    assert_eq!(drops.load(SeqCst), 2);
    drop(c);
    assert_eq!(drops.load(SeqCst), 3);
}

#[test]
fn downcast_ref() {
    // `Error::msg(T)`
    let e = Error::msg("str");
    assert_eq!(e.downcast_ref::<&str>().copied().unwrap(), "str");
    assert!(e.downcast_ref::<TestError>().is_none());
    assert!(e.downcast_ref::<OutOfMemory>().is_none());

    // Context chain.
    let e = e.context(TestError(42));
    assert_eq!(e.downcast_ref::<&str>().copied().unwrap(), "str");
    assert_eq!(e.downcast_ref::<TestError>().unwrap().0, 42);
    assert!(e.downcast_ref::<OutOfMemory>().is_none());

    // Multiple `T`s in the context chain gives you the first one.
    let e = e.context("another str");
    assert_eq!(e.downcast_ref::<&str>().copied().unwrap(), "another str");
    assert_eq!(e.downcast_ref::<TestError>().unwrap().0, 42);
    assert!(e.downcast_ref::<OutOfMemory>().is_none());

    // `Error::from(T)`
    let e = Error::from(TestError(36));
    assert_eq!(e.downcast_ref::<TestError>().unwrap().0, 36);
    assert!(e.downcast_ref::<&str>().is_none());
    assert!(e.downcast_ref::<OutOfMemory>().is_none());

    // `Error::from(OutOfMemory)`
    let e = Error::from(OutOfMemory::new());
    assert!(e.downcast_ref::<OutOfMemory>().is_some());
    assert!(e.downcast_ref::<TestError>().is_none());
    assert!(e.downcast_ref::<&str>().is_none());
}

#[test]
fn downcast_mut() {
    // `Error::msg(T)`
    let mut e = Error::msg("str");
    assert!(e.downcast_mut::<TestError>().is_none());
    assert!(e.downcast_mut::<OutOfMemory>().is_none());
    *e.downcast_mut::<&str>().unwrap() = "whoops";
    assert_eq!(*e.downcast_ref::<&str>().unwrap(), "whoops");

    // Context chain.
    let mut e = e.context(TestError(42));
    assert!(e.downcast_mut::<OutOfMemory>().is_none());
    *e.downcast_mut::<&str>().unwrap() = "uh oh";
    assert_eq!(*e.downcast_ref::<&str>().unwrap(), "uh oh");
    e.downcast_mut::<TestError>().unwrap().0 += 1;
    assert_eq!(e.downcast_ref::<TestError>().unwrap().0, 43);

    // Multiple `T`s in the context chain gives you the first one.
    let mut e = e.context("another str");
    *e.downcast_mut::<&str>().unwrap() = "yikes";
    assert_eq!(*e.downcast_ref::<&str>().unwrap(), "yikes");
    assert_eq!(format!("{e:#}"), "yikes: TestError(43): uh oh");

    // `Error::from(T)`
    let mut e = Error::from(TestError(36));
    assert!(e.downcast_mut::<&str>().is_none());
    assert!(e.downcast_mut::<OutOfMemory>().is_none());
    e.downcast_mut::<TestError>().unwrap().0 += 1;
    assert_eq!(e.downcast_ref::<TestError>().unwrap().0, 37);

    // `Error::from(OutOfMemory)`
    let mut e = Error::from(OutOfMemory::new());
    assert!(e.downcast_mut::<OutOfMemory>().is_some());
    assert!(e.downcast_mut::<TestError>().is_none());
    assert!(e.downcast_mut::<&str>().is_none());
}

#[test]
fn context_on_oom() {
    let error = Error::new(OutOfMemory::new());
    let error = error.context("yikes");
    assert!(error.is::<OutOfMemory>());
    assert!(
        !error.is::<&str>(),
        "shouldn't attempt to box up more context when we've already exhausted memory"
    );
}

#[test]
fn context_on_ok_result() {
    let result: Result<u32> = Ok(42);
    let result = result.context("uh oh").context(TestError(1337));
    assert_eq!(result.unwrap(), 42);
}

#[test]
fn context_on_err_result() {
    let result: Result<u32> = Err(Error::new(TestError(42))).context("uh oh");
    let error = result.unwrap_err();

    assert!(error.is::<TestError>());
    assert_eq!(error.downcast_ref::<TestError>().unwrap().0, 42);

    assert!(error.is::<&str>());
    assert_eq!(error.downcast_ref::<&str>().copied().unwrap(), "uh oh");
}

#[test]
fn context_on_some_option() {
    let option = Some(42);
    let result = option.context("uh oh").context(TestError(1337));
    assert_eq!(result.unwrap(), 42);
}

#[test]
fn context_on_none_option() {
    let option: Option<u32> = None;
    let result = option.context(TestError(42)).context("uh oh");
    let error = result.unwrap_err();

    assert!(error.is::<TestError>());
    assert_eq!(error.downcast_ref::<TestError>().unwrap().0, 42);

    assert!(error.is::<&str>());
    assert_eq!(error.downcast_ref::<&str>().copied().unwrap(), "uh oh");
}

#[test]
fn with_context_on_ok_result() {
    let result: Result<u32> = Ok(42);
    let result = result
        .with_context(|| "uh oh")
        .with_context(|| TestError(36));
    assert_eq!(result.unwrap(), 42);
}

#[test]
fn with_context_on_err_result() {
    let result: Result<u32> = Err(Error::new(TestError(36)));
    let result = result.with_context(|| "uh oh");
    let error = result.unwrap_err();

    assert!(error.is::<TestError>());
    assert!(error.is::<&str>());
    assert_eq!(error.downcast_ref::<&str>().copied().unwrap(), "uh oh");
}

#[test]
fn with_context_on_some_option() {
    let option = Some(36);
    let result = option
        .with_context(|| "uh oh")
        .with_context(|| TestError(42));
    assert_eq!(result.unwrap(), 36);
}

#[test]
fn with_context_on_none_option() {
    let option: Option<u32> = None;
    let result = option
        .with_context(|| "uh oh")
        .with_context(|| TestError(42));
    let error = result.unwrap_err();

    assert!(error.is::<&str>());
    assert_eq!(error.downcast_ref::<&str>().copied().unwrap(), "uh oh");

    assert!(error.is::<TestError>());
    assert_eq!(error.downcast_ref::<TestError>().unwrap().0, 42);
}

#[test]
fn fmt_debug() {
    let error = Error::msg("whoops").context("uh oh").context("yikes");
    let actual = format!("{error:?}");

    let expected = "yikes\n\
                        \n\
                        Caused by:\n\
                        \t0: uh oh\n\
                        \t1: whoops\n";

    #[cfg(feature = "backtrace")]
    {
        assert!(actual.starts_with(expected));
        if let BacktraceStatus::Captured = error.backtrace().status() {
            assert!(actual.contains("Stack backtrace:"));
        }
    }

    #[cfg(not(feature = "backtrace"))]
    {
        assert_eq!(actual, expected);
    }
}

#[test]
fn fmt_debug_alternate() {
    let error = ChainError::new("root cause", None);
    let error = ChainError::new("whoops", Some(Box::new(error)));
    let error = Error::new(error)
        .context(TestError(42))
        .context("yikes")
        .context("ouch");

    let actual = format!("{error:#?}");
    let actual = actual.trim();
    println!("actual `{{:#?}}` output:\n{actual}");

    let expected = r#"
Error {
    inner: DynError {
        error: ouch,
        source: DynError {
            error: yikes,
            source: DynError {
                error: TestError(
                    42,
                ),
                source: DynError {
                    error: ChainError {
                        message: "whoops",
                        source: Some(
                            ChainError {
                                message: "root cause",
                                source: None,
                            },
                        ),
                    },
                },
            },
        },
    },
}
    "#
    .trim();
    println!("expected `{{:#?}}` output:\n{expected}");

    assert_eq!(actual, expected);
}

#[test]
fn fmt_debug_alternate_with_oom() {
    let error = Error::new(OutOfMemory::new());

    let actual = format!("{error:#?}");
    let actual = actual.trim();
    println!("actual `{{:#?}}` output:\n{actual}");

    let expected = r#"
Error {
    inner: Oom(
        OutOfMemory,
    ),
}
    "#
    .trim();
    println!("expected `{{:#?}}` output:\n{expected}");

    assert_eq!(actual, expected);
}

#[test]
fn fmt_display() {
    let error = Error::msg("whoops").context("uh oh").context("yikes");
    assert_eq!(format!("{error}"), "yikes");
}

#[test]
fn fmt_display_alternate() {
    let error = Error::msg("ouch")
        .context("whoops")
        .context("uh oh")
        .context("yikes");
    assert_eq!(format!("{error:#}"), "yikes: uh oh: whoops: ouch");
}

#[test]
fn chain() {
    let error = Error::msg("failure")
        .context("uh oh")
        .context(TestError(42));

    let mut chain = error.chain();

    let e = chain.next().unwrap();
    assert_eq!(e.to_string(), "TestError(42)");

    let e = chain.next().unwrap();
    assert_eq!(e.to_string(), "uh oh");

    let e = chain.next().unwrap();
    assert_eq!(e.to_string(), "failure");

    assert!(chain.next().is_none());

    for _ in 0..100 {
        assert!(chain.next().is_none(), "`Chain` is a fused iterator");
    }
}

#[test]
fn chain_on_error_with_source() {
    let error = ChainError::new("yikes", None);
    let error = ChainError::new("whoops", Some(Box::new(error)));
    let error = ChainError::new("uh oh", Some(Box::new(error)));
    let error = Error::new(error).context("ouch").context("oof");

    let mut chain = error.chain();

    let e = chain.next().unwrap();
    assert_eq!(e.to_string(), "oof");

    let e = chain.next().unwrap();
    assert_eq!(e.to_string(), "ouch");

    let e = chain.next().unwrap();
    assert_eq!(e.to_string(), "uh oh");

    let e = chain.next().unwrap();
    assert_eq!(e.to_string(), "whoops");

    let e = chain.next().unwrap();
    assert_eq!(e.to_string(), "yikes");

    assert!(chain.next().is_none());
}

#[test]
fn root_cause() {
    let error = ChainError::new("yikes", None);
    let error = ChainError::new("whoops", Some(Box::new(error)));
    let error = ChainError::new("uh oh", Some(Box::new(error)));
    let error = Error::new(error).context("ouch").context("oof");
    let root = error.root_cause();
    assert_eq!(root.to_string(), "yikes");
    assert!(root.source().is_none());
}

#[test]
fn chain_with_leaf_sources() {
    #[derive(Debug)]
    struct ErrorWithSource(String, Box<dyn core::error::Error + Send + Sync + 'static>);

    impl fmt::Display for ErrorWithSource {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(&self.0)
        }
    }

    impl core::error::Error for ErrorWithSource {
        fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
            Some(&*self.1)
        }
    }

    let error = Error::new(ErrorWithSource("leaf".to_string(), Box::new(TestError(42))))
        .context("oof")
        .context("wow");

    let mut chain = error.chain();

    let e = chain.next().unwrap();
    assert_eq!(e.to_string(), "wow");

    let e = chain.next().unwrap();
    assert_eq!(e.to_string(), "oof");

    let e = chain.next().unwrap();
    assert_eq!(e.to_string(), "leaf");

    let e = chain.next().unwrap();
    assert_eq!(e.to_string(), "TestError(42)");

    assert!(chain.next().is_none());
}
