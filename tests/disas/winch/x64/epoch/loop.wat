;;! target = "x86_64"
;;! test = "winch"
;;! flags = "-Wepoch-interruption=y"

(module
  (func (export "run")
        (loop $l
              (br $l))))
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x80
;;   1b: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movq    0x20(%r14), %rdx
;;       movq    (%rdx), %rdx
;;       movq    8(%r14), %rcx
;;       movq    0x10(%rcx), %rcx
;;       cmpq    %rcx, %rdx
;;       jb      0x50
;;   43: movq    %r14, %rdi
;;       callq   0x18e
;;       movq    8(%rsp), %r14
;;       movq    0x20(%r14), %rdx
;;       movq    (%rdx), %rdx
;;       movq    8(%r14), %rcx
;;       movq    0x10(%rcx), %rcx
;;       cmpq    %rcx, %rdx
;;       jb      0x75
;;   68: movq    %r14, %rdi
;;       callq   0x18e
;;       movq    8(%rsp), %r14
;;       jmp     0x50
;;   7a: addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   80: ud2
