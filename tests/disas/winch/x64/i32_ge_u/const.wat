;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
        (i32.const 2)
        (i32.const 3)
        (i32.ge_u)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x42
;;   1b: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movl    $2, %eax
;;       cmpl    $3, %eax
;;       movl    $0, %eax
;;       setae   %al
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   42: ud2
