# Register Allocation in Cranelift

Cranelift uses a *decoupled, SSA-based* register allocator. Decoupled means that
register allocation is split into two primary phases: *spilling* and
*coloring*. SSA-based means that the code stays in SSA form throughout the
register allocator, and in fact is still in SSA form after register allocation.

Before the register allocator is run, all instructions in the function must be
*legalized*, which means that every instruction has an entry in the
`encodings` table. The encoding entries also provide register class
constraints on the instruction's operands that the register allocator must
satisfy.

After the register allocator has run, the `locations` table provides a
register or stack slot location for all SSA values used by the function. The
register allocator may have inserted `spill`, `fill`, and
`copy` instructions to make that possible.

## SSA-based register allocation

The phases of the SSA-based register allocator are:

Liveness analysis
    For each SSA value, determine exactly where it is live.

Coalescing
    Form *virtual registers* which are sets of SSA values that should be
    assigned to the same location. Split live ranges such that values that
    belong to the same virtual register don't have interfering live ranges.

Spilling
    The process of deciding which SSA values go in a stack slot and which
    values go in a register. The spilling phase can also split live ranges by
    inserting `copy` instructions, or transform the code in other ways to
    reduce the number of values kept in registers.

    After spilling, the number of live register values never exceeds the number
    of available registers.

Reload
    Insert `spill` and `fill` instructions as necessary such that
    instructions that expect their operands in registers won't see values that
    live on the stack and vice versa.

    Reuse registers containing values loaded from the stack as much as possible
    without exceeding the maximum allowed register pressure.

Coloring
    The process of assigning specific registers to the live values. It's a
    property of SSA form that this can be done in a linear scan of the
    dominator tree without causing any additional spills.

    Make sure that specific register operand constraints are satisfied.

The contract between the spilling and coloring phases is that the number of
values in registers never exceeds the number of available registers. This
sounds simple enough in theory, but in practice there are some complications.

### Real-world complications to SSA coloring

In practice, instruction set architectures don't have "K interchangeable
registers", and register pressure can't be measured with a single number. There
are complications:

Different register banks
    Most ISAs separate integer registers from floating point registers, and
    instructions require their operands to come from a specific bank. This is a
    fairly simple problem to deal with since the register banks are completely
    disjoint. We simply count the number of integer and floating-point values
    that are live independently, and make sure that each number does not exceed
    the size of their respective register banks.

Instructions with fixed operands
    Some instructions use a fixed register for an operand. This happens on the
    x86 ISAs:

    - Dynamic shift and rotate instructions take the shift amount in CL.
    - Division instructions use RAX and RDX for both input and output operands.
    - Wide multiply instructions use fixed RAX and RDX registers for input and
      output operands.
    - A few SSE variable blend instructions use a hardwired XMM0 input operand.

Operands constrained to register subclasses
    Some instructions can only use a subset of the registers for some operands.
    For example, the ARM NEON vmla (scalar) instruction requires the scalar
    operand to be located in D0-15 or even D0-7, depending on the data type.
    The other operands can be from the full D0-31 register set.

ABI boundaries
    Before making a function call, arguments must be placed in specific
    registers and stack locations determined by the ABI, and return values
    appear in fixed registers.

    Some registers can be clobbered by the call and some are saved by the
    callee. In some cases, only the low bits of a register are saved by the
    callee. For example, ARM64 callees save only the low 64 bits of v8-15, and
    Win64 callees only save the low 128 bits of AVX registers.

    ABI boundaries also affect the location of arguments to the entry block and
    return values passed to the `return` instruction.

Aliasing registers
    Different registers sometimes share the same bits in the register bank.
    This can make it difficult to measure register pressure. For example, the
    x86 registers RAX, EAX, AX, AL, and AH overlap.

    If only one of the aliasing registers can be used at a time, the aliasing
    doesn't cause problems since the registers can simply be counted as one
    unit.

Early clobbers
    Sometimes an instruction requires that the register used for an output
    operand does not alias any of the input operands. This happens for inline
    assembly and in some other special cases.


