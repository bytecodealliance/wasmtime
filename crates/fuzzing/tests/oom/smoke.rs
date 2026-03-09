use std::{
    alloc::{Layout, alloc, dealloc},
    ops::Deref,
};
use wasmtime::{Result, error::OutOfMemory};
use wasmtime_fuzzing::oom::OomTest;

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
pub(crate) fn smoke_test_ok() -> Result<()> {
    OomTest::new().test(|| Ok(()))
}

#[test]
pub(crate) fn smoke_test_missed_oom() -> Result<()> {
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
pub(crate) fn smoke_test_disallow_alloc_after_oom() -> Result<()> {
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
pub(crate) fn smoke_test_allow_alloc_after_oom() -> Result<()> {
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
