use wasmtime::Result;
use wasmtime_environ::collections::{TryClone, TryIndexMap, try_new};
use wasmtime_fuzzing::oom::OomTest;

#[test]
fn try_index_map_try_clone() -> Result<()> {
    OomTest::new()
        // `indexmap` will first try to double its capacity, and, if that fails,
        // will then try to allocate only as much as it absolutely must.
        .allow_alloc_after_oom(true)
        .test(|| {
            let mut map1 = TryIndexMap::new();
            map1.insert("a", try_new::<Box<_>>(42)?)?;
            map1.insert("b", try_new::<Box<_>>(36)?)?;
            let map2 = map1.try_clone()?;
            assert_eq!(map1, map2);
            Ok(())
        })
}

#[test]
fn try_index_map_with_capacity() -> Result<()> {
    OomTest::new()
        // `indexmap` will first try to double its capacity, and, if that fails,
        // will then try to allocate only as much as it absolutely must.
        .allow_alloc_after_oom(true)
        .test(|| {
            let _map = TryIndexMap::<&str, usize>::with_capacity(100)?;
            Ok(())
        })
}

#[test]
fn try_index_map_split_off() -> Result<()> {
    OomTest::new()
        // `indexmap` will first try to double its capacity, and, if that fails,
        // will then try to allocate only as much as it absolutely must.
        .allow_alloc_after_oom(true)
        .test(|| {
            let mut map1 = TryIndexMap::new();
            map1.insert("a", 42)?;
            map1.insert("b", 36)?;

            let map2 = map1.split_off(1)?;

            assert_eq!(map1.len(), 1);
            assert_eq!(map2.len(), 1);
            assert_eq!(map1[&"a"], 42);
            assert_eq!(map1[0], 42);
            assert_eq!(map2[&"b"], 36);
            assert_eq!(map2[0], 36);

            Ok(())
        })
}

#[test]
fn try_index_map_reserve() -> Result<()> {
    OomTest::new()
        // `indexmap` will first try to double its capacity, and, if that fails,
        // will then try to allocate only as much as it absolutely must.
        .allow_alloc_after_oom(true)
        .test(|| {
            let mut map = TryIndexMap::<u32, u32>::new();
            map.reserve(100)?;
            Ok(())
        })
}

#[test]
fn try_index_map_reserve_exact() -> Result<()> {
    OomTest::new().test(|| {
        let mut map = TryIndexMap::<u32, u32>::new();
        map.reserve_exact(100)?;
        Ok(())
    })
}

#[test]
fn try_index_map_insert() -> Result<()> {
    OomTest::new()
        // `indexmap` will first try to double its capacity, and, if that fails,
        // will then try to allocate only as much as it absolutely must.
        .allow_alloc_after_oom(true)
        .test(|| {
            let mut map = TryIndexMap::new();
            map.insert(10, 20)?;
            Ok(())
        })
}

#[test]
fn try_index_map_insert_full() -> Result<()> {
    OomTest::new()
        // `indexmap` will first try to double its capacity, and, if that fails,
        // will then try to allocate only as much as it absolutely must.
        .allow_alloc_after_oom(true)
        .test(|| {
            let mut map = TryIndexMap::new();
            map.insert_full(10, 20)?;
            Ok(())
        })
}

#[test]
fn try_index_map_insert_sorted() -> Result<()> {
    OomTest::new()
        // `indexmap` will first try to double its capacity, and, if that fails,
        // will then try to allocate only as much as it absolutely must.
        .allow_alloc_after_oom(true)
        .test(|| {
            let mut map = TryIndexMap::new();
            map.insert_sorted(10, 20)?;
            Ok(())
        })
}

#[test]
fn try_index_map_insert_sorted_by() -> Result<()> {
    OomTest::new()
        // `indexmap` will first try to double its capacity, and, if that fails,
        // will then try to allocate only as much as it absolutely must.
        .allow_alloc_after_oom(true)
        .test(|| {
            let mut map = TryIndexMap::new();
            map.insert_sorted_by(10, 20, |_k, _v, _k2, _v2| core::cmp::Ordering::Less)?;
            Ok(())
        })
}

#[test]
fn try_index_map_insert_sorted_by_key() -> Result<()> {
    OomTest::new()
        // `indexmap` will first try to double its capacity, and, if that fails,
        // will then try to allocate only as much as it absolutely must.
        .allow_alloc_after_oom(true)
        .test(|| {
            let mut map = TryIndexMap::new();
            map.insert_sorted_by_key(10, 20, |_k, v| *v)?;
            Ok(())
        })
}

#[test]
fn try_index_map_insert_before() -> Result<()> {
    OomTest::new()
        // `indexmap` will first try to double its capacity, and, if that fails,
        // will then try to allocate only as much as it absolutely must.
        .allow_alloc_after_oom(true)
        .test(|| {
            let mut map = TryIndexMap::new();
            map.insert("a", 20)?;
            map.insert("b", 30)?;
            map.insert_before(1, "c", 40)?;
            assert_eq!(map[0], 20);
            assert_eq!(map[1], 40);
            assert_eq!(map[2], 30);
            Ok(())
        })
}

#[test]
fn try_index_map_shift_insert() -> Result<()> {
    OomTest::new()
        // `indexmap` will first try to double its capacity, and, if that fails,
        // will then try to allocate only as much as it absolutely must.
        .allow_alloc_after_oom(true)
        .test(|| {
            let mut map = TryIndexMap::new();
            map.insert("a", 20)?;
            map.insert("b", 30)?;
            map.shift_insert(1, "c", 40)?;
            assert_eq!(map[0], 20);
            assert_eq!(map[1], 40);
            assert_eq!(map[2], 30);
            Ok(())
        })
}
