# Tiers of Support in Wasmtime

Wasmtime's support for platforms and features can be distinguished with three
different tiers of support. The description of these tiers are intended to be
inspired by the [Rust compiler's support tiers for
targets](https://doc.rust-lang.org/rustc/target-tier-policy.html) but are
additionally tailored for Wasmtime. Wasmtime's tiered support additionally
applies not only to platforms/targets themselves but additionally features
implemented within Wasmtime itself.

The purpose of this document is to provide a means by which to evaluate the
inclusion of new features and support for existing features within Wasmtime.
This should not be used to "lawyer" a change into Wasmtime on a precise
technical detail or similar since this document is itself not 100% precise and
will change over time.

## Current Tier Status

For explanations of what each tier means see below.

#### Tier 1

| Category             | Description                                |
|----------------------|--------------------------------------------|
| Target               | `x86_64-apple-darwin`                      |
| Target               | `x86_64-pc-windows-msvc`                   |
| Target               | `x86_64-unknown-linux-gnu`                 |
| Compiler Backend     | Cranelift                                  |
| WebAssembly Proposal | [`mutable-globals`]                        |
| WebAssembly Proposal | [`sign-extension-ops`]                     |
| WebAssembly Proposal | [`nontrapping-float-to-int-conversion`]    |
| WebAssembly Proposal | [`multi-value`]                            |
| WebAssembly Proposal | [`bulk-memory`]                            |
| WebAssembly Proposal | [`reference-types`]                        |
| WebAssembly Proposal | [`simd`]                                   |
| WebAssembly Proposal | [`component-model`]                        |
| WebAssembly Proposal | [`relaxed-simd`]                           |
| WebAssembly Proposal | [`multi-memory`]                           |
| WebAssembly Proposal | [`threads`]                                |
| WebAssembly Proposal | [`tail-call`]                              |
| WebAssembly Proposal | [`memory64`]                               |
| WASI Proposal        | [`wasi-io`]                                |
| WASI Proposal        | [`wasi-clocks`]                            |
| WASI Proposal        | [`wasi-filesystem`]                        |
| WASI Proposal        | [`wasi-random`]                            |
| WASI Proposal        | [`wasi-sockets`]                           |
| WASI Proposal        | [`wasi-http`]                              |
| WASI Proposal        | `wasi_snapshot_preview1`                   |
| WASI Proposal        | `wasi_unstable`                            |

[`mutable-globals`]: https://github.com/WebAssembly/mutable-global/blob/master/proposals/mutable-global/Overview.md
[`sign-extension-ops`]: https://github.com/WebAssembly/spec/blob/master/proposals/sign-extension-ops/Overview.md
[`nontrapping-float-to-int-conversion`]: https://github.com/WebAssembly/spec/blob/master/proposals/nontrapping-float-to-int-conversion/Overview.md
[`multi-value`]: https://github.com/WebAssembly/spec/blob/master/proposals/multi-value/Overview.md
[`bulk-memory`]: https://github.com/WebAssembly/bulk-memory-operations/blob/master/proposals/bulk-memory-operations/Overview.md
[`reference-types`]: https://github.com/WebAssembly/reference-types/blob/master/proposals/reference-types/Overview.md
[`simd`]: https://github.com/WebAssembly/simd/blob/master/proposals/simd/SIMD.md
[`wasi-clocks`]: https://github.com/WebAssembly/wasi-clocks
[`wasi-filesystem`]: https://github.com/WebAssembly/wasi-filesystem
[`wasi-io`]: https://github.com/WebAssembly/wasi-io
[`wasi-random`]: https://github.com/WebAssembly/wasi-random
[`wasi-sockets`]: https://github.com/WebAssembly/wasi-sockets
[`wasi-http`]: https://github.com/WebAssembly/wasi-http
[`tail-call`]: https://github.com/WebAssembly/tail-call/blob/main/proposals/tail-call/Overview.md

#### Tier 2

| Category             | Description                | Missing Tier 1 Requirements |
|----------------------|----------------------------|-----------------------------|
| Target               | `aarch64-unknown-linux-gnu`| Continuous fuzzing          |
| Target               | `aarch64-apple-darwin`     | Continuous fuzzing          |
| Target               | `s390x-unknown-linux-gnu`  | Continuous fuzzing          |
| Target               | `x86_64-pc-windows-gnu`    | Clear owner of the target   |
| Target               | Support for `#![no_std]`   | Support beyond CI checks    |
| WebAssembly Proposal | [`function-references`]    | Unstable wasm proposal      |
| WebAssembly Proposal | [`wide-arithmetic`]        | Unstable wasm proposal      |

[`memory64`]: https://github.com/WebAssembly/memory64/blob/master/proposals/memory64/Overview.md
[`multi-memory`]: https://github.com/WebAssembly/multi-memory/blob/master/proposals/multi-memory/Overview.md
[`threads`]: https://github.com/WebAssembly/threads/blob/master/proposals/threads/Overview.md
[`component-model`]: https://github.com/WebAssembly/component-model/blob/main/design/mvp/Explainer.md
[`relaxed-simd`]: https://github.com/WebAssembly/relaxed-simd/blob/main/proposals/relaxed-simd/Overview.md
[`function-references`]: https://github.com/WebAssembly/function-references/blob/main/proposals/function-references/Overview.md
[`wide-arithmetic`]: https://github.com/WebAssembly/wide-arithmetic/blob/main/proposals/wide-arithmetic/Overview.md

#### Tier 3

| Category             | Description                       | Missing Tier 2 Requirements |
|----------------------|-----------------------------------|-----------------------------|
| Target               | `aarch64-apple-ios`               | CI testing, full-time maintainer |
| Target               | `aarch64-linux-android`           | CI testing, full-time maintainer |
| Target               | `aarch64-pc-windows-msvc`         | CI testing, full-time maintainer |
| Target               | `aarch64-unknown-linux-musl` [^4] | CI testing, full-time maintainer |
| Target               | `armv7-unknown-linux-gnueabihf`   | full-time maintainer |
| Target               | `i686-pc-windows-msvc`            | CI testing, full-time maintainer |
| Target               | `i686-unknown-linux-gnu`          | full-time maintainer |
| Target               | `powerpc64le-unknown-linux-gnu`   | CI testing, full-time maintainer |
| Target               | `riscv32imac-unknown-none-elf`[^5]| CI testing, full-time maintainer |
| Target               | `riscv64gc-unknown-linux-gnu`     | full-time maintainer        |
| Target               | `wasm32-wasip1` [^3]              | Supported but not tested    |
| Target               | `x86_64-linux-android`            | CI testing, full-time maintainer |
| Target               | `x86_64-unknown-freebsd`          | CI testing, full-time maintainer |
| Target               | `x86_64-unknown-illumos`          | CI testing, full-time maintainer |
| Target               | `x86_64-unknown-linux-musl` [^4]  | CI testing, full-time maintainer |
| Target               | `x86_64-unknown-none` [^5]        | CI testing, full-time maintainer |
| Compiler Backend     | Winch on x86\_64                  | WebAssembly proposals (`simd`, `relaxed-simd`, `tail-call`, `reference-types`, `threads`)     |
| Compiler Backend     | Winch on aarch64                  | Complete implementation     |
| Execution Backend    | Pulley                            | fuzzing                     |
| WebAssembly Proposal | [`gc`]                            | Complete implementation     |
| WASI Proposal        | [`wasi-nn`]                       | More expansive CI testing   |
| WASI Proposal        | [`wasi-threads`]                  | More CI, unstable proposal  |
| WASI Proposal        | [`wasi-config`]                   | unstable proposal           |
| WASI Proposal        | [`wasi-keyvalue`]                 | unstable proposal           |
| *misc*               | Non-Wasmtime Cranelift usage [^1] | CI testing, full-time maintainer |
| *misc*               | DWARF debugging [^2]              | CI testing, full-time maintainer, improved quality |

[`wasi-nn`]: https://github.com/WebAssembly/wasi-nn
[`wasi-threads`]: https://github.com/WebAssembly/wasi-threads
[`wasi-config`]: https://github.com/WebAssembly/wasi-config
[`wasi-keyvalue`]: https://github.com/WebAssembly/wasi-keyvalue
[`gc`]: https://github.com/WebAssembly/gc

[^1]: This is intended to encompass features that Cranelift supports as a
general-purpose code generator such as integer value types other than `i32` and
`i64`, non-Wasmtime calling conventions, code model settings, relocation
restrictions, etc. These features aren't covered by Wasmtime's usage of
Cranelift because the WebAssembly instruction set doesn't leverage them. This
means that they receive far less testing and fuzzing than the parts of Cranelift
exercised by Wasmtime.

[^2]: Currently there is no active maintainer of DWARF debugging support and
support is currently best-effort. Additionally there are known shortcomings
and bugs. At this time there's no developer time to improve the situation here
as well.

[^3]: Wasmtime's `cranelift` and `winch` features can be compiled to WebAssembly
but not the `runtime` feature at this time. This means that
Wasmtime-compiled-to-wasm can itself compile wasm but cannot execute wasm.

[^4]: Binary artifacts for MUSL are dynamically linked, not statically
linked, meaning that they are not suitable for "run on any linux distribution"
style use cases. Wasmtime does not have static binary artifacts at this time and
that will require building from source.

[^5]: Rust targets that are `#![no_std]` don't support the entire feature set of
Wasmtime. For example the `threads` Cargo feature requires the standard library.
For more information see the [`no_std` documentation][nostd]. Additionally these
targets are sound in the presence of multiple threads but will panic on
contention of data structures. If you're doing multithreaded things in `no_std`
please file an issue so we can help solve your use case.

[nostd]: ./stability-platform-support.md

#### Unsupported features and platforms

While this is not an exhaustive list, Wasmtime does not currently have support
for the following features. Note that this is intended to document Wasmtime's
current state and does not mean Wasmtime does not want to ever support these
features; rather design discussion and PRs are welcome for many of the below
features to figure out how best to implement them and at least move them to Tier
3 above.

* Target: [AArch64 FreeBSD](https://github.com/bytecodealliance/wasmtime/issues/5499)
* Target: [NetBSD/OpenBSD](https://github.com/bytecodealliance/wasmtime/issues/6962)
* Cranelift Target: [i686 (32-bit Intel targets)](https://github.com/bytecodealliance/wasmtime/issues/1980)
* Cranelift Target: ARM 32-bit
* Cranelift Target: MIPS
* Cranelift Target: SPARC
* Cranelift Target: PowerPC
* Cranelift Target: RISC-V 32-bit
* WebAssembly Proposals: see [documentation here](./stability-wasm-proposals.md)
* [WASI proposal: `proxy-wasm`](https://github.com/proxy-wasm/spec)
* [WASI proposal: `wasi-blob-store`](https://github.com/WebAssembly/wasi-blob-store)
* [WASI proposal: `wasi-crypto`](https://github.com/WebAssembly/wasi-crypto)
* [WASI proposal: `wasi-data`](https://github.com/WebAssembly/wasi-data)
* [WASI proposal: `wasi-distributed-lock-service`](https://github.com/WebAssembly/wasi-distributed-lock-service)
* [WASI proposal: `wasi-grpc`](https://github.com/WebAssembly/wasi-grpc)
* [WASI proposal: `wasi-message-queue`](https://github.com/WebAssembly/wasi-message-queue)
* [WASI proposal: `wasi-parallel`](https://github.com/WebAssembly/wasi-parallel)
* [WASI proposal: `wasi-pubsub`](https://github.com/WebAssembly/wasi-pubsub)
* [WASI proposal: `wasi-sql`](https://github.com/WebAssembly/wasi-sql)
* [WASI proposal: `wasi-url`](https://github.com/WebAssembly/wasi-url)

## Tier Details

Wasmtime's precise definitions of tiers are not guaranteed to be constant over
time, so these descriptions are likely to change over time. Tier 1 is classified
as the highest level of support, confidence, and correctness for a component.
Each tier additionally encompasses all the guarantees of previous tiers.

Features classified under a particular tier may already meet the criteria for
later tiers as well. In situations like this it's not intended to use these
guidelines to justify removal of a feature at any one point in time. Guidance is
provided here for phasing out unmaintained features but it should be clear under
what circumstances work "can be avoided" for each tier.

#### Tier 3 - Not Production Ready

The general idea behind Tier 3 is that this is the baseline for inclusion of
code into the Wasmtime project. This is not intended to be a catch-all "if a
patch is sent it will be merged" tier. Instead the goal of this tier is to
outline what is expected of contributors adding new features to Wasmtime which
might be experimental at the time of addition. This is intentionally not a
relaxed tier of restrictions but already implies a significant commitment of
effort to a feature being included within Wasmtime.

Tier 3 features include:

* Inclusion of a feature does not impose unnecessary maintenance overhead on
  other components/features. Some examples of additions to Wasmtime which would
  not be accepted are:

  * An experimental feature doubles the time of CI for all PRs.
  * A change which makes it significantly more difficult to architecturally
    change Wasmtime's internal implementation.
  * A change which makes building Wasmtime more difficult for unrelated
    developers.

  In general Tier 3 features are off-by-default at compile time but still
  tested-by-default on CI.

* New features of Wasmtime cannot have major known bugs at the time of
  inclusion. Landing a feature in Wasmtime requires the feature to be correct
  and bug-free as best can be evaluated at the time of inclusion. Inevitably
  bugs will be found and that's ok, but anything identified during review must
  be addressed.

* Code included into the Wasmtime project must be of an acceptable level of
  quality relative to the rest of the code in Wasmtime.

* There must be a path to a feature being finished at the time of inclusion.
  Adding a new backend to Cranelift for example is a significant undertaking
  which may not be able to be done in a single PR. Partial implementations of a
  feature are acceptable so long as there's a clear path forward and schedule
  for completing the feature.

* New components in Wasmtime must have a clearly identified owner who is willing
  to be "on the hook" for review, updates to the internals of Wasmtime, etc. For
  example a new backend in Cranelift would need to have a maintainer who is
  willing to respond to changes in Cranelift's interfaces and the needs of
  Wasmtime.

This baseline level of support notably does not require any degree of testing,
fuzzing, or verification. As a result components classified as Tier 3 are
generally not production-ready as they have not been battle-tested much.

Features classified as Tier 3 may be disabled in CI or removed from the
repository as well. If a Tier 3 feature is preventing development of other
features then the owner will be notified. If no response is heard from within a
week then the feature will be disabled in CI. If no further response happens
for a month then the feature may be removed from the repository.

#### Tier 2 - Almost Production Ready

This tier is meant to encompass features and components of Wasmtime which are
well-maintained, tested well, but don't necessarily meet the stringent criteria
for Tier 1. Features in this category may already be "production ready" and safe
to use.

Tier 2 features include:

* Tests are run in CI for the Wasmtime project for this feature and everything
  passes. For example a Tier 2 platform runs in CI directly or via emulation.
  Features are otherwise fully tested on CI.

* Complete implementations for anything that's part of Tier 1. For example
  all Tier 2 targets must implement all of the Tier 1 WebAssembly proposals,
  and all Tier 2 features must be implemented on all Tier 1 targets.

* All existing developers are expected to handle minor changes which affect Tier
  2 components. For example if Cranelift's interfaces change then the developer
  changing the interface is expected to handle the changes for Tier 2
  architectures so long as the affected part is relatively minor. Note that if a
  more substantial change is required to a Tier 2 component then that falls
  under the next bullet.

* Maintainers of a Tier 2 feature are responsive (reply to requests within a
  week) and are available to accommodate architectural changes that affect their
  component. For example more expansive work beyond the previous bullet where
  contributors can't easily handle changes are expected to be guided or
  otherwise implemented by Tier 2 maintainers.

* Major changes otherwise requiring an RFC that affect Tier 2 components are
  required to consult Tier 2 maintainers in the course of the RFC. Major changes
  to Tier 2 components themselves do not require an RFC, however.

Features at this tier generally are not turned off or disabled for very long.
Maintainers are already required to be responsive to changes and will be
notified of any unrelated change which affects their component. It's recommended
that if a component breaks for one reason or another due to an unrelated change
that the maintainer either contributes to the PR-in-progress or otherwise has a
schedule for the implementation of the feature.

#### Tier 1 - Production Ready

This tier is intended to be the highest level of support in Wasmtime for any
particular feature, indicating that it is suitable for production environments.
This conveys a high level of confidence in the Wasmtime project about the
specified features.

Tier 1 features include:

* Continuous fuzzing is required for WebAssembly proposals. This means that any
  WebAssembly proposal must have support in the `wasm-smith` crate and existing
  fuzz targets must be running and exercising the new code paths. Where possible
  differential fuzzing should also be implemented to compare results with other
  implementations.

* Continuous fuzzing is required for the architecture of supported targets. For
  example currently there are three x86\_64 targets that are considered Tier 1
  but only `x86_64-unknown-linux-gnu` is fuzzed.

* CVEs and security releases will be performed as necessary for any bugs found
  in features and targets.

* Major changes affecting this component may require help from maintainers with
  specialized expertise, but otherwise it should be reasonable to expect most
  Wasmtime developers to be able to maintain Tier 1 features.

* Major changes affecting Tier 1 features require an RFC and prior agreement on
  the change before an implementation is committed.

* WebAssembly proposals meet [all stabilization
  requirements](./stability-wasm-proposals.md).

A major inclusion point for this tier is intended to be the continuous fuzzing
of Wasmtime. This implies a significant commitment of resources for fixing
issues, hardware to execute Wasmtime, etc. Additionally this tier comes with the
broadest expectation of "burden on everyone else" in terms of what changes
everyone is generally expected to handle.

Features classified as Tier 1 are rarely, if ever, turned off or removed from
Wasmtime.
