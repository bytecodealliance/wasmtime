;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param f64) (result i32)
        (local.get 0)
        (i32.trunc_f64_u)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x18, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x86
;;   1b: movq    %rdi, %r14
;;       subq    $0x18, %rsp
;;       movq    %rdi, 0x10(%rsp)
;;       movq    %rsi, 8(%rsp)
;;       movsd   %xmm0, (%rsp)
;;       movsd   (%rsp), %xmm1
;;       movabsq $0x41e0000000000000, %r11
;;       movq    %r11, %xmm15
;;       ucomisd %xmm15, %xmm1
;;       jae     0x65
;;       jp      0x88
;;   56: cvttsd2si %xmm1, %eax
;;       cmpl    $0, %eax
;;       jge     0x80
;;   63: ud2
;;       movaps  %xmm1, %xmm0
;;       subsd   %xmm15, %xmm0
;;       cvttsd2si %xmm0, %eax
;;       cmpl    $0, %eax
;;       jl      0x8a
;;   7a: addl    $0x80000000, %eax
;;       addq    $0x18, %rsp
;;       popq    %rbp
;;       retq
;;   86: ud2
;;   88: ud2
;;   8a: ud2
