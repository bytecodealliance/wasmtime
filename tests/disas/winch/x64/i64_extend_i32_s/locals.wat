;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i64)
        (local i32)

        (local.get 0)
        (i64.extend_i32_s)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x42
;;   1b: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movq    $0, 8(%rsp)
;;       movl    0xc(%rsp), %eax
;;       movslq  %eax, %rax
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   42: ud2
