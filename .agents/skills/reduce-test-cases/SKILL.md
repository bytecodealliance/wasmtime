---
name: reduce-test-cases
description: >
  Reduce, shrink, or minimize a failing Wasmtime/Cranelift test case
  (.wasm/.wat, .wast, or .clif) down to the smallest input that still
  reproduces a bug, crash, trap, panic, or miscompilation. Gives you a
  predicate/command skeleton per reducer (wasm-tools shrink, Binaryen
  wasm-reduce, creduce, clif-util bugpoint), a wasm fallback chain for when a
  reducer can't parse your module, and a manual reduction playbook.
---

# Reduce Test Cases

A minimal reproducer is the most valuable artifact in a bug report. Reduce a
test case in two phases:

1. **Automated reducers** — the approach from
   `docs/contributing-reducing-test-cases.md`, extended to `.wast` and `.clif`
   and to a fallback chain for wasm the primary reducer can't parse.
2. **Manual reasoning** — when a reducer stalls, doesn't apply (e.g. a `.clif`
   miscompile, which `bugpoint` can't touch), or over-shrinks past your bug.

**The predicate is the whole game.** Every reducer does the same dumb loop:
mutate the input, ask a yes/no script "does this still reproduce the bug?", keep
the mutation if yes. The reducer is generic; the *predicate* is where all the
per-bug intelligence lives. This skill does **not** ship a do-everything driver
— your bug is not knowable in advance. Instead `templates/` holds a **skeleton
per reducer** that you **copy and edit** to pin your exact symptom:

| Reducer | How it hands you the candidate | "Interesting" means | Skeleton |
|---|---|---|---|
| `wasm-tools shrink` | as `$1` | script **exits 0** | `templates/predicate-shrink.sh` |
| Binaryen `wasm-reduce` | writes a **fixed `--test` file** | script's **stdout _and_ exit code both match the original** | `templates/command-wasm-reduce.sh` |
| `creduce` | candidate by **basename**, fresh temp dir | script **exits 0** | `templates/predicate-creduce-{wat,wast,clif}.sh` |
| `clif-util bugpoint` | *(no predicate)* | compile **panics** | *(built in)* |

For a **miscompile** — no trap or panic to grep, just a wrong result — the
predicate diffs *behavior* across two configs instead. That shape is orthogonal
to the reducer, so it has its own skeleton, `templates/predicate-differential.sh`,
that you drop into the shrink or creduce contract above. In practice this was the
most common real-world case, so reach for it first when the bug is a wrong answer.

All paths below are relative to the repo root.

## Prerequisites

Install the reducers (`wasm-tools` ships with the toolchain; the rest are
external):

```bash
cargo install wasm-tools     # shrink + print (often already present)
brew install creduce         # generic text reducer for .clif / .wast / .wat
brew install binaryen        # wasm-reduce, the alternate wasm reducer
```

Build the engines your predicates will run. A **debug** build is required to
reproduce `debug_assert!` panics; a `--release` build iterates much faster for
everything else, which matters when a reducer runs the predicate thousands of
times:

```bash
cargo build --bin wasmtime                                                    # debug: target/debug/wasmtime
cargo build --release --bin wasmtime                                          # fast: target/release/wasmtime
cargo build --release --manifest-path=./cranelift/Cargo.toml --bin clif-util  # target/release/clif-util
```

Every skeleton has a `WASMTIME=` / `CLIF_UTIL=` line you must set to an
**absolute** path — reducers run from other directories, so a relative path
breaks. Print the absolute path and paste it in:

```bash
echo "$PWD/target/release/wasmtime"
echo "$PWD/target/release/clif-util"
```

## Workflow

Work in a scratch directory holding your failing input. The loop never changes:

1. **Reproduce the bug once, by hand,** to pin the *exact* symptom — the specific
   trap string, panic message, ISLE error, or diverging output. This is the most
   important step: a vague predicate reduces to a *different* bug. For a
   miscompile, also **confirm it's deterministic** — run each config a few times
   and check every run of a config agrees and the two configs disagree; a flaky
   config makes a differential predicate unsound.
