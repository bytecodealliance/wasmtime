;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
        (i32.const 1)
        (i32.extend16_s)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x3a
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movl    $1, %eax
;;       movswl  %ax, %eax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   3a: ud2
