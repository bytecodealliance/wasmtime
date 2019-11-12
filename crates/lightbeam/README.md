# Lightbeam

Lightbeam is an optimising one-pass streaming compiler for WebAssembly, intended for use in [Wasmtime][wasmtime].

[wasmtime]: https://github.com/bytecodealliance/wasmtime

## Quality of output

Already - with a very small number of relatively simple optimisation rules - Lightbeam produces surprisingly high-quality output considering how restricted it is. It even produces better code than Cranelift, Firefox or both for some workloads. Here's a very simple example, this recursive fibonacci function in Rust:

```rust
fn fib(n: i32) -> i32 {
    if n == 0 || n == 1 {
        1
    } else {
        fib(n - 1) + fib(n - 2)
    }
}
```

When compiled with optimisations enabled, rustc will produce the following WebAssembly:

```rust
(module
  (func $fib (param $p0 i32) (result i32)
    (local $l1 i32)
    (set_local $l1
      (i32.const 1))
    (block $B0
      (br_if $B0
        (i32.lt_u
          (get_local $p0)
          (i32.const 2)))
      (set_local $l1
        (i32.const 1))
      (loop $L1
        (set_local $l1
          (i32.add
            (call $fib
              (i32.add
                (get_local $p0)
                (i32.const -1)))
            (get_local $l1)))
        (br_if $L1
          (i32.gt_u
            (tee_local $p0
              (i32.add
                (get_local $p0)
                (i32.const -2)))
            (i32.const 1)))))
    (get_local $l1)))
```

Firefox's optimising compiler produces the following assembly (labels cleaned up somewhat):

```asm
fib:
  sub rsp, 0x18
  cmp qword ptr [r14 + 0x28], rsp
  jae stack_overflow
  mov dword ptr [rsp + 0xc], edi
  cmp edi, 2
  jae .Lelse
  mov eax, 1
  mov dword ptr [rsp + 8], eax
  jmp .Lreturn
.Lelse:
  mov dword ptr [rsp + 0xc], edi
  mov eax, 1
  mov dword ptr [rsp + 8], eax
.Lloop:
  mov edi, dword ptr [rsp + 0xc]
  add edi, -1
  call 0
  mov ecx, dword ptr [rsp + 8]
  add ecx, eax
  mov dword ptr [rsp + 8], ecx
  mov ecx, dword ptr [rsp + 0xc]
  add ecx, -2
  mov dword ptr [rsp + 0xc], ecx
  cmp ecx, 1
  ja .Lloop
.Lreturn:
  mov eax, dword ptr [rsp + 8]
  nop
  add rsp, 0x18
  ret
```

Cranelift with optimisations enabled produces similar:

```asm
fib:
  push   rbp
  mov    rbp, rsp
  sub    rsp, 0x20
  mov    qword ptr [rsp + 0x10], rdi
  mov    dword ptr [rsp + 0x1c], esi
  mov    eax, 1
  mov    dword ptr [rsp + 0x18], eax
  mov    eax, dword ptr [rsp + 0x1c]
  cmp    eax, 2
  jb     .Lreturn
  movabs rax, 0
  mov    qword ptr [rsp + 8], rax
.Lloop:
  mov    eax, dword ptr [rsp + 0x1c]
  add    eax, -1
  mov    rcx, qword ptr [rsp + 8]
  mov    rdx, qword ptr [rsp + 0x10]
  mov    rdi, rdx
  mov    esi, eax
  call   rcx
  mov    ecx, dword ptr [rsp + 0x18]
  add    eax, ecx
  mov    dword ptr [rsp + 0x18], eax
  mov    eax, dword ptr [rsp + 0x1c]
  add    eax, -2
  mov    dword ptr [rsp + 0x1c], eax
  mov    eax, dword ptr [rsp + 0x1c]
  cmp    eax, 1
  ja     .Lloop
.Lreturn:
  mov    eax, dword ptr [rsp + 0x18]
  add    rsp, 0x20
  pop    rbp
  ret
```

Whereas Lightbeam produces smaller code with far fewer memory accesses than both (and fewer blocks than Firefox's output):

```asm
fib:
  cmp  esi, 2
  mov  eax, 1
  jb   .Lreturn
  mov  eax, 1
.Lloop:
  mov  rcx, rsi
  add  ecx, 0xffffffff
  push rsi
  push rax
  push rax
  mov  rsi, rcx
  call fib
  add  eax, [rsp + 8]
  mov  rcx, [rsp + 0x10]
  add  ecx, 0xfffffffe
  cmp  ecx, 1
  mov  rsi, rcx
  lea  rsp, [rsp + 0x18]
  ja   .Lloop
.Lreturn:
  ret
```

Now obviously I'm not advocating for replacing Firefox's optimising compiler with Lightbeam since the latter can only really produce better code when receiving optimised WebAssembly (and so debug-mode or hand-written WebAssembly may produce much worse output). However, this shows that even with the restrictions of a streaming compiler it's absolutely possible to produce high-quality assembly output. For the assembly above, the Lightbeam output runs within 15% of native speed. This is paramount for one of Lightbeam's intended usecases for real-time systems that want good runtime performance but cannot tolerate compiler bombs.

## Specification compliance

Lightbeam passes 100% of the specification test suite, but that doesn't necessarily mean that it's 100% specification-compliant. Hopefully as we run a fuzzer against it we can find any issues and get Lightbeam to a state where it can be used in production.

## Getting involved

You can file issues in the [Wasmtime issue tracker][issue tracker]. If you want to get involved jump into the [CraneStation Gitter room][cranestation-gitter] and someone can direct you to the right place. I wish I could say "the most useful thing you can do is play with it and open issues where you find problems" but until it passes the spec suite that won't be very helpful.

[cranestation-gitter]: https://gitter.im/CraneStation/Lobby
[Wasmtime issue tracker]: https://github.com/bytecodealliance/wasmtime/issues
