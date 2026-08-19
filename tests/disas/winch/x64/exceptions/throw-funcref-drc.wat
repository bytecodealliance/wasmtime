;;! target = "x86_64"
;;! test = "winch"
;;! flags = "-W exceptions -C collector=drc"

;; A function reference is interned before it is stored in the exception.
(module
  (tag $e (param funcref))
  (func (param funcref)
    (throw $e (local.get 0))))
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x40, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x13a
;;   1c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movq    %rdx, 8(%rsp)
;;       movq    8(%rsp), %r11
;;       pushq   %r11
;;       subq    $8, %rsp
;;       movq    %r14, %rdi
;;       callq   0x216
;;       addq    $8, %rsp
;;       movq    0x20(%rsp), %r14
;;       movq    0x28(%r14), %rcx
;;       movl    4(%rcx), %ecx
;;       subq    $4, %rsp
;;       movl    %eax, (%rsp)
;;       subq    $4, %rsp
;;       movl    %ecx, (%rsp)
;;       movq    %r14, %rdi
;;       movl    $0x4000000, %esi
;;       movl    (%rsp), %edx
;;       movl    $0x28, %ecx
;;       movl    $8, %r8d
;;       callq   0x19a
;;       addq    $4, %rsp
;;       movq    0x24(%rsp), %r14
;;       movq    8(%r14), %rcx
;;       movq    0x28(%rcx), %rdx
;;       movq    0x20(%rcx), %rcx
;;       movq    %rcx, %rdx
;;       addq    %rax, %rdx
;;       movl    (%rsp), %ecx
;;       addq    $4, %rsp
;;       movl    %ecx, 0x18(%rdx)
;;       movl    $0, 0x1c(%rdx)
;;       popq    %rcx
;;       pushq   %rdx
;;       subq    $4, %rsp
;;       movl    %eax, (%rsp)
;;       pushq   %rcx
;;       subq    $0xc, %rsp
;;       movq    %r14, %rdi
;;       movq    0xc(%rsp), %rsi
;;       callq   0x1e9
;;       addq    $0xc, %rsp
;;       addq    $8, %rsp
;;       movq    0x24(%rsp), %r14
;;       movl    (%rsp), %ecx
;;       addq    $4, %rsp
;;       popq    %rdx
;;       movl    %eax, 0x20(%rdx)
;;       subq    $4, %rsp
;;       movl    %ecx, (%rsp)
;;       subq    $0xc, %rsp
;;       movq    %r14, %rdi
;;       movl    0xc(%rsp), %esi
;;       callq   0x243
;;       addq    $0xc, %rsp
;;       addq    $4, %rsp
;;       movq    0x18(%rsp), %r14
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;  13a: ud2
