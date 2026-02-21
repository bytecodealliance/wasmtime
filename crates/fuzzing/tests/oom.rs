use cranelift_bitset::CompoundBitSet;
use std::{
    alloc::{Layout, alloc, dealloc},
    fmt::{self, Write},
    iter,
    ops::Deref,
    sync::atomic::{AtomicU32, Ordering::SeqCst},
};
use wasmtime::{error::OutOfMemory, *};
use wasmtime_core::alloc::TryCollect;
use wasmtime_environ::{
    EntityRef, PrimaryMap,
    collections::{self, *},
};
use wasmtime_fuzzing::oom::{OomTest, OomTestAllocator};

#[global_allocator]
static GLOBAL_ALOCATOR: OomTestAllocator = OomTestAllocator::new();

/// RAII wrapper around a raw allocation to deallocate it on drop.
struct Alloc {
    layout: Layout,
    ptr: *mut u8,
}

impl Drop for Alloc {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe {
                dealloc(self.ptr, self.layout);
            }
        }
    }
}

impl Deref for Alloc {
    type Target = *mut u8;

    fn deref(&self) -> &Self::Target {
        &self.ptr
    }
}

impl Alloc {
    /// Safety: same as `std::alloc::alloc`.
    unsafe fn new(layout: Layout) -> Self {
        let ptr = unsafe { alloc(layout) };
        Alloc { layout, ptr }
    }
}

#[test]
fn smoke_test_ok() -> Result<()> {
    OomTest::new().test(|| Ok(()))
}

#[test]
fn smoke_test_missed_oom() -> Result<()> {
    let err = OomTest::new()
        .test(|| unsafe {
            let _ = Alloc::new(Layout::new::<u64>());
            Ok(())
        })
        .unwrap_err();
    let err = format!("{err:?}");
    assert!(
        err.contains("OOM test function missed an OOM"),
        "should have missed an OOM, got: {err}"
    );
    Ok(())
}

#[test]
fn smoke_test_disallow_alloc_after_oom() -> Result<()> {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = OomTest::new().test(|| unsafe {
            let layout = Layout::new::<u64>();
            let p = Alloc::new(layout);
            let _q = Alloc::new(layout);
            if p.is_null() {
                Err(OutOfMemory::new(layout.size()).into())
            } else {
                Ok(())
            }
        });
    }));
    assert!(result.is_err());
    Ok(())
}

#[test]
fn smoke_test_allow_alloc_after_oom() -> Result<()> {
    OomTest::new().allow_alloc_after_oom(true).test(|| unsafe {
        let layout = Layout::new::<u64>();
        let p = Alloc::new(layout);
        let q = Alloc::new(layout);
        if p.is_null() || q.is_null() {
            Err(OutOfMemory::new(layout.size()).into())
        } else {
            Ok(())
        }
    })
}

#[test]
#[cfg(arc_try_new)]
fn try_new_arc() -> Result<()> {
    use std::sync::Arc;

    OomTest::new().test(|| {
        let _arc = try_new::<Arc<u32>>(42)?;
        Ok(())
    })
}

#[test]
fn try_new_box() -> Result<()> {
    OomTest::new().test(|| {
        let _box = try_new::<Box<u32>>(36)?;
        Ok(())
    })
}

#[test]
fn compound_bit_set_try_with_capacity() -> Result<()> {
    OomTest::new().test(|| {
        let _bitset = CompoundBitSet::<usize>::try_with_capacity(32)?;
        Ok(())
    })
}

