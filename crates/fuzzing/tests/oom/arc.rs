#![cfg(arc_try_new)]

use std::sync::Arc;
use wasmtime::Result;
use wasmtime_environ::collections::try_new;
use wasmtime_fuzzing::oom::OomTest;

#[test]
pub(crate) fn try_new_arc() -> Result<()> {
    OomTest::new().test(|| {
        let _arc = try_new::<Arc<u32>>(42)?;
        Ok(())
    })
}
