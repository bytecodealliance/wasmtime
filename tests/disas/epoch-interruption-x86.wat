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
;;       ja      0x7e
;;   19: subq    $0x20, %rsp
;;       movq    %r13, (%rsp)
;;       movq    %r14, 8(%rsp)
;;       movq    %r15, 0x10(%rsp)
;;       movq    0x18(%rdi), %r14
;;       movq    (%r14), %rcx
;;       movq    8(%rdi), %r13
;;       movq    %rdi, %r15
;;       movq    8(%r13), %rax
;;       cmpq    %rax, %rcx
;;       jae     0x57
;;   46: movq    (%r14), %r11
;;       cmpq    %rax, %r11
;;       jae     0x64
;;       jmp     0x46
;;   57: movq    %r15, %rdi
;;       callq   0xf6
;;       jmp     0x46
;;   64: movq    8(%r13), %rax
;;       cmpq    %rax, %r11
;;       jb      0x46
;;   71: movq    %r15, %rdi
;;       callq   0xf6
;;       jmp     0x46
;;   7e: ud2
