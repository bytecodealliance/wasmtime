use super::Key;
use cranelift_bforest::{Map, MapForest};
use wasmtime::Result;
use wasmtime_fuzzing::oom::OomTest;

#[test]
fn bforest_map() -> Result<()> {
    OomTest::new().test(|| {
        let mut forest = MapForest::new();
        let mut map = Map::new();
        for i in 0..100 {
            map.try_insert(Key(i), i, &mut forest, &())?;
        }
        for i in 0..100 {
            assert_eq!(map.get(Key(i), &forest, &()), Some(i));
        }
        Ok(())
    })
}

#[test]
fn bforest_map_failed_insert_preserves_entries() -> Result<()> {
    OomTest::new()
        // Allow the assertion below to allocate its panic message.
        .allow_alloc_after_oom(true)
        .alloc_succeeds_after_oom(true)
        .test(|| {
            let mut forest = MapForest::new();
            let mut map = Map::new();
            for i in 0..1000 {
                if let Err(e) = map.try_insert(Key(i), i, &mut forest, &()) {
                    // A failed insert must leave every existing entry in place.
                    for j in 0..i {
                        assert_eq!(
                            map.get(Key(j), &forest, &()),
                            Some(j),
                            "key {j} lost after failed insert of key {i}"
                        );
                    }
                    return Err(e.into());
                }
            }
            Ok(())
        })
}
