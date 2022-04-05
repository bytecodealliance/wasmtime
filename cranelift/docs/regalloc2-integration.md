# Overview of changes for regalloc2 integration

## Register Types

- Reg, RealReg, VirtualReg wrap the regalloc2-level VReg. These types
  are meant to be basically compatible with regalloc's
  types. Instructions continue to carry Regs.

- Operands are distinct from Regs, and contain constraints as well as
  the Reg (actually the underlying VReg). They can be "early" or
  "late" and uses/defs/mods, can pin a def to the same register as an
  inbound use ("reuse" constraints), or can pin a vreg to a particular
  physical reg.

  - We also have "pinned VRegs" that are guaranteed to be allocated in
    the PRegs they're pinned to at each use/def site, and these are
    the direct translations of RealRegs. All backend code that
    generates moves to/from RealRegs continues to work. Over time we
    can migrate e.g. callsite handling to emit a single call
    instruction with vregs bound just at that site via fixed-reg
    constraints, rather than a sequence of moves to/from RealRegs.

## VCode Invariants and Structure

- The `Function` trait required by regalloc2 asks for a slightly
  different interface, but not conceptually too distant from that of
  regalloc.rs.
  - Block preds as well as succs; we compute this when finalizing
    just-built VCode.
  - blockparams. We translate CLIF-level blockparams directly to VCode
    blockparams.
  - blockparam branch args. We translate these directly as well. Note
    that this is a 2D list (a list of lists): a branch has, for the
    list of succs, a list of args for each succ.
    - We represent the list-of-lists using the same "contiguous-array
      with separate array of (start, end) ranges" technique that we
      use for succs/preds. This results in storage/allocation
      efficiency (a small constant number of large block allocations,
      and really fast traversal vs pointer-chasing). The added
      subtlety with a 2D list is that we have two levels of this. A
      bit mind-bending but I promise the performance is worth it!

- regalloc2's checks for critical edges in the CFG are more strict. It
  turns out that `BlockLoweringOrder` was generating a CFG in the
  VCode that wasn't *quite* free of critical edges, but in a way that
  didn't matter in practice with the old arrangement. In particular,
  when there are multiple edges from A to B (i.e., the original CLIF
  CFG is actually a hypergraph) -- this can happen easily coming from
  a `br_table` for example -- then only one edge block for (A, B) was
  inserted, and shared by all edges out of A.
  
  When using regalloc.rs this didn't matter because we only used edge
  blocks from within Cranelift's lowering (lower.rs) where we emitted
  moves for blockparams, and CLIF did not allow blockparam args on
  br_table targets.
  
  But now regalloc2 handles blockparams, and does a full/strict check
  for critical edges, so we have to get this right. The fix is to add
  a "successor index" in the `LoweredBlock` enum arms that include an
  edge, to disambiguate e.g. `(A, 0, B)` and `(A, 1, B)`, the two
  edges from `A` to `B`.

## Architecture: Compilation Pipeline

- The biggest change is that we lean fully into regalloc2's "immutable
  view of code -> separate output" design. It's more idiomatic Rust,
  and it also results in higher performance. regalloc.rs would edit
  instructions' Regs in its final stage, and also take ownership of
  the Vec and return a new Vec, with moves/spills/reloads
  inserted. This required careful bookkeeping to translate block
  boundaries and such, and also required an expensive "incorporate
  regalloc results back into VCode" pass.
  
  We now structure things so that the regalloc2 result
  (`regalloc2::Output`) is provided directly to `VCode::emit`. The
  instructions always carry their original `Reg`s. Some of these will
  be virtual and some will be real; that's fine. We build a separate
  abstraction, the `AllocationConsumer`, that for a given instruction
  reads the allocations in the `Output` assigned to this instruction,
  takes the original as well, and provides a RealReg, guaranteed. This
  is carefully structured so that instructions that are built as part
  of other instructions' emission (macroinsts that lower to simpler
  insts), and those in the prologue/epilogues, all of which are
  created after regalloc completes, work as well: an empty
  `&[Allocation]` to `MachInst::emit` means "I assume you have only
  `RealReg`s in your operands", and this Just Works.
  
  This structure has some consequences on pretty-printing and on ABI
  code (prologues and epilogues) as well. Regarding ABI sequences: we
  previously inserted these in the post-regalloc fixup, where we at
  the same time reorder blocks into final machine-code order. Then the
  `VCode` is transformed into a more-or-less direct analogue of the
  machine code. Now, in order to avoid mutating `VCode` after
  regalloc, the final sequence exists only ephemerally as we emit: we
  generate the prologue and emit its instructions right away at the
  top of `emit`, and the epilogue and emit its instructions right away
  at each `ret`.
  
  Because we no longer construct the machine-code-analogue state in
  the `VCode`, it does not make muchs ense to pretty-print it after
  regalloc to see that result either. Instead, the disassembly is
  constructed alongside machine-code emission (in a way such that it
  would be hard for the two to diverge) only if requested.
  
  regalloc2 provides an iterator that traverses original instruction
  indices and inserted edits in an interspersed manner, making this
  very easy to do. We match on `InstOrEdit` and emit either an inst,
  or generate an edit (move, spill, reload), at each step.
  
  This whole scheme means we avoid a lot of shuffling to build state
  in memory that we just emit and throw away -- much faster! And note
  that the `Operand` and `Allocation` arrays that are separately (i)
  fed into regalloc2 and (ii) produced by regalloc2 also exist in some
  form in regalloc.rs, they just aren't exposed at the API layer; so
  these are not really additional overhead.
  
  Because VCode is not mutated, some information that was magically
  stored on it post-regalloc or even post-emit (e.g. layout info for
  debug) is now returned in a separate `EmitResult` struct.
  
  Ideally `VCode::emit` would take a `&self`. Right now it consumes
  the VCode (takes `self`) only because the `ABICallee` saves some
  state when generating the prologue (it only computes clobbers and
  hence frame size at that point) and I didn't want to play tricks
  with cells, or clone it or whatnot, to make this work.

