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
