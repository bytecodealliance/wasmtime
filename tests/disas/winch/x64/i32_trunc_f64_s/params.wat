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
;;       movq    (%r11), %r11
;;       addq    $0x18, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x7d
;;   1b: movq    %rdi, %r14
;;       subq    $0x18, %rsp
;;       movq    %rdi, 0x10(%rsp)
;;       movq    %rsi, 8(%rsp)
;;       movsd   %xmm0, (%rsp)
;;       movsd   (%rsp), %xmm0
;;       cvttsd2si %xmm0, %eax
;;       cmpl    $1, %eax
;;       jno     0x77
;;   43: ucomisd %xmm0, %xmm0
;;       jp      0x7f
;;   4d: movabsq $13970166044105375744, %r11
;;       movq    %r11, %xmm15
;;       ucomisd %xmm15, %xmm0
;;       jbe     0x81
;;   67: xorpd   %xmm15, %xmm15
;;       ucomisd %xmm0, %xmm15
;;       jb      0x83
;;   77: addq    $0x18, %rsp
;;       popq    %rbp
;;       retq
;;   7d: ud2
;;   7f: ud2
;;   81: ud2
;;   83: ud2
