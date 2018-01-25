=================
Cretonne in Rustc
=================

The Rust compiler currently uses LLVM as its optimizer and code generator for both debug and
release builds. The Cretonne project does not intend to compete with LLVM when it comes to
optimizing release builds, but for debug builds where compilation speed is paramount, it makes
sense to use Cretonne instead of LLVM.

- Cretonne is designed to take advantage of multi-core CPUs, making parallel code generation quite
  easy. This is harder with LLVM which was designed before multi-core CPUs where mainstream.
- Cretonne is designed with compilation speed in mind. It makes engineering tradeoffs that favor
  compilation speed over advanced optimizations.

See `the discussion on the Rust internals forum
<https://internals.rust-lang.org/t/possible-alternative-compiler-backend-cretonne>`_.
