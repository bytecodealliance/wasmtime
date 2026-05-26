use cranelift_bitset::CompoundBitSet;
use wasmtime::Result;
use wasmtime_fuzzing::oom::OomTest;

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
