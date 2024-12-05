;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
        (i64.const 2)
        (i64.const 3)
        (i64.ge_s)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x46
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movq    $2, %rax
;;       cmpq    $3, %rax
;;       movl    $0, %eax
;;       setge   %al
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   46: ud2
