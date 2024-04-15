# Cranelift IR Reference

## Forward

This document is likely to be outdated and missing some important
information. It is recommended to look at the list of instructions as
documented in [the `InstBuilder` documentation].

[the `InstBuilder` documentation]: https://docs.rs/cranelift-codegen/latest/cranelift_codegen/ir/trait.InstBuilder.html

## Intro

The Cranelift intermediate representation ([IR]) has two primary forms:
an *in-memory data structure* that the code generator library is using, and a
*text format* which is used for test cases and debug output.
Files containing Cranelift textual IR have the `.clif` filename extension.

This reference uses the text format to describe IR semantics but glosses over
the finer details of the lexical and syntactic structure of the format.

## Overall structure

Cranelift compiles functions independently. A `.clif` IR file may contain
multiple functions, and the programmatic API can create multiple function
handles at the same time, but the functions don't share any data or reference
each other directly.

This is a simple C function that computes the average of an array of floats:

```c
float average(const float *array, size_t count)
{
    double sum = 0;
    for (size_t i = 0; i < count; i++)
        sum += array[i];
    return sum / count;
}
```

Here is the same function compiled into Cranelift IR:

```
test verifier

function %average(i32, i32) -> f32 system_v {
    ss0 = explicit_slot 8         ; Stack slot for `sum`.

block1(v0: i32, v1: i32):
    v2 = f64const 0x0.0
    stack_store v2, ss0
    brif v1, block2, block5                  ; Handle count == 0.

block2:
    v3 = iconst.i32 0
    jump block3(v3)

block3(v4: i32):
    v5 = imul_imm v4, 4
    v6 = iadd v0, v5
    v7 = load.f32 v6              ; array[i]
    v8 = fpromote.f64 v7
    v9 = stack_load.f64 ss0
    v10 = fadd v8, v9
    stack_store v10, ss0
    v11 = iadd_imm v4, 1
    v12 = icmp ult v11, v1
    brif v12, block3(v11), block4 ; Loop backedge.

block4:
    v13 = stack_load.f64 ss0
    v14 = fcvt_from_uint.f64 v1
    v15 = fdiv v13, v14
    v16 = fdemote.f32 v15
    return v16

block5:
    v100 = f32const +NaN
    return v100
}
```

The first line of a function definition provides the function *name* and
the [function signature] which declares the parameter and return types.
Then follows the [function preamble] which declares a number of entities
that can be referenced inside the function. In the example above, the preamble
declares a single explicit stack slot, `ss0`.

After the preamble follows the [function body] which consists of [basic block]s
(BBs), the first of which is the [entry block]. Every BB ends with a
[terminator instruction], so execution can never fall through to the next BB
without an explicit branch.

A `.clif` file consists of a sequence of independent function definitions:

```
function_list : { function }
function      : "function" function_name signature "{" preamble function_body "}"
preamble      : { preamble_decl }
function_body : { basic_block }
```

### Static single assignment form

The instructions in the function body use and produce *values* in SSA form. This
means that every value is defined exactly once, and every use of a value must be
dominated by the definition.

Cranelift does not have phi instructions but uses [BB parameter]s
instead. A BB can be defined with a list of typed parameters. Whenever control
is transferred to the BB, argument values for the parameters must be provided.
When entering a function, the incoming function parameters are passed as
arguments to the entry BB's parameters.

Instructions define zero, one, or more result values. All SSA values are either
BB parameters or instruction results.

In the example above, the loop induction variable `i` is represented
as three SSA values: In `block2`, `v3` is the initial value. In the
loop block `block3`, the BB parameter `v4` represents the value of the
induction variable during each iteration. Finally, `v11` is computed
as the induction variable value for the next iteration.

The `cranelift_frontend` crate contains utilities for translating from programs
containing multiple assignments to the same variables into SSA form for
Cranelift [IR].

Such variables can also be presented to Cranelift as [stack slot]s.
Stack slots are accessed with the `stack_store` and `stack_load` instructions,
and can have their address taken with `stack_addr`, which supports C-like
programming languages where local variables can have their address taken.

## Value types

All SSA values have a type which determines the size and shape (for SIMD
vectors) of the value. Many instructions are polymorphic -- they can operate on
different types.

