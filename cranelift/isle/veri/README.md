# VeriISLE

VeriISLE is an in-development [SMT](https://smt-lib.org)-based verifier for the
[ISLE language](../docs/language-reference.md).

It analyzes chains of ISLE rules, using a combination of hand-written `spec`s and
specifications derived from authoritative ISA semantics, such as [ASL](https://developer.arm.com/architectures/architecture%20specification%20language) for the `aarch64` backend.

The verification work is detailed in two academic papers:
- The most recent OOPSLA 2025 paper described the automatic rule chaining, authoritative ISA specification derivations, and our current state modeling approach: [Scaling Instruction-Selection Verification against Authoritative ISA Semantics](https://doi.org/10.1145/3764383).
  Michael McLoughlin, Ashley Sheng, Chris Fallin, Bryan Parno, Fraser Brown, and
  Alexa VanHattum. OOPSLA 2025.
- The earlier ASPLOS 2024 paper described the overall verification strategy and more bugs this work prevented and/or reproduced: [Lightweight, Modular Verification for WebAssembly-to-Native Instruction Selection](https://doi.org/10.1145/3617232.3624862).
  Alexa VanHattum, Monica Pardeshi, Chris Fallin, Adrian Sampson, and Fraser
  Brown. ASPLOS 2024.

## Dependencies

To run the verifier you will need a backend SMT solver installed. The default
configuration uses both [cvc5](https://cvc5.github.io/) and
[z3](https://github.com/Z3Prover/z3): most expansions are checked with `cvc5`,
while expansions tagged `solver_z3` (for example floating-point operations) are
checked with `z3`.

On MacOS, you can install both via homebrew:

```
brew install cvc5/homebrew-cvc5/cvc5
brew install z3
```

Alternatively, on Linux or MacOS you can install from Github release with:

```
./setup/install-cvc5.sh -i <install_path>
./setup/install-z3.sh -b <install_path>/bin
```

If you use this method, ensure that `<install_path>/bin` is on your `$PATH`.

## Sharing the cache with CI

To keep local runs fast, CI maintains a shared copy of the verifier's SMT query
cache. On every push to `main`, the "ISLE Verifier" workflow
([`.github/workflows/isle-veri.yml`](../../.github/workflows/isle-veri.yml))
runs `verify.sh rebuild-cache` on top of the previously published cache and
publishes the result as the `isle-veri-cache.tar.gz` asset on the rolling
[`dev` release](https://github.com/bytecodealliance/wasmtime/releases/tag/dev).
Pull requests also run the verifier against the latest `main` cache (read-only).

To start a local session from the cache CI is currently using, run:

```
./cranelift/isle/veri/setup/download-cache.sh
```

This downloads the latest asset from the `dev` release and installs it as
`cranelift/isle/veri/cache`. You can then verify incrementally on top of it
with `./cranelift/isle/veri/verify.sh` (and check full cache coverage with
`verify.sh cache-only`).

## Configuration files

Rather than configuring arguments on the command line, you can store
them in a configuration file and point the verifier at it with `--config`:

```
cargo run -p cranelift-isle-veri --bin veri -- --config cranelift/isle/veri/configs/aarch64-fast.args
```

A configuration file lists one or more per line command-line arguments per line.
Blank lines and anything following a `#` (whole-line or trailing comments) are ignored.
The arguments from the file are applied *before* any passed on the command line, so the command line always takes precedence (for example, you can reuse a config but override its `--timeout`).
Multi-valued arguments such as `--filter` accumulate, while single-valued arguments (like `--name`) take their last value.

Three example configurations live in [`configs/`](configs):

| File                              | Equivalent to                                                       |
| --------------------------------- | ------------------------------------------------------------------- |
| `aarch64-fast.args`               | `--default-excludes` (the AArch64 run below)                        |
| `aarch64.args`                    | the default AArch64 excludes but with `slow` expansions included    |
| `opt-fast.args`                   | `--only-root simplify --default-excludes` (the mid-end run)         |
| `opt.args`                        | the mid-end run with `slow` expansions and instantiations included  |
| `x64-iadd-base-case.args`         | `--name x64 --rule iadd_base_case_32_or_64_lea` (the x64 example below) |

## Running for `aarch64`

To run the verifier, run:

```
cargo run -p cranelift-isle-veri --bin veri -- --default-excludes
```

This will run verification on the default AArch64 backend. `--default-excludes` will skip ISLE terms
that are either currently not well-supported or slow to verify, such as vector operations
and expensive division operations.

The verification bin will default to running on a number of threads
based on the number of logical CPUs on your current machine, pass `--num-threads=n` to
override this. On a 12-core M2 MacBook, the command above takes about a minute.

By default the verifier attempts every expansion it can reach. It seeds an
expansion at every term that has rules and a constructor, and verifies all rule
chains reachable from those roots.

A term that is seeded but turns out to have no usable spec (its own or a term it
reaches) is reported as an *expansion error* rather than silently dropped, so
these coverage gaps stay visible; see the `errors.out` summary in the log
directory. The exception is a term that is only reachable *from* (conceptually, later
in a rule chain from)  an excluded starting rule (for example an `i128`- or
`narrowfloat`-tagged lowering rule when `--default-excludes` is set).

Expansions tagged `TODO` are skipped by default (pass`--no-skip-todo` to include them).

### Filtering expansions

During development you may want to focus on a subset of expansions. Pass one or
more `--filter` arguments, each of the form `[include:|exclude:]<predicate>`. The
supported predicates are:

| Predicate         | Matches an expansion where...                              |
| ----------------- | ---------------------------------------------------------- |
| `tag:<tag>`       | the root term, a rule, or any chained term carries `<tag>` |
| `root:<term>`     | the root term is `<term>`                                  |
| `rule:<rule>`     | the expansion contains the named `<rule>`                  |
| `not:<predicate>` | `<predicate>` does not match                               |
| `<p>,<q>`         | both `<p>` and `<q>` match (logical and)                   |

Filters are evaluated in order and the **last** matching filter wins. Every
expansion is **included by default**, so a filter list behaves like a denylist:
`exclude:` filters narrow the set, while `include:` filters carve exceptions back
out of a preceding `exclude:`. A bare predicate with no prefix is treated as
`include:`.

Because the default is to include everything, an `include:` filter only has an
effect when it follows an `exclude:` that would otherwise drop the expansion. To
*restrict* verification to expansions matching a predicate, exclude its negation.
For example, `--filter exclude:not:root:<term>` limits to a single root term.
Alternatively, `--only-root <term>` scopes expansion itself to one root rather
than filtering after the fact.

### Focusing on a single rule

To verify just the expansions containing one rule (first add a name to the rule
if it does not have one), pass `--rule <rule>`:

```
cargo run -p cranelift-isle-veri --bin veri -- --rule <rule>
```

This seeds expansion from the rule's root term and then narrows to the
expansions that actually contain the rule, so it reaches the rule even when that
root term has no standalone spec (for example, the x64 `lower` term).

## Running for `x64`

The x86-64 backend does not currently have the same coverage, but you can still run the
verifier on specific rules.

For example, the following should succeed in verifying 46 possible expansions (rule chains with monomorphized types) for the base case of an `x64` `iadd` of 32 or 64 bit values.

```
cargo run -p cranelift-isle-veri --bin veri -- --name x64 --rule iadd_base_case_32_or_64_lea
```

Here, `--name` specifies the ISLE compilation unit name, and `iadd_base_case_32_or_64_lea` scopes to a single
`lower` rule.

## Running for the mid-end (`opt`)

The mid-end optimization (`opt`) rules can be verified with the same tool using
`--name opt` (for the `opt`imization compilation unit). The mid-end rules rewrite CLIF values
primarily via the `simplify` term. There are also `simplify_skeleton` rules, but support for those
has not yet been added.

For most chains of `simplify` rules, the soundness condition is that the
rewritten value is equivalent to the original (floating point rules are the exception, as detailed below).

To verify a specific rule, such as the `x + 0 == x` rewrite in `cranelift/codegen/src/opts/arithmetic.isle`, name
the rule:

```
;; x+0 == x.
(rule iadd_x_plus_zero (simplify (iadd ty x (iconst_u ty 0)))
      (subsume x))
```

and run:

```
cargo run -p cranelift-isle-veri --bin veri -- --name opt --rule iadd_x_plus_zero
```

This verifies the rule across the monomorphized integer types (`i8`, `i16`, `i32`, `i64`):

```
Type instantiations: 4
Applicable:          4
Verification passed: 4
```

Specs for the mid-end helper terms live in
[`cranelift/codegen/src/spec/opt.isle`](../../codegen/src/spec/opt.isle).
This is where the `simplify` soundness contract is stated (`result == arg`,
relaxed to floating-point equivalence for NaN-producing rewrites; see
[Floating-point rewrites](#floating-point-rewrites) below),
along with specs for relevant helpers.
Most helper terms are verified by rule chaining; the `iconst_u`/`iconst_s`
helpers are recursive and thus cyclic, so they have explicit hand-written specs.
Other terms with external extractors also have hand-written specs.

### Floating-point rewrites

Floating-point `simplify` rules need a weaker soundness contract than integer
rules. A CLIF floating-point *arithmetic* operation that produces a NaN may
return *any* arithmetic NaN (any sign and payload with the top fraction bit set),
so a rewrite that yields a different NaN bit pattern is still correct; requiring
exact bitwise equality (`result == arg`) wrongly rejects sound rules.

This is modeled with the same execution-state mechanism as traps (see the
`(state ...)`/`(modifies ...)` forms in `inst_specs.isle`), so it lives entirely
in the specs with no special case in the verifier. A `relax_nan` state flag
defaults to false; each floating-point arithmetic op (`fadd`/`fsub`/`fmul`/
`fdiv`/`sqrt`/`fmin`/`fmax`/...) declares `(modifies relax_nan ...)` and sets it
true exactly when it produces a NaN. Deterministic bit-operations (`fneg`/
`fabs`/`fcopysign`) leave it alone. The `simplify` contract then reads the flag:

```
(if relax_nan (fp_equiv! result arg) (= result arg))
```

where `fp_equiv` holds when the two values are bitwise equal *or* both arithmetic
NaNs. So a rewrite is checked for exact equality unless one of its values came
from a NaN-producing arithmetic op, in which case NaN payload differences are
allowed.

For example, `(fmul (fneg x) (fneg y)) => (fmul x y)` is sound only under this
relaxation: when `x` or `y` is a NaN the two sides produce NaNs of different
sign. Verify it with:

```
cargo run -p cranelift-isle-veri --bin veri -- --name opt --rule fmul_fneg_fneg
```

```
Type instantiations: 2
Applicable:          2
Verification passed: 2
```

#### Caveat: relaxation assumes float-typed results

`relax_nan` is a single execution-state flag, not a per-value property, and
`fp_equiv!` interprets the bits of both values as floats. The relaxation is
therefore only sound when the rewritten value really is float-typed. It matches
Wasm semantics for the current rules, which all compose float operations into a
float result.

It could become unsound for a future rule that computes on floats but discards
the float result in favor of returning e.g. an integer. A NaN-producing arithmetic
op anywhere in the expansion sets `relax_nan`, so `simplify` would check the
integer-typed result with `fp_equiv!` and wrongly accept two integers whose bit
patterns merely both happen to look like arithmetic NaNs, even though the integer
values differ. In practice, such a rule would also be incorrect on other integer
bitpatterns that do not look like an arithmetic NaN, so the rule would fail on those
counterexamples.

### Running the whole mid-end suite

To sweep every mid-end rewrite in one run, seed from `simplify` and apply the
default excludes (see [above](#verifying-a-family-of-rules-at-once) for why these
flags are needed):

```
cargo run -p cranelift-isle-veri --bin veri -- --name opt --only-root simplify --default-excludes
```

On a 12-core M2 MacBook this takes about four minutes:

```
Total expansions:    1331
In scope expansions: 1054
Type instantiations: 5148
Applicable:          5134
Verification passed: 5134
Verification failed: 0
Verification unknown: 0
```

Equivalently, `--config cranelift/isle/veri/configs/opt-fast.args`.

## ISA Specifications

Where possible, we derive ISA specifications in VeriISLE format from
authoritative specifications distributed by vendors. Currently this is only
in place for the AArch64 backend, with specifications derived from ARM's Machine
Readable Specification in Architecture Specification Language (ASL). We rely on
the [ASLp](https://github.com/UQ-PAC/aslp) tool to assist with distilling down
the original verbose specifications to usable semantics for verification.

The resulting ISA specifications are
[checked in to the repository](../../codegen/src/isa/aarch64/spec), so there is
no requirement to install ASLp unless you want to alter existing or derive more
specifications with it.

### Generating ISA Specifications

To run ISA specification generation, you will first need to install ASLp:

1.  [Install `opam`](https://opam.ocaml.org/doc/Install.html), the OCaml Package
    Manager. The "Binary distribution" method is recommended. Ensure it is
    initialized with `opam init`; the install script assumes a working opam.
2.  Install ASLp with `./setup/install-aslp.sh`. This creates a dedicated
    OCaml 5.x opam switch named `aslp` and installs the upstream
    [ASLp](https://github.com/UQ-PAC/aslp) and
    [aslp-rpc](https://github.com/UQ-PAC/aslp-rpc) packages into it. This
    provides both the `aslp_server_http` server (used by generation) and the
    `asli` CLI (used by the `aslp` crate's test-data scripts). Set the
    `ASLP_SWITCH` environment variable to use a different switch name (the same
    variable is read by those scripts). Remove it later with
    `opam switch remove aslp`.

To run ISA specification generation, from the `isaspec` directory run:

```
./script/generate.sh -l
```

This will:

1.  Launch an instance of the `aslp_server_http` server (via `opam exec` in the
    `aslp` switch). Communicating with ASLp over a server connection allows us
    to pay the initialization cost of reading the large ASL specification once.
2.  Build and execute the `isaspec` tool.
3.  Write outputs to the `cranelift/codegen/src/isa/aarch64/spec/` directory.