- The existing scheme for lowering code (`Lower` in lower.rs) does a
  several-step list-reversing dance. The overall traversal is
  bottom-to-top, because we need to see whether isel of an instruction
  merges any of its input instructions (i.e., how far up the tree it
  matches) before knowing whether we even need to generate some of the
  values produced by instructions further up.
  
  Within the context of lowering a single CLIF instruction, though,
  VCode instructions are emitted in forward order.
  
  We currently reverse three times: once when finishing the VCode
  instructions that map from an IR instruction, so they are in
  *reverse* at the end of the current block; then again when finishing
  a block, which now has CLIF insts in reverse and VCode insts per
  CLIF inst also in reverse, so we have the whole block of VCode insts
  in forward order. But we still have blocks in a reverse order. Then
  finally, post-regalloc, we copy instructions into the final block
  order.
  
  We will eliminate the post-regalloc phase (above), so we reconsider
  the way we do this reversal. We still reverse VCode insts per IR
  inst, but then when finishing the IR inst we append the reversed seq
  to `vcode.insts`. So in the end, because the pass was bottom-to-top,
  `vcode.insts` is in exactly the reverse order as the final
  machine-code ordering. So we can reverse just once.
  
  We need to rewrite InsnIndex values after this reversal. We also
  need to reverse all per-block lists. Note however that when we have
  a contiguous-backing-array-indexed-by-ranges storage scheme, we only
  need to reverse the toplevel array of ranges, not the underlying
  storage array (it is effectively an unordered arena).
  
  All of this is done in `VCode::reverse` and allows us to do a
  backward traversal and end up with final-order code *without* the
  post-regalloc phase.
  
  Finally, this has been designed in a slightly forward-looking way,
  with the reversal an optional thing (the VCodeBuilder has "backward"
  and "forward" modes), with the idea that we could eventually use
  this as an intermediate container that collects the results of
  either backward or forward transform passes.
  
- To avoid the need to have a map-regs method on VCode instructions at
  all (and hence have only one canonical method that produces the
  operands, rather than two that have to stay in sync), the new design
  replaces the "register renaming" with a vreg alias
  mechanism. Instruction lowering can return whatever vreg (or group
  of vregs, for multi-reg values) it likes as the result, and we will
  alias the original vreg that was created for this Value to the
  returned VReg. Other instructions that have already been generated
  and use this Value will refer to the original VReg. But when we
  generate Operands to pass to regalloc2, we translate through the
  alias table.
  
  There is some precedent for this design in other use-cases (see
  Value aliasing in CLIF), and it also means we avoid an intermediate
  buffering stage for Insts, which cuts down on memory traffic.
  
  It also means that the `match` on `Inst`, to find the operands,
  happens only twice (in `collect_operands` and later in `emit`),
  rather than four times (`map_regs` during renaming, `get_regs`
  called by regalloc.rs, later `map_regs` called by regalloc.rs to
  insert allocations, then finally in `emit`). This is actually pretty
  important, as the match is very control-flow intensive and hard to
  predict.

- Debug info is handled natively by the regalloc now, rather than
  reconstructed from an awkward post-pass. regalloc.rs did not have a
  way of tracking where a value ends up based on arbitrary labels, so
  we inserted pseudo-instructions ("value markers") that consumed
  program-level values, let those go through regalloc, then did a
  dataflow analysis on the machine code with code offsets as locations
  (!) to build debug info. This was awkward, slow, and
  error-prone. regalloc2 now has a mechanism to apply labels to VRegs
  for certain ranges, and will indicate which Allocations have those
  labels on the output side.
  
## x86-64 backend effects

- Instructions really become three-operand, including during
  pretty-print. This is kind of awkward, but necessary to show
  pre-regalloc vcode correctly. Previously, many of the instructions
  (e.g., `AluRmiR` and similar) had been made three-operand (or more
  precisely, to have separate sources and a dest) as part of the ISLE
  migration, but pretty-printing showed them in conventional
  two-operand x86 form.
  
  Pretty-printing takes an `&[Allocation]` as well and uses the same
  `AllocationConsumer` to print allocated registers when producing the
  post-regalloc disassembly.

- As a consequence of reused-input operands, we can specify the
  constraint that (e.g.) `src1 == dst` by emission time directly to
  regalloc2, and get the same allocation for both, so we can emit an
  actual x86 two-operand instruction. This completely replaces the
  separate-move idiom, mostly generated by mov-mitosis in today's
  backend. This ends up being both more efficient (we start with many
  fewer instructions and the regalloc inserts moves only where
  necessary, rather than the other way around relying on move elision)
  and cleaner to read in pre-regalloc disassemblies.

- Can still generate non-SSA code and use mod ("modify") regalloc
  operands when necessary, and can still use fixed RealRegs.
  
  Aside: we should work to reduce these over time, and ideally we
  should eventually remove mod operands and non-SSA support from
  regalloc2; this would allow additional optimizations. (We can, for
  example, handle spilling a little more efficiently by skipping if we
  know a reloaded vreg hasn't changed with a new def. The
  redundant-move eliminator in regalloc2 kind of does this today, but
  not perfectly.)

## other backends

- This is still in progress. Aarch64 should be relatively easy: most
  of its instructions are already in three-operand form. It's a mostly
  mechanical transformation of the "instruction library". S390x is
  possibly a bit more involved, but now that all design decisions are
  settled, I expect these both together to take a few days at most.
