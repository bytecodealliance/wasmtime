use super::Key;
use wasmtime::Result;
use wasmtime_environ::PrimaryMap;
use wasmtime_fuzzing::oom::OomTest;

#[test]
fn primary_map_try_with_capacity() -> Result<()> {
    OomTest::new().test(|| {
        let _map = PrimaryMap::<Key, u32>::try_with_capacity(32)?;
        Ok(())
    })
}

#[test]
fn primary_map_try_reserve() -> Result<()> {
    OomTest::new().test(|| {
        let mut map = PrimaryMap::<Key, u32>::new();
        map.try_reserve(100)?;
        Ok(())
    })
}

#[test]
fn primary_map_try_reserve_exact() -> Result<()> {
    OomTest::new().test(|| {
        let mut map = PrimaryMap::<Key, u32>::new();
        map.try_reserve_exact(13)?;
        Ok(())
    })
}
