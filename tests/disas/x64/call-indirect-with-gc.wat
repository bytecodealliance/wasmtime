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
;;       addq    $0x30, %r10
;;       cmpq    %rsp, %r10
;;       ja      0xc7
;;   19: subq    $0x20, %rsp
;;       movq    %rbx, (%rsp)
;;       movq    %r12, 8(%rsp)
;;       movq    %r13, 0x10(%rsp)
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
;;       je      0xb5
;;   5b: movq    %r9, %rbx
;;       movl    0x10(%rax), %esi
;;       movq    %rax, %r13
;;       movq    0x28(%rbx), %rax
;;       movl    4(%rax), %edx
;;       cmpl    %edx, %esi
;;       sete    %al
;;       movzbl  %al, %eax
;;       cmpl    %edx, %esi
;;       je      0x83
;;   7b: movq    %rbx, %rdi
;;       callq   0x26e
;;       testl   %eax, %eax
;;       je      0xc9
;;   8b: movq    %r13, %rcx
;;       movq    8(%rcx), %rax
;;       movq    0x18(%rcx), %rdi
;;       movq    %r12, %rdx
;;       movq    %rbx, %rsi
;;       callq   *%rax
;;       movq    (%rsp), %rbx
;;       movq    8(%rsp), %r12
;;       movq    0x10(%rsp), %r13
;;       addq    $0x20, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   b5: xorl    %esi, %esi
;;   b7: movq    %r9, %rbx
;;   ba: movq    %rbx, %rdi
;;   bd: callq   0x243
;;   c2: jmp     0x5e
;;   c7: ud2
;;   c9: ud2