### Integer types

Integer values have a fixed size and can be interpreted as either signed or
unsigned. Some instructions will interpret an operand as a signed or unsigned
number, others don't care.

- i8
- i16
- i32
- i64
- i128

Of these types, i32 and i64 are the most heavily-tested because of their use by 
Wasmtime. There are no known bugs in i8, i16, and i128, but their use may not 
be supported by all instructions in all backends (that is, they may cause 
the compiler to crash during code generation with an error that an instruction
is unsupported). 

The function `valid_for_target` within the [fuzzgen function generator][fungen] 
contains information about which instructions support which types. 

[fungen]: https://github.com/bytecodealliance/wasmtime/blob/main/cranelift/fuzzgen/src/function_generator.rs

### Floating point types

The floating point types have the IEEE 754 semantics that are supported by most
hardware, except that non-default rounding modes, unmasked exceptions, and
exception flags are not currently supported.

There is currently no support for higher-precision types like quad-precision,
double-double, or extended-precision, nor for narrower-precision types like
half-precision.

NaNs are encoded following the IEEE 754-2008 recommendation, with quiet NaN
being encoded with the MSB of the trailing significand set to 1, and signaling
NaNs being indicated by the MSB of the trailing significand set to 0.

Except for bitwise and memory instructions, NaNs returned from arithmetic
instructions are encoded as follows:

- If all NaN inputs to an instruction are quiet NaNs with all bits of the
  trailing significand other than the MSB set to 0, the result is a quiet
  NaN with a nondeterministic sign bit and all bits of the trailing
  significand other than the MSB set to 0.
- Otherwise the result is a quiet NaN with a nondeterministic sign bit
  and all bits of the trailing significand other than the MSB set to
  nondeterministic values.

- f32
- f64

### SIMD vector types

A SIMD vector type represents a vector of values from one of the scalar types
(integer, and floating point). Each scalar value in a SIMD type is called a
*lane*. The number of lanes must be a power of two in the range 2-256.

i%Bx%N
    A SIMD vector of integers. The lane type `iB` is one of the integer
    types `i8` ... `i64`.

    Some concrete integer vector types are `i32x4`, `i64x8`, and
    `i16x4`.

    The size of a SIMD integer vector in memory is :math:`N B\over 8` bytes.

f32x%N
    A SIMD vector of single precision floating point numbers.

    Some concrete `f32` vector types are: `f32x2`, `f32x4`,
    and `f32x8`.

    The size of a `f32` vector in memory is :math:`4N` bytes.

f64x%N
    A SIMD vector of double precision floating point numbers.

    Some concrete `f64` vector types are: `f64x2`, `f64x4`,
    and `f64x8`.

    The size of a `f64` vector in memory is :math:`8N` bytes.

### Pseudo-types and type classes

These are not concrete types, but convenient names used to refer to real types
in this reference.

iAddr
    A Pointer-sized integer representing an address.

    This is either `i32`, or `i64`, depending on whether the target
    platform has 32-bit or 64-bit pointers.

iB
    Any of the scalar integer types `i8` -- `i64`.

Int
    Any scalar *or vector* integer type: `iB` or `iBxN`.

fB
    Either of the floating point scalar types: `f32` or `f64`.

Float
    Any scalar *or vector* floating point type: `fB` or `fBxN`.

%Tx%N
    Any SIMD vector type.

Mem
    Any type that can be stored in memory: `Int` or `Float`.

Testable
    `iN`

### Immediate operand types

These types are not part of the normal SSA type system. They are used to
indicate the different kinds of immediate operands on an instruction.

imm64
    A 64-bit immediate integer. The value of this operand is interpreted as a
    signed two's complement integer. Instruction encodings may limit the valid
    range.

    In the textual format, `imm64` immediates appear as decimal or
    hexadecimal literals using the same syntax as C.

offset32
    A signed 32-bit immediate address offset.

    In the textual format, `offset32` immediates always have an explicit
    sign, and a 0 offset may be omitted.

ieee32
    A 32-bit immediate floating point number in the IEEE 754-2008 binary32
    interchange format. All bit patterns are allowed.

ieee64
    A 64-bit immediate floating point number in the IEEE 754-2008 binary64
    interchange format. All bit patterns are allowed.

