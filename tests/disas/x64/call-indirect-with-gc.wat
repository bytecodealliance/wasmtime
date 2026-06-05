;;! target = "x86_64"
;;! flags = "-W function-references,gc"
;;! test = "compile"

(module
  (table (export "t") 0 100 funcref)
  (func (export "f") (param i32 i32) (result i32)
    (call_indirect (param i32) (result i32) (local.get 0) (local.get 1))
  )
)

;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r10
;;       movq    0x18(%r10), %r10
;;       addq    $0x20, %r10
;;       cmpq    %rsp, %r10
;;       ja      0x9d
;;   19: subq    $0x10, %rsp
;;       movq    %rbx, (%rsp)
;;       movq    %r12, 8(%rsp)
;;       movq    %rdx, %r12
;;       movq    0x38(%rdi), %rax
;;       movq    0x30(%rdi), %r8
;;       movq    %rdi, %r9
;;       xorq    %rsi, %rsi
;;       movl    %ecx, %edx
;;       leaq    (%r8, %rdx, 8), %rdi
;;       cmpl    %eax, %ecx
;;       cmovaeq %rsi, %rdi
;;       movq    (%rdi), %rcx
;;       movq    %rcx, %rax
;;       andq    $0xfffffffffffffffe, %rax
;;       testq   %rcx, %rcx
;;       je      0x8b
;;   56: movq    %r9, %rbx
;;       movl    0x10(%rax), %ecx
;;       movq    0x28(%rbx), %rdx
;;       cmpl    4(%rdx), %ecx
;;       jne     0x9f
;;   69: movq    8(%rax), %rcx
;;       movq    0x18(%rax), %rdi
;;       movq    %r12, %rdx
;;       movq    %rbx, %rsi
;;       callq   *%rcx
;;       movq    (%rsp), %rbx
;;       movq    8(%rsp), %r12
;;       addq    $0x10, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   8b: xorl    %esi, %esi
;;   8d: movq    %r9, %rbx
;;   90: movq    %rbx, %rdi
;;   93: callq   0x219
;;   98: jmp     0x59
;;   9d: ud2
;;   9f: ud2
