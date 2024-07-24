;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param f64) (result i64)
        (local.get 0)
        (i64.trunc_f64_u)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x93
;;   1b: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movsd   %xmm0, 8(%rsp)
;;       movsd   8(%rsp), %xmm1
;;       movabsq $0x43e0000000000000, %r11
;;       movq    %r11, %xmm15
;;       ucomisd %xmm15, %xmm1
;;       jae     0x69
;;       jp      0x95
;;   58: cvttsd2si %xmm1, %rax
;;       cmpq    $0, %rax
;;       jge     0x8d
;;   67: ud2
;;       movaps  %xmm1, %xmm0
;;       subsd   %xmm15, %xmm0
;;       cvttsd2si %xmm0, %rax
;;       cmpq    $0, %rax
;;       jl      0x97
;;   80: movabsq $9223372036854775808, %r11
;;       addq    %r11, %rax
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   93: ud2
;;   95: ud2
;;   97: ud2
