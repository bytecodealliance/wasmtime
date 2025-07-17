;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i64)
        (f64.const 1.0)
        (i64.trunc_f64_u)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x8d
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movsd   0x61(%rip), %xmm1
;;       movabsq $0x43e0000000000000, %r11
;;       movq    %r11, %xmm15
;;       ucomisd %xmm15, %xmm1
;;       jae     0x60
;;       jp      0x8f
;;   53: cvttsd2si %xmm1, %rax
;;       cmpq    $0, %rax
;;       jge     0x84
;;   5e: ud2
;;       movaps  %xmm1, %xmm0
;;       subsd   %xmm15, %xmm0
;;       cvttsd2si %xmm0, %rax
;;       cmpq    $0, %rax
;;       jl      0x91
;;   77: movabsq $9223372036854775808, %r11
;;       addq    %r11, %rax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   8d: ud2
;;   8f: ud2
;;   91: ud2
;;   93: addb    %al, (%rax)
;;   95: addb    %al, (%rax)
;;   97: addb    %al, (%rax)
;;   99: addb    %al, (%rax)
;;   9b: addb    %al, (%rax)
;;   9d: addb    %dh, %al
