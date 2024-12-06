;;! target = "x86_64"
;;! test = "winch"
;;! flags = "-Wfuel=1"
(module
  (func (export "run")))
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x5e
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movq    8(%r14), %rcx
;;       movq    (%rcx), %rcx
;;       cmpq    $0, %rcx
;;       jl      0x4a
;;   3d: movq    %r14, %rdi
;;       callq   0x16c
;;       movq    8(%rsp), %r14
;;       movq    8(%r14), %rax
;;       movq    (%rax), %r11
;;       addq    $1, %r11
;;       movq    %r11, (%rax)
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   5e: ud2
