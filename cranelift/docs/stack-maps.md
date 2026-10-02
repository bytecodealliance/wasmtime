# Stack maps

While Cranelift is primarily meant to compile WebAssembly, many aspects of the
implementation process can overlap with many other use-cases, such as compiling
custom programming languages via JIT or AOT. And in the same vein, many programming
language compilers have an interest in implementing some garbage collection algorithm,
simplifying the process of managing memory considerably. While Cranelift can't
provide any pre-built solution, it does include the facilities to implement a
competent tracing garbage collector using *stack maps*.

This document assumes you already know a little bit about garbage collection and
safepoints. If not, you can read [New Stack Maps for Wasmtime and Cranelift].

[New Stack Maps for Wasmtime and Cranelift]: https://bytecodealliance.org/articles/new-stack-maps-for-wasmtime#background-garbage-collection-safepoints-and-stack-maps

There is an example project which can be used as a reference when creating
a similar implementation, which can be found in the [`collector` example].

[`collector` example]: https://github.com/bytecodealliance/wasmtime/tree/main/cranelift/jit/examples/collector

## Declaring objects

Before stack maps can be used, they need to be populated. Oftentimes, stack maps
are populated by declaring when a value or register should be inserted into the
stack map. The user must manually declare all the references which need to be
declared in the stack map, otherwise the stack map will be empty.

Declaring a value and register can be done with the [`declare_value_needs_stack_map`]
and [`declare_var_needs_stack_map`] methods, respectively.

[`declare_value_needs_stack_map`]: https://docs.rs/cranelift-frontend/latest/cranelift_frontend/struct.FunctionBuilder.html#method.declare_value_needs_stack_map
[`declare_var_needs_stack_map`]: https://docs.rs/cranelift-frontend/latest/cranelift_frontend/struct.FunctionBuilder.html#method.declare_var_needs_stack_map

This is a simple, although redundant, function to create a square with some given dimensions:

```rs
fn create_square(width: u32, height: u32) -> Square {
    let square = Square { width, height };

    square.ensure_valid();

    square
}
```

Here is the same function compiled into Cranelift IR (with 64-bit pointers):
```
function %create_square(i32, i32) -> i64 {
block1(v0: i32, v1: i32):
    v2 = iconst i64 8
    v3 = call allocate(v2)    ; v2 = 8

    store v0, v3+0
    store v1, v3+4

    call Square::ensure_valid(v3)

    return v3
```

In this example, we'd want to keep `square` alive, at least until the call to `Square::ensure_valid`
returns. What you'd likely do is declare the value `square` as needing a stack map right
after it is allocated, so that is included in the stack map on the call to `Square::ensure_valid`.
This will cause `square` to be moved from volatile registers into well-known locations
in the stack map.

After declaring `square` as needing a stack map entry, the same CLIR will now look like this:
```
function %create_square(i32, i32) -> i64 {
    ss0 = explicit_slot 8, align = 8

block1(v0: i32, v1: i32):
    v2 = iconst i64 8
    v3 = call allocate(v2)    ; v2 = 8

    store v0, v3+0
    store v1, v3+4

    ;; Store the allocated object to the stack right
    ;; before the call to `ensure_valid`.
    v5 = stack_addr.i64 ss0
    store notrap v3, v5

    ;; Annotate the call with the created stack map
    call Square::ensure_valid(v3), stack_map=[i64 @ ss0+0]

    ;; Load the value from the stack again
    v6 = load.i64 notrap v5

    ;; Return the loaded stack value instead of
    ;; the reference in `v3`.
    return v6
```

## Using stack maps

Stack maps can be inspected and used right after a function has been compiled
and finalized by Cranelift. This is backend-agnostic, so there is no difference in
whether the function is compiled via JIT, object emission or any other backend.

To get the stack maps which were generated for a given function, you can introspect
the compiled code from the `Context`, in which the function is compiled:

```rs
module.define_function(func_id, &mut ctx)?;

let compiled_code = ctx.compiled_code().expect("expected context to be compiled");

for (offset, length, map) in compiled_code.buffer.user_stack_maps() {
    let items = map.entries().map(|(_, offset)| offset as usize).collect::<Vec<_>>();

    println!("Stack map:");
    println!("  Offset:   {}", *offset);
    println!("  Size:     {}", *length);
    println!("  Entries:  {items:?}");
}
```

From the example above, you can expect something similar to the following as output:
```
Stack map:
  Offset:   96
  Size:     64
  Entries:  [0]
```

