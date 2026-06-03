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
./veri/script/install/cvc5.sh -i <install_path>
./veri/script/install/z3.sh -b <install_path>/bin
```

If you use this method, ensure that `<install_path>/bin` is on your `$PATH`.

## Running

To run the verifier, from the `cranelift/isle/veri/veri` directory run:

```
./script/veri.sh
```

This will run verification on the default AArch64 backend. To run on the X64
backend, add the `-a x64` option.

By default the verifier attempts every expansion it can reach. It seeds an
expansion at every term that has rules, a constructor, and an explicit
specification, and verifies all rule chains reachable from those roots. Terms
without a spec are not verified standalone; they are only checked when chained
(inlined) into a specified root. Expansions tagged `TODO` are skipped by default
(pass `--no-skip-todo` to include them).

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
For example, to focus on all expansions involving a given rule (first add a name
to the rule if it does not have one):

```
./script/veri.sh -- --filter exclude:not:rule:<rule>
```

Similarly, `--filter exclude:not:root:<term>` limits to a single root term.
Alternatively, `--only-root <term>` scopes expansion itself to one root rather
than filtering after the fact.

## ISA Specifications

Where possible we derive ISA specifications in VeriISLE format from
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
    Manager. The "Binary distribution" method is recommended.
2.  Install ASLp with `./veri/script/install/aslp.sh -i <aslp_install_path>`.
3.  Ensure ASLp tools are available by adding `<aslp_install_path>/bin` to your
    `PATH`.

To run ISA specification generation, from the `isaspec` directory run:

```
./script/generate.sh -l
```

This will:

1.  Launch an instance of the `aslp-server`. Communicating with ASLp over a
    server connection allows us to pay the initialization cost of reading the
    large ASL specification once.
2.  Build and execute the `isaspec` tool.
3.  Write outputs to the `cranelift/codegen/src/isa/aarch64/spec/` directory.

On a clean checkout this should be a no-op.
