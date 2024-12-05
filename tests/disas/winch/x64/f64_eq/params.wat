;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param f64) (param f64) (result i32)
        (local.get 0)
        (local.get 1)
        (f64.eq)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x63
;;   1c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movsd   %xmm0, 8(%rsp)
;;       movsd   %xmm1, (%rsp)
;;       movsd   (%rsp), %xmm0
;;       movsd   8(%rsp), %xmm1
;;       ucomisd %xmm0, %xmm1
;;       movl    $0, %eax
;;       sete    %al
;;       movl    $0, %r11d
;;       setnp   %r11b
;;       andq    %r11, %rax
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   63: ud2