2. **Copy the right skeleton** from `templates/` next to your input.
3. **Edit it:** set the absolute engine path, and replace the example symptom
   with yours. Keep it tight — a loose predicate lets the reducer converge on
   some other, spec-legal failure.
4. **Test the predicate on the ORIGINAL.** If it doesn't fire here, it never
   will on a smaller file. (For `shrink`/`creduce`: `./pred.sh input` should exit
   0. For `wasm-reduce`: the command should print your marker.)
5. **Run the reducer** (per format, below).
6. **Eyeball the result** and re-confirm it's still *your* bug — reducers are
   happy to find a smaller program that fails for an unrelated reason.
7. **Re-verify the final case independently** against the predicate — especially
   anything a subagent/fork produced, or a hand-reduction. Convert every
   candidate to the same form before comparing sizes (wasm bytes *and* canonical
   `wasm-tools print` line count); raw creduce/hand whitespace isn't comparable.
8. **Re-author the winner to be legible**, not just small. Give locals/functions
   readable names, add a one-paragraph comment explaining the mechanism, and for
   a `.wast` make it self-checking with `assert_trap` / `assert_return` so the
   file *proves* the bug on its own. The best artifact teaches the fix.

## wasm / wat — a three-tool fallback chain

Each wasm reducer only understands the proposals its own parser implements, and
those subsets differ. Work down the chain until one bites:

### 1. `wasm-tools shrink` — try this first

Smartest, most structural, produces the smallest output. Uses
`templates/predicate-shrink.sh` (candidate is `$1`, exit 0 == reproduces):

```bash
cp .agents/skills/reduce-test-cases/templates/predicate-shrink.sh pred.sh
# edit pred.sh: set WASMTIME=<abs path>, set the symptom grep
chmod +x pred.sh
./pred.sh input.wasm && echo "reproduces on original"   # sanity check first
wasm-tools shrink ./pred.sh input.wasm -o reduced.wasm
wasm-tools print reduced.wasm
```

Verified end-to-end: a 273-byte module (noise functions, globals, a table, data
segments) reduced to **42 bytes** — exactly the trapping core:

```
(module
  (type (;0;) (func))
  (export "_start" (func 0))
  (func (;0;) (type 0)
    i32.const 42
    i32.const 0
    i32.div_s
    drop))
```

### 2. Binaryen `wasm-reduce` — if shrink can't parse your module

Different proposal coverage from shrink — and the gap is real, not theoretical:
Binaryen can't parse wide-arithmetic (`i64.mul_wide_s`, `i64.add128`) or some
SIMD, while `wasm-tools shrink` handles them. So for a module using those, skip
straight from shrink to creduce; `wasm-reduce` is a non-starter. It also has a
**different contract**: it writes each candidate to a fixed `--test` file (not
`$1`), and keeps a candidate only if your command's **stdout and exit code both
match the original run** — so print one canonical marker when the bug
reproduces. Uses `templates/command-wasm-reduce.sh`:

```bash
cp .agents/skills/reduce-test-cases/templates/command-wasm-reduce.sh cmd.sh
# edit cmd.sh: set WASMTIME=<abs path>, set the symptom marker
cp input.wasm t.wasm && bash cmd.sh          # sanity check: prints your marker
wasm-reduce input.wasm --command 'bash ./cmd.sh' --test t.wasm --working reduced.wasm -f
```

The `-f` matters: before reducing, `wasm-reduce` round-trips your module through
Binaryen's own parser/writer, and if it can't (an unsupported proposal) it prints
`failed to read and write the binary` and stops. `-f` forces past that. Verified:
with `-f`, the same 273-byte module reduced to **91 bytes** and still trapped —
larger than shrink's 42 bytes (it kept the table/memory/globals), which is
exactly why this is a fallback, not the first choice.

### 3. Disassemble + `creduce` — if neither reducer can parse it

`creduce` reduces **text** and never re-encodes the wasm, so **only the engine
has to understand the proposal** — the reducer stays out of the way. This always
*runs*, at the cost of being purely textual: because it doesn't understand `.wat`
structure it tends to plateau early on parenthesis-balance constraints (in one
real case it stalled at ~43% of the original and no further). Treat its output as
a starting point for the manual playbook, not the finish line. `wasmtime` runs a
`.wat` directly, so no reassembly is needed. Uses `templates/predicate-creduce-wat.sh`:

