;;! target = "x86_64"
;;! test = "compile"
;;! flags = ["-Wepoch-interruption=y"]

(module (func (loop (br 0))))

;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       subq    $0x20, %rsp
;;       movq    %r13, (%rsp)
;;       movq    %r14, 8(%rsp)
;;       movq    %r15, 0x10(%rsp)
;;       movq    0x20(%rdi), %r14
;;       movq    (%r14), %rcx
;;       movq    8(%rdi), %r13
;;       movq    %rdi, %r15
;;       movq    8(%r13), %rax
;;       cmpq    %rax, %rcx
;;       jae     0x42
;;   31: movq    (%r14), %r11
;;       cmpq    %rax, %r11
;;       jae     0x4f
;;       jmp     0x31
;;   42: movq    %r15, %rdi
;;       callq   0xee
;;       jmp     0x31
;;   4f: movq    8(%r13), %rax
;;       cmpq    %rax, %r11
;;       jb      0x31
;;   5c: movq    %r15, %rdi
;;       callq   0xee
;;       jmp     0x31
