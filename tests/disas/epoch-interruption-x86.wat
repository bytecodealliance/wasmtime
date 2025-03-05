;;! target = "x86_64"
;;! test = "compile"
;;! flags = ["-Wepoch-interruption=y"]

(module (func (loop (br 0))))

;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r10
;;       movq    0x10(%r10), %r10
;;       addq    $0x30, %r10
;;       cmpq    %rsp, %r10
;;       ja      0x80
;;   19: subq    $0x20, %rsp
;;       movq    %rbx, (%rsp)
;;       movq    %r12, 8(%rsp)
;;       movq    %r13, 0x10(%rsp)
;;       movq    0x20(%rdi), %r12
;;       movq    (%r12), %r9
;;       movq    8(%rdi), %rbx
;;       movq    %rdi, %r13
;;       movq    8(%rbx), %rax
;;       cmpq    %rax, %r9
;;       jae     0x59
;;   47: movq    (%r12), %rdi
;;       cmpq    %rax, %rdi
;;       jae     0x66
;;       jmp     0x47
;;   59: movq    %r13, %rdi
;;       callq   0x107
;;       jmp     0x47
;;   66: movq    8(%rbx), %rax
;;       cmpq    %rax, %rdi
;;       jb      0x47
;;   73: movq    %r13, %rdi
;;       callq   0x107
;;       jmp     0x47
;;   80: ud2
