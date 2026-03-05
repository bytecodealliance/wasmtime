use std::iter;
use wasmtime::Result;
use wasmtime_environ::collections::{TryCollect, TryVec, try_vec};
use wasmtime_fuzzing::oom::OomTest;

#[test]
fn try_vec_with_capacity() -> Result<()> {
    OomTest::new().test(|| {
        let _v = wasmtime_environ::collections::TryVec::<usize>::with_capacity(100)?;
        Ok(())
    })
}

#[test]
fn try_vec_reserve() -> Result<()> {
    OomTest::new().test(|| {
        let mut v = wasmtime_environ::collections::TryVec::<usize>::new();
        v.reserve(10)?;
        Ok(())
    })
}

#[test]
fn try_vec_reserve_exact() -> Result<()> {
    OomTest::new().test(|| {
        let mut v = wasmtime_environ::collections::TryVec::<usize>::new();
        v.reserve_exact(3)?;
        Ok(())
    })
}

#[test]
fn try_vec_push() -> Result<()> {
    OomTest::new().test(|| {
        let mut v = wasmtime_environ::collections::TryVec::new();
        v.push(42)?;
        Ok(())
    })
}

#[test]
fn try_vec_and_boxed_slice() -> Result<()> {
    use wasmtime_core::alloc::TryVec;

    OomTest::new().test(|| {
        // Nonzero-sized type.
        let mut vec = TryVec::new();
        vec.push(1)?;
        let slice = vec.into_boxed_slice()?; // len > 0, cap > 0

        let mut vec = TryVec::from(slice);
        vec.pop();
        let slice = vec.into_boxed_slice()?; // len = 0, cap > 0

        let vec = TryVec::from(slice);
        let _slice = vec.into_boxed_slice()?; // len = 0, cap = 0

        let mut vec = TryVec::new();
        vec.reserve_exact(3)?;
        vec.push(2)?;
        vec.push(2)?;
        vec.push(2)?;
        let _slice = vec.into_boxed_slice()?; // len = cap, len > 0

        for i in 0..12 {
            let mut vec = TryVec::new();
            for j in 0..i {
                vec.push(j)?;
            }
            let _slice = vec.into_boxed_slice()?; // len ?= cap
        }

        // Zero-sized type.
        let mut vec = TryVec::new();
        vec.push(())?;
        let slice = vec.into_boxed_slice()?; // len > 0, cap > 0
        let mut vec = TryVec::from(slice);
        vec.pop();
        let slice = vec.into_boxed_slice()?; // len = 0, cap > 0
        let vec = TryVec::from(slice);
        let _ = vec.into_boxed_slice()?; // len = 0, cap = 0

        Ok(())
    })
}

#[test]
fn try_vec_shrink_to_fit() -> Result<()> {
    use wasmtime_core::alloc::TryVec;

    #[derive(Default)]
    struct ZeroSized;

    #[derive(Default)]
    struct NonZeroSized {
        _unused: usize,
    }

    fn do_test<T: Default>() -> Result<()> {
        // len == cap == 0
        let mut v = TryVec::<T>::new();
        v.shrink_to_fit()?;

        // len == 0 < cap
        let mut v = TryVec::<T>::with_capacity(4)?;
        v.shrink_to_fit()?;

        // 0 < len < cap
        let mut v = TryVec::with_capacity(4)?;
        v.push(T::default())?;
        v.shrink_to_fit()?;

        // 0 < len == cap
        let mut v = TryVec::new();
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
fn try_vec_resize() -> Result<()> {
    OomTest::new().test(|| {
        let mut v = TryVec::new();
        v.resize(10, 'a')?; // Grow.
        v.resize(1, 'b')?; // Truncate.
        v.resize(1, 'c')?; // Same length.
        v.resize(3, 'd')?; // Grow again.
        assert_eq!(&*v, &['a', 'd', 'd']);
        Ok(())
    })
}

#[test]
fn try_vec_try_collect() -> Result<()> {
    OomTest::new().test(|| {
        iter::repeat(1).take(0).try_collect::<TryVec<_>, _>()?;
        iter::repeat(1).take(1).try_collect::<TryVec<_>, _>()?;
        iter::repeat(1).take(100).try_collect::<TryVec<_>, _>()?;
        iter::repeat(()).take(100).try_collect::<TryVec<_>, _>()?;
        Ok(())
    })
}

#[test]
fn try_vec_extend() -> Result<()> {
    use wasmtime_core::alloc::{TryExtend, TryVec};
    OomTest::new().test(|| {
        let mut vec = TryVec::new();
        vec.try_extend([])?;
        vec.try_extend([1])?;
        vec.try_extend([1, 2, 3, 4])?;

        let mut vec = TryVec::new();
        vec.try_extend([])?;
        vec.try_extend([()])?;
        vec.try_extend([(), (), ()])?;
        Ok(())
    })
}

#[test]
fn try_vec_macro_elems() -> Result<()> {
    OomTest::new().test(|| {
        let v = try_vec![100, 200, 300, 400]?;
        assert_eq!(&*v, &[100, 200, 300, 400]);
        Ok(())
    })
}

#[test]
fn try_vec_macro_elem_len() -> Result<()> {
    OomTest::new().test(|| {
        let v = try_vec![100; 3]?;
        assert_eq!(&*v, &[100, 100, 100]);
        Ok(())
    })
}
