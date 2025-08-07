# Fuzzing

## External Fuzzing Campaigns

The Wasmtime maintainers appreciate bug reports from external fuzzing campaigns
— when done thoughtfully and responsibly. Triaging and diagnosing bug reports,
particularly reports for bugs found by fuzzers, takes a lot of time and effort
on the part of Wasmtime maintainers. We ask that you match that effort by
following the guidelines below.

### Talk To Us First

We would love to collaborate and help you find bugs in Wasmtime! We do lots of
fuzzing already (see the docs below about our internal fuzzing infrastructure)
but we always have ideas for new kinds of generators for directed fuzzing, new
oracles that we wish we had, areas of code that we wish were better fuzzed, and
etc... We can share which of Wasmtime's properties are most important to us and
what kinds of bugs we value discovering the most. It is also good for us to know
that an external fuzzing campaign is spinning up and that we should be on the
look out for new issues being filed.

[Come say hello on our Zulip and introduce yourself
:)](https://bytecodealliance.zulipchat.com/#narrow/stream/217126-wasmtime)

### If a Bug Might be a Security Vulnerability, Do Not File a Public Issue

When you find a new bug, first evaluate it against our
[guidelines](./security-what-is-considered-a-security-vulnerability.md) for what
constitutes a security vulnerability.

If you determine that the bug is not a security vulnerability, then [file an
issue on our public
tracker](https://github.com/bytecodealliance/wasmtime/issues/new/choose).

If you think the bug might be considered a security vulnerability, **do not open
a public issue detailing the bug!** Instead, follow the vulnerability reporting
process documented [here](https://bytecodealliance.org/security).

### Write Good Bug Reports

The Wasmtime maintainers appreciate bug reports with the following:

1. **A minimal test case:** You should integrate [automatic test-case
   reduction](./contributing-reducing-test-cases.md) into your fuzzing campaign.

* **Steps to reproduce:** Simple, unambiguous steps that can be performed to
  reproduce the buggy behavior with the test case. These steps should include
  all options you configured Wasmtime with, such as CLI flags and
  `wasmtime::Config` method calls. Ideally these steps are as simple for
  maintainers to execute as running `wasmtime [OPTIONS] testcase.wasm` or a Rust
  `#[test]` that can be run via `cargo test`.

* **Expected behavior:** A description of the expected, non-buggy behavior, as
  well as your rationale for *why* that behavior is expected. For example, just
  because another Wasm engine or an alternative Wasmtime execution strategy
  produces a different result from default Wasmtime, that is not necessarily a
  bug. See the [documentation
  below](#divergent-webassembly-behavior-across-runtimes) for examples of known divergent
  behavior of one module in two runtimes. If applicable, make sure to account
  for this in your rationale and analysis of the bug.

* **Actual behavior:** A description of the actual, buggy behavior. This should
  include things various things like incorrect computation results, assertion
  failure messages, stack traces, signals raised, and etc... when applicable.

* **Wasmtime version and system information:** Include the version of Wasmtime
  you are using (either the exact release or the git commit hash) and your ISA,
  operating system, distribution, and kernel version in the bug report.

Including the above information is extra important for bugs discovered
mechanically, whether by fuzzing or other means, since the associated test cases
will often be pseudo-random or otherwise unintuitive to debug.

### Divergent WebAssembly behavior across runtimes

WebAssembly has a variety of sources of [non-determinism] which means that the
exact same module is allowed to behave differently under the same inputs
across multiple runtimes. These specifics don't often arise in "real world"
modules but can quickly arise during fuzzing. Some example behaviors are:

* **NaN bit patterns** - floating-point operations which produce NaN as a result
  are allowed to produce any one of a set of patterns of NaN. This means that
  the exact bit-representation of the result of a floating-point operation may
  diverge across engines. When fuzzing you can update your source-generation to
  automatically canonicalize NaN values after all floating point operations.
  Wasmtime has built-in options to curb this [non-determinism] as well.

* **Relaxed SIMD** - the `relaxed-simd` proposal to WebAssembly explicitly has
  multiple allowed results for instructions given particular inputs. These
  instructions are inherently non-deterministic across implementations. When
  fuzzing you can avoid these instructions entirely, canonicalize the results,
  or use Wasmtime's built-in options to curb the [non-determinism].

* **Call stack exhaustion** - the WebAssembly specification requires that all
  function calls consume a nonzero-amount of some resource which can eventually
  be exhausted. This means that infinite recursion is not allowed in any
  WebAssembly engine. Bounded, but very large, recursion is allowed in
  WebAssembly but is not guaranteed to work across WebAssembly engines. One
  engine may have different stack settings than another engine and/or runtime
  parameters may tune how much stack space is taken (e.g. optimizations on/off).
  If one engine stack overflows and another doesn't then that's not necessarily
  a bug in either engine. Short of banning recursion there's no known great way
  to handle this apart from throwing out fuzz test cases that stack overflow.

* **Memory exhaustion** - the `memory.grow` and `table.grow` instructions in
  WebAssembly are not always guaranteed to either fail or succeed. This means
  that growth may succeed in one engine but fail in another depending on various
  settings. To handle this in fuzzing it's recommended to generate memories with
  a maximum size and ensure that each engine being fuzzed can grow memory all
  the way to the maximum size.

* **WASIp1 API behavior** - the initial specification of WASI, WASIp1 or
  `wasi_snapshot_preview1`, effectively is not suitable for differential fuzzing
  across engines. The APIs are not thoroughly specified enough nor is there a
  rigorous enough test suite to codify what exactly should happen in all
  situations on all platforms. This means that exactly what kind of error arises
  or various other edge cases may behave differently across engines. The lack of
  specificity of WASIp1 means that there is no great oracle as to whether an
  engine is right or wrong. Development of WASIp1 has ceased and the Component
  Model is being worked on instead (e.g. WASIp2 and beyond) which is more
  suitable for differential fuzzing.

[non-determinism]: ./examples-deterministic-wasm-execution.md

### Do Not Report the Same Bug Multiple Times

Fuzzers will often trigger the same bug multiple times in multiple different
ways. Do not just file an issue for every single test case where the fuzzer
triggers an assertion failure. Many, or even most, of those test cases will
trigger the exact same assertion failure, but perhaps with a slightly different
stack trace. Spend some amount of effort deduplicating bugs before reporting
them.

### Do Not Report Too Many Issues At Once

Please do not clog up our issue tracker by filing dozens and dozens of bug
reports all at the same time. Choose a handful of the bugs your fuzzer has
discovered, prioritizing the ones that seem most serious, and file issues for
those bugs first. As those issues are resolved, then file a few more issues, and
so on.

### Further Reading

Here are some more helpful resources to help your external fuzzing efforts
succeed:

* Blog post: [Responsible and Effective Bugfinding by John
  Regehr](https://blog.regehr.org/archives/2037)

## Wasmtime's Internal Fuzzing Infrastructure

The Wasmtime project leverages extensive fuzzing for its safety and correctness
assurances, and therefore already has a fairly large amount of fuzzing
infrastructure. [Our fuzzers run continuously on
OSS-Fuzz.](https://github.com/google/oss-fuzz/tree/master/projects/wasmtime)

### Test Case Generators and Oracles

Test case generators and oracles live in the `wasmtime-fuzzing` crate, located
in the `crates/fuzzing` directory.

A *test case generator* takes raw, unstructured input from a fuzzer and
translates that into a test case. This might involve interpreting the raw input
as "DNA" or pre-determined choices through a decision tree and using it to
generate an in-memory data structure, or it might be a no-op where we interpret
the raw bytes as if they were Wasm.

An *oracle* takes a test case and determines whether we have a bug. For example,
one of the simplest oracles is to take a Wasm binary as an input test case,
validate and instantiate it, and (implicitly) check that no assertions failed or
segfaults happened. A more complicated oracle might compare the result of
executing a Wasm file with and without optimizations enabled, and make sure that
the two executions are observably identical.

Our test case generators and oracles strive to be fuzzer-agnostic: they can be
reused with libFuzzer or AFL or any other fuzzing engine or driver.

### libFuzzer and `cargo fuzz` Fuzz Targets

We combine a test case generator and one more oracles into a *fuzz
target*. Because the target needs to pipe the raw input from a fuzzer into the
test case generator, it is specific to a particular fuzzer. This is generally
fine, since they're only a couple of lines of glue code.

Currently, all of our fuzz targets are written for
[libFuzzer](https://www.llvm.org/docs/LibFuzzer.html) and [`cargo
fuzz`](https://rust-fuzz.github.io/book/cargo-fuzz.html). They are defined in
the `fuzz` subdirectory.

See
[`fuzz/README.md`](https://github.com/bytecodealliance/wasmtime/blob/main/fuzz/README.md)
for details on how to run these fuzz targets and set up a corpus of seed inputs.
