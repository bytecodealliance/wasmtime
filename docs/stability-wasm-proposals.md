# Wasm Proposals

This document is intended to describe the current status of WebAssembly
proposals in Wasmtime. For information about implementing a proposal in Wasmtime
see the [associated
documentation](./contributing-implementing-wasm-proposals.md).

WebAssembly proposals that want to be [tier 2 or above](./stability-tiers.md)
are required to check all boxes in this matrix. An explanation of each matrix
column is below.

## On-by-default proposals

|  Proposal                | Phase 4 | Tests | Finished | Fuzzed | API | C API |
|--------------------------|---------|-------|----------|--------|-----|-------|
| [`mutable-globals`]      | ✅      | ✅    | ✅       | ✅     | ✅  | ✅    |
| [`sign-extension-ops`]   | ✅      | ✅    | ✅       | ✅     | ✅  | ✅    |
| [`nontrapping-fptoint`]  | ✅      | ✅    | ✅       | ✅     | ✅  | ✅    |
| [`multi-value`]          | ✅      | ✅    | ✅       | ✅     | ✅  | ✅    |
| [`bulk-memory`]          | ✅      | ✅    | ✅       | ✅     | ✅  | ✅    |
| [`reference-types`]      | ✅      | ✅    | ✅       | ✅     | ✅  | ✅    |
| [`simd`]                 | ✅      | ✅    | ✅       | ✅     | ✅  | ✅    |
| [`component-model`]      | ❌[^1]  | ✅    | ✅       | ⚠️[^2]  | ✅  | ❌[^5]|
| [`relaxed-simd`]         | ✅      | ✅    | ✅       | ✅     | ✅  | ✅    |
| [`multi-memory`]         | ✅      | ✅    | ✅       | ✅     | ✅  | ✅    |
| [`threads`]              | ✅      | ✅    | ✅       | ❌[^3] | ✅  | ✅    |
| [`tail-call`]            | ✅      | ✅    | ✅       | ✅     | ✅  | ✅    |
| [`extended-const`]       | ✅      | ✅    | ✅       | ❌[^4] | ✅  | ✅    |

[^1]: The `component-model` proposal is not at phase 4 in the standardization
    process but it is still enabled-by-default in Wasmtime.
[^2]: Various shapes of components are fuzzed but full-on fuzzing along the
    lines of `wasm-smith` are not implemented for components.
[^3]: Fuzzing with threads is an open implementation question that is expected
    to get fleshed out as the [`shared-everything-threads`] proposal advances.
[^4]: This was a mistake in Wasmtime's stabilization process. Support for
    [`extended-const`] is not yet implemented in `wasm-smith`.
[^5]: Support for the C API for components is desired by many embedders but
    does not currently have anyone lined up to implement it.

## Off-by-default proposals

|  Proposal                | Phase 4 | Tests | Finished | Fuzzed | API | C API |
|--------------------------|---------|-------|----------|--------|-----|-------|
| [`memory64`]             | ❌      | ✅    | ✅       | ✅     | ✅  | ✅    |
| [`function-references`]  | ✅      | ✅    | ❌       | ❌     | ✅  | ❌    |
| [`gc`] [^6]              | ✅      | ✅    | ❌[^7]   | ❌     | ✅  | ❌    |
| [`wide-arithmetic`]      | ❌      | ✅    | ✅       | ✅     | ✅  | ✅    |
| [`custom-page-sizes`]    | ❌      | ✅    | ⚠️[^8]    | ✅     | ✅  | ❌    |

