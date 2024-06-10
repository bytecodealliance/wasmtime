;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param f64) (result i64)
        (local.get 0)
        (i64.trunc_f64_s)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x18, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x7f
;;   1b: movq    %rdi, %r14
;;       subq    $0x18, %rsp
;;       movq    %rdi, 0x10(%rsp)
;;       movq    %rsi, 8(%rsp)
;;       movsd   %xmm0, (%rsp)
;;       movsd   (%rsp), %xmm0
;;       cvttsd2si %xmm0, %rax
;;       cmpq    $1, %rax
;;       jno     0x79
;;   45: ucomisd %xmm0, %xmm0
;;       jp      0x81
;;   4f: movabsq $14114281232179134464, %r11
;;       movq    %r11, %xmm15
;;       ucomisd %xmm15, %xmm0
;;       jb      0x83
;;   69: xorpd   %xmm15, %xmm15
;;       ucomisd %xmm0, %xmm15
;;       jb      0x85
;;   79: addq    $0x18, %rsp
;;       popq    %rbp
;;       retq
;;   7f: ud2
;;   81: ud2
;;   83: ud2
;;   85: ud2
