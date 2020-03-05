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
float
average(const float *array, size_t count)
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
    brz v1, block5                  ; Handle count == 0.
    jump block2

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
    brnz v12, block3(v11)           ; Loop backedge.
    jump block4

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

After the preamble follows the [function body] which consists of
[extended basic block]s (EBBs), the first of which is the
[entry block]. Every EBB ends with a [terminator instruction], so
execution can never fall through to the next EBB without an explicit branch.

A `.clif` file consists of a sequence of independent function definitions:

.. productionlist::
    function_list : { function }
    function      : "function" function_name signature "{" preamble function_body "}"
    preamble      : { preamble_decl }
    function_body : { extended_basic_block }

### Static single assignment form

The instructions in the function body use and produce *values* in SSA form. This
means that every value is defined exactly once, and every use of a value must be
dominated by the definition.

Cranelift does not have phi instructions but uses [EBB parameter]s
instead. An EBB can be defined with a list of typed parameters. Whenever control
is transferred to the EBB, argument values for the parameters must be provided.
When entering a function, the incoming function parameters are passed as
arguments to the entry EBB's parameters.

Instructions define zero, one, or more result values. All SSA values are either
EBB parameters or instruction results.

In the example above, the loop induction variable `i` is represented as three
SSA values: In the entry block, `v4` is the initial value. In the loop block
`ebb2`, the EBB parameter `v5` represents the value of the induction
variable during each iteration. Finally, `v12` is computed as the induction
variable value for the next iteration.

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

### Boolean types

Boolean values are either true or false.

The `b1` type represents an abstract boolean value. It can only exist as
an SSA value, and can't be directly stored in memory. It can, however, be
converted into an integer with value 0 or 1 by the `bint` instruction (and
converted back with `icmp_imm` with 0).

Several larger boolean types are also defined, primarily to be used as SIMD
element types. They can be stored in memory, and are represented as either all
zero bits or all one bits.

- b1
- b8
- b16
- b32
- b64

### Integer types

Integer values have a fixed size and can be interpreted as either signed or
unsigned. Some instructions will interpret an operand as a signed or unsigned
number, others don't care.

The support for i8 and i16 arithmetic is incomplete and use could lead to bugs.

- i8
- i16
- i32
- i64

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

### CPU flags types

Some target ISAs use CPU flags to represent the result of a comparison. These
CPU flags are represented as two value types depending on the type of values
compared.

Since some ISAs don't have CPU flags, these value types should not be used
until the legalization phase of compilation where the code is adapted to fit
the target ISA. Use instructions like `icmp` instead.

The CPU flags types are also restricted such that two flags values can not be
live at the same time. After legalization, some instruction encodings will
clobber the flags, and flags values are not allowed to be live across such
instructions either. The verifier enforces these rules.

- iflags
- fflags

### SIMD vector types

A SIMD vector type represents a vector of values from one of the scalar types
(boolean, integer, and floating point). Each scalar value in a SIMD type is
called a *lane*. The number of lanes must be a power of two in the range 2-256.

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

b1x%N
    A boolean SIMD vector.

    Boolean vectors are used when comparing SIMD vectors. For example,
    comparing two `i32x4` values would produce a `b1x4` result.

    Like the `b1` type, a boolean vector cannot be stored in memory.

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
    Either `b1` or `iN`.

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

bool
    A boolean immediate value, either false or true.

    In the textual format, `bool` immediates appear as 'false'
    and 'true'.

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

Branches transfer control to a new EBB and provide values for the target EBB's
arguments, if it has any. Conditional branches only take the branch if their
condition is satisfied, otherwise execution continues at the following
instruction in the EBB.

JT = jump_table [EBB0, EBB1, ..., EBBn]
    Declare a jump table in the [function preamble].

    This declares a jump table for use by the `br_table` indirect branch
    instruction. Entries in the table are EBB names.

    The EBBs listed must belong to the current function, and they can't have
    any arguments.

    :arg EBB0: Target EBB when `x = 0`.
    :arg EBB1: Target EBB when `x = 1`.
    :arg EBBn: Target EBB when `x = n`.
    :result: A jump table identifier. (Not an SSA value).

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

.. productionlist::
    signature    : "(" [paramlist] ")" ["->" retlist] [call_conv]
    paramlist    : param { "," param }
    retlist      : paramlist
    param        : type [paramext] [paramspecial]
    paramext     : "uext" | "sext"
    paramspecial : "sret" | "link" | "fp" | "csr" | "vmctx" | "sigid" | "stack_limit"
    callconv     : "fast" | "cold" | "system_v" | "fastcall" | "baldrdash_system_v" | "baldrdash_windows"

A function's calling convention determines exactly how arguments and return
values are passed, and how stack frames are managed. Since all of these details
depend on both the instruction set /// architecture and possibly the operating
system, a function's calling convention is only fully determined by a
`(TargetIsa, CallConv)` tuple.

