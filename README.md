# Lightbeam

Lightbeam is an optimising one-pass streaming compiler for WebAssembly, intended for use in [Wasmtime][wasmtime].

[wasmtime]: https://github.com/CraneStation/wasmtime

## Quality of output

Already - with a very small number of relatively simple optimisation rules - Lightbeam produces surprisingly high-quality output considering how restricted it is. It even produces better code than Cranelift, FireFox or both for some workloads. Here's a very simple example, this recursive fibonacci function in Rust:

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

FireFox's optimising compiler produces the following assembly (labels cleaned up somewhat):

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
.Lreturn
  mov    eax, dword ptr [rsp + 0x18]
  add    rsp, 0x20
  pop    rbp
  ret
```

Whereas Lightbeam produces code with far fewer memory accesses than both (and fewer blocks than FireFox's output):

```asm
fib:
  xor  eax, eax
  cmp  esi, 2
  setb al
  mov  ecx, 1
  test eax, eax
  jne  .Lreturn
  mov  eax, 1
.Lloop:
  mov  rcx, rsi
  add  ecx, 0xffffffff
  push rsi
  push rax
  mov  rsi, rcx
  call 0
  add  eax, dword ptr [rsp]
  mov  rcx, qword ptr [rsp + 8]
  add  ecx, 0xfffffffe
  xor  edx, edx
  cmp  ecx, 1
  seta dl
  mov  rsi, rcx
  add  rsp, 0x10
  test edx, edx
  jne  .Lloop
  mov  rcx, rax
.Lreturn:
  mov  rax, rcx
  ret
```

Now obviously I'm not advocating for replacing FireFox's optimising compiler with Lightbeam since the latter can only really produce better code when receiving optimised WebAssembly (and so debug-mode or hand-written WebAssembly may produce much worse output). However, this shows that even with the restrictions of a streaming compiler it's absolutely possible to produce high-quality assembly output. For the assembly above, the Lightbeam output runs within 15% of native speed. This is paramount for one of Lightbeam's intended usecases for real-time systems that want good runtime performance but cannot tolerate compiler bombs.

## Specification compliance

It's hard to judge, since each test in the spec testsuite covers a wide range of features (to check their interactions), but currently 65 out of 74 of the spec suite tests pass when run in Wasmtime with Lightbeam as a backend. Here's the full test output:

```
running 74 tests
test spec_testsuite::binary                 ... ok
test spec_testsuite::align                  ... ok
test spec_testsuite::block                  ... ok
test spec_testsuite::br                     ... ok
test spec_testsuite::break_drop             ... ok
test spec_testsuite::br_if                  ... ok
test spec_testsuite::address                ... ok
test spec_testsuite::comments               ... ok
test spec_testsuite::const_                 ... ok
test spec_testsuite::call                   ... ok
test spec_testsuite::custom                 ... ok
test spec_testsuite::custom_section         ... ok
test spec_testsuite::data                   ... ok
test spec_testsuite::elem                   ... ok
test spec_testsuite::br_table               ... FAILED
test spec_testsuite::conversions            ... ok
test spec_testsuite::call_indirect          ... ok
test spec_testsuite::exports                ... ok
test spec_testsuite::endianness             ... ok
test spec_testsuite::f32_bitwise            ... ok
test spec_testsuite::f64_bitwise            ... ok
test spec_testsuite::f32                    ... ok
test spec_testsuite::f32_cmp                ... ok
test spec_testsuite::fac                    ... ok
test spec_testsuite::f64                    ... ok
test spec_testsuite::f64_cmp                ... ok
test spec_testsuite::float_memory           ... ok
test spec_testsuite::forward                ... ok
test spec_testsuite::float_literals         ... ok
test spec_testsuite::float_misc             ... ok
test spec_testsuite::func_ptrs              ... ok
test spec_testsuite::get_local              ... ok
test spec_testsuite::func                   ... ok
test spec_testsuite::globals                ... ok
test spec_testsuite::i32                    ... ok
test spec_testsuite::i64                    ... ok
test spec_testsuite::inline_module          ... ok
test spec_testsuite::imports                ... ok
test spec_testsuite::if_                    ... ok
test spec_testsuite::int_literals           ... ok
test spec_testsuite::labels                 ... ok
test spec_testsuite::linking                ... ok
test spec_testsuite::int_exprs              ... ok
test spec_testsuite::loop_                  ... ok
test spec_testsuite::left_to_right          ... ok
test spec_testsuite::memory_redundancy      ... ok
test spec_testsuite::memory                 ... ok
test spec_testsuite::memory_grow            ... ok
test spec_testsuite::memory_trap            ... ok
test spec_testsuite::resizing               ... ok
test spec_testsuite::float_exprs            ... ok
test spec_testsuite::return_minimal         ... ok
test spec_testsuite::return_                ... ok
test spec_testsuite::select                 ... ok
test spec_testsuite::set_local              ... ok
test spec_testsuite::nop                    ... ok
test spec_testsuite::skip_stack_guard_page  ... FAILED
test spec_testsuite::store_retval           ... ok
test spec_testsuite::stack                  ... ok
test spec_testsuite::start                  ... ok
test spec_testsuite::token                  ... ok
test spec_testsuite::switch                 ... ok
test spec_testsuite::type_                  ... ok
test spec_testsuite::typecheck              ... ok
test spec_testsuite::traps                  ... ok
test spec_testsuite::unreached_invalid      ... ok
test spec_testsuite::unwind                 ... FAILED
test spec_testsuite::utf8_custom_section_id ... ok
test spec_testsuite::utf8_import_field      ... ok
test spec_testsuite::utf8_import_module     ... ok
test spec_testsuite::tee_local              ... ok
test spec_testsuite::utf8_invalid_encoding  ... ok
test spec_testsuite::unreachable            ... ok
test spec_testsuite::names                  ... ok

test result: FAILED. 71 passed; 3 failed; 0 ignored; 0 measured; 3 filtered out
```

## Getting involved

Our [issue tracker][issue tracker] is pretty barren right now since this is currently more-or-less a one-person project, but if you want to get involved jump into the [CraneStation Gitter room][cranestation-gitter] and someone can direct you to the right place. I wish I could say "the most useful thing you can do is play with it and open issues where you find problems" but until it passes the spec suite that won't be very helpful.

[cranestation-gitter]: https://gitter.im/CraneStation/Lobby
[issue tracker]: https://github.com/CraneStation/lightbeam/issues
