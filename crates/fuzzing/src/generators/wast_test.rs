//! Arbitrarily choose a spec test from the list of known spec tests.

use arbitrary::{Arbitrary, Unstructured};

// See `build.rs` for how the `FILES` array is generated.
include!(concat!(env!("OUT_DIR"), "/wasttests.rs"));

/// A wast test from this repository.
#[derive(Debug)]
pub struct WastTest {
    /// The filename of the spec test
    pub file: &'static str,
    /// The `*.wast` contents of the spec test
    pub contents: &'static str,
}

impl<'a> Arbitrary<'a> for WastTest {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        // NB: this does get a uniform value in the provided range.
        let i = u.int_in_range(0..=FILES.len() - 1)?;
        let (file, contents) = FILES[i];
        Ok(WastTest { file, contents })
    }

    fn size_hint(_depth: usize) -> (usize, Option<usize>) {
        (1, Some(std::mem::size_of::<usize>()))
    }
}
