use super::Key;
use cranelift_bforest::{Set, SetForest};
use wasmtime::Result;
use wasmtime_fuzzing::oom::OomTest;

#[test]
fn bforest_set() -> Result<()> {
    OomTest::new().test(|| {
        let mut forest = SetForest::new();
        let mut set = Set::new();
        for i in 0..100 {
            set.try_insert(Key(i), &mut forest, &())?;
        }
        for i in 0..100 {
            assert!(set.contains(Key(i), &forest, &()));
        }
        Ok(())
    })
}
