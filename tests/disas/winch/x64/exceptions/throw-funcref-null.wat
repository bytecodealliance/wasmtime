;;! target = "x86_64"
;;! test = "winch"
;;! flags = "-W exceptions -C collector=null"

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
;;       ja      0x1b4
;;   1c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movq    %rdx, 8(%rsp)
;;       movq    8(%rsp), %r11
;;       pushq   %r11
;;       subq    $8, %rsp
;;       movq    %r14, %rdi
;;       callq   0x28c
;;       addq    $8, %rsp
;;       movq    0x20(%rsp), %r14
;;       movq    0x20(%r14), %r11
;;       movl    (%r11), %ecx
;;       addl    $7, %ecx
;;       jb      0x1b6
;;   6a: andl    $0xfffffff8, %ecx
;;       movl    %ecx, %edx
;;       addl    $0x18, %edx
;;       jb      0x1b8
;;   7e: subq    $4, %rsp
;;       movl    %eax, (%rsp)
;;       subq    $4, %rsp
;;       movl    %ecx, (%rsp)
;;       subq    $4, %rsp
;;       movl    %edx, (%rsp)
;;       movl    (%rsp), %eax
;;       addq    $4, %rsp
;;       movq    8(%r14), %rcx
;;       movq    0x28(%rcx), %rdx
;;       movq    0x20(%rcx), %rcx
;;       cmpq    %rdx, %rax
;;       jbe     0xe6
;;   bb: subq    %rdx, %rax
;;       pushq   %rax
;;       subq    $8, %rsp
;;       movq    %r14, %rdi
;;       movq    8(%rsp), %rsi
;;       callq   0x218
;;       addq    $8, %rsp
;;       addq    $8, %rsp
;;       movq    0x28(%rsp), %r14
;;       movl    (%rsp), %eax
;;       addq    $4, %rsp
;;       movq    8(%r14), %rcx
;;       movq    0x28(%rcx), %rdx
;;       movq    0x20(%rcx), %rcx
;;       movq    %rcx, %rdx
;;       addq    %rax, %rdx
;;       movl    $0x4000018, (%rdx)
;;       movq    0x28(%r14), %r11
;;       movl    4(%r11), %r11d
;;       movl    %r11d, 4(%rdx)
;;       movq    0x20(%r14), %rcx
;;       movl    %eax, %r11d
;;       addl    $0x18, %r11d
;;       movl    %r11d, (%rcx)
;;       movl    (%rsp), %ecx
;;       addq    $4, %rsp
;;       movl    %ecx, 8(%rdx)
;;       movl    $0, 0xc(%rdx)
;;       popq    %rcx
;;       pushq   %rdx
;;       subq    $4, %rsp
;;       movl    %eax, (%rsp)
;;       pushq   %rcx
;;       subq    $0xc, %rsp
;;       movq    %r14, %rdi
;;       movq    0xc(%rsp), %rsi
;;       callq   0x25f
;;       addq    $0xc, %rsp
;;       addq    $8, %rsp
;;       movq    0x24(%rsp), %r14
;;       movl    (%rsp), %ecx
;;       addq    $4, %rsp
;;       popq    %rdx
;;       movl    %eax, 0x10(%rdx)
;;       subq    $4, %rsp
;;       movl    %ecx, (%rsp)
;;       subq    $0xc, %rsp
;;       movq    %r14, %rdi
;;       movl    0xc(%rsp), %esi
;;       callq   0x2b9
;;       addq    $0xc, %rsp
;;       addq    $4, %rsp
;;       movq    0x18(%rsp), %r14
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;  1b4: ud2
;;  1b6: ud2
;;  1b8: ud2
