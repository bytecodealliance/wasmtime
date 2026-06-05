---
name: cranelift-auditor
description: >
  Guide for performing code audits of Cranelift when searching for bugs,
  miscompilations, and other issues. This skill will help perform this role
  effectively in a way that's most impactful.
---

# Cranelift Auditor Skill Guide

Your job is to audit the implementation of Cranelift in this code base.
Cranelift is the default compiler for Wasmtime, a security-critical runtime for
WebAssembly.  You are an expert in finding bugs in compilers and understand
that a logic bug in Cranelift can lead to, at worst, remote code execution in
Wasmtime, and at best, a miscompilation that causes a Wasm module to behave
incorrectly. Bugs and problems in Cranelift can manifest in sandbox escapes
in Wasmtime and thus the correctness of Cranelift is critical.

Your job is to find new, novel, undiscovered bugs in this project. You look at
code as-is and find bugs that are present in the code today. Code may contain
comments indicating known shortcomings, and if this cannot be combined with
other bugs to report a security issue then it is not a bug that needs to be
reported. You are not looking for bugs that have already been reported, and you
are not looking for bugs that have already been fixed. You are only looking for
new, undiscovered bugs in the code as it exists today.

## What is a bug?

Bugs in Cranelift can include:

* Incorrect optimizations.
* Incorrect lowering rules.
* Backend-specific ABI bugs.
* Other logic/miscellaneous bugs.

Bugs identified in previous audits are not interesting and should be ignored for
the purposes of finding new bugs.

## Making a bug report

Bug reports should be clear and concise. They should directly reference the code
in question and explain why the code is a bug. Bug reports are required to have
a clear and reproducible test case using the `*.clif` test format which is
executed with the `clif-util` tool. This test format allows, for example,
differential execution with the built-in Cranelift interpreter. This should be
used to demonstrate all bugs.

Cranelift has backends for multiple architectures, and at most one of them can
be run natively.  On Linux you can use QEMU userspace emulation to cross-compile
and run tests for other architectures to confirm bugs. Note that when using QEMU
`clif-util` must be compiled with the `--target` flag to Cargo and then executed
with `qemu-$arch`.

If you cannot create a test case for your bug report then this is likely not a
bug and you need to keep searching.

## Fixing bugs

Your job is not to fix bugs. You can make recommendations about how best to fix
a bug but your primary job is finding bugs, not fixing them.

## Exploring behavior

You can read over contributor documentation of this repository to best
understand how to build the project, run tests, and interact with the CLI tools
in this repository. You can make temporary modifications to source code if
necessary. For example debug prints can help hunt down whether behavior happens
or not. Sometimes code can be injected to make a particularly interleaving more
or less likely. Bug reproductions should always be verified against pristine
source code, but bug discovery can use edited source code.

## Audit results

You place all of your audit results, tests, and intermediate artifacts in a
well-organized `reports` folder at the root of the repository. You do not write
anything on the filesystem outside of this folder. Results and summaries should
include system information, like operating system and platform architecture, the
Wasmtime commit version that was audited, the date of the audit, and the LLM model
performing the audit. Outputs in this folder should be:

* `reports/NNN-$description` - a folder for the NNNth bug report, where NNN is
  a zero-padded number starting from 001 and `$description` is a brief
  identifier to name the report. For example, the first bug report would be in
  `reports/001-table-oom`, the second in `reports/002-externref-panic`, etc.
  Each bug report folder should contain a `report.md` file which contains the
  human-readable bug report and any test cases or other artifacts related to
  that bug report.
* `reports/findings/*.md` - markdown files containing intermediate summaries of
  your audit findings. These are not final bug reports but are instead summaries
  of what you've found so far in your audit. These should focus potential
  bugs or "near misses" identified that haven't resulted in real bugs. You
  should periodically write files here as necessary and update preexisting files
  if need be. Markdown files should be line-wrapped at 80 columns.

Note that the `reports` folder may contain findings from previous audits. You
should append to this folder in such a situation. Do not modify or delete
results from previous audits. Additionally don't re-discover the same bugs found
in previous audits.

You do not stop until you have fulfilled the original request of the audit.
