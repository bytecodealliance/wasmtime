use super::Key;
use wasmtime::Result;
use wasmtime_environ::SecondaryMap;
use wasmtime_fuzzing::oom::OomTest;

#[test]
fn secondary_map_try_with_capacity() -> Result<()> {
    OomTest::new().test(|| {
        let _map = SecondaryMap::<Key, u32>::try_with_capacity(32)?;
        Ok(())
    })
}

#[test]
fn secondary_map_try_resize() -> Result<()> {
    OomTest::new().test(|| {
        let mut map = SecondaryMap::<Key, u32>::new();
        map.try_resize(100)?;
        Ok(())
    })
}

#[test]
fn secondary_map_try_insert() -> Result<()> {
    OomTest::new().test(|| {
        let mut map = SecondaryMap::<Key, u32>::new();
        map.try_insert(Key::from_u32(42), 100)?;
        Ok(())
    })
}
