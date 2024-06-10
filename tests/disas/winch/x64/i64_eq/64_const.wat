;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
        (i64.const 9223372036854775806)
        (i64.const 9223372036854775807)
        (i64.eq)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x51
;;   1b: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movabsq $0x7ffffffffffffffe, %rax
;;       movabsq $0x7fffffffffffffff, %r11
;;       cmpq    %r11, %rax
;;       movl    $0, %eax
;;       sete    %al
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   51: ud2