intcc
    An integer condition code. See the `icmp` instruction for details.

floatcc
    A floating point condition code. See the `fcmp` instruction for details.

The two IEEE floating point immediate types `ieee32` and `ieee64`
are displayed as hexadecimal floating point literals in the textual [IR]
format. Decimal floating point literals are not allowed because some computer
systems can round differently when converting to binary. The hexadecimal
floating point format is mostly the same as the one used by C99, but extended
to represent all NaN bit patterns:

Normal numbers
    Compatible with C99: `-0x1.Tpe` where `T` are the trailing
    significand bits encoded as hexadecimal, and `e` is the unbiased exponent
    as a decimal number. `ieee32` has 23 trailing significand bits. They
    are padded with an extra LSB to produce 6 hexadecimal digits. This is not
    necessary for `ieee64` which has 52 trailing significand bits
    forming 13 hexadecimal digits with no padding.

Zeros
    Positive and negative zero are displayed as `0.0` and `-0.0` respectively.

Subnormal numbers
    Compatible with C99: `-0x0.Tpemin` where `T` are the trailing
    significand bits encoded as hexadecimal, and `emin` is the minimum exponent
    as a decimal number.

Infinities
    Either `-Inf` or `Inf`.

Quiet NaNs
    Quiet NaNs have the MSB of the trailing significand set. If the remaining
    bits of the trailing significand are all zero, the value is displayed as
    `-NaN` or `NaN`. Otherwise, `-NaN:0xT` where `T` are the trailing
    significand bits encoded as hexadecimal.

Signaling NaNs
    Displayed as `-sNaN:0xT`.


## Control flow

Branches transfer control to a new BB and provide values for the target BB's
arguments, if it has any. Conditional branches terminate a BB, and transfer to
the first BB if the condition is satisfied, and the second otherwise.

The `br_table v, BB(args), [BB1(args)...BBn(args)]` looks up the index `v` in
the inline jump table given as the third argument, and jumps to that BB. If `v`
is out of bounds for the jump table, the default BB (second argument) is used
instead.

Traps stop the program because something went wrong. The exact behavior depends
on the target instruction set architecture and operating system. There are
explicit trap instructions defined below, but some instructions may also cause
traps for certain input value. For example, `udiv` traps when the divisor
is zero.


## Function calls

A function call needs a target function and a [function signature]. The
target function may be determined dynamically at runtime, but the signature must
be known when the function call is compiled. The function signature describes
how to call the function, including parameters, return values, and the calling
convention:

```
signature    : "(" [paramlist] ")" ["->" retlist] [call_conv]
paramlist    : param { "," param }
retlist      : paramlist
param        : type [paramext] [paramspecial]
paramext     : "uext" | "sext"
paramspecial : "sarg" ( num ) | "sret" | "vmctx" | "stack_limit"
callconv     : "fast" | "cold" | "system_v" | "windows_fastcall"
             | "wasmtime_system_v" | "wasmtime_fastcall"
             | "apple_aarch64" | "wasmtime_apple_aarch64"
             | "probestack"
```

A function's calling convention determines exactly how arguments and return
values are passed, and how stack frames are managed. Since all of these details
depend on both the instruction set /// architecture and possibly the operating
system, a function's calling convention is only fully determined by a
`(TargetIsa, CallConv)` tuple.

| Name      | Description |
| ----------| ----------  |
| sarg      | pointer to a struct argument of the given size |
| sret      | pointer to a return value in memory |
| vmctx     | VM context pointer, which may contain pointers to heaps etc. |
| stack_limit | limit value for the size of the stack |

| Name      | Description |
| --------- | ----------- |
| fast      |  not-ABI-stable convention for best performance |
| cold      |  not-ABI-stable convention for infrequently executed code |
| system_v  |  System V-style convention used on many platforms |
| fastcall  |  Windows "fastcall" convention, also used for x64 and ARM |

The "not-ABI-stable" conventions do not follow an external specification and
may change between versions of Cranelift.

The "fastcall" convention is not yet implemented.

Parameters and return values have flags whose meaning is mostly target
dependent. These flags support interfacing with code produced by other
compilers.

Functions that are called directly must be declared in the [function preamble]:

