use wasmtime::Result;
use wasmtime_environ::collections::try_new;
use wasmtime_fuzzing::oom::OomTest;

#[test]
fn try_new_box() -> Result<()> {
    OomTest::new().test(|| {
        let _box = try_new::<Box<u32>>(36)?;
        Ok(())
    })
}