| Name      | Description |
| ----------| ----------  |
| sret      | pointer to a return value in memory |
| link      | return address |
| fp        | the initial value of the frame pointer |
| csr       | callee-saved register |
| vmctx     | VM context pointer, which may contain pointers to heaps etc. |
| sigid     | signature id, for checking caller/callee signature compatibility |
| stack_limit | limit value for the size of the stack |

| Name      | Description |
| --------- | ----------- |
| fast      |  not-ABI-stable convention for best performance |
| cold      |  not-ABI-stable convention for infrequently executed code |
| system_v  |  System V-style convention used on many platforms |
| fastcall  |  Windows "fastcall" convention, also used for x64 and ARM |
| baldrdash_system_v |  SpiderMonkey WebAssembly convention on platforms natively using SystemV. |
| baldrdash_windows  | SpiderMonkey WebAssembly convention on platforms natively using Windows. |

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
    brz v1, block3
    jump block2

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

### Heaps

Code compiled from WebAssembly or asm.js runs in a sandbox where it can't access
all process memory. Instead, it is given a small set of memory areas to work
in, and all accesses are bounds checked. Cranelift models this through the
concept of *heaps*.

A heap is declared in the function preamble and can be accessed with the
`heap_addr` instruction that [traps] on out-of-bounds accesses or
returns a pointer that is guaranteed to trap. Heap addresses can be smaller than
the native pointer size, for example unsigned `i32` offsets on a 64-bit
architecture.

.. digraph:: static
    :align: center
    :caption: Heap address space layout

    node [
        shape=record,
        fontsize=10,
        fontname="Vera Sans, DejaVu Sans, Liberation Sans, Arial, Helvetica, sans"
    ]
    "static" [label="mapped\npages|unmapped\npages|offset_guard\npages"]

A heap appears as three consecutive ranges of address space:

1. The *mapped pages* are the [accessible] memory range in the heap. A
   heap may have a minimum guaranteed size which means that some mapped pages
   are always present.
2. The *unmapped pages* is a possibly empty range of address space that may be
   mapped in the future when the heap is grown. They are [addressable] but
   not [accessible].
3. The *offset-guard pages* is a range of address space that is guaranteed to
   always cause a trap when accessed. It is used to optimize bounds checking for
   heap accesses with a shared base pointer. They are [addressable] but
   not [accessible].

The *heap bound* is the total size of the mapped and unmapped pages. This is
the bound that `heap_addr` checks against. Memory accesses inside the
heap bounds can trap if they hit an unmapped page (which is not
[accessible]).

Two styles of heaps are supported, *static* and *dynamic*. They behave
differently when resized.

#### Static heaps

A *static heap* starts out with all the address space it will ever need, so it
never moves to a different address. At the base address is a number of mapped
pages corresponding to the heap's current size. Then follows a number of
unmapped pages where the heap can grow up to its maximum size. After the
unmapped pages follow the offset-guard pages which are also guaranteed to
generate a trap when accessed.

H = static Base, min MinBytes, bound BoundBytes, offset_guard OffsetGuardBytes
    Declare a static heap in the preamble.

    :arg Base: Global value holding the heap's base address.
    :arg MinBytes: Guaranteed minimum heap size in bytes. Accesses below this
            size will never trap.
    :arg BoundBytes: Fixed heap bound in bytes. This defines the amount of
            address space reserved for the heap, not including the offset-guard
            pages.
    :arg OffsetGuardBytes: Size of the offset-guard pages in bytes.

#### Dynamic heaps

A *dynamic heap* can be relocated to a different base address when it is
resized, and its bound can move dynamically. The offset-guard pages move when
the heap is resized. The bound of a dynamic heap is stored in a global value.

H = dynamic Base, min MinBytes, bound BoundGV, offset_guard OffsetGuardBytes
    Declare a dynamic heap in the preamble.

    :arg Base: Global value holding the heap's base address.
    :arg MinBytes: Guaranteed minimum heap size in bytes. Accesses below this
            size will never trap.
    :arg BoundGV: Global value containing the current heap bound in bytes.
    :arg OffsetGuardBytes: Size of the offset-guard pages in bytes.

#### Heap examples

The SpiderMonkey VM prefers to use fixed heaps with a 4 GB bound and 2 GB of
offset-guard pages when running WebAssembly code on 64-bit CPUs. The combination
of a 4 GB fixed bound and 1-byte bounds checks means that no code needs to be
generated for bounds checks at all:

```
test verifier

function %add_members(i32, i64 vmctx) -> f32 baldrdash_system_v {
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+64
    heap0 = static gv1, min 0x1000, bound 0x1_0000_0000, offset_guard 0x8000_0000

block0(v0: i32, v5: i64):
    v1 = heap_addr.i64 heap0, v0, 1
    v2 = load.f32 v1+16
    v3 = load.f32 v1+20
    v4 = fadd v2, v3
    return v4
}
```

A static heap can also be used for 32-bit code when the WebAssembly module
declares a small upper bound on its memory. A 1 MB static bound with a single 4
KB offset-guard page still has opportunities for sharing bounds checking code:

