;;! target = "x86_64"
;;! flags = "-W function-references=n,gc=n"
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
;;       ja      0x97
;;   19: subq    $0x10, %rsp
;;       movq    %rbx, (%rsp)
;;       movq    %r12, 8(%rsp)
;;       movq    %rdx, %rbx
;;       movq    0x38(%rdi), %rax
;;       movq    0x30(%rdi), %r8
;;       xorq    %rsi, %rsi
;;       movl    %ecx, %edx
;;       leaq    (%r8, %rdx, 8), %r8
;;       cmpl    %eax, %ecx
;;       cmovaeq %rsi, %r8
;;       movq    (%r8), %rcx
;;       movq    %rcx, %rax
;;       andq    $0xfffffffffffffffe, %rax
;;       testq   %rcx, %rcx
;;       je      0x85
;;   53: movl    0x10(%rax), %ecx
;;       movq    0x28(%rdi), %rdx
;;       movq    %rdi, %rsi
;;       cmpl    4(%rdx), %ecx
;;       jne     0x99
;;   66: movq    8(%rax), %rcx
;;       movq    0x18(%rax), %rdi
;;       movq    %rbx, %rdx
;;       callq   *%rcx
;;       movq    (%rsp), %rbx
;;       movq    8(%rsp), %r12
;;       addq    $0x10, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   85: movq    %rdi, %r12
;;   88: xorl    %esi, %esi
;;   8a: callq   0x240
;;   8f: movq    %r12, %rdi
;;   92: jmp     0x53
;;   97: ud2
;;   99: ud2
