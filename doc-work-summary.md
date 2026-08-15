# Wasmtime contribution summary

## Scope and issue selected

I worked in the upstream Wasmtime repository and selected a genuinely scoped, compiler-correctness bug in Cranelift:

- Issue: #14131, "Debug assert in alias analysis pass"
- Area: Cranelift alias analysis / optimizer correctness
- Why it is a meaningful contribution: it is a real bug in optimizer bookkeeping rather than a cosmetic change, and it is aligned with Rust, WebAssembly, Cranelift, and compiler infrastructure.

This was a good fit because it involved:
- a real, reported bug,
- a narrow compiler-infra code path,
- reproducible failing CLIF input,
- and a small, testable fix within an existing project convention.

## Initial repo and contribution review

I inspected the project contribution entry points and repo standards in [CONTRIBUTING.md](CONTRIBUTING.md), then narrowed to an active upstream issue. The issue was selected because it matched the project’s real bugfix workflow and was small enough to validate with a focused filetest.

I also checked the relevant Cranelift code and existing alias-analysis filetests to keep the patch consistent with project conventions.

## Root cause

The root cause was an overly strict debug assertion in `AliasAnalysis::process_inst` inside [cranelift/codegen/src/alias_analysis.rs](cranelift/codegen/src/alias_analysis.rs).

Before the fix, the code did this:

- it precomputed a function-wide `observed_stores` set in `compute_block_input_states`,
- then later during instruction processing it called `state.update(...)`,
- and expected the `observed_stores` length to stay exactly constant.

That expectation was not valid in this optimizer. After a dead-store or idempotent-store rewrite, later processing can discover additional observations that were not known in the earlier block-state pass. In other words, the set is monotonic: it may grow, but it should not shrink.

The bug report reproduced that exact problem with a minimal CLIF snippet. A later store rewrote state and discovered a new observation, causing the assertion to fire even though the optimizer was behaving consistently.

## Fix implemented

I updated the debug assertion in [cranelift/codegen/src/alias_analysis.rs](cranelift/codegen/src/alias_analysis.rs) to assert the actual invariant:

- the observed-store set should never shrink,
- it may grow as later optimization passes discover more observations.

That preserves the original intent of the assertion while matching actual dataflow behavior.

This is a clean fix because it does not change optimization semantics; it only corrects the debug check that was rejecting valid analysis behavior.

## Regression test added

I added a focused test file:

- [cranelift/filetests/filetests/alias/issue-14131.clif](cranelift/filetests/filetests/alias/issue-14131.clif)

This file captures the exact reproducer from the issue and is consistent with the project’s filetest-based regression workflow for Cranelift alias analysis.

## Verification performed

I ran the targeted checks under the repo’s required Rust toolchain (Rust 1.95.0):

1. Single reproducer test:
   - command: `cargo +1.95.0 run -p cranelift-tools -- test cranelift/filetests/filetests/alias/issue-14131.clif`
   - result: passed

2. Alias-analysis filetests subset:
   - command: `cargo +1.95.0 run -p cranelift-tools -- test cranelift/filetests/filetests/alias`
   - result: passed, with the suite reporting 40 tests and no failures

I also reviewed the final diff to confirm the patch stayed narrow and free of unrelated edits.

## Summary of the patch

The patch is intentionally minimal and targeted:

- no feature work,
- no unrelated cleanup,
- no cosmetic edits,
- only the debug assertion fix and the regression test required to protect it.

This qualifies as a proper upstream-quality bugfix aligned with Wasmtime’s compiler infrastructure goals.

## PR-ready summary

Title:
`cranelift: fix alias-analysis debug invariant for observed stores`

Description:

> Fix a debug-only assertion in Cranelift alias analysis that incorrectly assumed the `observed_stores` set cannot grow during later instruction processing. In reality, additional observations may be discovered after dead-store/idempotent-store rewrites, so the set should be monotonic rather than fixed-size. Add a focused filetest reproducer for the fuzzed regression and keep the fix scoped to the underlying invariant.
