use wasmtime::Result;
use wasmtime_environ::collections::{TryBTreeMap, btree_map::Entry};
use wasmtime_fuzzing::oom::OomTest;

type M = TryBTreeMap<usize, f32>;

#[test]
fn btree_map() -> Result<()> {
    OomTest::new().test(|| {
        let mut m = M::new();

        m.insert(100, 100.0)?;

        m.entry(0).or_insert(99.0)?;
        m.entry(0).or_default()?;

        match m.entry(1) {
            Entry::Occupied(_) => unreachable!(),
            Entry::Vacant(e) => {
                let e = e.insert_entry(42.0)?;
                e.insert(43.0);
            }
        };

        match m.entry(1) {
            Entry::Occupied(e) => {
                e.remove_entry();
            }
            Entry::Vacant(_) => unreachable!(),
        }

        match m.entry(2) {
            Entry::Occupied(_) => unreachable!(),
            Entry::Vacant(e) => {
                e.insert(99.0)?;
            }
        };

        let _ = m.iter().count();
        let _ = m.iter_mut().count();
        let _ = m.keys().count();
        let _ = m.values().count();
        let _ = m.values_mut().count();
        let _ = m.range(..3).count();
        let _ = m.range_mut(..3).count();

        Ok(())
    })
}