```bash
wasm-tools print input.wasm -o case.wat          # disassemble
cp .agents/skills/reduce-test-cases/templates/predicate-creduce-wat.sh pred.sh
# edit pred.sh: set WASMTIME=<abs path>, set the symptom; CAND=case.wat
chmod +x pred.sh
mkdir -p /tmp/reduce && cp pred.sh case.wat /tmp/reduce && cd /tmp/reduce
creduce ./pred.sh case.wat                        # reduces case.wat in place
```

Verified: a 52-line disassembly reduced to two lines that still trap:

```
(module(export "_start"(func $a))(
    func $a i32.const 2 i32.const 0 i32.div_s drop))
```

## wast — `creduce`

`wasm-tools shrink` reduces **one module**; a `.wast` is a whole *script*
(multiple modules + assertions), so reduce the text with `creduce`. Run the
script with `wasmtime wast`. Uses `templates/predicate-creduce-wast.sh`:

```bash
target/release/wasmtime wast input.wast           # reproduce first
cp .agents/skills/reduce-test-cases/templates/predicate-creduce-wast.sh pred.sh
# edit pred.sh: set WASMTIME=<abs path>, set the symptom; CAND=input.wast
chmod +x pred.sh
mkdir -p /tmp/reduce && cp pred.sh input.wast /tmp/reduce && cd /tmp/reduce
creduce ./pred.sh input.wast
```

Verified: a 30-line, 3-module script (with passing-assertion noise) reduced to a
single line that still traps with `integer divide by zero`:

```
(module(func $a i32.const 2 i32.const 0 i32.div_s drop)(start $a))
```

Feature flags come from `-W` on the CLI (e.g. `-W gc,function-references`), **not**
from the `;;! feature = true` header — that header is only read by
`cargo test --test wast`. `creduce` will delete the `;;!` header (the CLI ignores
it), so re-add it if the reduced case is destined for `tests/`.

## clif — `bugpoint` (panics only) or `creduce` (everything else)

`clif-util bugpoint <FILE> <TARGET>` reduces **compile panics only**. It has no
predicate hook — it catches panics internally and treats a clean compile *or a
verifier error* as "not interesting". On a `.clif` that fails *gracefully* (the
common `test compile expect-fail` case) it bails:

```bash
target/release/clif-util bugpoint be.clif riscv64
# Warning: Given function compiled successfully or gave a verifier error.  (exit 0)
```

For anything that isn't a hard panic — miscompiles, wrong output, graceful
codegen errors, `test`/`run` failures — use `creduce` with a `clif-util`
predicate. Uses `templates/predicate-creduce-clif.sh`:

```bash
cp .agents/skills/reduce-test-cases/templates/predicate-creduce-clif.sh pred.sh
# edit pred.sh: set CLIF_UTIL=<abs path>, pick the sub-command, set the symptom
chmod +x pred.sh
mkdir -p /tmp/reduce && cp pred.sh be.clif /tmp/reduce && cd /tmp/reduce
creduce ./pred.sh be.clif
```

Verified: a 287-line, 46-function `big-endian.clif` reduced to **one function**
that still hits the target codegen error:

```
function % (i64)->i128 { block0(v0 : i64) : v1 = load.i128 big v0 return v1 }
```

```bash
target/release/clif-util compile --target riscv64 be.reduced.clif
# Error: Unsupported feature: should be implemented in ISLE: inst = `v1 = load.i128 big v0`, type = `Some(types::I128)`
```

The predicate's sub-command is your choice: `clif-util compile` (codegen; target
on the CLI), `clif-util test` (honors in-file `; run:` / `; check:` directives),
`clif-util run` (host JIT), or `clif-util interpret` (backend-independent — good
for miscompile differentials). `creduce` deletes the `test`/`set`/`target` header
lines, so re-add them if you want the reduced file to run under `clif-util test`.

## Miscompiles — differential predicates

A miscompile has no trap or panic to grep; "interesting" means *the answer is
wrong*, which you detect by running two configs and checking they **disagree**.
This was the most common real-world case. The reducer doesn't care — it's just a
predicate shape — so `templates/predicate-differential.sh` drops into either the
shrink (`$1`) or creduce (basename) contract. The load-bearing detail is the
guard:

