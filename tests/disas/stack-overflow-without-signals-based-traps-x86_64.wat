;;! target = "x86_64-unknown-linux-gnu"
;;! test = "compile"
;;! flags = "-Osignals-based-traps=n"
;;! objdump = "--funcs all"

;; Without signals-based traps, the stack-limit check is done at the very
;; start of each function, before frame setup, and on overflow tail-calls the
;; `stack_overflow` builtin's trampoline.

(module
  (func $callee)
  (func (param i64 i64) (result i64)
    call $callee
    (i64.add (local.get 0) (local.get 1))
  )
)
;; wasm[0]::function[0]::callee:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[1]:
;;       movq    8(%rdi), %r10
;;       movq    0x10(%r10), %r10
;;       addq    $0x30, %r10
;;       cmpq    %rsp, %r10
;;       ja      0x13b
;;   35: pushq   %rbp
;;       movq    %rsp, %rbp
;;       subq    $0x10, %rsp
;;       movq    %r12, (%rsp)
;;       movq    %r13, 8(%rsp)
;;       movq    %rdi, %rsi
;;       movq    %rcx, %r12
;;       movq    %rdx, %r13
;;       callq   0
;;       movq    %r12, %rcx
;;       movq    %r13, %rdx
;;       leaq    (%rdx, %rcx), %rax
;;       movq    (%rsp), %r12
;;       movq    8(%rsp), %r13
;;       addq    $0x10, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; signatures[0]::wasm_to_array_trampoline:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       subq    $0x10, %rsp
;;       movq    %rbx, (%rsp)
;;       movq    %rbp, %rcx
;;       movq    8(%rsi), %rax
;;       movq    %rcx, 0x28(%rax)
;;       movq    %rbp, %rcx
;;       movq    8(%rcx), %rcx
;;       movq    %rcx, 0x30(%rax)
;;       movq    8(%rdi), %r8
;;       leaq    (%rsp), %rdx
;;       xorq    %rcx, %rcx
;;       movq    %rsi, %rbx
;;       callq   *%r8
;;       testb   %al, %al
;;       je      0xb8
;;   ab: movq    (%rsp), %rbx
;;       addq    $0x10, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   b8: movq    0x10(%rbx), %rax
;;   bc: movq    0x140(%rax), %rax
;;   c3: movq    %rbx, %rdi
;;   c6: callq   *%rax
;;   c8: ud2
;;
;; signatures[1]::wasm_to_array_trampoline:
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
;;       movq    %rax, (%rsp)
;;       movq    %rcx, 0x10(%rsp)
;;       movq    8(%rdi), %r8
;;       movl    $2, %ecx
;;       movq    %rsi, %rbx
;;       callq   *%r8
;;       testb   %al, %al
;;       je      0x129
;;  117: movq    (%rsp), %rax
;;       movq    0x20(%rsp), %rbx
;;       addq    $0x30, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;  129: movq    0x10(%rbx), %rax
;;  12d: movq    0x140(%rax), %rax
;;  134: movq    %rbx, %rdi
;;  137: callq   *%rax
;;  139: ud2
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
;;       je      0x18e
;;  17c: movq    (%rsp), %r12
;;       movq    8(%rsp), %r13
;;       addq    $0x10, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;  18e: movq    0x140(%r12), %rax
;;  196: movq    %r13, %rdi
;;  199: callq   *%rax
;;  19b: ud2
