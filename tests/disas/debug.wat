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
;;       movq    8(%rdi), %r10
;;       movq    0x10(%r10), %r10
;;       addq    $0x40, %r10
;;       cmpq    %rsp, %r10
;;       ja      0x194
;;   15: pushq   %rbp
;;       movq    %rsp, %rbp
;;       subq    $0x20, %rsp
;;       movl    %edx, 8(%rsp)
;;       movl    %ecx, 0xc(%rsp)
;;       movq    %rdi, (%rsp)
;;       nopl    (%rax, %rax)
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x24, slot at FP-0x20, locals I32 @ slot+0x8, I32 @ slot+0xc, stack 
;;       ╰─╼ breakpoint patch: wasm PC 0x24, patch bytes [232, 200, 1, 0, 0]
;;       movl    %edx, 0x10(%rsp)
;;       nopl    (%rax, %rax)
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x26, slot at FP-0x20, locals I32 @ slot+0x8, I32 @ slot+0xc, stack I32 @ slot+0x10
;;       ╰─╼ breakpoint patch: wasm PC 0x26, patch bytes [232, 191, 1, 0, 0]
;;       movl    %ecx, 0x14(%rsp)
;;       nopl    (%rax, %rax)
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x28, slot at FP-0x20, locals I32 @ slot+0x8, I32 @ slot+0xc, stack I32 @ slot+0x10, I32 @ slot+0x14
;;       ╰─╼ breakpoint patch: wasm PC 0x28, patch bytes [232, 182, 1, 0, 0]
;;       leal    (%rdx, %rcx), %eax
;;       movl    %eax, 0x10(%rsp)
;;       nopl    (%rax, %rax)
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x29, slot at FP-0x20, locals I32 @ slot+0x8, I32 @ slot+0xc, stack I32 @ slot+0x10
;;       ╰─╼ breakpoint patch: wasm PC 0x29, patch bytes [232, 170, 1, 0, 0]
;;       movl    %eax, 0x10(%rsp)
;;       addq    $0x20, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
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
;;       cmpq    $2, %rcx
;;       jb      0x124
;;   84: movl    (%rdx), %r8d
;;       movl    0x10(%rdx), %ecx
;;       movq    %rdx, (%rsp)
;;       movq    %rbp, %r9
;;       movq    8(%rdi), %r10
;;       movq    %r9, 0x40(%r10)
;;       movq    %rsp, %r9
;;       movq    %r9, 0x38(%r10)
;;       leaq    0x3e(%rip), %r9
;;       movq    %r9, 0x48(%r10)
;;       movq    %r8, %rdx
;;       movq    %r10, 8(%rsp)
;;       callq   0
;;       ├─╼ exception frame offset: SP = FP - 0x40
;;       ╰─╼ exception handler: default handler, no dynamic context, handler=0xe5
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
;;   e5: movq    8(%rsp), %r10
;;   ea: movq    $1, 0x80(%r10)
;;   f5: movq    $0, 0x90(%r10)
;;  100: xorl    %eax, %eax
;;  102: movq    0x10(%rsp), %rbx
;;  107: movq    0x18(%rsp), %r12
;;  10c: movq    0x20(%rsp), %r13
;;  111: movq    0x28(%rsp), %r14
;;  116: movq    0x30(%rsp), %r15
;;  11b: addq    $0x40, %rsp
;;  11f: movq    %rbp, %rsp
;;  122: popq    %rbp
;;  123: retq
;;  124: ud2
;;
;; signatures[0]::wasm_to_array_trampoline:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       subq    $0x30, %rsp
;;       movq    %rbx, 0x20(%rsp)
;;       movq    %rdx, %r8
;;       movq    %rbp, %rdx
;;       movq    8(%rsi), %rax
;;       movq    %rdx, 0x28(%rax)
;;       movq    %rbp, %rdx
;;       movq    8(%rdx), %rdx
;;       movq    %rdx, 0x30(%rax)
;;       leaq    (%rsp), %rdx
;;       movq    %r8, %rax
;;       movl    %eax, (%rsp)
;;       movl    %ecx, 0x10(%rsp)
;;       movq    8(%rdi), %r8
;;       movl    $2, %ecx
;;       movq    %rsi, %rbx
;;       callq   *%r8
;;       testb   %al, %al
;;       je      0x182
;;  171: movl    (%rsp), %eax
;;       movq    0x20(%rsp), %rbx
;;       addq    $0x30, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;  182: movq    0x10(%rbx), %rax
;;  186: movq    0x140(%rax), %rax
;;  18d: movq    %rbx, %rdi
;;  190: callq   *%rax
;;  192: ud2
;;
;; wasmtime_builtin_stack_overflow:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       subq    $0x10, %rsp
;;       movq    %r12, (%rsp)
;;       movq    %r13, 8(%rsp)
;;       movq    %rbp, %r10
;;       movq    8(%rdi), %r9
;;       movq    %r10, 0x28(%r9)
;;       movq    %rbp, %r10
;;       movq    8(%r10), %r11
;;       movq    %r11, 0x30(%r9)
;;       movq    0x10(%rdi), %r12
;;       movq    %rdi, %r13
;;       movq    0x148(%r12), %r11
;;       callq   *%r11
;;       testb   %al, %al
;;       je      0x1e7
;;  1d5: movq    (%rsp), %r12
;;       movq    8(%rsp), %r13
;;       addq    $0x10, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;  1e7: movq    0x140(%r12), %rax
;;  1ef: movq    %r13, %rdi
;;  1f2: callq   *%rax
;;  1f4: ud2
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
;;       movq    %r10, 0x28(%r9)
;;       movq    %rbp, %r10
;;       movq    8(%r10), %r11
;;       movq    %r11, 0x30(%r9)
;;       movq    0x10(%rdi), %r12
;;       movq    %rdi, %r13
;;       movq    0x170(%r12), %r11
;;       callq   *%r11
;;       testb   %al, %al
;;       je      0x3cd
;;  2f9: movq    (%rsp), %rax
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
;;  3cd: movq    0x140(%r12), %rax
;;  3d5: movq    %r13, %rdi
;;  3d8: callq   *%rax
;;  3da: ud2
