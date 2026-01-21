use cranelift_bitset::CompoundBitSet;
use std::{
    alloc::{Layout, alloc},
    fmt::{self, Write},
    sync::atomic::{AtomicU32, Ordering::SeqCst},
};
use wasmtime::{Config, Error, Result, error::OutOfMemory, format_err};
use wasmtime_environ::collections::*;
use wasmtime_fuzzing::oom::{OomTest, OomTestAllocator};

#[global_allocator]
static GLOBAL_ALOCATOR: OomTestAllocator = OomTestAllocator::new();

#[test]
fn smoke_test_ok() -> Result<()> {
    OomTest::new().test(|| Ok(()))
}

#[test]
fn smoke_test_missed_oom() -> Result<()> {
    let err = OomTest::new()
        .test(|| {
            let _ = unsafe { alloc(Layout::new::<u64>()) };
            Ok(())
        })
        .unwrap_err();
    let err = format!("{err:?}");
    assert!(
        err.contains("OOM test function missed an OOM"),
        "should have missed an OOM, got: {err}"
    );
    Ok(())
}

#[test]
#[cfg(arc_try_new)]
fn try_new_arc() -> Result<()> {
    use std::sync::Arc;

    OomTest::new().test(|| {
        let _arc = try_new::<Arc<u32>>(42)?;
        Ok(())
    })
}

#[test]
fn try_new_box() -> Result<()> {
    OomTest::new().test(|| {
        let _box = try_new::<Box<u32>>(36)?;
        Ok(())
    })
}

#[test]
fn compound_bit_set_try_with_capacity() -> Result<()> {
    OomTest::new().test(|| {
        let _bitset = CompoundBitSet::<usize>::try_with_capacity(32)?;
        Ok(())
    })
}

#[test]
fn compound_bit_set_try_ensure_capacity() -> Result<()> {
    OomTest::new().test(|| {
        let mut bitset = CompoundBitSet::new();
        bitset.try_ensure_capacity(100)?;
        Ok(())
    })
}

#[test]
fn config_new() -> Result<()> {
    OomTest::new().test(|| {
        let mut config = Config::new();
        config.enable_compiler(false);
        Ok(())
    })
}

fn ok_if_not_oom(error: Error) -> Result<()> {
    if error.is::<OutOfMemory>() {
        Err(error)
    } else {
        Ok(())
    }
}

#[test]
fn error_new() -> Result<()> {
    OomTest::new().test(|| {
        let error = Error::new(u8::try_from(u32::MAX).unwrap_err());
        ok_if_not_oom(error)
    })
}

#[test]
fn error_msg() -> Result<()> {
    OomTest::new().test(|| {
        let error = Error::msg("ouch");
        ok_if_not_oom(error)
    })
}

static X: AtomicU32 = AtomicU32::new(42);

#[test]
fn error_fmt() -> Result<()> {
    OomTest::new().test(|| {
        let x = X.load(SeqCst);
        let error = format_err!("ouch: {x}");
        ok_if_not_oom(error)
    })
}

#[test]
fn error_context() -> Result<()> {
    OomTest::new().test(|| {
        let error = Error::msg("hello");
        let error = error.context("goodbye");
        ok_if_not_oom(error)
    })
}

#[test]
fn error_chain() -> Result<()> {
    OomTest::new().test(|| {
        let error = Error::msg("hello");
        let error = error.context("goodbye");
        for _ in error.chain() {
            // Nothing to do here, just exercising the iteration.
        }
        ok_if_not_oom(error)
    })
}

struct Null;
impl Write for Null {
    fn write_str(&mut self, _s: &str) -> fmt::Result {
        Ok(())
    }
}

#[test]
fn display_fmt_error() -> Result<()> {
    OomTest::new().test(|| {
        let error = Error::msg("hello");
        let error = error.context("goodbye");
        write!(&mut Null, "{error}").unwrap();
        ok_if_not_oom(error)
    })
}

#[test]
fn alternate_display_fmt_error() -> Result<()> {
    OomTest::new().test(|| {
        let error = Error::msg("hello");
        let error = error.context("goodbye");
        write!(&mut Null, "{error:?}").unwrap();
        ok_if_not_oom(error)
    })
}

#[test]
fn debug_fmt_error() -> Result<()> {
    OomTest::new().test(|| {
        let error = Error::msg("hello");
        let error = error.context("goodbye");
        write!(&mut Null, "{error:?}").unwrap();
        ok_if_not_oom(error)
    })
}

#[test]
fn alternate_debug_fmt_error() -> Result<()> {
    OomTest::new().test(|| {
        let error = Error::msg("hello");
        let error = error.context("goodbye");
        write!(&mut Null, "{error:#?}").unwrap();
        ok_if_not_oom(error)
    })
}