```bash
a=$("$WASMTIME" -O opt-level=0 --invoke f "$CAND" 2>/dev/null); ra=$?
b=$("$WASMTIME"                --invoke f "$CAND" 2>/dev/null); rb=$?   # default == opt-level=2
[ $ra -eq 0 ] && [ $rb -eq 0 ] && [ -n "$a" ] && [ -n "$b" ] && [ "$a" != "$b" ]
```

Both configs must run cleanly *and* produce output *and* differ. Drop the
exit-0/non-empty guards and a candidate that merely became invalid or started
trapping satisfies `a != b` for free, so the reducer converges on a broken
program instead of a miscompile. Real fuzzer repros invoke a **named export with
arguments** and need the right **`-W` feature flags** (`-Wwide-arithmetic
--invoke f "$CAND" 1 -1`) — the defaults in the trap skeletons won't cut it.
Confirm determinism first (Workflow step 1). For a `.wast`, prefer the
self-checking form: encode the right answer as `assert_return` / `assert_trap`
and make "interesting" == "opt-level=0 passes but default fails".

## Manual reduction playbook

When the reducer stalls, over-shrinks, or the bug is a miscompile no predicate
cleanly captures, reduce by hand. Reason from the bug's *mechanism* back to the
minimal trigger, and re-verify after every cut:

- **Bisect coarse first.** Delete whole functions / modules / assertions, half at
  a time; keep the half that still reproduces. `clif-util bugpoint`'s own strategy
  is a good mental model: remove instruction → replace with const → replace with
  trap → remove block → drop unused entities → merge blocks.
- **Then bisect instructions** inside the surviving function the same way.
- **Neutralize data.** Shrink memory/table sizes to the minimum; truncate or zero
  data/element segments; drop unused globals, types, imports, and exports.
- **Simplify operands.** Replace computed values with constants; collapse control
  flow (drop unused blocks/params); inline single-use locals.
- **Re-verify each step** with `wasm-tools print` / `clif-util cat` and by
  re-running the engine. If a cut kills the repro, revert it and try another.
- **Keep a copy of the last-good reduction** so a bad cut is always recoverable.

For a **miscompile**, the manual pass is where you *win* — automated reducers
have no idea which bytes carry the bug, so they grind on structure while the
real reduction is semantic. Hunt the diverging value directly:

- **Neutralize inputs one at a time, don't delete them.** Replace each
  `local.get` / `global.get` (or `.clif` value) feeding the wrong result with a
  constant, one at a time; the one whose neutralization makes the divergence
  *vanish* is carrying the corruption. Prefer this over deleting/truncating:
  cutting code changes *what the optimizer does*, so a cut that "fixes" the bug
  may have perturbed the pass rather than removed the cause — neutralization
  keeps the shape and isolates the value.
- **Return the suspect directly.** Once you've found the diverging value,
  `return (local.get $culprit)` (or make it the sole result) collapses the
  entire downstream computation in one edit — often dropping the bulk of the
  program at a stroke.
- **Constant-fold the setup.** Hash/mix stages, loop trip counts, and data
  segments that only *produce* the suspect value can be replaced by the constant
  they compute. Collapse a loop to the smallest trip count that still diverges.
- **Then rebuild from scratch.** Once the mechanism is clear, hand-writing a
  fresh minimal module from the understood cause beats continuing to chip at the
  reduced-but-messy one — and yields the legible artifact you actually want. In
  one real fuzzer case this took a 649-line module to ~22 lines (4%), where
  creduce plateaued at 43% and shrink barely moved it.

**Race the reducers; don't serialize them.** shrink, creduce, and a manual pass
are cheap to run at once — start each in its **own** temp dir (creduce reduces
in place and will collide otherwise), let creduce run in the background (it far
exceeds a single tool-call timeout), and spin up a **fork/subagent for the
semantic reduction** with full context on the bug's mechanism. Then take the
smallest legible result. The manual fork usually wins the hard cases.

The goal is the smallest input where the bug's root cause is still obvious — not
just small, but *legible*.

