*******************************
Register Allocation in Cretonne
*******************************

.. default-domain:: cton
.. highlight:: rust

Cretonne uses a *decoupled, SSA-based* register allocator. Decoupled means that
register allocation is split into two primary phases: *spilling* and
*coloring*. SSA-based means that the code stays in SSA form throughout the
register allocator, and in fact is still in SSA form after register allocation.

Before the register allocator is run, all instructions in the function must be
*legalized*, which means that every instruction has an entry in the
``encodings`` table. The encoding entries also provide register class
constraints on the instruction's operands that the register allocator must
satisfy.

After the register allocator has run, the ``locations`` table provides a
register or stack slot location for all SSA values used by the function. The
register allocator may have inserted :inst:`spill`, :inst:`fill`, and
:inst:`copy` instructions to make that possible.

SSA-based register allocation
=============================

The phases of the SSA-based register allocator are:

Liveness analysis
    For each SSA value, determine exactly where it is live.

Spilling
    The process of deciding which SSA values go in a stack slot and which
    values go in a register. The spilling phase can also split live ranges by
    inserting :inst:`copy` instructions, or transform the code in other ways to
    reduce the number of values kept in registers.

    After spilling, the number of live register values never exceeds the number
    of available registers.

Coloring
    The process of assigning specific registers to the live values. It's a
    property of SSA form that this can be done in a linear scan of the
    dominator tree without causing any additional spills.

EBB argument fixup
    The coloring phase does not guarantee that EBB arguments are placed in the
    correct registers and/or stack slots before jumping to the EBB. It will
    try its best, but not making this guarantee is essential to the speed of
    the coloring phase. (EBB arguments correspond to PHI nodes in traditional
    SSA form).

    The argument fixup phase inserts 'shuffle code' before jumps and branches
    to place the argument values in their expected locations.

The contract between the spilling and coloring phases is that the number of
values in registers never exceeds the number of available registers. This
sounds simple enough in theory, but in practice there are some complications.

Real-world complications to SSA coloring
----------------------------------------

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
    Intel ISAs:

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
    return values passed to the :inst:`return` instruction.

Aliasing registers
    Different registers sometimes share the same bits in the register bank.
    This can make it difficult to measure register pressure. For example, the
    Intel registers RAX, EAX, AX, AL, and AH overlap.

    If only one of the aliasing registers can be used at a time, the aliasing
    doesn't cause problems since the registers can simply be counted as one
    unit.

Early clobbers
    Sometimes an instruction requires that the register used for an output
    operand does not alias any of the input operands. This happens for inline
    assembly and in some other special cases.


Liveness Analysis
=================

Both spilling and coloring need to know exactly where SSA values are live. The
liveness analysis computes this information.

The data structure representing the live range of a value uses the linear
layout of the function. All instructions and EBB headers are assigned a
*program position*. A starting point for a live range can be one of the
following:

- The instruction where the value is defined.
- The EBB header where the value is an EBB argument.
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

This representation is very similar to LLVM's ``LiveInterval`` data structure
with a few important differences:

- The Cretonne ``LiveRange`` only covers a single SSA value, while LLVM's
  ``LiveInterval`` represents the union of multiple related SSA values in a
  virtual register. This makes Cretonne's representation smaller because
  individual segments don't have to annotated with a value number.
- Cretonne stores the def-interval separately from a list of coalesced live-in
  intervals, while LLVM stores an array of segments. The two representations
  are equivalent, but Cretonne optimizes for the common case of a value that is
  only used locally.
- It is simpler to check if two live ranges are overlapping. The dominance
  properties of SSA form means that it is only necessary to check the
  def-interval of each live range against the intervals of the other range. It
  is not necessary to check for overlap between the two sets of live-in
  intervals. This makes the overlap check logarithmic in the number of live-in
  intervals instead of linear.
- LLVM represents a program point as ``SlotIndex`` which holds a pointer to a
  32-byte ``IndexListEntry`` struct. The entries are organized in a double
  linked list that mirrors the ordering of instructions in a basic block. This
  allows 'tombstone' program points corresponding to instructions that have
  been deleted.

  Cretonne uses a 32-bit program point representation that encodes an
  instruction or EBB number directly. There are no 'tombstones' for deleted
  instructions, and no mirrored linked list of instructions. Live ranges must
  be updated when instructions are deleted.

A consequence of Cretonne's more compact representation is that two program
points can't be compared without the context of a function layout.


Spilling algorithm
==================

There is no one way of implementing spilling, and different tradeoffs between
compilation time and code quality are possible. Any spilling algorithm will
need a way of tracking the register pressure so the colorability condition can
be satisfied.

Coloring algorithm
==================

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
- The set of arguments to the EBB. These values should all be live-in, although
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
