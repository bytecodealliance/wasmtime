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
;;       movq    0x10(%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x82
;;   1c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movsd   %xmm0, 8(%rsp)
;;       movsd   8(%rsp), %xmm0
;;       cvttsd2si %xmm0, %rax
;;       cmpq    $1, %rax
;;       jno     0x7c
;;   48: ucomisd %xmm0, %xmm0
;;       jp      0x84
;;   52: movabsq $14114281232179134464, %r11
;;       movq    %r11, %xmm15
;;       ucomisd %xmm15, %xmm0
;;       jb      0x86
;;   6c: xorpd   %xmm15, %xmm15
;;       ucomisd %xmm0, %xmm15
;;       jb      0x88
;;   7c: addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   82: ud2
;;   84: ud2
;;   86: ud2
;;   88: ud2
