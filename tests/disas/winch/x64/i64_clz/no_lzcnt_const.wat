;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i64)
        (i64.const 1)
        (i64.clz)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x55
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movl    $1, %eax
;;       bsrq    %rax, %rax
;;       movq    $18446744073709551615, %r11
;;       cmoveq  %r11, %rax
;;       negq    %rax
;;       addq    $0x3f, %rax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   55: ud2
