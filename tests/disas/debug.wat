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
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x24, slot at FP-0x30, locals I32 @ slot+0x8, I32 @ slot+0xc, stack 
;;       ╰─╼ breakpoint patch: wasm PC 0x24, patch bytes [232, 212, 1, 0, 0]
;;       movl    %edx, 0x10(%rsp)
;;       nopl    (%rax, %rax)
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x26, slot at FP-0x30, locals I32 @ slot+0x8, I32 @ slot+0xc, stack I32 @ slot+0x10
;;       ╰─╼ breakpoint patch: wasm PC 0x26, patch bytes [232, 203, 1, 0, 0]
;;       movl    %ecx, 0x14(%rsp)
;;       nopl    (%rax, %rax)
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x28, slot at FP-0x30, locals I32 @ slot+0x8, I32 @ slot+0xc, stack I32 @ slot+0x10, I32 @ slot+0x14
;;       ╰─╼ breakpoint patch: wasm PC 0x28, patch bytes [232, 194, 1, 0, 0]
;;       leal    (%rdx, %rcx), %eax
;;       movl    %eax, 0x10(%rsp)
;;       nopl    (%rax, %rax)
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x29, slot at FP-0x30, locals I32 @ slot+0x8, I32 @ slot+0xc, stack I32 @ slot+0x10
;;       ╰─╼ breakpoint patch: wasm PC 0x29, patch bytes [232, 182, 1, 0, 0]
;;       movl    %eax, 0x10(%rsp)
;;       movq    0x20(%rsp), %r12
;;       addq    $0x30, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   62: movq    %rdi, %r12
;;   65: xorl    %esi, %esi
;;   67: callq   0x1a8
;;   6c: movq    %r12, %rdi
;;   6f: callq   0x1d9
;;       ╰─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x23, slot at FP-0x30, locals I32 @ slot+0x8, I32 @ slot+0xc, stack 
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
;;       movl    (%rdx), %r8d
;;       movl    0x10(%rdx), %ecx
;;       movq    %rdx, (%rsp)
;;       movq    %rbp, %r9
;;       movq    8(%rdi), %r10
;;       movq    %r9, 0x48(%r10)
;;       movq    %rsp, %r9
;;       movq    %r9, 0x40(%r10)
;;       leaq    0x3e(%rip), %r9
;;       movq    %r9, 0x50(%r10)
;;       movq    %r8, %rdx
;;       movq    %r10, 8(%rsp)
;;       callq   0
;;       ├─╼ exception frame offset: SP = FP - 0x40
;;       ╰─╼ exception handler: default handler, no dynamic context, handler=0xf8
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
;;   f8: movq    8(%rsp), %r10
;;   fd: movq    $1, 0x88(%r10)
;;  108: xorl    %eax, %eax
;;  10a: movq    0x10(%rsp), %rbx
;;  10f: movq    0x18(%rsp), %r12
;;  114: movq    0x20(%rsp), %r13
;;  119: movq    0x28(%rsp), %r14
;;  11e: movq    0x30(%rsp), %r15
;;  123: addq    $0x40, %rsp
;;  127: movq    %rbp, %rsp
;;  12a: popq    %rbp
;;  12b: retq
;;
;; signatures[0]::wasm_to_array_trampoline:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       subq    $0x30, %rsp
;;       movq    %rbx, 0x20(%rsp)
;;       movq    %r15, 0x28(%rsp)
;;       movq    %rdx, %r8
;;       movq    %rbp, %rdx
;;       movq    8(%rsi), %r15
;;       movq    %rdx, 0x30(%r15)
;;       movq    %rbp, %rax
;;       movq    8(%rax), %rax
;;       movq    %rax, 0x38(%r15)
;;       leaq    (%rsp), %rdx
;;       movq    %r8, %rax
;;       movl    %eax, (%rsp)
;;       movl    %ecx, 0x10(%rsp)
;;       movq    8(%rdi), %rax
;;       movl    $2, %ecx
;;       movq    %rsi, %rbx
;;       callq   *%rax
;;       addq    $1, 0x10(%r15)
;;       testb   %al, %al
;;       je      0x196
;;  180: movl    (%rsp), %eax
;;       movq    0x20(%rsp), %rbx
;;       movq    0x28(%rsp), %r15
;;       addq    $0x30, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;  196: movq    0x10(%rbx), %rax
;;  19a: movq    0x148(%rax), %rax
;;  1a1: movq    %rbx, %rdi
;;  1a4: callq   *%rax
;;  1a6: ud2
;;
;; wasmtime_builtin_trap:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    %rbp, %r10
;;       movq    8(%rdi), %r9
;;       movq    %r10, 0x30(%r9)
;;       movq    %rbp, %r10
;;       movq    8(%r10), %r11
;;       movq    %r11, 0x38(%r9)
;;       movq    0x10(%rdi), %r11
;;       movq    0x140(%r11), %r11
;;       movzbq  %sil, %rsi
;;       callq   *%r11
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasmtime_builtin_raise:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    %rbp, %r9
;;       movq    8(%rdi), %r8
;;       movq    %r9, 0x30(%r8)
;;       movq    %rbp, %r9
;;       movq    8(%r9), %r9
;;       movq    %r9, 0x38(%r8)
;;       movq    0x10(%rdi), %r9
;;       movq    0x148(%r9), %r9
;;       callq   *%r9
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasmtime_patchable_builtin_breakpoint:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       subq    $0x160, %rsp
;;       movq    %rax, (%rsp)
;;       movq    %rcx, 8(%rsp)
;;       movq    %rdx, 0x10(%rsp)
;;       movq    %rsi, 0x18(%rsp)
;;       movq    %rdi, 0x20(%rsp)
;;       movq    %r8, 0x28(%rsp)
;;       movq    %r9, 0x30(%rsp)
;;       movq    %r10, 0x38(%rsp)
;;       movq    %r11, 0x40(%rsp)
;;       movq    %r12, 0x48(%rsp)
;;       movq    %r13, 0x50(%rsp)
;;       movdqu  %xmm0, 0x60(%rsp)
;;       movdqu  %xmm1, 0x70(%rsp)
;;       movdqu  %xmm2, 0x80(%rsp)
;;       movdqu  %xmm3, 0x90(%rsp)
;;       movdqu  %xmm4, 0xa0(%rsp)
;;       movdqu  %xmm5, 0xb0(%rsp)
;;       movdqu  %xmm6, 0xc0(%rsp)
;;       movdqu  %xmm7, 0xd0(%rsp)
;;       movdqu  %xmm8, 0xe0(%rsp)
;;       movdqu  %xmm9, 0xf0(%rsp)
;;       movdqu  %xmm10, 0x100(%rsp)
;;       movdqu  %xmm11, 0x110(%rsp)
;;       movdqu  %xmm12, 0x120(%rsp)
;;       movdqu  %xmm13, 0x130(%rsp)
;;       movdqu  %xmm14, 0x140(%rsp)
;;       movdqu  %xmm15, 0x150(%rsp)
;;       movq    %rbp, %r10
;;       movq    8(%rdi), %r9
;;       movq    %r10, 0x30(%r9)
;;       movq    %rbp, %r10
;;       movq    8(%r10), %r11
;;       movq    %r11, 0x38(%r9)
;;       movq    0x10(%rdi), %r12
;;       movq    %rdi, %r13
;;       movq    0x170(%r12), %r11
;;       callq   *%r11
;;       testb   %al, %al
;;       je      0x3dd
;;  309: movq    (%rsp), %rax
;;       movq    8(%rsp), %rcx
;;       movq    0x10(%rsp), %rdx
;;       movq    0x18(%rsp), %rsi
;;       movq    0x20(%rsp), %rdi
;;       movq    0x28(%rsp), %r8
;;       movq    0x30(%rsp), %r9
;;       movq    0x38(%rsp), %r10
;;       movq    0x40(%rsp), %r11
;;       movq    0x48(%rsp), %r12
;;       movq    0x50(%rsp), %r13
;;       movdqu  0x60(%rsp), %xmm0
;;       movdqu  0x70(%rsp), %xmm1
;;       movdqu  0x80(%rsp), %xmm2
;;       movdqu  0x90(%rsp), %xmm3
;;       movdqu  0xa0(%rsp), %xmm4
;;       movdqu  0xb0(%rsp), %xmm5
;;       movdqu  0xc0(%rsp), %xmm6
;;       movdqu  0xd0(%rsp), %xmm7
;;       movdqu  0xe0(%rsp), %xmm8
;;       movdqu  0xf0(%rsp), %xmm9
;;       movdqu  0x100(%rsp), %xmm10
;;       movdqu  0x110(%rsp), %xmm11
;;       movdqu  0x120(%rsp), %xmm12
;;       movdqu  0x130(%rsp), %xmm13
;;       movdqu  0x140(%rsp), %xmm14
;;       movdqu  0x150(%rsp), %xmm15
;;       addq    $0x160, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;  3dd: movq    0x148(%r12), %rax
;;  3e5: movq    %r13, %rdi
;;  3e8: callq   *%rax
;;  3ea: ud2
