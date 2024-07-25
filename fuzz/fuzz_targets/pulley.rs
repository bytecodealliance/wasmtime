#![no_main]

use libfuzzer_sys::{arbitrary::*, fuzz_target};
use pulley_interpreter_fuzz::{interp, roundtrip};

fuzz_target!(|data| {
    let _ = fuzz(data);
});

fn fuzz(data: &[u8]) -> Result<()> {
    let _ = env_logger::try_init();

    let mut u = Unstructured::new(data);
    match u.int_in_range(0..=1)? {
        0 => roundtrip(Arbitrary::arbitrary_take_rest(u)?),
        1 => interp(Arbitrary::arbitrary_take_rest(u)?),
        _ => unreachable!(),
    }

    Ok(())
}
