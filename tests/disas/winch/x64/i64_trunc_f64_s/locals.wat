;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i64)
        (local f64)  

        (local.get 0)
        (i64.trunc_f64_s)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x85
;;   1c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movq    $0, 8(%rsp)
;;       movsd   8(%rsp), %xmm0
;;       cvttsd2si %xmm0, %rax
;;       cmpq    $1, %rax
;;       jno     0x7f
;;   4b: ucomisd %xmm0, %xmm0
;;       jp      0x87
;;   55: movabsq $14114281232179134464, %r11
;;       movq    %r11, %xmm15
;;       ucomisd %xmm15, %xmm0
;;       jb      0x89
;;   6f: xorpd   %xmm15, %xmm15
;;       ucomisd %xmm0, %xmm15
;;       jb      0x8b
;;   7f: addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   85: ud2
;;   87: ud2
;;   89: ud2
;;   8b: ud2
