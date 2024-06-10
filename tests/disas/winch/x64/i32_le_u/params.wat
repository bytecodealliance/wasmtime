;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param i32) (param i32) (result i32)
        (local.get 0)
        (local.get 1)
        (i32.le_u)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x18, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x4d
;;   1b: movq    %rdi, %r14
;;       subq    $0x18, %rsp
;;       movq    %rdi, 0x10(%rsp)
;;       movq    %rsi, 8(%rsp)
;;       movl    %edx, 4(%rsp)
;;       movl    %ecx, (%rsp)
;;       movl    (%rsp), %eax
;;       movl    4(%rsp), %ecx
;;       cmpl    %eax, %ecx
;;       movl    $0, %ecx
;;       setbe   %cl
;;       movl    %ecx, %eax
;;       addq    $0x18, %rsp
;;       popq    %rbp
;;       retq
;;   4d: ud2
