//! Arbitrarily choose a spec test from the list of known spec tests.

use arbitrary::{Arbitrary, Unstructured};

// See `build.rs` for how the `FILES` array is generated.
include!(concat!(env!("OUT_DIR"), "/wasttests.rs"));

/// A wast test from this repository.
#[derive(Debug)]
pub struct WastTest {
    #[expect(missing_docs, reason = "self-describing field")]
    pub test: wasmtime_wast_util::WastTest,
}

impl<'a> Arbitrary<'a> for WastTest {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        log::debug!("{}", u.is_empty());
        Ok(WastTest {
            test: u.choose(FILES)?(),
        })
    }

    fn size_hint(_depth: usize) -> (usize, Option<usize>) {
        (1, Some(std::mem::size_of::<usize>()))
    }
}
