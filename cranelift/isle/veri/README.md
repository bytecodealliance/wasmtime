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

## Configuration files

Rather than configuring arguments on the command line, you can store
them in a configuration file and point the verifier at it with `--config`:

```
cargo run -p cranelift-isle-veri --bin veri -- --config cranelift/isle/veri/configs/aarch64-default-excludes.args
```

A configuration file lists one or more per line command-line arguments per line.
Blank lines and anything following a `#` (whole-line or trailing comments) are
ignored. The arguments from the file are applied *before* any passed on the
command line, so the command line always takes precedence (for example, you can
reuse a config but override its `--timeout`). Multi-valued arguments such as
`--filter` accumulate, while single-valued arguments (like `--name`) take their
last value.

Three example configurations live in [`configs/`](configs):

| File                              | Equivalent to                                                       |
| --------------------------------- | ------------------------------------------------------------------- |
| `aarch64-fast.args`               | `--default-excludes` (the default AArch64 run, see below)           |
| `aarch64.args`                    | the default AArch64 excludes but with `slow` expansions included    |
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
override this. On a 12-core M2 MacBook, the command above takes about 6 minutes.

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
./script/veri.sh -- --rule <rule>
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

On a clean checkout this should be a no-op.
