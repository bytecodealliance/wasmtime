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
;;       movq    0x10(%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x50
;;   1c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movl    %edx, 0xc(%rsp)
;;       movl    %ecx, 8(%rsp)
;;       movl    8(%rsp), %eax
;;       movl    0xc(%rsp), %ecx
;;       cmpl    %eax, %ecx
;;       movl    $0, %ecx
;;       setbe   %cl
;;       movl    %ecx, %eax
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   50: ud2
