;;! target = "x86_64"
;;! test = "compile"
;;! flags = ["-Wepoch-interruption=y"]

(module (func (loop (br 0))))

;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r10
;;       movq    (%r10), %r10
;;       addq    $0x30, %r10
;;       cmpq    %rsp, %r10
;;       ja      0x7f
;;   18: subq    $0x20, %rsp
;;       movq    %rbx, (%rsp)
;;       movq    %r12, 8(%rsp)
;;       movq    %r13, 0x10(%rsp)
;;       movq    8(%rdi), %r12
;;       movq    0x20(%rdi), %rbx
;;       movq    %rdi, %r13
;;       movq    (%rbx), %r9
;;       movq    0x10(%r12), %rax
;;       cmpq    %rax, %r9
;;       jae     0x57
;;   46: movq    (%rbx), %rdi
;;       cmpq    %rax, %rdi
;;       jae     0x64
;;       jmp     0x46
;;   57: movq    %r13, %rdi
;;       callq   0xdf
;;       jmp     0x46
;;   64: movq    0x10(%r12), %rax
;;       cmpq    %rax, %rdi
;;       jb      0x46
;;   72: movq    %r13, %rdi
;;       callq   0xdf
;;       jmp     0x46
;;   7f: ud2
