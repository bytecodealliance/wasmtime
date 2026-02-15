;;! target = "x86_64"
;;! test = "compile"
;;! flags = ["-Dguest-debug=yes"]
;;! objdump = "--funcs all"

(module
  (func (export "main") (param i32 i32) (result i32)
    local.get 0
    local.get 1
    i32.add))
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       subq    $0x30, %rsp
;;       movq    %r12, 0x20(%rsp)
;;       movl    %edx, 8(%rsp)
;;       movl    %ecx, 0xc(%rsp)
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       movq    %rsp, %rax
;;       cmpq    %r11, %rax
;;       jb      0x62
;;   29: movq    %rdi, (%rsp)
;;       nopl    (%rax, %rax)
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 36, slot at FP-0x30, locals I32 @ slot+0x8, I32 @ slot+0xc, stack 
;;       ╰─╼ breakpoint patch: wasm PC 36, patch bytes [232, 184, 1, 0, 0]
;;       movl    %edx, 0x10(%rsp)
;;       nopl    (%rax, %rax)
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 38, slot at FP-0x30, locals I32 @ slot+0x8, I32 @ slot+0xc, stack I32 @ slot+0x10
;;       ╰─╼ breakpoint patch: wasm PC 38, patch bytes [232, 175, 1, 0, 0]
;;       movl    %ecx, 0x14(%rsp)
;;       nopl    (%rax, %rax)
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 40, slot at FP-0x30, locals I32 @ slot+0x8, I32 @ slot+0xc, stack I32 @ slot+0x10, I32 @ slot+0x14
;;       ╰─╼ breakpoint patch: wasm PC 40, patch bytes [232, 166, 1, 0, 0]
;;       leal    (%rdx, %rcx), %eax
;;       movl    %eax, 0x10(%rsp)
;;       nopl    (%rax, %rax)
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 41, slot at FP-0x30, locals I32 @ slot+0x8, I32 @ slot+0xc, stack I32 @ slot+0x10
;;       ╰─╼ breakpoint patch: wasm PC 41, patch bytes [232, 154, 1, 0, 0]
;;       movl    %eax, 0x10(%rsp)
;;       movq    0x20(%rsp), %r12
;;       addq    $0x30, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   62: movq    %rdi, %r12
;;   65: xorl    %esi, %esi
;;   67: callq   0x18c
;;   6c: movq    %r12, %rdi
;;   6f: callq   0x1bd
;;   74: ud2
;;
;; wasm[0]::array_to_wasm_trampoline[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       subq    $0x40, %rsp
;;       movq    %rbx, 0x10(%rsp)
;;       movq    %r12, 0x18(%rsp)
;;       movq    %r13, 0x20(%rsp)
;;       movq    %r14, 0x28(%rsp)
;;       movq    %r15, 0x30(%rsp)
;;       movl    (%rdx), %eax
;;       movl    0x10(%rdx), %ecx
;;       movq    %rdx, (%rsp)
;;       movq    8(%rdi), %r8
;;       movq    %rbp, %r9
;;       movq    %r9, 0x48(%r8)
;;       movq    %rsp, %r9
;;       movq    %r9, 0x40(%r8)
;;       leaq    0x39(%rip), %r9
;;       movq    %r9, 0x50(%r8)
;;       movq    %rax, %rdx
;;       callq   0
;;       ├─╼ exception frame offset: SP = FP - 0x40
;;       ╰─╼ exception handler: default handler, no dynamic context, handler=0xf2
;;       movq    (%rsp), %rdx
;;       movl    %eax, (%rdx)
;;       movl    $1, %eax
;;       movq    0x10(%rsp), %rbx
;;       movq    0x18(%rsp), %r12
;;       movq    0x20(%rsp), %r13
;;       movq    0x28(%rsp), %r14
;;       movq    0x30(%rsp), %r15
;;       addq    $0x40, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   f2: xorl    %eax, %eax
;;   f4: movq    0x10(%rsp), %rbx
;;   f9: movq    0x18(%rsp), %r12
;;   fe: movq    0x20(%rsp), %r13
;;  103: movq    0x28(%rsp), %r14
;;  108: movq    0x30(%rsp), %r15
;;  10d: addq    $0x40, %rsp
;;  111: movq    %rbp, %rsp
;;  114: popq    %rbp
;;  115: retq
;;
;; signatures[0]::wasm_to_array_trampoline:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       subq    $0x30, %rsp
;;       movq    %rbx, 0x20(%rsp)
;;       movq    %rdx, %r8
;;       movq    8(%rsi), %rax
;;       movq    %rbp, %rdx
;;       movq    %rdx, 0x30(%rax)
;;       movq    %rbp, %rdx
;;       movq    8(%rdx), %rdx
;;       movq    %rdx, 0x38(%rax)
;;       leaq    (%rsp), %rdx
;;       movq    %r8, %rax
;;       movl    %eax, (%rsp)
;;       movl    %ecx, 0x10(%rsp)
;;       movq    8(%rdi), %rax
;;       movl    $2, %ecx
;;       movq    %rsi, %rbx
;;       callq   *%rax
;;       movq    8(%rbx), %rcx
;;       addq    $1, 0x10(%rcx)
;;       testb   %al, %al
;;       je      0x17a
;;  169: movl    (%rsp), %eax
;;       movq    0x20(%rsp), %rbx
;;       addq    $0x30, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;  17a: movq    0x10(%rbx), %rax
;;  17e: movq    0x198(%rax), %rax
;;  185: movq    %rbx, %rdi
;;  188: callq   *%rax
;;  18a: ud2
;;
;; wasmtime_builtin_trap:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r9
;;       movq    %rbp, %r10
;;       movq    %r10, 0x30(%r9)
;;       movq    %rbp, %r10
;;       movq    8(%r10), %r11
;;       movq    %r11, 0x38(%r9)
;;       movq    0x10(%rdi), %r11
;;       movq    0x190(%r11), %r11
;;       movzbq  %sil, %rsi
;;       callq   *%r11
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasmtime_builtin_raise:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r8
;;       movq    %rbp, %r9
;;       movq    %r9, 0x30(%r8)
;;       movq    %rbp, %r9
;;       movq    8(%r9), %r9
;;       movq    %r9, 0x38(%r8)
;;       movq    0x10(%rdi), %r9
;;       movq    0x198(%r9), %r9
;;       callq   *%r9
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasmtime_patchable_builtin_breakpoint:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       subq    $0x150, %rsp
;;       movq    %rax, (%rsp)
;;       movq    %rcx, 8(%rsp)
;;       movq    %rdx, 0x10(%rsp)
;;       movq    %rbx, 0x18(%rsp)
;;       movq    %rsi, 0x20(%rsp)
;;       movq    %rdi, 0x28(%rsp)
;;       movq    %r8, 0x30(%rsp)
;;       movq    %r9, 0x38(%rsp)
;;       movq    %r10, 0x40(%rsp)
;;       movq    %r11, 0x48(%rsp)
;;       movdqu  %xmm0, 0x50(%rsp)
;;       movdqu  %xmm1, 0x60(%rsp)
;;       movdqu  %xmm2, 0x70(%rsp)
;;       movdqu  %xmm3, 0x80(%rsp)
;;       movdqu  %xmm4, 0x90(%rsp)
;;       movdqu  %xmm5, 0xa0(%rsp)
;;       movdqu  %xmm6, 0xb0(%rsp)
;;       movdqu  %xmm7, 0xc0(%rsp)
;;       movdqu  %xmm8, 0xd0(%rsp)
;;       movdqu  %xmm9, 0xe0(%rsp)
;;       movdqu  %xmm10, 0xf0(%rsp)
;;       movdqu  %xmm11, 0x100(%rsp)
;;       movdqu  %xmm12, 0x110(%rsp)
;;       movdqu  %xmm13, 0x120(%rsp)
;;       movdqu  %xmm14, 0x130(%rsp)
;;       movdqu  %xmm15, 0x140(%rsp)
;;       movq    8(%rdi), %r10
;;       movq    %rbp, %r11
;;       movq    %r11, 0x30(%r10)
;;       movq    %rbp, %r11
;;       movq    8(%r11), %rax
;;       movq    %rax, 0x38(%r10)
;;       movq    0x10(%rdi), %rax
;;       movq    0x1c8(%rax), %rcx
;;       movq    %rdi, %rbx
;;       callq   *%rcx
;;       testb   %al, %al
;;       je      0x3af
;;  2e3: movq    (%rsp), %rax
;;       movq    8(%rsp), %rcx
;;       movq    0x10(%rsp), %rdx
;;       movq    0x18(%rsp), %rbx
;;       movq    0x20(%rsp), %rsi
;;       movq    0x28(%rsp), %rdi
;;       movq    0x30(%rsp), %r8
;;       movq    0x38(%rsp), %r9
;;       movq    0x40(%rsp), %r10
;;       movq    0x48(%rsp), %r11
;;       movdqu  0x50(%rsp), %xmm0
;;       movdqu  0x60(%rsp), %xmm1
;;       movdqu  0x70(%rsp), %xmm2
;;       movdqu  0x80(%rsp), %xmm3
;;       movdqu  0x90(%rsp), %xmm4
;;       movdqu  0xa0(%rsp), %xmm5
;;       movdqu  0xb0(%rsp), %xmm6
;;       movdqu  0xc0(%rsp), %xmm7
;;       movdqu  0xd0(%rsp), %xmm8
;;       movdqu  0xe0(%rsp), %xmm9
;;       movdqu  0xf0(%rsp), %xmm10
;;       movdqu  0x100(%rsp), %xmm11
;;       movdqu  0x110(%rsp), %xmm12
;;       movdqu  0x120(%rsp), %xmm13
;;       movdqu  0x130(%rsp), %xmm14
;;       movdqu  0x140(%rsp), %xmm15
;;       addq    $0x150, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;  3af: movq    0x10(%rbx), %rax
;;  3b3: movq    0x198(%rax), %rax
;;  3ba: movq    %rbx, %rdi
;;  3bd: callq   *%rax
;;  3bf: ud2