#[test]
fn compound_bit_set_try_ensure_capacity() -> Result<()> {
    OomTest::new().test(|| {
        let mut bitset = CompoundBitSet::new();
        bitset.try_ensure_capacity(100)?;
        Ok(())
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Key(u32);
wasmtime_environ::entity_impl!(Key);

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

#[test]
fn secondary_map_with_capacity() -> Result<()> {
    OomTest::new().test(|| {
        let _map = SecondaryMap::<Key, u32>::with_capacity(32)?;
        Ok(())
    })
}

#[test]
fn secondary_map_resize() -> Result<()> {
    OomTest::new().test(|| {
        let mut map = SecondaryMap::<Key, u32>::new();
        map.resize(100)?;
        Ok(())
    })
}

#[test]
fn secondary_map_insert() -> Result<()> {
    OomTest::new().test(|| {
        let mut map = SecondaryMap::<Key, u32>::new();
        map.insert(Key::from_u32(42), 100)?;
        Ok(())
    })
}

#[test]
fn secondary_map_remove() -> Result<()> {
    OomTest::new().test(|| {
        let mut default = Vec::new();
        default.push(3)?;
        default.push(3)?;
        default.push(3)?;

        let mut map = collections::SecondaryMap::<Key, Vec<u32>>::with_default(default);
        assert_eq!(&*map[Key::new(0)], &[3, 3, 3]);

        map.insert(Key::new(0), Vec::new())?;
        assert!(map[Key::new(0)].is_empty());

        // This may fail because it requires `TryClone`ing the default value.
        let old = map.remove(Key::new(0))?;

        assert!(old.unwrap().is_empty());
        assert_eq!(&*map[Key::new(0)], &[3, 3, 3]);

        Ok(())
    })
}

#[test]
fn entity_set_ensure_capacity() -> Result<()> {
    OomTest::new().test(|| {
        let mut set = EntitySet::<Key>::new();
        set.ensure_capacity(100)?;
        Ok(())
    })
}

#[test]
fn entity_set_insert() -> Result<()> {
    OomTest::new().test(|| {
        let mut set = EntitySet::<Key>::new();
        set.insert(Key::from_u32(256))?;
        Ok(())
    })
}

#[test]
fn hash_set_with_capacity() -> Result<()> {
    OomTest::new().test(|| {
        let _s = HashSet::<usize>::with_capacity(100)?;
        Ok(())
    })
}

#[test]
fn hash_set_reserve() -> Result<()> {
    OomTest::new().test(|| {
        let mut set = HashSet::<usize>::new();
        set.reserve(100)?;
        Ok(())
    })
}

#[test]
fn hash_set_insert() -> Result<()> {
    OomTest::new().test(|| {
        let mut set = HashSet::<usize>::new();
        for i in 0..1024 {
            set.insert(i)?;
        }
        for i in 0..1024 {
            set.insert(i)?;
        }
        Ok(())
    })
}

#[test]
fn hash_map_with_capacity() -> Result<()> {
    OomTest::new().test(|| {
        let _s = HashMap::<usize, usize>::with_capacity(100)?;
        Ok(())
    })
}

#[test]
fn hash_map_reserve() -> Result<()> {
    OomTest::new().test(|| {
        let mut map = HashMap::<usize, usize>::new();
        map.reserve(100)?;
        Ok(())
    })
}

#[test]
fn hash_map_insert() -> Result<()> {
    OomTest::new().test(|| {
        let mut map = HashMap::<usize, usize>::new();
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
fn hash_map_try_clone() -> Result<()> {
    OomTest::new().test(|| {
        let mut map = HashMap::new();
        for i in 0..10 {
            map.insert(i, i * 2)?;
        }
        let map2 = map.try_clone()?;
        assert_eq!(map, map2);
        Ok(())
    })
}

#[test]
fn vec_with_capacity() -> Result<()> {
    OomTest::new().test(|| {
        let _v = wasmtime_environ::collections::Vec::<usize>::with_capacity(100)?;
        Ok(())
    })
}

#[test]
fn vec_reserve() -> Result<()> {
    OomTest::new().test(|| {
        let mut v = wasmtime_environ::collections::Vec::<usize>::new();
        v.reserve(10)?;
        Ok(())
    })
}

#[test]
fn vec_reserve_exact() -> Result<()> {
    OomTest::new().test(|| {
        let mut v = wasmtime_environ::collections::Vec::<usize>::new();
        v.reserve_exact(3)?;
        Ok(())
    })
}

#[test]
fn vec_push() -> Result<()> {
    OomTest::new().test(|| {
        let mut v = wasmtime_environ::collections::Vec::new();
        v.push(42)?;
        Ok(())
    })
}

#[test]
fn string_with_capacity() -> Result<()> {
    OomTest::new().test(|| {
        let _s = String::with_capacity(100)?;
        Ok(())
    })
}

#[test]
fn string_reserve() -> Result<()> {
    OomTest::new().test(|| {
        let mut s = String::new();
        s.reserve(10)?;
        Ok(())
    })
}

#[test]
fn string_reserve_exact() -> Result<()> {
    OomTest::new().test(|| {
        let mut s = String::new();
        s.reserve_exact(3)?;
        Ok(())
    })
}

#[test]
fn string_push() -> Result<()> {
    OomTest::new().test(|| {
        let mut s = String::new();
        s.push('c')?;
        Ok(())
    })
}

#[test]
fn string_push_str() -> Result<()> {
    OomTest::new().test(|| {
        let mut s = String::new();
        s.push_str("hello")?;
        Ok(())
    })
}

#[test]
fn string_shrink_to_fit() -> Result<()> {
    OomTest::new().test(|| {
        // len == cap == 0
        let mut s = String::new();
        s.shrink_to_fit()?;

        // len == 0 < cap
        let mut s = String::with_capacity(4)?;
        s.shrink_to_fit()?;

        // 0 < len < cap
        let mut s = String::with_capacity(4)?;
        s.push('a')?;
        s.shrink_to_fit()?;

        // 0 < len == cap
        let mut s = String::new();
        s.reserve_exact(2)?;
        s.push('a')?;
        s.push('a')?;
        s.shrink_to_fit()?;

        Ok(())
    })
}

#[test]
fn string_into_boxed_str() -> Result<()> {
    OomTest::new().test(|| {
        // len == cap == 0
        let s = String::new();
        let _ = s.into_boxed_str()?;

        // len == 0 < cap
        let s = String::with_capacity(4)?;
        let _ = s.into_boxed_str()?;

        // 0 < len < cap
        let mut s = String::with_capacity(4)?;
        s.push('a')?;
        let _ = s.into_boxed_str()?;

        // 0 < len == cap
        let mut s = String::new();
        s.reserve_exact(2)?;
        s.push('a')?;
        s.push('a')?;
        let _ = s.into_boxed_str()?;

        Ok(())
    })
}

#[test]
fn config_new() -> Result<()> {
    OomTest::new().test(|| {
        let mut config = Config::new();
        config.enable_compiler(false);
        Ok(())
    })
}

#[test]
#[cfg(arc_try_new)]
fn engine_new() -> Result<()> {
    OomTest::new().test(|| {
        let mut config = Config::new();
        config.enable_compiler(false);
        let _ = Engine::new(&config)?;
        Ok(())
    })
}

#[test]
#[cfg(arc_try_new)]
fn func_type_try_new() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    let engine = Engine::new(&config)?;

    // Run this OOM test a few times to make sure that we leave the engine's
    // type registry in a good state when failing to register new types.
    for i in 1..6 {
        OomTest::new().test(|| {
            let ty1 = FuncType::try_new(
                &engine,
                std::iter::repeat(ValType::ANYREF).take(i),
                std::iter::repeat(ValType::ANYREF).take(i),
            )?;
            assert_eq!(ty1.params().len(), i);
            assert_eq!(ty1.results().len(), i);

            let ty2 = FuncType::try_new(
                &engine,
                std::iter::repeat(ValType::ANYREF).take(i),
                std::iter::repeat(ValType::ANYREF).take(i),
            )?;
            assert_eq!(ty2.params().len(), i);
            assert_eq!(ty2.results().len(), i);

            let ty3 = FuncType::try_new(&engine, [], [])?;
            assert_eq!(ty3.params().len(), 0);
            assert_eq!(ty3.results().len(), 0);

            assert!(
                !FuncType::eq(&ty2, &ty3),
                "{ty2:?} should not be equal to {ty3:?}"
            );

            Ok(())
        })?;
    }

    Ok(())
}

#[test]
#[cfg(arc_try_new)]
fn linker_new() -> Result<()> {
    OomTest::new().test(|| {
        let mut config = Config::new();
        config.enable_compiler(false);
        let engine = Engine::new(&config)?;
        let _linker = Linker::<()>::new(&engine);
        Ok(())
    })
}

#[test]
#[cfg(arc_try_new)]
fn linker_func_wrap() -> Result<()> {
    OomTest::new().test(|| {
        let mut config = Config::new();
        config.enable_compiler(false);
        let engine = Engine::new(&config)?;
        let mut linker = Linker::<()>::new(&engine);
        linker.func_wrap("module", "func", |x: i32| x * 2)?;
        Ok(())
    })
}

#[test]
#[cfg(arc_try_new)]
fn store_try_new() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    OomTest::new().test(|| {
        let _ = Store::try_new(&engine, ())?;
        Ok(())
    })
}

fn ok_if_not_oom(error: Error) -> Result<()> {
    if error.is::<OutOfMemory>() {
        Err(error)
    } else {
        Ok(())
    }
}

#[test]
fn error_new() -> Result<()> {
    OomTest::new().test(|| {
        let error = Error::new(u8::try_from(u32::MAX).unwrap_err());
        ok_if_not_oom(error)
    })
}

#[test]
fn error_msg() -> Result<()> {
    OomTest::new().test(|| {
        let error = Error::msg("ouch");
        ok_if_not_oom(error)
    })
}

static X: AtomicU32 = AtomicU32::new(42);

#[test]
fn error_fmt() -> Result<()> {
    OomTest::new().test(|| {
        let x = X.load(SeqCst);
        let error = format_err!("ouch: {x}");
        ok_if_not_oom(error)
    })
}

#[test]
fn error_context() -> Result<()> {
    OomTest::new().test(|| {
        let error = Error::msg("hello");
        let error = error.context("goodbye");
        ok_if_not_oom(error)
    })
}

#[test]
fn error_chain() -> Result<()> {
    OomTest::new().test(|| {
        let error = Error::msg("hello");
        let error = error.context("goodbye");
        for _ in error.chain() {
            // Nothing to do here, just exercising the iteration.
        }
        ok_if_not_oom(error)
    })
}

struct Null;
impl Write for Null {
    fn write_str(&mut self, _s: &str) -> fmt::Result {
        Ok(())
    }
}

#[test]
fn display_fmt_error() -> Result<()> {
    OomTest::new().test(|| {
        let error = Error::msg("hello");
        let error = error.context("goodbye");
        write!(&mut Null, "{error}").unwrap();
        ok_if_not_oom(error)
    })
}

#[test]
fn alternate_display_fmt_error() -> Result<()> {
    OomTest::new().test(|| {
        let error = Error::msg("hello");
        let error = error.context("goodbye");
        write!(&mut Null, "{error:?}").unwrap();
        ok_if_not_oom(error)
    })
}

#[test]
fn debug_fmt_error() -> Result<()> {
    OomTest::new().test(|| {
        let error = Error::msg("hello");
        let error = error.context("goodbye");
        write!(&mut Null, "{error:?}").unwrap();
        ok_if_not_oom(error)
    })
}

#[test]
fn alternate_debug_fmt_error() -> Result<()> {
    OomTest::new().test(|| {
        let error = Error::msg("hello");
        let error = error.context("goodbye");
        write!(&mut Null, "{error:#?}").unwrap();
        ok_if_not_oom(error)
    })
}

#[test]
fn vec_and_boxed_slice() -> Result<()> {
    use wasmtime_core::alloc::Vec;

    OomTest::new().test(|| {
        // Nonzero-sized type.
        let mut vec = Vec::new();
        vec.push(1)?;
        let slice = vec.into_boxed_slice()?; // len > 0, cap > 0

        let mut vec = Vec::from(slice);
        vec.pop();
        let slice = vec.into_boxed_slice()?; // len = 0, cap > 0

        let vec = Vec::from(slice);
        let _slice = vec.into_boxed_slice()?; // len = 0, cap = 0

        let mut vec = Vec::new();
        vec.reserve_exact(3)?;
        vec.push(2)?;
        vec.push(2)?;
        vec.push(2)?;
        let _slice = vec.into_boxed_slice()?; // len = cap, len > 0

        for i in 0..12 {
            let mut vec = Vec::new();
            for j in 0..i {
                vec.push(j)?;
            }
            let _slice = vec.into_boxed_slice()?; // len ?= cap
        }

        // Zero-sized type.
        let mut vec = Vec::new();
        vec.push(())?;
        let slice = vec.into_boxed_slice()?; // len > 0, cap > 0
        let mut vec = Vec::from(slice);
        vec.pop();
        let slice = vec.into_boxed_slice()?; // len = 0, cap > 0
        let vec = Vec::from(slice);
        let _ = vec.into_boxed_slice()?; // len = 0, cap = 0

        Ok(())
    })
}

#[test]
fn vec_shrink_to_fit() -> Result<()> {
    use wasmtime_core::alloc::Vec;

    #[derive(Default)]
    struct ZeroSized;

    #[derive(Default)]
    struct NonZeroSized {
        _unused: usize,
    }

    fn do_test<T: Default>() -> Result<()> {
        // len == cap == 0
        let mut v = Vec::<T>::new();
        v.shrink_to_fit()?;

        // len == 0 < cap
        let mut v = Vec::<T>::with_capacity(4)?;
        v.shrink_to_fit()?;

        // 0 < len < cap
        let mut v = Vec::with_capacity(4)?;
        v.push(T::default())?;
        v.shrink_to_fit()?;

        // 0 < len == cap
        let mut v = Vec::new();
        v.reserve_exact(2)?;
        v.push(T::default())?;
        v.push(T::default())?;
        v.shrink_to_fit()?;

        Ok(())
    }

    OomTest::new().test(|| do_test::<ZeroSized>())?;
    OomTest::new().test(|| do_test::<NonZeroSized>())?;
    Ok(())
}

#[test]
fn vec_resize() -> Result<()> {
    OomTest::new().test(|| {
        let mut v = Vec::new();
        v.resize(10, 'a')?; // Grow.
        v.resize(1, 'b')?; // Truncate.
        v.resize(1, 'c')?; // Same length.
        v.resize(3, 'd')?; // Grow again.
        assert_eq!(&*v, &['a', 'd', 'd']);
        Ok(())
    })
}

#[test]
fn vec_try_collect() -> Result<()> {
    OomTest::new().test(|| {
        iter::repeat(1).take(0).try_collect::<Vec<_>, _>()?;
        iter::repeat(1).take(1).try_collect::<Vec<_>, _>()?;
        iter::repeat(1).take(100).try_collect::<Vec<_>, _>()?;
        iter::repeat(()).take(100).try_collect::<Vec<_>, _>()?;
        Ok(())
    })
}

#[test]
fn vec_extend() -> Result<()> {
    use wasmtime_core::alloc::{TryExtend, Vec};
    OomTest::new().test(|| {
        let mut vec = Vec::new();
        vec.try_extend([])?;
        vec.try_extend([1])?;
        vec.try_extend([1, 2, 3, 4])?;

        let mut vec = Vec::new();
        vec.try_extend([])?;
        vec.try_extend([()])?;
        vec.try_extend([(), (), ()])?;
        Ok(())
    })
}

#[test]
fn vec_macro_elems() -> Result<()> {
    OomTest::new().test(|| {
        let v = collections::vec![100, 200, 300, 400]?;
        assert_eq!(&*v, &[100, 200, 300, 400]);
        Ok(())
    })
}

#[test]
fn vec_macro_elem_len() -> Result<()> {
    OomTest::new().test(|| {
        let v = collections::vec![100; 3]?;
        assert_eq!(&*v, &[100, 100, 100]);
        Ok(())
    })
}

#[test]
fn index_map_try_clone() -> Result<()> {
    OomTest::new()
        // `indexmap` will first try to double its capacity, and, if that fails,
        // will then try to allocate only as much as it absolutely must.
        .allow_alloc_after_oom(true)
        .test(|| {
            let mut map1 = IndexMap::new();
            map1.insert("a", try_new::<Box<_>>(42)?)?;
            map1.insert("b", try_new::<Box<_>>(36)?)?;
            let map2 = map1.try_clone()?;
            assert_eq!(map1, map2);
            Ok(())
        })
}

#[test]
fn index_map_with_capacity() -> Result<()> {
    OomTest::new()
        // `indexmap` will first try to double its capacity, and, if that fails,
        // will then try to allocate only as much as it absolutely must.
        .allow_alloc_after_oom(true)
        .test(|| {
            let _map = IndexMap::<&str, usize>::with_capacity(100)?;
            Ok(())
        })
}

#[test]
fn index_map_split_off() -> Result<()> {
    OomTest::new()
        // `indexmap` will first try to double its capacity, and, if that fails,
        // will then try to allocate only as much as it absolutely must.
        .allow_alloc_after_oom(true)
        .test(|| {
            let mut map1 = IndexMap::new();
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
fn index_map_reserve() -> Result<()> {
    OomTest::new()
        // `indexmap` will first try to double its capacity, and, if that fails,
        // will then try to allocate only as much as it absolutely must.
        .allow_alloc_after_oom(true)
        .test(|| {
            let mut map = IndexMap::<u32, u32>::new();
            map.reserve(100)?;
            Ok(())
        })
}

#[test]
fn index_map_reserve_exact() -> Result<()> {
    OomTest::new().test(|| {
        let mut map = IndexMap::<u32, u32>::new();
        map.reserve_exact(100)?;
        Ok(())
    })
}

#[test]
fn index_map_insert() -> Result<()> {
    OomTest::new()
        // `indexmap` will first try to double its capacity, and, if that fails,
        // will then try to allocate only as much as it absolutely must.
        .allow_alloc_after_oom(true)
        .test(|| {
            let mut map = IndexMap::new();
            map.insert(10, 20)?;
            Ok(())
        })
}

#[test]
fn index_map_insert_full() -> Result<()> {
    OomTest::new()
        // `indexmap` will first try to double its capacity, and, if that fails,
        // will then try to allocate only as much as it absolutely must.
        .allow_alloc_after_oom(true)
        .test(|| {
            let mut map = IndexMap::new();
            map.insert_full(10, 20)?;
            Ok(())
        })
}

#[test]
fn index_map_insert_sorted() -> Result<()> {
    OomTest::new()
        // `indexmap` will first try to double its capacity, and, if that fails,
        // will then try to allocate only as much as it absolutely must.
        .allow_alloc_after_oom(true)
        .test(|| {
            let mut map = IndexMap::new();
            map.insert_sorted(10, 20)?;
            Ok(())
        })
}

#[test]
fn index_map_insert_sorted_by() -> Result<()> {
    OomTest::new()
        // `indexmap` will first try to double its capacity, and, if that fails,
        // will then try to allocate only as much as it absolutely must.
        .allow_alloc_after_oom(true)
        .test(|| {
            let mut map = IndexMap::new();
            map.insert_sorted_by(10, 20, |_k, _v, _k2, _v2| core::cmp::Ordering::Less)?;
            Ok(())
        })
}

#[test]
fn index_map_insert_sorted_by_key() -> Result<()> {
    OomTest::new()
        // `indexmap` will first try to double its capacity, and, if that fails,
        // will then try to allocate only as much as it absolutely must.
        .allow_alloc_after_oom(true)
        .test(|| {
            let mut map = IndexMap::new();
            map.insert_sorted_by_key(10, 20, |_k, v| *v)?;
            Ok(())
        })
}

#[test]
fn index_map_insert_before() -> Result<()> {
    OomTest::new()
        // `indexmap` will first try to double its capacity, and, if that fails,
        // will then try to allocate only as much as it absolutely must.
        .allow_alloc_after_oom(true)
        .test(|| {
            let mut map = IndexMap::new();
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
fn index_map_shift_insert() -> Result<()> {
    OomTest::new()
        // `indexmap` will first try to double its capacity, and, if that fails,
        // will then try to allocate only as much as it absolutely must.
        .allow_alloc_after_oom(true)
        .test(|| {
            let mut map = IndexMap::new();
            map.insert("a", 20)?;
            map.insert("b", 30)?;
            map.shift_insert(1, "c", 40)?;
            assert_eq!(map[0], 20);
            assert_eq!(map[1], 40);
            assert_eq!(map[2], 30);
            Ok(())
        })
}
