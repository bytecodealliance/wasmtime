//! Fuzz testing utilities related to AST pattern matching.

use peepmatic_runtime::PeepholeOptimizations;
use std::path::Path;
use std::str;

// To avoid timeouts, don't deal with inputs larger than this.
const MAX_LEN: usize = 2048;

/// Attempt to interpret the given bytes as UTF-8 and then compile them as if
/// they were source text of our DSL.
pub fn compile(data: &[u8]) {
    if data.len() > MAX_LEN {
        return;
    }

    let source = match str::from_utf8(data) {
        Err(_) => return,
        Ok(s) => s,
    };

    let opt = match peepmatic::compile_str(source, Path::new("fuzz")) {
        Err(_) => return,
        Ok(o) => o,
    };

    // Should be able to serialize and deserialize the peephole optimizer.
    let opt_bytes = bincode::serialize(&opt).expect("should serialize peephole optimizations OK");
    let _: PeepholeOptimizations =
        bincode::deserialize(&opt_bytes).expect("should deserialize peephole optimizations OK");

    // Compiling the same source text again should be deterministic.
    let opt2 = peepmatic::compile_str(source, Path::new("fuzz"))
        .expect("should be able to compile source text again, if it compiled OK the first time");
    let opt2_bytes =
        bincode::serialize(&opt2).expect("should serialize second peephole optimizations OK");
    assert_eq!(opt_bytes, opt2_bytes);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_compile() {
        crate::check(|s: String| compile(s.as_bytes()));
    }

    #[test]
    fn regression_0() {
        compile(
            b"
            (=> (bor (bor $x $y) $y) $x)
            (=> (bor (bor $x $z) $y) $x)
        ",
        );
    }

    #[test]
    fn regression_1() {
        compile(
            b"
            (=> (bor (bor $x $y) 0) $x)
            (=> (bor $x 0) $x)
            (=> (bor $y $x) $x)
        ",
        );
    }

    #[test]
    fn regression_2() {
        compile(
            b"
            (=> (sshr $x 11111111110) $x)
        ",
        );
    }
}
