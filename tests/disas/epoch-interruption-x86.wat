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
;;       movq    8(%rdi), %r12
;;       movq    0x20(%rdi), %rbx
;;       movq    %rdi, %r13
;;       movq    (%rbx), %r9
;;       movq    8(%r12), %rax
;;       cmpq    %rax, %r9
;;       jae     0x58
;;   47: movq    (%rbx), %rdi
;;       cmpq    %rax, %rdi
;;       jae     0x65
;;       jmp     0x47
;;   58: movq    %r13, %rdi
;;       callq   0x107
;;       jmp     0x47
;;   65: movq    8(%r12), %rax
;;       cmpq    %rax, %rdi
;;       jb      0x47
;;   73: movq    %r13, %rdi
;;       callq   0x107
;;       jmp     0x47
;;   80: ud2
