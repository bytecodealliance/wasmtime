use wasmtime::Result;
use wasmtime_environ::collections::TryString;
use wasmtime_fuzzing::oom::OomTest;

#[test]
fn try_string_with_capacity() -> Result<()> {
    OomTest::new().test(|| {
        let _s = TryString::with_capacity(100)?;
        Ok(())
    })
}

#[test]
fn try_string_reserve() -> Result<()> {
    OomTest::new().test(|| {
        let mut s = TryString::new();
        s.reserve(10)?;
        Ok(())
    })
}

#[test]
fn try_string_reserve_exact() -> Result<()> {
    OomTest::new().test(|| {
        let mut s = TryString::new();
        s.reserve_exact(3)?;
        Ok(())
    })
}

#[test]
fn try_string_push() -> Result<()> {
    OomTest::new().test(|| {
        let mut s = TryString::new();
        s.push('c')?;
        Ok(())
    })
}

#[test]
fn try_string_push_str() -> Result<()> {
    OomTest::new().test(|| {
        let mut s = TryString::new();
        s.push_str("hello")?;
        Ok(())
    })
}

#[test]
fn try_string_shrink_to_fit() -> Result<()> {
    OomTest::new().test(|| {
        // len == cap == 0
        let mut s = TryString::new();
        s.shrink_to_fit()?;

        // len == 0 < cap
        let mut s = TryString::with_capacity(4)?;
        s.shrink_to_fit()?;

        // 0 < len < cap
        let mut s = TryString::with_capacity(4)?;
        s.push('a')?;
        s.shrink_to_fit()?;

        // 0 < len == cap
        let mut s = TryString::new();
        s.reserve_exact(2)?;
        s.push('a')?;
        s.push('a')?;
        s.shrink_to_fit()?;

        Ok(())
    })
}

#[test]
fn try_string_into_boxed_str() -> Result<()> {
    OomTest::new().test(|| {
        // len == cap == 0
        let s = TryString::new();
        let _ = s.into_boxed_str()?;

        // len == 0 < cap
        let s = TryString::with_capacity(4)?;
        let _ = s.into_boxed_str()?;

        // 0 < len < cap
        let mut s = TryString::with_capacity(4)?;
        s.push('a')?;
        let _ = s.into_boxed_str()?;

        // 0 < len == cap
        let mut s = TryString::new();
        s.reserve_exact(2)?;
        s.push('a')?;
        s.push('a')?;
        let _ = s.into_boxed_str()?;

        Ok(())
    })
}
