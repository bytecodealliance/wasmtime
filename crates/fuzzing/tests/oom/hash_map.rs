use wasmtime::Result;
use wasmtime_environ::collections::{TryClone, TryHashMap};
use wasmtime_fuzzing::oom::OomTest;

#[test]
fn try_hash_map_with_capacity() -> Result<()> {
    OomTest::new().test(|| {
        let _s = TryHashMap::<usize, usize>::with_capacity(100)?;
        Ok(())
    })
}

#[test]
fn try_hash_map_reserve() -> Result<()> {
    OomTest::new().test(|| {
        let mut map = TryHashMap::<usize, usize>::new();
        map.reserve(100)?;
        Ok(())
    })
}

#[test]
fn try_hash_map_insert() -> Result<()> {
    OomTest::new().test(|| {
        let mut map = TryHashMap::<usize, usize>::new();
        for i in 0..1024 {
            map.insert(i, i * 2)?;
        }
        for i in 0..1024 {
            map.insert(i, i * 2)?;
        }
        Ok(())
    })
}

#[test]
fn try_hash_map_try_clone() -> Result<()> {
    OomTest::new().test(|| {
        let mut map = TryHashMap::new();
        for i in 0..10 {
            map.insert(i, i * 2)?;
        }
        let map2 = map.try_clone()?;
        assert_eq!(map, map2);
        Ok(())
    })
}