[^6]: There is also a [tracking
    issue](https://github.com/bytecodealliance/wasmtime/issues/5032) for the
    GC proposal.
[^7]: The implementation of GC has [known performance
    issues](https://github.com/bytecodealliance/wasmtime/issues/9351) which can
    affect non-GC code when the GC proposal is enabled.
[^8]: Using custom-page-sizes is [known to have issues when combined with shared
    memories](https://github.com/bytecodealliance/wasmtime/issues/9523).

## Unimplemented proposals

| Proposal                      | Tracking Issue |
|-------------------------------|----------------|
| [`branch-hinting`]            | [#9463](https://github.com/bytecodealliance/wasmtime/issues/9463) |
| [`exception-handling`]        | [#3427](https://github.com/bytecodealliance/wasmtime/issues/3427) |
| [`flexible-vectors`]          | [#9464](https://github.com/bytecodealliance/wasmtime/issues/9464) |
| [`memory-control`]            | [#9467](https://github.com/bytecodealliance/wasmtime/issues/9467) |
| [`stack-switching`]           | [#9465](https://github.com/bytecodealliance/wasmtime/issues/9465) |
| [`shared-everything-threads`] | [#9466](https://github.com/bytecodealliance/wasmtime/issues/9466) |

[`mutable-globals`]: https://github.com/WebAssembly/mutable-global/blob/master/proposals/mutable-global/Overview.md
[`sign-extension-ops`]: https://github.com/WebAssembly/spec/blob/master/proposals/sign-extension-ops/Overview.md
[`nontrapping-fptoint`]: https://github.com/WebAssembly/spec/blob/master/proposals/nontrapping-float-to-int-conversion/Overview.md
[`multi-value`]: https://github.com/WebAssembly/spec/blob/master/proposals/multi-value/Overview.md
[`bulk-memory`]: https://github.com/WebAssembly/bulk-memory-operations/blob/master/proposals/bulk-memory-operations/Overview.md
[`reference-types`]: https://github.com/WebAssembly/reference-types/blob/master/proposals/reference-types/Overview.md
[`simd`]: https://github.com/WebAssembly/simd/blob/master/proposals/simd/SIMD.md
[`tail-call`]: https://github.com/WebAssembly/tail-call/blob/main/proposals/tail-call/Overview.md
[`branch-hinting`]: https://github.com/WebAssembly/branch-hinting
[`exception-handling`]: https://github.com/WebAssembly/exception-handling
[`extended-const`]: https://github.com/WebAssembly/extended-const
[`flexible-vectors`]: https://github.com/WebAssembly/flexible-vectors
[`memory-control`]: https://github.com/WebAssembly/memory-control
[`stack-switching`]: https://github.com/WebAssembly/stack-switching
[`shared-everything-threads`]: https://github.com/WebAssembly/shared-everything-threads
[`memory64`]: https://github.com/WebAssembly/memory64/blob/master/proposals/memory64/Overview.md
[`multi-memory`]: https://github.com/WebAssembly/multi-memory/blob/master/proposals/multi-memory/Overview.md
[`threads`]: https://github.com/WebAssembly/threads/blob/master/proposals/threads/Overview.md
[`component-model`]: https://github.com/WebAssembly/component-model/blob/main/design/mvp/Explainer.md
[`relaxed-simd`]: https://github.com/WebAssembly/relaxed-simd/blob/main/proposals/relaxed-simd/Overview.md
[`function-references`]: https://github.com/WebAssembly/function-references/blob/main/proposals/function-references/Overview.md
[`wide-arithmetic`]: https://github.com/WebAssembly/wide-arithmetic/blob/main/proposals/wide-arithmetic/Overview.md
[`gc`]: https://github.com/WebAssembly/gc
[`custom-page-sizes`]: https://github.com/WebAssembly/custom-page-sizes

## Feature requirements

For each column in the above tables, this is a further explanation of its meaning:

* **Phase 4** - The proposal must be in phase 4, or greater, of [the
  WebAssembly standardization process][phases].

* **Tests** - All spec tests must be passing in Wasmtime and where appropriate
  Wasmtime-specific tests, for example for the API, should be passing. Tests
  must pass at least for Cranelift on all [tier 1](./stability-tiers.md)
  platforms, but missing other platforms is otherwise acceptable.

* **Finished** - No open questions, design concerns, or serious known bugs. The
  implementation should be complete to the extent that is possible. Support
  must be implemented for all [tier 1](./stability-tiers.md) targets and
  compiler backends.

* **Fuzzed** - Has been fuzzed for at least a week minimum. We are also
  confident that the fuzzers are fully exercising the proposal's functionality.
  The `module_generation_uses_expected_proposals` test in the `wasmtime-fuzzing`
  crate must be updated to include this proposal.

  > For example, it would *not* have been enough to simply enable reference
  > types in the `compile` fuzz target to enable that proposal by
  > default. Compiling a module that uses reference types but not instantiating
  > it nor running any of its functions doesn't exercise any of the GC
  > implementation and does not run the inline fast paths for `table` operations
  > emitted by the JIT. Exercising these things was the motivation for writing
  > the custom fuzz target for `table.{get,set}` instructions.

  One indication of the status of fuzzing is [this
  file](https://github.com/bytecodealliance/wasmtime/blob/main/crates/fuzzing/src/generators/module.rs#L16)
  which controls module configuration during fuzzing.

* **API** - The proposal's functionality is exposed in the `wasmtime` crate's
  API. At minimum this is `Config::wasm_the_proposal` but proposals such as
  [`gc`] also add new types to the API.

* **C API** - The proposal's functionality is exposed in the C API.

[phases]: https://github.com/WebAssembly/meetings/blob/master/process/phases.md
