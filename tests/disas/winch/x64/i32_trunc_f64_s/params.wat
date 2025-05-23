;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param f64) (result i32)
        (local.get 0)
        (i32.trunc_f64_s)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x86
;;   1c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movsd   %xmm0, 8(%rsp)
;;       movsd   8(%rsp), %xmm0
;;       cvttsd2si %xmm0, %eax
;;       cmpl    $1, %eax
;;       jno     0x7d
;;   49: ucomisd %xmm0, %xmm0
;;       jp      0x88
;;   53: movabsq $13970166044105375744, %r11
;;       movq    %r11, %xmm15
;;       ucomisd %xmm15, %xmm0
;;       jbe     0x8a
;;   6d: xorpd   %xmm15, %xmm15
;;       ucomisd %xmm0, %xmm15
;;       jb      0x8c
;;   7d: addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   86: ud2
;;   88: ud2
;;   8a: ud2
;;   8c: ud2
