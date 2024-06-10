;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i64)
        (i64.const 1)
        (i64.const 2)
        (i64.shr_s)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x3c
;;   1b: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movq    $1, %rax
;;       sarq    $2, %rax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   3c: ud2