FN = [colocated] NAME signature
    Declare a function so it can be called directly.

    If the colocated keyword is present, the symbol's definition will be
    defined along with the current function, such that it can use more
    efficient addressing.

    :arg NAME: Name of the function, passed to the linker for resolution.
    :arg signature: Function signature. See below.
    :result FN: A function identifier that can be used with `call`.

This simple example illustrates direct function calls and signatures:

```
test verifier

function %gcd(i32 uext, i32 uext) -> i32 uext system_v {
    fn0 = %divmod(i32 uext, i32 uext) -> i32 uext, i32 uext

block1(v0: i32, v1: i32):
    brif v1, block2, block3

block2:
    v2, v3 = call fn0(v0, v1)
    return v2

block3:
    return v0
}
```

Indirect function calls use a signature declared in the preamble.

## Memory

Cranelift provides fully general `load` and `store` instructions for accessing
memory, as well as [extending loads and truncating stores](#extending-loads-and-truncating-stores).

If the memory at the given address is not [addressable], the behavior of
these instructions is undefined. If it is addressable but not
[accessible], they [trap].

There are also more restricted operations for accessing specific types of memory
objects.

Additionally, instructions are provided for handling multi-register addressing.

### Memory operation flags

Loads and stores can have flags that loosen their semantics in order to enable
optimizations.

| Flag     | Description |
| -------- | ----------- |
| notrap   | Memory is assumed to be [accessible]. |
| aligned  | Trapping allowed for misaligned accesses. |
| readonly | The data at the specified address will not modified between when this function is called and exited. |

When the `accessible` flag is set, the behavior is undefined if the memory
is not [accessible].

Loads and stores are *misaligned* if the resultant address is not a multiple of
the expected alignment. By default, misaligned loads and stores are allowed,
but when the `aligned` flag is set, a misaligned memory access is allowed to
[trap].

### Explicit Stack Slots

One set of restricted memory operations access the current function's stack
frame. The stack frame is divided into fixed-size stack slots that are
allocated in the [function preamble]. Stack slots are not typed, they
simply represent a contiguous sequence of [accessible] bytes in the stack
frame.

SS = explicit_slot Bytes, Flags...
    Allocate a stack slot in the preamble.

    If no alignment is specified, Cranelift will pick an appropriate alignment
    for the stack slot based on its size and access patterns.

    :arg Bytes: Stack slot size on bytes.
    :flag align(N): Request at least N bytes alignment.
    :result SS: Stack slot index.

The dedicated stack access instructions are easy for the compiler to reason
about because stack slots and offsets are fixed at compile time. For example,
the alignment of these stack memory accesses can be inferred from the offsets
and stack slot alignments.

It's also possible to obtain the address of a stack slot, which can be used
in [unrestricted loads and stores](#memory).

The `stack_addr` instruction can be used to macro-expand the stack access
instructions before instruction selection:

    v0 = stack_load.f64 ss3, 16
    ; Expands to:
    v1 = stack_addr ss3, 16
    v0 = load.f64 v1

When Cranelift code is running in a sandbox, it can also be necessary to include
stack overflow checks in the prologue.

### Global values

A *global value* is an object whose value is not known at compile time. The
value is computed at runtime by `global_value`, possibly using
information provided by the linker via relocations. There are multiple
kinds of global values using different methods for determining their value.
Cranelift does not track the type of a global value, for they are just
values stored in non-stack memory.

When Cranelift is generating code for a virtual machine environment, globals can
be used to access data structures in the VM's runtime. This requires functions
to have access to a *VM context pointer* which is used as the base address.
Typically, the VM context pointer is passed as a hidden function argument to
Cranelift functions.

Chains of global value expressions are possible, but cycles are not allowed.
They will be caught by the IR verifier.

GV = vmctx
    Declare a global value of the address of the VM context struct.

    This declares a global value which is the VM context pointer which may
    be passed as a hidden argument to functions JIT-compiled for a VM.

    Typically, the VM context is a `#[repr(C, packed)]` struct.

    :result GV: Global value.

A global value can also be derived by treating another global variable as a
struct pointer and loading from one of its fields. This makes it possible to
chase pointers into VM runtime data structures.

GV = load.Type BaseGV [Offset]
    Declare a global value pointed to by BaseGV plus Offset, with type Type.

    It is assumed the BaseGV plus Offset resides in accessible memory with the
    appropriate alignment for storing a value with type Type.

    :arg BaseGV: Global value providing the base pointer.
    :arg Offset: Offset added to the base before loading.
    :result GV: Global value.

GV = iadd_imm BaseGV, Offset
    Declare a global value which has the value of BaseGV offset by Offset.

    :arg BaseGV: Global value providing the base value.
    :arg Offset: Offset added to the base value.

GV = [colocated] symbol Name
    Declare a symbolic address global value.

    The value of GV is symbolic and will be assigned a relocation, so that
    it can be resolved by a later linking phase.

    If the colocated keyword is present, the symbol's definition will be
    defined along with the current function, such that it can use more
    efficient addressing.

    :arg Name: External name.
    :result GV: Global value.

### Constant materialization

A few instructions have variants that take immediate operands, but in general
an instruction is required to load a constant into an SSA value: `iconst`,
`f32const`, `f64const` and `bconst` serve this purpose.

### Bitwise operations

The bitwise operations and operate on any value type: Integers, and floating
point numbers. When operating on integer or floating point types, the bitwise
operations are working on the binary representation of the values.

The shift and rotate operations only work on integer types (scalar and vector).
The shift amount does not have to be the same type as the value being shifted.
Only the low `B` bits of the shift amount is significant.

When operating on an integer vector type, the shift amount is still a scalar
type, and all the lanes are shifted the same amount. The shift amount is masked
to the number of bits in a *lane*, not the full size of the vector type.

The bit-counting instructions are scalar only.

### Floating point operations

These operations generally follow IEEE 754-2008 semantics.

#### Sign bit manipulations

The sign manipulating instructions work as bitwise operations, so they don't
have special behavior for signaling NaN operands. The exponent and trailing
significand bits are always preserved.

#### Minimum and maximum

These instructions return the larger or smaller of their operands. Note that
unlike the IEEE 754-2008 `minNum` and `maxNum` operations, these instructions
return NaN when either input is NaN.

When comparing zeroes, these instructions behave as if :math:`-0.0 < 0.0`.

#### Rounding

These instructions round their argument to a nearby integral value, still
represented as a floating point number.

### Extending loads and truncating stores

Most ISAs provide instructions that load an integer value smaller than a register
and extends it to the width of the register. Similarly, store instructions that
only write the low bits of an integer register are common.

In addition to the normal `load` and `store` instructions, Cranelift
provides extending loads and truncation stores for 8, 16, and 32-bit memory
accesses.

These instructions succeed, trap, or have undefined behavior, under the same
conditions as [normal loads and stores](#memory).

## ISA-specific instructions

Target ISAs can define supplemental instructions that do not make sense to
support generally.

### x86

Instructions that can only be used by the x86 target ISA.

## Codegen implementation instructions

Frontends don't need to emit the instructions in this section themselves;
Cranelift will generate them automatically as needed.

### Legalization operations

These instructions are used as helpers when legalizing types and operations for
the target ISA.

### Special register operations

The prologue and epilogue of a function needs to manipulate special registers like the stack
pointer and the frame pointer. These instructions should not be used in regular code.

### CPU flag operations

These operations are for working with the "flags" registers of some CPU
architectures.

### Live range splitting

Cranelift's register allocator assigns each SSA value to a register or a spill
slot on the stack for its entire live range. Since the live range of an SSA
value can be quite large, it is sometimes beneficial to split the live range
into smaller parts.

A live range is split by creating new SSA values that are copies or the
original value or each other. The copies are created by inserting `copy`,
`spill`, or `fill` instructions, depending on whether the values
are assigned to registers or stack slots.

This approach permits SSA form to be preserved throughout the register
allocation pass and beyond.

## Instruction groups

All of the shared instructions are part of the `base` instruction
group.

Target ISAs may define further instructions in their own instruction groups.

## Implementation limits

Cranelift's intermediate representation imposes some limits on the size of
functions and the number of entities allowed. If these limits are exceeded, the
implementation will panic.

Number of instructions in a function
    At most :math:`2^{31} - 1`.

Number of BBs in a function
    At most :math:`2^{31} - 1`.

    Every BB needs at least a terminator instruction anyway.

Number of secondary values in a function
    At most :math:`2^{31} - 1`.

    Secondary values are any SSA values that are not the first result of an
    instruction.

Other entities declared in the preamble
    At most :math:`2^{32} - 1`.

    This covers things like stack slots, jump tables, external functions, and
    function signatures, etc.

Number of arguments to a BB
    At most :math:`2^{16}`.

Number of arguments to a function
    At most :math:`2^{16}`.

    This follows from the limit on arguments to the entry BB. Note that
    Cranelift may add a handful of ABI register arguments as function signatures
    are lowered. This is for representing things like the link register, the
    incoming frame pointer, and callee-saved registers that are saved in the
    prologue.

Size of function call arguments on the stack
    At most :math:`2^{32} - 1` bytes.

    This is probably not possible to achieve given the limit on the number of
    arguments, except by requiring extremely large offsets for stack arguments.

## Glossary

    addressable
        Memory in which loads and stores have defined behavior. They either
        succeed or [trap], depending on whether the memory is
        [accessible].

    accessible
        [Addressable] memory in which loads and stores always succeed
        without [trapping], except where specified otherwise (eg. with the
        `aligned` flag). Heaps, globals, tables, and the stack may contain
        accessible, merely addressable, and outright unaddressable regions.
        There may also be additional regions of addressable and/or accessible
        memory not explicitly declared.

    basic block
        A maximal sequence of instructions that can only be entered from the
        top, and that contains no branch or terminator instructions except for
        the last instruction.

    entry block
        The [BB] that is executed first in a function. Currently, a
        Cranelift function must have exactly one entry block which must be the
        first block in the function. The types of the entry block arguments must
        match the types of arguments in the function signature.

    BB parameter
        A formal parameter for a BB is an SSA value that dominates everything
        in the BB. For each parameter declared by a BB, a corresponding
        argument value must be passed when branching to the BB. The function's
        entry BB has parameters that correspond to the function's parameters.

    BB argument
        Similar to function arguments, BB arguments must be provided when
        branching to a BB that declares formal parameters. When execution
        begins at the top of a BB, the formal parameters have the values of
        the arguments passed in the branch.

    function signature
        A function signature describes how to call a function. It consists of:

        - The calling convention.
        - The number of arguments and return values. (Functions can return
          multiple values.)
        - Type and flags of each argument.
        - Type and flags of each return value.

        Not all function attributes are part of the signature. For example, a
        function that never returns could be marked as `noreturn`, but that
        is not necessary to know when calling it, so it is just an attribute,
        and not part of the signature.

    function preamble
        A list of declarations of entities that are used by the function body.
        Some of the entities that can be declared in the preamble are:

        - Stack slots.
        - Functions that are called directly.
        - Function signatures for indirect function calls.
        - Function flags and attributes that are not part of the signature.

    function body
        The basic blocks which contain all the executable code in a function.
        The function body follows the function preamble.

    intermediate representation
    IR
        The language used to describe functions to Cranelift. This reference
        describes the syntax and semantics of Cranelift IR. The IR has two
        forms: Textual, and an in-memory data structure.

    stack slot
        A fixed size memory allocation in the current function's activation
        frame. These include [explicit stack slot]s and
        [spill stack slot]s.

    explicit stack slot
        A fixed size memory allocation in the current function's activation
        frame. These differ from [spill stack slot]s in that they can
        be created by frontends and they may have their addresses taken.

    spill stack slot
        A fixed size memory allocation in the current function's activation
        frame. These differ from [explicit stack slot]s in that they are
        only created during register allocation, and they may not have their
        address taken.

    terminator instruction
        A control flow instruction that unconditionally directs the flow of
        execution somewhere else. Execution never continues at the instruction
        following a terminator instruction.

        The basic terminator instructions are `br`, `return`, and
        `trap`. Conditional branches and instructions that trap
        conditionally are not terminator instructions.

    trap
    traps
    trapping
        Terminates execution of the current thread. The specific behavior after
        a trap depends on the underlying OS. For example, a common behavior is
        delivery of a signal, with the specific signal depending on the event
        that triggered it.