```
test verifier

function %add_members(i32, i32 vmctx) -> f32 baldrdash_system_v {
    gv0 = vmctx
    gv1 = load.i32 notrap aligned gv0+64
    heap0 = static gv1, min 0x1000, bound 0x10_0000, offset_guard 0x1000

block0(v0: i32, v5: i32):
    v1 = heap_addr.i32 heap0, v0, 1
    v2 = load.f32 v1+16
    v3 = load.f32 v1+20
    v4 = fadd v2, v3
    return v4
}
```

If the upper bound on the heap size is too large, a dynamic heap is required
instead.

Finally, a runtime environment that simply allocates a heap with
`malloc()` may not have any offset-guard pages at all. In that case,
full bounds checking is required for each access:

```
test verifier

function %add_members(i32, i64 vmctx) -> f32 baldrdash_system_v {
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+64
    gv2 = load.i32 notrap aligned gv0+72
    heap0 = dynamic gv1, min 0x1000, bound gv2, offset_guard 0

block0(v0: i32, v6: i64):
    v1 = heap_addr.i64 heap0, v0, 20
    v2 = load.f32 v1+16
    v3 = heap_addr.i64 heap0, v0, 24
    v4 = load.f32 v3+20
    v5 = fadd v2, v4
    return v5
}
```

### Tables

Code compiled from WebAssembly often needs access to objects outside of its
linear memory. WebAssembly uses *tables* to allow programs to refer to opaque
values through integer indices.

A table is declared in the function preamble and can be accessed with the
`table_addr` instruction that [traps] on out-of-bounds accesses.
Table addresses can be smaller than the native pointer size, for example
unsigned `i32` offsets on a 64-bit architecture.

A table appears as a consecutive range of address space, conceptually
divided into elements of fixed sizes, which are identified by their index.
The memory is [accessible].

The *table bound* is the number of elements currently in the table. This is
the bound that `table_addr` checks against.

A table can be relocated to a different base address when it is resized, and
its bound can move dynamically. The bound of a table is stored in a global
value.

T = dynamic Base, min MinElements, bound BoundGV, element_size ElementSize
    Declare a table in the preamble.

    :arg Base: Global value holding the table's base address.
    :arg MinElements: Guaranteed minimum table size in elements.
    :arg BoundGV: Global value containing the current heap bound in elements.
    :arg ElementSize: Size of each element.

### Constant materialization

A few instructions have variants that take immediate operands, but in general
an instruction is required to load a constant into an SSA value: `iconst`,
`f32const`, `f64const` and `bconst` serve this purpose.

### Bitwise operations

The bitwise operations and operate on any value type: Integers, floating point
numbers, and booleans. When operating on integer or floating point types, the
bitwise operations are working on the binary representation of the values. When
operating on boolean values, the bitwise operations work as logical operators.

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

Register values can be temporarily diverted to other registers by the
`regmove` instruction, and to and from stack slots by `regspill`
and `regfill`.

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

Number of EBBs in a function
    At most :math:`2^{31} - 1`.

    Every EBB needs at least a terminator instruction anyway.

Number of secondary values in a function
    At most :math:`2^{31} - 1`.

    Secondary values are any SSA values that are not the first result of an
    instruction.

Other entities declared in the preamble
    At most :math:`2^{32} - 1`.

    This covers things like stack slots, jump tables, external functions, and
    function signatures, etc.

Number of arguments to an EBB
    At most :math:`2^{16}`.

Number of arguments to a function
    At most :math:`2^{16}`.

    This follows from the limit on arguments to the entry EBB. Note that
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
        The [EBB] that is executed first in a function. Currently, a
        Cranelift function must have exactly one entry block which must be the
        first block in the function. The types of the entry block arguments must
        match the types of arguments in the function signature.

    extended basic block
    EBB
        A maximal sequence of instructions that can only be entered from the
        top, and that contains no [terminator instruction]s except for
        the last one. An EBB can contain conditional branches that can fall
        through to the following instructions in the block, but only the first
        instruction in the EBB can be a branch target.

        The last instruction in an EBB must be a [terminator instruction],
        so execution cannot flow through to the next EBB in the function. (But
        there may be a branch to the next EBB.)

        Note that some textbooks define an EBB as a maximal *subtree* in the
        control flow graph where only the root can be a join node. This
        definition is not equivalent to Cranelift EBBs.

    EBB parameter
        A formal parameter for an EBB is an SSA value that dominates everything
        in the EBB. For each parameter declared by an EBB, a corresponding
        argument value must be passed when branching to the EBB. The function's
        entry EBB has parameters that correspond to the function's parameters.

    EBB argument
        Similar to function arguments, EBB arguments must be provided when
        branching to an EBB that declares formal parameters. When execution
        begins at the top of an EBB, the formal parameters have the values of
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
        The extended basic blocks which contain all the executable code in a
        function. The function body follows the function preamble.

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
