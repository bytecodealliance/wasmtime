# Steps for Completing the goals of `design_doc.md`

1. Modify the existing p3_http_proxy.rs benchmark so that it doesn't need to
   explicitly run cargo to build the component in release mode with debug
   symbols. The `test-programs` crate should be modified to be able to do that
   itself, but it shouldn't be building all the Wasm components/modules in
   release mode by default to run tests. It should just be modified to
   optionally do that (perhaps with a cargo feature, but feel free to consider
   other options).

2. Add the other helper benchmarks mentioned it `design_doc.md`.

3. Run these benchmarks with the additional changes made in branches
   abk/opt2a-lazy-context-slots, abk/opt3f-typed-table-boxed-over-48, and
   abk/opt-combined and provide an analysis of the benchmark performance
   improvements, if any.

4. Further profile the p3_http_proxy benchmark with whatever optimizations from
   step 3 have been applied. Be sure to apply the optimal benchmarking settings
   as described in `design_doc.md`. Identify the areas that currently contribute
   most to overall latency. Describe a few potential areas for investigation
   that may further improve the benchmark.

5. Document and describe the resource leake found when benchmarking earlier.
   Determine if the leak is present on main or not.
