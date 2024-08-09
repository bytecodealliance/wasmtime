;;! target = "x86_64"
;;! test = "compile"
;;! flags = ["-Wepoch-interruption=y"]

(module (func (loop (br 0))))

;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       subq    $0x20, %rsp
;;       movq    %rbx, (%rsp)
;;       movq    %r12, 8(%rsp)
;;       movq    %r13, 0x10(%rsp)
;;       movq    8(%rdi), %r12
;;       movq    0x20(%rdi), %rbx
;;       movq    %rdi, %r13
;;       movq    (%rbx), %r9
;;       movq    0x10(%r12), %rax
;;       cmpq    %rax, %r9
;;       jae     0x43
;;   32: movq    (%rbx), %rdi
;;       cmpq    %rax, %rdi
;;       jae     0x50
;;       jmp     0x32
;;   43: movq    %r13, %rdi
;;       callq   0xde
;;       jmp     0x32
;;   50: movq    0x10(%r12), %rax
;;       cmpq    %rax, %rdi
;;       jb      0x32
;;   5e: movq    %r13, %rdi
;;       callq   0xde
;;       jmp     0x32