## Gotchas

Battle scars from actually running these:

- **`wasm-reduce` preserves behavior, not "exit 0".** It keeps a candidate only
  if stdout *and* exit code match the original run — unlike shrink/creduce, which
  just want exit 0. Collapse "reproduces" to one canonical stdout marker and keep
  the exit code stable. And it reads the fixed `--test` file, **not `$1`**.
- **`wasm-reduce` needs `-f` for unsupported proposals.** It round-trips the
  module through Binaryen first and prints `failed to read and write the binary`
  if it can't. `-f` forces past it; if even that gets nowhere, drop to
  disassemble + creduce. Some proposals it simply can't represent (e.g.
  wide-arithmetic's `i64.mul_wide_s`) — for those, `-f` won't save it; skip to
  creduce.
- **Environment variables don't reliably reach `wasm-reduce`'s `--command`.**
  Hardcode the absolute engine path in the command script (the skeleton does).
- **creduce strips header directives.** It deletes anything that doesn't change
  interestingness — the `.clif` `test`/`set`/`target` lines and the `.wast`
  `;;! feature = true` header included. Fine for a repro whose predicate bakes the
  target/features into the command, but the reduced file won't run under
  `cargo test --test wast` or `clif-util test` unmodified. Re-add the header (or
  the matching `-W` / `--target` / `--set` flags) afterward.
- **creduce runs in a fresh temp dir** containing only the candidate, referenced
  by *basename*. Predicates must reference the candidate by basename and every
  tool by an **absolute** path. If you hand-roll a wrapper that `cd`s, resolve the
  predicate to an absolute path *before* changing directory.
- **Don't put `set -e` in a shrink/creduce predicate.** A trapping module makes
  the engine exit non-zero; under `set -e` the `out=$(engine …)` line aborts the
  script before your grep runs, so the predicate never reports "interesting". Let
  the final grep be the exit status.
- **`bugpoint` is panic-only, and only on the plain compile flow.** It ignores
  verifier and graceful codegen errors ("compiled successfully or gave a verifier
  error"), and it only drives `clif-util compile` — a panic that fires only under
  a `clif-util test <flavor>` pass (e.g. `test alias-analysis`) is invisible to
  it. It cannot reduce a miscompile either. For any of these, use `creduce`.
- **shrink plateaus on already-minimized or fragile fuzzer input.** If the bug
  report says it was minimized with `wasm-tools shrink`, re-running shrink buys
  almost nothing (one real case moved <2%) — you're at a shrink local minimum.
  Likewise a fragile miscompile where removing almost *anything* makes the
  divergence vanish resists every structural reducer. Recognize the plateau early
  and switch to creduce and the manual playbook rather than grinding.
- **Fuzzy predicates drift.** A reduced module can fail the same way for a
  *different*, spec-legal reason. Pin the exact message and eyeball the result.
- **Debug vs release — check the panic site.** A plain `panic!` / `.unwrap()` /
  `.expect()` fires in a `--release` engine too, so release (which iterates far
  faster) is enough; only `debug_assert!` needs a debug build. Look at the assert
  that fires before rebuilding. Point the skeleton's `WASMTIME=` / `CLIF_UTIL=`
  at whichever you actually need.

## Troubleshooting

- **Predicate fails on the original** — it doesn't reproduce yet. Wrong trap
  string, wrong `--invoke` export, missing `-W` feature flag, or wrong engine
  build. Fix the predicate before reducing anything.
- **`wasm-reduce` says "failed to read and write the binary" / "very unlikely
  reduction can succeed"** — Binaryen can't parse a proposal in your module. Add
  `-f`; if that stalls too, disassemble + creduce.
- **`bugpoint` prints "compiled successfully or gave a verifier error" (exit
  0)** — the input fails *gracefully*, it doesn't panic. Expected for
  `expect-fail` files. Switch to `creduce`.
- **`creduce` finishes but the file is unchanged** — the predicate never passed
  on any mutation. Usual causes: a relative tool path that broke in creduce's temp
  dir, a predicate that isn't `chmod +x`, or `set -e` swallowing a trapping exit.
  Re-test the predicate on the original and confirm absolute tool paths.