## Liveness Analysis

All the register allocator passes need to know exactly where SSA values are
live. The liveness analysis computes this information.

The data structure representing the live range of a value uses the linear
layout of the function. All instructions and EBB headers are assigned a
*program position*. A starting point for a live range can be one of the
following:

- The instruction where the value is defined.
- The EBB header where the value is an EBB parameter.
- An EBB header where the value is live-in because it was defined in a
  dominating block.

The ending point of a live range can be:

- The last instruction to use the value.
- A branch or jump to an EBB where the value is live-in.

When all the EBBs in a function are laid out linearly, the live range of a
value doesn't have to be a contiguous interval, although it will be in a
majority of cases. There can be holes in the linear live range.

The part of a value's live range that falls inside a single EBB will always be
an interval without any holes. This follows from the dominance requirements of
SSA. A live range is represented as:

- The interval inside the EBB where the value is defined.
- A set of intervals for EBBs where the value is live-in.

Any value that is only used inside a single EBB will have an empty set of
live-in intervals. Some values are live across large parts of the function, and
this can often be represented with coalesced live-in intervals covering many
EBBs. It is important that the live range data structure doesn't have to grow
linearly with the number of EBBs covered by a live range.

This representation is very similar to LLVM's `LiveInterval` data structure
with a few important differences:

- The Cranelift `LiveRange` only covers a single SSA value, while LLVM's
  `LiveInterval` represents the union of multiple related SSA values in a
  virtual register. This makes Cranelift's representation smaller because
  individual segments don't have to annotated with a value number.
- Cranelift stores the def-interval separately from a list of coalesced live-in
  intervals, while LLVM stores an array of segments. The two representations
  are equivalent, but Cranelift optimizes for the common case of a value that is
  only used locally.
- It is simpler to check if two live ranges are overlapping. The dominance
  properties of SSA form means that it is only necessary to check the
  def-interval of each live range against the intervals of the other range. It
  is not necessary to check for overlap between the two sets of live-in
  intervals. This makes the overlap check logarithmic in the number of live-in
  intervals instead of linear.
- LLVM represents a program point as `SlotIndex` which holds a pointer to a
  32-byte `IndexListEntry` struct. The entries are organized in a double
  linked list that mirrors the ordering of instructions in a basic block. This
  allows 'tombstone' program points corresponding to instructions that have
  been deleted.

  Cranelift uses a 32-bit program point representation that encodes an
  instruction or EBB number directly. There are no 'tombstones' for deleted
  instructions, and no mirrored linked list of instructions. Live ranges must
  be updated when instructions are deleted.

A consequence of Cranelift's more compact representation is that two program
points can't be compared without the context of a function layout.

## Coalescing algorithm

Unconstrained SSA form is not well suited to register allocation because of the problems
that can arise around EBB parameters and arguments. Consider this simple example:

```
    function %interference(i32, i32) -> i32 {
    ebb0(v0: i32, v1: i32):
        brz v0, ebb1(v1)
        jump ebb1(v0)

    ebb1(v2: i32):
        v3 = iadd v1, v2
        return v3
    }
```

Here, the value `v1` is both passed as an argument to `ebb1` *and* it is
live in to the EBB because it is used by the  `iadd` instruction. Since
EBB arguments on the `brz` instruction need to be in the same register as
the corresponding EBB parameter `v2`, there is going to be interference
between `v1` and `v2` in the `ebb1` block.

The interference can be resolved by isolating the SSA values passed as EBB arguments:

```
    function %coalesced(i32, i32) -> i32 {
    ebb0(v0: i32, v1: i32):
        v5 = copy v1
        brz v0, ebb1(v5)
        v6 = copy v0
        jump ebb1(v6)

    ebb1(v2: i32):
        v3 = iadd.i32 v1, v2
        return v3
    }
```

Now the EBB argument is `v5` which is *not* itself live into `ebb1`,
resolving the interference.