Stack maps are emitted once per safepoint, which happens on each `call` instruction. Since
the example function only performs a single call with a stack map annotation, there is only
a single safepoint in the function.

Each stack map will contain:
- **Offset**: the offset of the program counter from which the stack map is applicable. The
  offset is relative to the address of the first instruction in the owning function.
- **Size**: the length of the interval in which the stack map is applicable, in bytes.
- **Entries**: will be covered a little later.

As you may notice, the `offset` and `size` fields can be used to create an address interval
in which the stack map is valid. Say a function is compiled and exists at address `0xBFC00000`
and have 2 stack maps:
- **Stack map 1**:
  - Offset: 24
  - Size: 64
- **Stack map 2**:
  - Offset: 96
  - Size: 32

Stack map 1 will be valid in the interval of `0xBFC00018-0xBFC00058` and stack map 2 will be
valid in the interval of `0xBFC00060-0xBFC00080`. Whenever a call is made inside a safepoint,
the return address will exist within one of these intervals, which then indicates which objects
are alive at that given point.

### Entries in the stack map

Remember earlier when the generated CLIR would store valid objects in the current stack frame?
The entries within the stack map are address offsets, which point to these live objects. The
offsets are relative to the stack pointer, at the time the objects were spilled to the stack.
When added together, you get an address to inside the stack frame which holds a pointer to the
object.

Because the address you get back is an address to the object *inside the stack frame*, this allows
for generational or compacting collectors, which can relocate the object entirely during collection.
You would only need to overwrite the pointer in the stack frame to the new object location, after
which it will be reloaded again after the call.

## Invoking a collection

While that's well and all, how would we actually trigger an allocation in the garbage collector,
let alone get the appropriate program counter and stack pointer?

Well, since safepoints are emitted on every call instruction, we can place an implicit call to
trigger the collection just before other function calls, effectively "stealing" the stack map
at that particular point. In practice, the CLIR might look similar to this:
```
function %create_square(i32, i32) -> i64 {
    ss0 = explicit_slot 8, align = 8

block1(v0: i32, v1: i32):
    v2 = iconst i64 8
    v3 = call allocate(v2)    ; v2 = 8

    store v0, v3+0
    store v1, v3+4

    ;; Store the allocated object to the stack right
    ;; before the next call instruction.
    v5 = stack_addr.i64 ss0
    store notrap v3, v5

    ;; Trigger the collector inside the safepoint, so all
    ;; objects exist on the stack.
    call GC::trigger(), stack_map=[i64 @ ss0+0]

    ;; Load the value from the stack again, since the object
    ;; may have been relocated.
    v6 = load.i64 notrap v5

    ;; Pass the loaded stack value instead of the reference in `v3`.
    ;; Notice how this call now no longer has any stack map annotation.
    call Square::ensure_valid(v6)

    return v6
```

Inside the `GC::trigger` function is where you'd handle the actual garbage collection
itself. This function can be an external symbol, Rust function, etc. and does not need
to be compiled using Cranelift.

Inside this function, you'd need to find the stack pointer and program counter from just
before the call, so you know which stack map to use. To do this, you might want to employ
something called **stack walking** or **frame walking**. While outside of the scope of this
article, you can see how Wasmtime implements it for different architectures in [the unwinder crate] or
see the [collector example project].

[the unwinder crate]: https://github.com/bytecodealliance/wasmtime/blob/main/crates/unwinder/src/stackwalk.rs
[collector example project]: https://github.com/bytecodealliance/wasmtime/tree/main/cranelift/jit/examples/collector

After finding all live objects for a given point, it's only a matter of filtering them out from
all allocations made, to find the set of all dead objects which can be deallocated.

## Which values should be added to the stack map?

Depending on the way objects and allocations are used in your implementation, there might
be some confusion as to *what* should be included in stack maps. Below are some general
guidelines which will work for most scenarios, but maybe not all.

In general, you should declare variables and/or values if they:
- are managed objects themselves or point to an object inside the managed heap,
- refer to some offset of a managed object (ie. object field references),
- or are somehow derived from a managed object (e.g. an element of an array)

On the other hand, you should not declare variables and/or values if they:
- represent an immediate value, such as integers, floats, booleans, etc.,
- have been allocated outside the scope of the garbage collector (e.g. static data),
- or points to an address which isn't a managed object

It should also be noted that whenever a new block parameter is created which accepts a
reference to a managed object, that parameter may also need to be declared as needing
a stack map. Following the example from earlier, an implementation of `ensure_valid`
would need to declare it's parameter as needing a stack map, since the passed `square`
value is a managed object.
