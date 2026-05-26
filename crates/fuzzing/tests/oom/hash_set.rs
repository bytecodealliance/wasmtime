use wasmtime::Result;
use wasmtime_environ::collections::TryHashSet;
use wasmtime_fuzzing::oom::OomTest;

#[test]
fn try_hash_set_with_capacity() -> Result<()> {
    OomTest::new().test(|| {
        let _s = TryHashSet::<usize>::with_capacity(100)?;
        Ok(())
    })
}

#[test]
fn try_hash_set_reserve() -> Result<()> {
    OomTest::new().test(|| {
        let mut set = TryHashSet::<usize>::new();
        set.reserve(100)?;
        Ok(())
    })
}

#[test]
fn try_hash_set_insert() -> Result<()> {
    OomTest::new().test(|| {
        let mut set = TryHashSet::<usize>::new();
        for i in 0..1024 {
            set.insert(i)?;
        }
        for i in 0..1024 {
            set.insert(i)?;
        }
        Ok(())
    })
}
