use cranelift_bitset::CompoundBitSet;
use std::{
    alloc::{Layout, alloc},
    fmt::{self, Write},
    iter,
    sync::atomic::{AtomicU32, Ordering::SeqCst},
};
use wasmtime::{error::OutOfMemory, *};
use wasmtime_core::alloc::TryCollect;
use wasmtime_environ::{PrimaryMap, SecondaryMap, collections::*};
use wasmtime_fuzzing::oom::{OomTest, OomTestAllocator};

#[global_allocator]
static GLOBAL_ALOCATOR: OomTestAllocator = OomTestAllocator::new();

#[test]
fn smoke_test_ok() -> Result<()> {
    OomTest::new().test(|| Ok(()))
}

#[test]
fn smoke_test_missed_oom() -> Result<()> {
    let err = OomTest::new()
        .test(|| {
            let _ = unsafe { alloc(Layout::new::<u64>()) };
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
        // nonzero-sized type
        let mut vec = Vec::new();
        vec.push(1)?;
        let slice = vec.into_boxed_slice()?; // len > 0, cap > 0
        let mut vec = Vec::from(slice);
        vec.pop();
        let slice = vec.into_boxed_slice()?; // len = 0, cap > 0
        let vec = Vec::from(slice);
        let slice = vec.into_boxed_slice()?; // len = 0, cap = 0
        let mut vec = Vec::from(slice);
        vec.push(2)?;
        vec.push(2)?;
        vec.push(2)?;
        let _ = vec.into_boxed_slice()?;

        // zero-sized type
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
