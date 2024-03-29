;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param i64) (result i32)
        (local.get 0)
        (i32.wrap_i64)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x18, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x3c
;;   1b: movq    %rdi, %r14
;;       subq    $0x18, %rsp
;;       movq    %rdi, 0x10(%rsp)
;;       movq    %rsi, 8(%rsp)
;;       movq    %rdx, (%rsp)
;;       movq    (%rsp), %rax
;;       movl    %eax, %eax
;;       addq    $0x18, %rsp
;;       popq    %rbp
;;       retq
;;   3c: ud2
