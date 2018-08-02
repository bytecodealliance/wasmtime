**************************
Cranelift compared to LLVM
**************************

`LLVM <https://llvm.org>`_ is a collection of compiler components implemented as
a set of C++ libraries. It can be used to build both JIT compilers and static
compilers like `Clang <https://clang.llvm.org>`_, and it is deservedly very
popular. `Chris Lattner's chapter about LLVM
<https://www.aosabook.org/en/llvm.html>`_ in the `Architecture of Open Source
Applications <https://aosabook.org/en/index.html>`_ book gives an excellent
overview of the architecture and design of LLVM.

Cranelift and LLVM are superficially similar projects, so it is worth
highlighting some of the differences and similarities. Both projects:

- Use an ISA-agnostic input language in order to mostly abstract away the
  differences between target instruction set architectures.
- Depend extensively on SSA form.
- Have both textual and in-memory forms of their primary intermediate
  representation. (LLVM also has a binary bitcode format; Cranelift doesn't.)
- Can target multiple ISAs.
- Can cross-compile by default without rebuilding the code generator.

Cranelift's scope is much smaller than that of LLVM. The classical three main
parts of a compiler are:

1. The language-dependent front end parses and type-checks the input program.
2. Common optimizations that are independent of both the input language and the
   target ISA.
3. The code generator which depends strongly on the target ISA.

LLVM provides both common optimizations *and* a code generator. Cranelift only
provides the last part, the code generator. LLVM additionally provides
infrastructure for building assemblers and disassemblers. Cranelift does not
handle assembly at all---it only generates binary machine code.

Intermediate representations
============================

LLVM uses multiple intermediate representations as it translates a program to
binary machine code:

`LLVM IR <https://llvm.org/docs/LangRef.html>`_
    This is the primary intermediate representation which has textual, binary, and
    in-memory forms. It serves two main purposes:

    - An ISA-agnostic, stable(ish) input language that front ends can generate
      easily.
    - Intermediate representation for common mid-level optimizations. A large
      library of code analysis and transformation passes operate on LLVM IR.

`SelectionDAG <https://llvm.org/docs/CodeGenerator.html#instruction-selection-section>`_
    A graph-based representation of the code in a single basic block is used by
    the instruction selector. It has both ISA-agnostic and ISA-specific
    opcodes. These main passes are run on the SelectionDAG representation:

    - Type legalization eliminates all value types that don't have a
      representation in the target ISA registers.
    - Operation legalization eliminates all opcodes that can't be mapped to
      target ISA instructions.
    - DAG-combine cleans up redundant code after the legalization passes.
    - Instruction selection translates ISA-agnostic expressions to ISA-specific
      instructions.

    The SelectionDAG representation automatically eliminates common
    subexpressions and dead code.

`MachineInstr <https://llvm.org/docs/CodeGenerator.html#machine-code-representation>`_
    A linear representation of ISA-specific instructions that initially is in
    SSA form, but it can also represent non-SSA form during and after register
    allocation. Many low-level optimizations run on MI code. The most important
    passes are:

    - Scheduling.
    - Register allocation.

`MC <https://llvm.org/docs/CodeGenerator.html#the-mc-layer>`_
    MC serves as the output abstraction layer and is the basis for LLVM's
    integrated assembler. It is used for:

    - Branch relaxation.
    - Emitting assembly or binary object code.
    - Assemblers.
    - Disassemblers.

There is an ongoing "global instruction selection" project to replace the
SelectionDAG representation with ISA-agnostic opcodes on the MachineInstr
representation. Some target ISAs have a fast instruction selector that can
translate simple code directly to MachineInstrs, bypassing SelectionDAG when
possible.

:doc:`Cranelift <ir>` uses a single intermediate representation to cover
these levels of abstraction. This is possible in part because of Cranelift's
smaller scope.

- Cranelift does not provide assemblers and disassemblers, so it is not
  necessary to be able to represent every weird instruction in an ISA. Only
  those instructions that the code generator emits have a representation.
- Cranelift's opcodes are ISA-agnostic, but after legalization / instruction
  selection, each instruction is annotated with an ISA-specific encoding which
  represents a native instruction.
- SSA form is preserved throughout. After register allocation, each SSA value
  is annotated with an assigned ISA register or stack slot.

The Cranelift intermediate representation is similar to LLVM IR, but at a slightly
lower level of abstraction.

Program structure
-----------------

In LLVM IR, the largest representable unit is the *module* which corresponds
more or less to a C translation unit. It is a collection of functions and
global variables that may contain references to external symbols too.

In Cranelift IR, the largest representable unit is the *function*. This is so
that functions can easily be compiled in parallel without worrying about
references to shared data structures. Cranelift does not have any
inter-procedural optimizations like inlining.

An LLVM IR function is a graph of *basic blocks*. A Cranelift IR function is a
graph of *extended basic blocks* that may contain internal branch instructions.
The main difference is that an LLVM conditional branch instruction has two
target basic blocks---a true and a false edge. A Cranelift branch instruction
only has a single target and falls through to the next instruction when its
condition is false. The Cranelift representation is closer to how machine code
works; LLVM's representation is more abstract.

LLVM uses `phi instructions
<https://llvm.org/docs/LangRef.html#phi-instruction>`_ in its SSA
representation. Cranelift passes arguments to EBBs instead. The two
representations are equivalent, but the EBB arguments are better suited to
handle EBBs that may contain multiple branches to the same destination block
with different arguments. Passing arguments to an EBB looks a lot like passing
arguments to a function call, and the register allocator treats them very
similarly. Arguments are assigned to registers or stack locations.

Value types
-----------

:ref:`Cranelift's type system <value-types>` is mostly a subset of LLVM's type
system. It is less abstract and closer to the types that common ISA registers
can hold.

- Integer types are limited to powers of two from :clif:type:`i8` to
  :clif:type:`i64`. LLVM can represent integer types of arbitrary bit width.
- Floating point types are limited to :clif:type:`f32` and :clif:type:`f64`
  which is what WebAssembly provides. It is possible that 16-bit and 128-bit
  types will be added in the future.
- Addresses are represented as integers---There are no Cranelift pointer types.
  LLVM currently has rich pointer types that include the pointee type. It may
  move to a simpler 'address' type in the future. Cranelift may add a single
  address type too.
- SIMD vector types are limited to a power-of-two number of vector lanes up to
  256. LLVM allows an arbitrary number of SIMD lanes.
- Cranelift has no aggregate types. LLVM has named and anonymous struct types as
  well as array types.

Cranelift has multiple boolean types, whereas LLVM simply uses `i1`. The sized
Cranelift boolean types are used to represent SIMD vector masks like ``b32x4``
where each lane is either all 0 or all 1 bits.

Cranelift instructions and function calls can return multiple result values. LLVM
instead models this by returning a single value of an aggregate type.

Instruction set
---------------

LLVM has a small well-defined basic instruction set and a large number of
intrinsics, some of which are ISA-specific. Cranelift has a larger instruction
set and no intrinsics. Some Cranelift instructions are ISA-specific.

Since Cranelift instructions are used all the way until the binary machine code
is emitted, there are opcodes for every native instruction that can be
generated. There is a lot of overlap between different ISAs, so for example the
:clif:inst:`iadd_imm` instruction is used by every ISA that can add an
immediate integer to a register. A simple RISC ISA like RISC-V can be defined
with only shared instructions, while x86 needs a number of specific
instructions to model addressing modes.

Undefined behavior
==================

Cranelift does not generally exploit undefined behavior in its optimizations.
LLVM's mid-level optimizations do, but it should be noted that LLVM's low-level code
generator rarely needs to make use of undefined behavior either.

LLVM provides ``nsw`` and ``nuw`` flags for its arithmetic that invoke
undefined behavior on overflow. Cranelift does not provide this functionality.
Its arithmetic instructions either produce a value or a trap.

LLVM has an ``unreachable`` instruction which is used to indicate impossible
code paths. Cranelift only has an explicit :clif:inst:`trap` instruction.

Cranelift does make assumptions about aliasing. For example, it assumes that it
has full control of the stack objects in a function, and that they can only be
modified by function calls if their address have escaped. It is quite likely
that Cranelift will admit more detailed aliasing annotations on load/store
instructions in the future. When these annotations are incorrect, undefined
behavior ensues.
