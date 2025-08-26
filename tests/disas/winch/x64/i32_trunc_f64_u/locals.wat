;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
        (local f64)  

        (local.get 0)
        (i32.trunc_f64_u)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x91
;;   1c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movq    $0, 8(%rsp)
;;       movsd   8(%rsp), %xmm1
;;       movabsq $0x41e0000000000000, %r11
;;       movq    %r11, %xmm15
;;       ucomisd %xmm15, %xmm1
;;       jae     0x6e
;;       jp      0x93
;;   5f: cvttsd2si %xmm1, %eax
;;       cmpl    $0, %eax
;;       jge     0x88
;;   6c: ud2
;;       movaps  %xmm1, %xmm0
;;       subsd   %xmm15, %xmm0
;;       cvttsd2si %xmm0, %eax
;;       cmpl    $0, %eax
;;       jl      0x95
;;   83: addl    $0x80000000, %eax
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   91: ud2
;;   93: ud2
;;   95: ud2
