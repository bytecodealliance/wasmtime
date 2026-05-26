use super::Key;
use wasmtime::Result;
use wasmtime_environ::collections::TryEntitySet;
use wasmtime_fuzzing::oom::OomTest;

#[test]
fn try_entity_set_ensure_capacity() -> Result<()> {
    OomTest::new().test(|| {
        let mut set = TryEntitySet::<Key>::new();
        set.ensure_capacity(100)?;
        Ok(())
    })
}

#[test]
fn try_entity_set_insert() -> Result<()> {
    OomTest::new().test(|| {
        let mut set = TryEntitySet::<Key>::new();
        set.insert(Key::from_u32(256))?;
        Ok(())
    })
}
