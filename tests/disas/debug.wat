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
;;       movq    %rbx, 0x20(%rsp)
;;       movl    %edx, 8(%rsp)
;;       movl    %ecx, 0xc(%rsp)
;;       movq    8(%rdi), %rax
;;       movq    0x18(%rax), %rax
;;       movq    %rsp, %r8
;;       cmpq    %rax, %r8
;;       jb      0x62
;;   29: movq    %rdi, (%rsp)
;;       nopl    (%rax, %rax)
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 36, slot at FP-0x30, locals I32 @ slot+0x8, I32 @ slot+0xc, stack 
;;       ╰─╼ breakpoint patch: wasm PC 36, patch bytes [232, 190, 1, 0, 0]
;;       movl    %edx, 0x10(%rsp)
;;       nopl    (%rax, %rax)
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 38, slot at FP-0x30, locals I32 @ slot+0x8, I32 @ slot+0xc, stack I32 @ slot+0x10
;;       ╰─╼ breakpoint patch: wasm PC 38, patch bytes [232, 181, 1, 0, 0]
;;       movl    %ecx, 0x14(%rsp)
;;       nopl    (%rax, %rax)
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 40, slot at FP-0x30, locals I32 @ slot+0x8, I32 @ slot+0xc, stack I32 @ slot+0x10, I32 @ slot+0x14
;;       ╰─╼ breakpoint patch: wasm PC 40, patch bytes [232, 172, 1, 0, 0]
;;       leal    (%rdx, %rcx), %eax
;;       movl    %eax, 0x10(%rsp)
;;       nopl    (%rax, %rax)
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 41, slot at FP-0x30, locals I32 @ slot+0x8, I32 @ slot+0xc, stack I32 @ slot+0x10
;;       ╰─╼ breakpoint patch: wasm PC 41, patch bytes [232, 160, 1, 0, 0]
;;       movl    %eax, 0x10(%rsp)
;;       movq    0x20(%rsp), %rbx
;;       addq    $0x30, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   62: movq    %rdi, %rbx
;;   65: xorl    %esi, %esi
;;   67: callq   0x194
;;   6c: movq    %rbx, %rdi
;;   6f: callq   0x1c4
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
;;       movl    (%rdx), %r10d
;;       movl    0x10(%rdx), %ecx
;;       movq    %rdx, (%rsp)
;;       movq    8(%rdi), %r11
;;       movq    %rbp, %rax
;;       movq    %rax, 0x48(%r11)
;;       movq    %rsp, %rax
;;       movq    %rax, 0x40(%r11)
;;       leaq    0x39(%rip), %rax
;;       movq    %rax, 0x50(%r11)
;;       movq    %r10, %rdx
;;       callq   0
;;       ├─╼ exception frame offset: SP = FP - 0x40
;;       ╰─╼ exception handler: default handler, no dynamic context, handler=0xf3
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
;;   f3: xorl    %eax, %eax
;;   f5: movq    0x10(%rsp), %rbx
;;   fa: movq    0x18(%rsp), %r12
;;   ff: movq    0x20(%rsp), %r13
;;  104: movq    0x28(%rsp), %r14
;;  109: movq    0x30(%rsp), %r15
;;  10e: addq    $0x40, %rsp
;;  112: movq    %rbp, %rsp
;;  115: popq    %rbp
;;  116: retq
;;
;; signatures[0]::wasm_to_array_trampoline:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       subq    $0x30, %rsp
;;       movq    %r15, 0x20(%rsp)
;;       movq    %rdx, %rax
;;       movq    8(%rsi), %r10
;;       movq    %rsi, %r8
;;       movq    %rbp, %r11
;;       movq    %r11, 0x30(%r10)
;;       movq    %rbp, %r11
;;       movq    8(%r11), %rsi
;;       movq    %rsi, 0x38(%r10)
;;       leaq    (%rsp), %rdx
;;       movq    %rax, %rsi
;;       movl    %esi, (%rsp)
;;       movl    %ecx, 0x10(%rsp)
;;       movq    8(%rdi), %rax
;;       movl    $2, %ecx
;;       movq    %r8, %r15
;;       movq    %r15, %rsi
;;       callq   *%rax
;;       movq    8(%r15), %rcx
;;       addq    $1, 0x10(%rcx)
;;       testb   %al, %al
;;       je      0x181
;;  170: movl    (%rsp), %eax
;;       movq    0x20(%rsp), %r15
;;       addq    $0x30, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;  181: movq    0x10(%r15), %r8
;;  185: movq    0x198(%r8), %r8
;;  18c: movq    %r15, %rdi
;;  18f: callq   *%r8
;;  192: ud2
;;
;; wasmtime_builtin_trap:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %rax
;;       movq    %rbp, %rcx
;;       movq    %rcx, 0x30(%rax)
;;       movq    %rbp, %rcx
;;       movq    8(%rcx), %rcx
;;       movq    %rcx, 0x38(%rax)
;;       movq    0x10(%rdi), %rax
;;       movq    0x190(%rax), %rax
;;       movzbq  %sil, %rsi
;;       callq   *%rax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasmtime_builtin_raise:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r10
;;       movq    %rbp, %r11
;;       movq    %r11, 0x30(%r10)
;;       movq    %rbp, %r11
;;       movq    8(%r11), %rsi
;;       movq    %rsi, 0x38(%r10)
;;       movq    0x10(%rdi), %rsi
;;       movq    0x198(%rsi), %rsi
;;       callq   *%rsi
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
;;       movq    8(%rdi), %rax
;;       movq    %rbp, %rcx
;;       movq    %rcx, 0x30(%rax)
;;       movq    %rbp, %rcx
;;       movq    8(%rcx), %rcx
;;       movq    %rcx, 0x38(%rax)
;;       movq    0x10(%rdi), %rcx
;;       movq    0x1c8(%rcx), %rcx
;;       movq    %rdi, %rbx
;;       callq   *%rcx
;;       testb   %al, %al
;;       je      0x3b5
;;  2e9: movq    (%rsp), %rax
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
;;  3b5: movq    0x10(%rbx), %r8
;;  3b9: movq    0x198(%r8), %r8
;;  3c0: movq    %rbx, %rdi
;;  3c3: callq   *%r8
;;  3c6: ud2
