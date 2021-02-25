//! Utilities for fuzzing.
//!
//! The actual fuzz targets are defined in `peepmatic/fuzz/*`. This crate just
//! has oracles and generators for fuzzing.

#![deny(missing_debug_implementations)]
#![deny(missing_docs)]

use arbitrary::{Arbitrary, Unstructured};
use rand::prelude::*;
use std::fmt::Debug;
use std::time;

pub mod automata;
pub mod compile;
pub mod interp;
pub mod parser;

/// A quickcheck-style runner for fuzz targets.
///
/// This is *not* intended to replace a long-running, coverage-guided fuzzing
/// engine like libFuzzer! This is only for defining quick, purely random tests
/// for use with `cargo test` and CI.
pub fn check<A>(mut f: impl FnMut(A))
where
    A: Clone + Debug + for<'a> Arbitrary<'a>,
{
    let seed = rand::thread_rng().gen();
    let mut rng = rand::rngs::SmallRng::seed_from_u64(seed);

    const INITIAL_LENGTH: usize = 16;
    const MAX_LENGTH: usize = 4096;

    let mut buf: Vec<u8> = (0..INITIAL_LENGTH).map(|_| rng.gen()).collect();
    let mut num_checked = 0;

    let time_budget = time::Duration::from_secs(2);
    let then = time::Instant::now();

    loop {
        if num_checked > 0 && time::Instant::now().duration_since(then) > time_budget {
            eprintln!("Checked {} random inputs.", num_checked);
            return;
        }

        match <A as Arbitrary>::arbitrary_take_rest(Unstructured::new(&buf)) {
            Ok(input) => {
                num_checked += 1;
                eprintln!("Checking input: {:#?}", input);
                f(input.clone());
            }
            Err(e @ arbitrary::Error::NotEnoughData) => {
                eprintln!("warning: {}", e);
                if *buf.last().unwrap() == 0 {
                    if buf.len() < MAX_LENGTH {
                        let new_size = std::cmp::min(buf.len() * 2, MAX_LENGTH);
                        eprintln!("Growing buffer size to {}", new_size);
                        let delta = new_size - buf.len();
                        buf.reserve(delta);
                        for _ in 0..delta {
                            buf.push(rng.gen());
                        }
                        continue;
                    } else {
                        // Regenerate `buf` in the loop below and see if that
                        // fixes things...
                        eprintln!("Regenerating buffer data.");
                    }
                } else {
                    // Shrink values in the end of `buf`, which is where
                    // `Arbitrary` pulls container lengths from. Then try again.
                    eprintln!("Shrinking buffer's tail values.");
                    let i = (buf.len() as f64).sqrt() as usize;
                    for j in i..buf.len() {
                        buf[j] /= 2;
                    }
                    continue;
                }
            }
            Err(e) => {
                eprintln!("warning: {}", e);
                // Usually this happens because `A` requires a sequence utf-8
                // bytes but its given sequence wasn't valid utf-8. Just skip
                // this iteration and try again after we've updated `buf` below.
            }
        };

        // Double the size of the buffer every so often, so we don't only
        // explore small inputs.
        if num_checked == buf.len() {
            buf.resize(std::cmp::min(buf.len() * 2, MAX_LENGTH), 0);
        }

        for i in 0..buf.len() {
            buf[i] = rng.gen();
        }
    }
}
