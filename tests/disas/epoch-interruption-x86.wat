;;! target = "x86_64"
;;! test = "compile"
;;! flags = ["-Wepoch-interruption=y"]

(module (func (loop (br 0))))

;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r10
;;       movq    0x18(%r10), %r10
;;       addq    $0x30, %r10
;;       cmpq    %rsp, %r10
;;       ja      0x90
;;   19: subq    $0x20, %rsp
;;       movq    %r12, (%rsp)
;;       movq    %r13, 8(%rsp)
;;       movq    %r14, 0x10(%rsp)
;;       movq    0x18(%rdi), %r13
;;       movq    (%r13), %rcx
;;       movq    8(%rdi), %r12
;;       movq    8(%r12), %rax
;;       cmpq    %rax, %rcx
;;       jae     0x57
;;   45: movq    (%r13), %rcx
;;       cmpq    %rax, %rcx
;;       jae     0x67
;;       jmp     0x45
;;   57: movq    %rdi, %r14
;;       callq   0xec
;;       movq    %r14, %rdi
;;       jmp     0x45
;;   67: movq    %rdi, %r14
;;       movq    8(%r12), %rax
;;       cmpq    %rax, %rcx
;;       jae     0x80
;;   78: movq    %r14, %rdi
;;       jmp     0x45
;;   80: movq    %r14, %rdi
;;       callq   0xec
;;       movq    %r14, %rdi
;;       jmp     0x45
;;   90: ud2