The coalescing pass groups the SSA values into sets called *virtual registers*
and inserts copies such that:

1. Whenever a value is passed as an EBB argument, the corresponding EBB
   parameter value belongs to the same virtual register as the passed argument
   value.
2. The live ranges of values belonging to the same virtual register do not
   interfere, i.e. they don't overlap anywhere.

Most virtual registers contains only a single isolated SSA value because most
SSA values are never passed as EBB arguments. The `VirtRegs` data structure
doesn't store any information about these singleton virtual registers, it only
tracks larger virtual registers and assumes that any value it doesn't know about
is its own singleton virtual register

Once the values have been partitioned into interference-free virtual registers,
the code is said to be in `conventional SSA form (CSSA)
<http://citeseerx.ist.psu.edu/viewdoc/summary?doi=10.1.1.107.7249>`_. A program
in CSSA form can be register allocated correctly by assigning all the values in
a virtual register to the same stack or register location.

Conventional SSA form and the virtual registers are maintained through all the
register allocator passes.


## Spilling algorithm

The spilling pass is responsible for lowering the register pressure enough that
the coloring pass is guaranteed to be able to find a coloring solution. It does
this by assigning whole virtual registers to stack slots.

Besides just counting registers, the spiller also has to look at the
instruction's operand constraints because sometimes the constraints can require
extra registers to solve, raising the register pressure:

- If a single value is used more than once by an instruction, and the operands
  have conflicting constraints, two registers must be used. The most common case is
  when a single value is passed as two separate arguments to a function call.
- If an instruction has a *tied operand constraint* where one of the input operands
  must use the same register as the output operand, the spiller makes sure that
  the tied input value doesn't interfere with the output value by inserting a copy
  if needed.

The spilling heuristic used by Cranelift is very simple. Whenever the spiller
determines that the register pressure is too high at some instruction, it picks
the live SSA value whose definition is farthest away as the spill candidate.
Then it spills all values in the corresponding virtual register to the same
spill slot. It is important that all values in a virtual register get the same
spill slot, otherwise we could need memory-to-memory copies when passing spilled
arguments to a spilled EBB parameter.

This simple heuristic tends to spill values with long live ranges, and it
depends on the reload pass to do a good job of reusing registers reloaded from
spill slots if the spilled value gets used a lot. The idea is to minimize stack
*write* traffic with the spilling heuristic and to minimize stack *read* traffic
with the reload pass.

## Coloring algorithm

The SSA coloring algorithm is based on a single observation: If two SSA values
interfere, one of the values must be live where the other value is defined.

We visit the EBBs in a topological order such that all dominating EBBs are
visited before the current EBB. The instructions in an EBB are visited in a
top-down order, and each value define by the instruction is assigned an
available register. With this iteration order, every value that is live at an
instruction has already been assigned to a register.

This coloring algorithm works if the following condition holds:

    At every instruction, consider the values live through the instruction. No
    matter how the live values have been assigned to registers, there must be
    available registers of the right register classes available for the values
    defined by the instruction.

We'll need to modify this condition in order to deal with the real-world
complications.

The coloring algorithm needs to keep track of the set of live values at each
instruction. At the top of an EBB, this set can be computed as the union of:

- The set of live values before the immediately dominating branch or jump
  instruction. The topological iteration order guarantees that this set is
  available. Values whose live range indicate that they are not live-in to the
  current EBB should be filtered out.
- The set of parameters the EBB. These values should all be live-in, although
  it is possible that some are dead and never used anywhere.

For each live value, we also track its kill point in the current EBB. This is
the last instruction to use the value in the EBB. Values that are live-out
through the EBB terminator don't have a kill point. Note that the kill point
can be a branch to another EBB that uses the value, so the kill instruction
doesn't have to be a use of the value.

When advancing past an instruction, the live set is updated:

- Any values whose kill point is the current instruction are removed.
- Any values defined by the instruction are added, unless their kill point is
  the current instruction. This corresponds to a dead def which has no uses.
